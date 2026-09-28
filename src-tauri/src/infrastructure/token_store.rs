use danxi_core::{AppError, CampusCredentialStore, CampusCredentials, SessionStore, TokenPair};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::RwLock,
};

/// Plain-file secret storage (user's explicit choice over the system keychain).
///
/// One JSON file in the OS app-data directory holds both the community token
/// and the campus credentials; the file is written atomically with 0600
/// permissions on Unix. Old keychain entries from earlier builds are removed
/// best-effort on startup.
pub struct FileSecretStore {
    path: PathBuf,
    secrets: RwLock<SecretFile>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct SecretFile {
    #[serde(default)]
    community_token: Option<TokenPair>,
    #[serde(default)]
    campus_credentials: Option<CampusCredentials>,
}

impl FileSecretStore {
    pub fn open() -> Result<Self, AppError> {
        let path = secrets_path()?;
        let secrets = read_secret_file(&path)?;
        if path.exists() {
            restrict_permissions(&path);
        }
        let store = Self {
            path,
            secrets: RwLock::new(secrets),
        };
        store.migrate_from_keychain();
        Ok(store)
    }

    /// Remove keychain entries written by earlier builds; they are replaced
    /// by the file store and would otherwise linger forever.
    fn migrate_from_keychain(&self) {
        for account in ["community-forum", "campus-uis"] {
            if let Ok(entry) = keyring::Entry::new("io.github.danxi-dev.danxi.desktop", account) {
                let _ = entry.delete_credential();
            }
        }
    }

    fn update<F>(&self, mutate: F) -> Result<(), AppError>
    where
        F: FnOnce(&mut SecretFile),
    {
        let mut secrets = self.secrets.write().map_err(|_| poisoned())?;
        let mut updated = secrets.clone();
        mutate(&mut updated);
        let serialized = serde_json::to_string_pretty(&updated)
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let tmp = self.path.with_extension("tmp");
        fs::write(&tmp, serialized).map_err(|error| AppError::Storage(error.to_string()))?;
        restrict_permissions(&tmp);
        fs::rename(&tmp, &self.path).map_err(|error| AppError::Storage(error.to_string()))?;
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

fn read_secret_file(path: &Path) -> Result<SecretFile, AppError> {
    if !path.exists() {
        return Ok(SecretFile::default());
    }
    let raw = fs::read_to_string(path).map_err(|error| AppError::Storage(error.to_string()))?;
    serde_json::from_str(&raw).map_err(|_| AppError::Storage("登录信息文件无法解析".to_owned()))
}

fn secrets_path() -> Result<PathBuf, AppError> {
    if let Ok(dir) = std::env::var("DANXI_DATA_DIR") {
        if !dir.trim().is_empty() {
            let path = Path::new(&dir).join("secrets.json");
            ensure_parent(&path)?;
            return Ok(path);
        }
    }

    #[cfg(target_os = "macos")]
    let base = std::env::var("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support"))
        .map_err(|_| AppError::Storage("无法定位用户目录".to_owned()))?;

    #[cfg(target_os = "windows")]
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .map_err(|_| AppError::Storage("无法定位用户目录".to_owned()))?;

    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map_err(|_| AppError::Storage("无法定位用户目录".to_owned()))?;

    let path = base.join("danxi-next").join("secrets.json");
    let legacy_path = base.join("danxi-desktop").join("secrets.json");

    if !path.exists() && legacy_path.exists() {
        ensure_parent(&path)?;
        if fs::rename(&legacy_path, &path).is_err() {
            return Ok(legacy_path);
        }
    }

    ensure_parent(&path)?;
    Ok(path)
}

fn ensure_parent(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| AppError::Storage(error.to_string()))?;
    }
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

fn poisoned() -> AppError {
    AppError::Storage("secret store lock poisoned".to_owned())
}
