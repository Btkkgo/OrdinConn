use thiserror::Error;

#[cfg(test)]
use std::{collections::BTreeMap, fmt, sync::Mutex};

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("secure credential storage is unavailable")]
    Unavailable,
    #[error("credential identifier is invalid")]
    InvalidIdentifier,
}

pub trait CredentialStore: Send + Sync {
    fn set(&self, provider_id: &str, secret: &str) -> Result<(), CredentialError>;
    fn get(&self, provider_id: &str) -> Result<Option<String>, CredentialError>;
}

#[derive(Default)]
#[cfg(test)]
pub struct MemoryCredentialStore {
    values: Mutex<BTreeMap<String, String>>,
}

#[cfg(test)]
impl fmt::Debug for MemoryCredentialStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MemoryCredentialStore")
            .field("values", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
impl CredentialStore for MemoryCredentialStore {
    fn set(&self, provider_id: &str, secret: &str) -> Result<(), CredentialError> {
        if provider_id.trim().is_empty() {
            return Err(CredentialError::InvalidIdentifier);
        }
        self.values
            .lock()
            .map_err(|_| CredentialError::Unavailable)?
            .insert(provider_id.into(), secret.into());
        Ok(())
    }
    fn get(&self, provider_id: &str) -> Result<Option<String>, CredentialError> {
        Ok(self
            .values
            .lock()
            .map_err(|_| CredentialError::Unavailable)?
            .get(provider_id)
            .cloned())
    }
}

#[derive(Default)]
pub struct SystemCredentialStore;

impl CredentialStore for SystemCredentialStore {
    fn set(&self, provider_id: &str, secret: &str) -> Result<(), CredentialError> {
        if provider_id.trim().is_empty() {
            return Err(CredentialError::InvalidIdentifier);
        }
        keyring::Entry::new("ai.ordinconn.desktop.model-provider", provider_id)
            .map_err(|_| CredentialError::Unavailable)?
            .set_password(secret)
            .map_err(|_| CredentialError::Unavailable)
    }
    fn get(&self, provider_id: &str) -> Result<Option<String>, CredentialError> {
        let entry = keyring::Entry::new("ai.ordinconn.desktop.model-provider", provider_id)
            .map_err(|_| CredentialError::Unavailable)?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(CredentialError::Unavailable),
        }
    }
}
