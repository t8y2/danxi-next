use aes::Aes128;
use cfb_mode::cipher::{AsyncStreamCipher, KeyIvInit};
use reqwest::Url;

pub const WEBVPN_HOST: &str = "webvpn.fudan.edu.cn";
pub const WEBVPN_LOGIN_URL: &str = "https://webvpn.fudan.edu.cn/login?cas_login=true";

const WEBVPN_KEY: &[u8; 16] = b"wrdvpnisthebest!";
const ALLOWED_HOSTS: &[&str] = &[
    "www.fduhole.com",
    "auth.fduhole.com",
    "danke.fduhole.com",
    "forum.fduhole.com",
    "image.fduhole.com",
    "yjsxk.fudan.edu.cn",
    "10.64.130.6",
];

type Aes128CfbEncryptor = cfb_mode::Encryptor<Aes128>;

pub fn supports(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
        && url
            .host_str()
            .is_some_and(|host| ALLOWED_HOSTS.contains(&host))
}

pub fn translate(url: &Url) -> Result<Option<Url>, crate::AppError> {
    if !supports(url) {
        return Ok(None);
    }

    let host = url
        .host_str()
        .ok_or_else(|| crate::AppError::Configuration("WebVPN 地址缺少主机名".to_owned()))?;
    let formatted_host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    let encrypted_host = encrypt_host(&formatted_host)?;
    let segment = match url.port() {
        Some(port) => format!("{}-{port}", url.scheme()),
        None => url.scheme().to_owned(),
    };
    let mut translated = format!(
        "https://{WEBVPN_HOST}/{segment}/{encrypted_host}{}",
        url.path()
    );
    if let Some(query) = url.query() {
        translated.push('?');
        translated.push_str(query);
    }
    if let Some(fragment) = url.fragment() {
        translated.push('#');
        translated.push_str(fragment);
    }

    Url::parse(&translated)
        .map(Some)
        .map_err(|_| crate::AppError::Configuration("WebVPN 地址转换失败".to_owned()))
}

pub fn is_login_redirect(url: &Url) -> bool {
    url.host_str() != Some(WEBVPN_HOST) || url.path().starts_with("/login")
}

fn encrypt_host(host: &str) -> Result<String, crate::AppError> {
    let original_length = host.len();
    let padded_length = original_length.div_ceil(16) * 16;
    let mut encrypted = vec![b'0'; padded_length];
    encrypted[..original_length].copy_from_slice(host.as_bytes());

    Aes128CfbEncryptor::new_from_slices(WEBVPN_KEY, WEBVPN_KEY)
        .map_err(|_| crate::AppError::Configuration("WebVPN 加密参数无效".to_owned()))?
        .encrypt(&mut encrypted);

    let mut encoded = hex_bytes(WEBVPN_KEY);
    encoded.push_str(&hex_bytes(&encrypted[..original_length]));
    Ok(encoded)
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypts_host_like_flutter_client() {
        assert_eq!(
            encrypt_host("auth.fduhole.com").expect("host encryption should work"),
            "77726476706e69737468656265737421f1e2559469366c45760785a9d6562c38"
        );
    }

    #[test]
    fn translates_allowed_url_with_query() {
        let original = Url::parse("https://forum.fduhole.com/api/holes?length=10").unwrap();
        let translated = translate(&original)
            .expect("translation should work")
            .expect("host is supported");
        assert_eq!(
            translated.as_str(),
            "https://webvpn.fudan.edu.cn/https/77726476706e69737468656265737421f6f853892a7e6e546b0086a09d1b203a46/api/holes?length=10"
        );
    }

    #[test]
    fn leaves_unlisted_hosts_direct() {
        let original = Url::parse("https://example.com/api").unwrap();
        assert!(translate(&original).unwrap().is_none());
    }
}
