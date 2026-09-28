use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};

use danxi_core::{
    AppError, CampusCredentials, ForumService, MemorySessionStore, SessionManager, SessionStore,
    TokenPair,
};

const SESSION_IDLE_TIMEOUT: Duration = Duration::from_secs(60 * 60 * 12);
const SESSION_COOKIE: &str = "danxi_session";

/// Server-side session registry backing the web gateway.
///
/// Each browser holds only an opaque `HttpOnly` cookie; community tokens stay
/// in this map and never reach the client.
pub struct SessionRegistry {
    sessions: RwLock<HashMap<String, Arc<WebSession>>>,
    forum: Arc<ForumService>,
}

impl SessionRegistry {
    pub fn new(forum: Arc<ForumService>) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            forum,
        }
    }

    /// Resolve a session id from the cookie header, dropping expired entries.
    pub fn resolve(&self, session_id: Option<&str>) -> Option<Arc<WebSession>> {
        let id = session_id?;
        let session = self.sessions.read().ok()?.get(id)?.clone();
        if session.expired() {
            if let Ok(mut sessions) = self.sessions.write()
                && sessions
                    .get(id)
                    .is_some_and(|current| Arc::ptr_eq(current, &session) && current.expired())
            {
                sessions.remove(id);
            }
            return None;
        }
        session.touch();
        Some(session)
    }

    /// Create a fresh session and return its id with the session handle.
    pub fn create(&self) -> Result<(String, Arc<WebSession>), AppError> {
        let id = generate_session_id()?;
        let store = Arc::new(WebTokenStore::default());
        let session = Arc::new(WebSession {
            manager: SessionManager::with_forum(store, self.forum.clone()),
            campus: tokio::sync::Mutex::new(None),
            last_used: RwLock::new(Instant::now()),
        });
        self.sessions
            .write()
            .map_err(|_| poisoned_lock())?
            .insert(id.clone(), session.clone());
        Ok((id, session))
    }

    /// Replace an authenticated browser's session id while preserving its
    /// server-side state, preventing a pre-login id from remaining valid.
    pub fn rotate(&self, previous_id: &str, session: Arc<WebSession>) -> Result<String, AppError> {
        let id = generate_session_id()?;
        let mut sessions = self.sessions.write().map_err(|_| poisoned_lock())?;
        if sessions
            .get(previous_id)
            .is_some_and(|current| Arc::ptr_eq(current, &session))
        {
            sessions.remove(previous_id);
        }
        sessions.insert(id.clone(), session);
        Ok(id)
    }

    pub fn anonymous_manager(&self) -> SessionManager {
        SessionManager::with_forum(Arc::new(MemorySessionStore::new()), self.forum.clone())
    }

    pub fn remove(&self, session_id: Option<&str>) {
        if let Some(id) = session_id {
            if let Ok(mut sessions) = self.sessions.write() {
                sessions.remove(id);
            }
        }
    }

    /// Drop sessions idle beyond [SESSION_IDLE_TIMEOUT].
    pub fn sweep(&self) {
        if let Ok(mut sessions) = self.sessions.write() {
            sessions.retain(|_, session| !session.expired());
        }
    }
}

/// Per-user session state bound to one browser: community token plus the
/// live campus (UIS) cookie session.
pub struct WebSession {
    manager: SessionManager,
    campus: tokio::sync::Mutex<Option<Arc<danxi_core::CampusSession>>>,
    last_used: RwLock<Instant>,
}

#[derive(Default)]
struct WebTokenStore {
    token: Mutex<Option<TokenPair>>,
}

impl SessionStore for WebTokenStore {
    fn load_token(&self) -> Result<Option<TokenPair>, AppError> {
        let token = self.token.lock().map_err(|_| poisoned_lock())?;
        Ok(token.clone())
    }

    fn save_token(&self, token: &TokenPair) -> Result<(), AppError> {
        let mut slot = self.token.lock().map_err(|_| poisoned_lock())?;
        *slot = Some(token.clone());
        Ok(())
    }

    fn clear_token(&self) -> Result<(), AppError> {
        let mut slot = self.token.lock().map_err(|_| poisoned_lock())?;
        *slot = None;
        Ok(())
    }
}

impl WebSession {
    pub fn manager(&self) -> &SessionManager {
        &self.manager
    }

    /// The per-session campus login, created on demand.
    pub async fn campus(&self) -> Arc<danxi_core::CampusSession> {
        let mut slot = self.campus.lock().await;
        if let Some(existing) = slot.as_ref() {
            return existing.clone();
        }
        // Credentials live only in this browser session's memory.
        let created = Arc::new(
            danxi_core::CampusSession::new(std::sync::Arc::new(MemoryCampusStore::default()))
                .expect("campus session is infallible"),
        );
        *slot = Some(created.clone());
        created
    }

    fn touch(&self) {
        if let Ok(mut last_used) = self.last_used.write() {
            *last_used = Instant::now();
        }
    }

    fn expired(&self) -> bool {
        self.last_used
            .read()
            .map(|last_used| last_used.elapsed() > SESSION_IDLE_TIMEOUT)
            .unwrap_or(true)
    }
}

pub fn session_cookie_header(session_id: &str, secure: bool) -> String {
    format!(
        "{SESSION_COOKIE}={session_id}; Path=/; HttpOnly; SameSite=Lax{}",
        secure_cookie_attribute(secure)
    )
}

pub fn clear_session_cookie_header(secure: bool) -> String {
    format!(
        "{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
        secure_cookie_attribute(secure)
    )
}

fn secure_cookie_attribute(secure: bool) -> &'static str {
    if secure { "; Secure" } else { "" }
}

pub fn session_id_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    let raw = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for pair in raw.split(';') {
        let mut parts = pair.trim().splitn(2, '=');
        if parts.next()? == SESSION_COOKIE {
            return parts.next().map(str::to_owned);
        }
    }
    None
}

/// 128 bits of randomness from the system CSPRNG, hex-encoded.
fn generate_session_id() -> Result<String, AppError> {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn poisoned_lock() -> AppError {
    AppError::Storage("web session store lock poisoned".to_owned())
}

/// In-memory campus credential holder for one browser session.
#[derive(Default)]
struct MemoryCampusStore {
    credentials: Mutex<Option<CampusCredentials>>,
}

impl danxi_core::CampusCredentialStore for MemoryCampusStore {
    fn load(&self) -> Result<Option<CampusCredentials>, AppError> {
        let credentials = self.credentials.lock().map_err(|_| poisoned_lock())?;
        Ok(credentials.clone())
    }

    fn save(&self, credentials: &CampusCredentials) -> Result<(), AppError> {
        let mut slot = self.credentials.lock().map_err(|_| poisoned_lock())?;
        *slot = Some(credentials.clone());
        Ok(())
    }

    fn clear(&self) -> Result<(), AppError> {
        let mut slot = self.credentials.lock().map_err(|_| poisoned_lock())?;
        *slot = None;
        Ok(())
    }
}
