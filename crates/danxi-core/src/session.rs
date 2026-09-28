use std::sync::RwLock;

use crate::{AppError, TokenPair};

/// Persistence boundary for community credentials.
///
/// Desktop runtimes back this with the app's local credential file; the web
/// gateway keeps tokens in server-side sessions. Implementations must never
/// log the token.
pub trait SessionStore: Send + Sync {
    fn load_token(&self) -> Result<Option<TokenPair>, AppError>;
    fn save_token(&self, token: &TokenPair) -> Result<(), AppError>;
    fn clear_token(&self) -> Result<(), AppError>;
}

/// In-memory fallback store, mainly useful for tests and previews.
#[derive(Default)]
pub struct MemorySessionStore {
    token: RwLock<Option<TokenPair>>,
}

impl MemorySessionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SessionStore for MemorySessionStore {
    fn load_token(&self) -> Result<Option<TokenPair>, AppError> {
        let token = self.token.read().map_err(|_| poisoned_lock())?;
        Ok(token.clone())
    }

    fn save_token(&self, token: &TokenPair) -> Result<(), AppError> {
        let mut slot = self.token.write().map_err(|_| poisoned_lock())?;
        *slot = Some(token.clone());
        Ok(())
    }

    fn clear_token(&self) -> Result<(), AppError> {
        let mut slot = self.token.write().map_err(|_| poisoned_lock())?;
        *slot = None;
        Ok(())
    }
}

fn poisoned_lock() -> AppError {
    AppError::Storage("session store lock poisoned".to_owned())
}
