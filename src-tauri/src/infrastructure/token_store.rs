use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use danxi_core::{AppError, CampusCredentialStore, CampusCredentials, SessionStore, TokenPair};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    sync::RwLock,
};

const ENCRYPTED_FILE_NAME: &str = "secrets.enc.json";
const KEY_FILE_NAME: &str = "secrets.key";
const LEGACY_FILE_NAME: &str = "secrets.json";
const ENVELOPE_VERSION: u8 = 1;
const ENCRYPTION_AAD: &[u8] = b"danxi-next/secrets/v1";

/// Encrypted file storage chosen explicitly instead of the system keychain.
///
/// The secret payload and its random encryption key are stored in separate
/// app-data files with owner-only permissions on Unix. This protects secrets
/// from accidental plaintext disclosure while preserving unattended startup.
pub struct FileSecretStore {
    path: PathBuf,
    key: [u8; 32],
    secrets: RwLock<SecretFile>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct SecretFile {
    #[serde(default)]
    community_token: Option<TokenPair>,
    #[serde(default)]
    campus_credentials: Option<CampusCredentials>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptedEnvelope {
    version: u8,
    nonce: String,
    ciphertext: String,
}

struct SecretPaths {
    encrypted: PathBuf,
    key: PathBuf,
    legacy_plaintext: Vec<PathBuf>,
}

impl FileSecretStore {
    pub fn open() -> Result<Self, AppError> {
        Self::open_paths(secret_paths()?)
    }

    fn open_paths(paths: SecretPaths) -> Result<Self, AppError> {
        ensure_parent(&paths.encrypted)?;
        recover_backup(&paths.encrypted)?;
        if paths.encrypted.exists() {
            restrict_permissions(&paths.encrypted)?;
        }
        let key = load_or_create_key(&paths.key)?;
        let (secrets, migrated) = if paths.encrypted.exists() {
            (read_encrypted_file(&paths.encrypted, &key)?, false)
        } else if let Some(path) = paths.legacy_plaintext.iter().find(|path| path.exists()) {
            (read_plaintext_file(path)?, true)
        } else {
            (SecretFile::default(), false)
        };

        let store = Self {
            path: paths.encrypted.clone(),
            key,
            secrets: RwLock::new(secrets),
        };

        if migrated {
            let current = store.secrets.read().map_err(|_| poisoned())?.clone();
            write_encrypted_file(&store.path, &store.key, &current)?;
        }
        for legacy_path in paths.legacy_plaintext {
            if legacy_path.exists() {
                fs::remove_file(legacy_path)
                    .map_err(|error| AppError::Storage(error.to_string()))?;
            }
        }

        Ok(store)
    }

    fn update<F>(&self, mutate: F) -> Result<(), AppError>
    where
        F: FnOnce(&mut SecretFile),
    {
        let mut secrets = self.secrets.write().map_err(|_| poisoned())?;
        let mut updated = secrets.clone();
        mutate(&mut updated);
        write_encrypted_file(&self.path, &self.key, &updated)?;
        *secrets = updated;
        Ok(())
    }
}

impl SessionStore for FileSecretStore {
    fn load_token(&self) -> Result<Option<TokenPair>, AppError> {
        Ok(self
            .secrets
            .read()
            .map_err(|_| poisoned())?
            .community_token
            .clone())
    }

    fn save_token(&self, token: &TokenPair) -> Result<(), AppError> {
        self.update(|secrets| secrets.community_token = Some(token.clone()))
    }

    fn clear_token(&self) -> Result<(), AppError> {
        self.update(|secrets| secrets.community_token = None)
    }
}

impl CampusCredentialStore for FileSecretStore {
    fn load(&self) -> Result<Option<CampusCredentials>, AppError> {
        Ok(self
            .secrets
            .read()
            .map_err(|_| poisoned())?
            .campus_credentials
            .clone())
    }

    fn save(&self, credentials: &CampusCredentials) -> Result<(), AppError> {
        self.update(|secrets| secrets.campus_credentials = Some(credentials.clone()))
    }

    fn clear(&self) -> Result<(), AppError> {
        self.update(|secrets| secrets.campus_credentials = None)
    }
}

fn read_encrypted_file(path: &Path, key: &[u8; 32]) -> Result<SecretFile, AppError> {
    let raw = fs::read_to_string(path).map_err(|error| AppError::Storage(error.to_string()))?;
    let envelope: EncryptedEnvelope = serde_json::from_str(&raw)
        .map_err(|_| AppError::Storage("登录信息文件无法解析".to_owned()))?;
    if envelope.version != ENVELOPE_VERSION {
        return Err(AppError::Storage("登录信息文件版本不受支持".to_owned()));
    }
    let nonce = STANDARD
        .decode(envelope.nonce)
        .map_err(|_| AppError::Storage("登录信息文件无法解析".to_owned()))?;
    if nonce.len() != 12 {
        return Err(AppError::Storage("登录信息文件无法解析".to_owned()));
    }
    let ciphertext = STANDARD
        .decode(envelope.ciphertext)
        .map_err(|_| AppError::Storage("登录信息文件无法解析".to_owned()))?;
    let plaintext = cipher(key)?
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: ENCRYPTION_AAD,
            },
        )
        .map_err(|_| AppError::Storage("登录信息文件无法解密".to_owned()))?;
    serde_json::from_slice(&plaintext)
        .map_err(|_| AppError::Storage("登录信息文件无法解析".to_owned()))
}

