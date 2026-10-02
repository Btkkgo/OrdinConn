use crate::credential_store::CredentialStore;
use ordinconn_app::AppRuntime;
use std::sync::Arc;

use crate::mobile::MobileHost;

pub struct AppState {
    pub runtime: Arc<AppRuntime>,
    pub credentials: Arc<dyn CredentialStore>,
    pub mobile_host: Arc<MobileHost>,
    pub manual_mobile: std::sync::Mutex<Option<Arc<std::sync::atomic::AtomicBool>>>,
}

impl AppState {
    pub fn new(
        runtime: Arc<AppRuntime>,
        credentials: Arc<dyn CredentialStore>,
        mobile_host: Arc<MobileHost>,
    ) -> Self {
        Self {
            runtime,
            credentials,
            mobile_host,
            manual_mobile: Default::default(),
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
