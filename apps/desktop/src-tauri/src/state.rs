use crate::credential_store::CredentialStore;
use ordinconn_app::AppRuntime;
use std::sync::Arc;

pub struct AppState {
    pub runtime: Arc<AppRuntime>,
    pub credentials: Arc<dyn CredentialStore>,
}

impl AppState {
    pub fn new(runtime: Arc<AppRuntime>, credentials: Arc<dyn CredentialStore>) -> Self {
        Self {
            runtime,
            credentials,
        }
    }
}

pub fn redact_error(message: &str) -> String {
    let lower = message.to_ascii_lowercase();
    if lower.contains("api key") || lower.contains("token") || lower.contains("secret") {
        "Operation failed: [REDACTED]".into()
    } else {
        message.chars().take(300).collect()
    }
}