fn read_plaintext_file(path: &Path) -> Result<SecretFile, AppError> {
    let raw = fs::read_to_string(path).map_err(|error| AppError::Storage(error.to_string()))?;
    serde_json::from_str(&raw).map_err(|_| AppError::Storage("登录信息文件无法解析".to_owned()))
}

fn write_encrypted_file(path: &Path, key: &[u8; 32], secrets: &SecretFile) -> Result<(), AppError> {
    let plaintext =
        serde_json::to_vec(secrets).map_err(|error| AppError::Storage(error.to_string()))?;
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher(key)?
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &plaintext,
                aad: ENCRYPTION_AAD,
            },
        )
        .map_err(|_| AppError::Storage("登录信息加密失败".to_owned()))?;
    let envelope = EncryptedEnvelope {
        version: ENVELOPE_VERSION,
        nonce: STANDARD.encode(nonce),
        ciphertext: STANDARD.encode(ciphertext),
    };
    let serialized = serde_json::to_vec_pretty(&envelope)
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&temporary, serialized).map_err(|error| AppError::Storage(error.to_string()))?;
    restrict_permissions(&temporary)?;
    replace_file(&temporary, path)?;
    restrict_permissions(path)?;
    Ok(())
}

fn replace_file(temporary: &Path, destination: &Path) -> Result<(), AppError> {
    let backup = destination.with_extension("bak");
    if backup.exists() {
        fs::remove_file(&backup).map_err(|error| AppError::Storage(error.to_string()))?;
    }
    if destination.exists() {
        fs::rename(destination, &backup).map_err(|error| AppError::Storage(error.to_string()))?;
    }
    if let Err(error) = fs::rename(temporary, destination) {
        if backup.exists() {
            let _ = fs::rename(&backup, destination);
        }
        return Err(AppError::Storage(error.to_string()));
    }
    if backup.exists() {
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

fn recover_backup(destination: &Path) -> Result<(), AppError> {
    let backup = destination.with_extension("bak");
    if !destination.exists() && backup.exists() {
        fs::rename(backup, destination).map_err(|error| AppError::Storage(error.to_string()))?;
    }
    Ok(())
}

fn cipher(key: &[u8; 32]) -> Result<Aes256Gcm, AppError> {
    Aes256Gcm::new_from_slice(key).map_err(|_| AppError::Storage("登录信息加密密钥无效".to_owned()))
}

fn load_or_create_key(path: &Path) -> Result<[u8; 32], AppError> {
    if path.exists() {
        restrict_permissions(path)?;
        return read_key(path);
    }

    ensure_parent(path)?;
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(mut file) => {
            file.write_all(&key)
                .map_err(|error| AppError::Storage(error.to_string()))?;
            file.sync_all()
                .map_err(|error| AppError::Storage(error.to_string()))?;
            restrict_permissions(path)?;
            Ok(key)
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            restrict_permissions(path)?;
            read_key(path)
        }
        Err(error) => Err(AppError::Storage(error.to_string())),
    }
}

fn read_key(path: &Path) -> Result<[u8; 32], AppError> {
    fs::read(path)
        .map_err(|error| AppError::Storage(error.to_string()))?
        .try_into()
        .map_err(|_| AppError::Storage("登录信息加密密钥无效".to_owned()))
}

fn secret_paths() -> Result<SecretPaths, AppError> {
    if let Ok(dir) = std::env::var("DANXI_DATA_DIR")
        && !dir.trim().is_empty()
    {
        return Ok(paths_for_base(PathBuf::from(dir), None));
    }

    #[cfg(target_os = "macos")]
    let root = std::env::var("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support"))
        .map_err(|_| AppError::Storage("无法定位用户目录".to_owned()))?;

    #[cfg(target_os = "windows")]
    let root = std::env::var("APPDATA")
        .map(PathBuf::from)
        .map_err(|_| AppError::Storage("无法定位用户目录".to_owned()))?;

    #[cfg(all(unix, not(target_os = "macos")))]
    let root = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map_err(|_| AppError::Storage("无法定位用户目录".to_owned()))?;

    Ok(paths_for_base(
        root.join("danxi-next"),
        Some(root.join("danxi-desktop")),
    ))
}

