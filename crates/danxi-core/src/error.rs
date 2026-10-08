use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum AppError {
    #[error("network request failed: {0}")]
    Network(String),
    #[error("application configuration is invalid: {0}")]
    Configuration(String),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("request validation failed: {0}")]
    Validation(String),
    #[error("enhanced authentication required: {0}")]
    EnhancedAuth(String),
    #[error("upstream service returned an error: {0}")]
    Upstream(String),
    #[error("secure credential storage failed: {0}")]
    Storage(String),
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        let message = if error.is_timeout() {
            "请求超时"
        } else if error.is_connect() {
            "无法连接上游服务"
        } else if error.is_redirect() {
            "上游服务重定向失败"
        } else if error.is_decode() {
            "上游响应读取失败"
        } else {
            "网络请求失败"
        };
        Self::Network(message.to_owned())
    }
}

impl AppError {
    /// Build an [AppError::Upstream] from an HTTP status without echoing the
    /// response body, which may contain upstream diagnostics or credentials.
    pub fn upstream_status(status: reqwest::StatusCode) -> Self {
        Self::Upstream(format!("HTTP {}", status.as_u16()))
    }
}