fn paths_for_base(base: PathBuf, legacy_base: Option<PathBuf>) -> SecretPaths {
    let mut legacy_plaintext = vec![base.join(LEGACY_FILE_NAME)];
    if let Some(legacy_base) = legacy_base {
        legacy_plaintext.push(legacy_base.join(LEGACY_FILE_NAME));
    }
    SecretPaths {
        encrypted: base.join(ENCRYPTED_FILE_NAME),
        key: base.join(KEY_FILE_NAME),
        legacy_plaintext,
    }
}

fn ensure_parent(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::Storage(error.to_string()))?;
        restrict_directory_permissions(parent)?;
    }
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<(), AppError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| AppError::Storage(error.to_string()))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<(), AppError> {
    Ok(())
}

#[cfg(unix)]
fn restrict_directory_permissions(path: &Path) -> Result<(), AppError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| AppError::Storage(error.to_string()))
}

#[cfg(not(unix))]
fn restrict_directory_permissions(_path: &Path) -> Result<(), AppError> {
    Ok(())
}

fn poisoned() -> AppError {
    AppError::Storage("secret store lock poisoned".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("danxi-next-{name}-{unique}"));
        fs::create_dir_all(&path).expect("temporary directory should be created");
        path
    }

    #[test]
    fn encrypts_and_restores_secrets_without_plaintext() {
        let directory = temporary_directory("encrypted-store");
        let paths = paths_for_base(directory.clone(), None);
        let encrypted_path = paths.encrypted.clone();
        let store = FileSecretStore::open_paths(paths).expect("store should open");
        store
            .save_token(&TokenPair {
                access: "private-access-token".to_owned(),
                refresh: "private-refresh-token".to_owned(),
            })
            .expect("token should save");
        store
            .save(&CampusCredentials {
                id: "23300000000".to_owned(),
                password: "private-campus-password".to_owned(),
                name: Some("张三".to_owned()),
                is_graduate: false,
            })
            .expect("campus credentials should save");
        drop(store);

        let encrypted = fs::read_to_string(&encrypted_path).expect("encrypted file should exist");
        assert!(!encrypted.contains("private-access-token"));
        assert!(!encrypted.contains("private-campus-password"));

        let reopened = FileSecretStore::open_paths(paths_for_base(directory.clone(), None))
            .expect("store should reopen");
        assert_eq!(
            reopened
                .load_token()
                .expect("token should load")
                .expect("token should exist")
                .access,
            "private-access-token"
        );
        assert_eq!(
            reopened
                .load()
                .expect("credentials should load")
                .expect("credentials should exist")
                .password,
            "private-campus-password"
        );

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }

    #[test]
    fn migrates_legacy_plaintext_file_once() {
        let directory = temporary_directory("plaintext-migration");
        let plaintext_path = directory.join(LEGACY_FILE_NAME);
        fs::write(
            &plaintext_path,
            r#"{"community_token":{"access":"old-access","refresh":"old-refresh"}}"#,
        )
        .expect("legacy file should be written");

        let paths = paths_for_base(directory.clone(), None);
        let encrypted_path = paths.encrypted.clone();
        let store = FileSecretStore::open_paths(paths).expect("legacy store should migrate");

        assert!(!plaintext_path.exists());
        assert!(encrypted_path.exists());
        assert_eq!(
            store
                .load_token()
                .expect("token should load")
                .expect("token should exist")
                .refresh,
            "old-refresh"
        );

        fs::remove_dir_all(directory).expect("temporary directory should be removed");
    }
}
