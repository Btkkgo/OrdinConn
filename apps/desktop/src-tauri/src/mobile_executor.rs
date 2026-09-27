//! Thin adapter: the production executor orchestrates the existing M2 MobileHost.
use crate::mobile::MobileHost;
use mobile_runtime::{MobileActionRequest, MobileCapture, execution::*};
use ordinconn_app::mobile_executor::{DeviceExecutionLease, ExecutorAction, MobileExecutorRuntime};
use std::sync::Arc;
pub(crate) struct DesktopMobileExecutorRuntime {
    host: Arc<MobileHost>,
    sdk: Option<String>,
    allowed: Vec<String>,
}
impl DesktopMobileExecutorRuntime {
    pub(crate) fn new(host: Arc<MobileHost>, sdk: Option<String>, allowed: Vec<String>) -> Self {
        Self { host, sdk, allowed }
    }
}
#[async_trait::async_trait]
impl MobileExecutorRuntime for DesktopMobileExecutorRuntime {
    fn current_capture(&self) -> Option<MobileCapture> {
        if self.host.is_session_active() {
            self.host.current_capture()
        } else {
            None
        }
    }
    async fn observe(&self) -> Result<MobileCapture, MobileGoalError> {
        let host = self.host.clone();
        let sdk = self.sdk.clone();
        let allowed = self.allowed.clone();
        tokio::task::spawn_blocking(move || host.observe_with_sdk(sdk.as_deref(), &allowed))
            .await
            .map_err(|_| MobileGoalError::new(MobileGoalErrorCode::ObserveFailed))?
            .map_err(|_error| {
                #[cfg(test)]
                eprintln!(
                    "ADB_OBSERVE_FAILED timeout={}",
                    _error.to_string().contains("timed out")
                );
                MobileGoalError::new(MobileGoalErrorCode::ObserveFailed)
            })
    }
    async fn action(
        &self,
        request: MobileActionRequest,
        lease: Arc<DeviceExecutionLease>,
        progress: ordinconn_app::mobile_executor::ExecutorProgressSender,
    ) -> Result<ExecutorAction, MobileGoalError> {
        let host = self.host.clone();
        let sdk = self.sdk.clone();
        let allowed = self.allowed.clone();
        tokio::task::spawn_blocking(move || {
            let _lease = lease;
            let result =
                host.execute_action_with_progress(request, sdk.as_deref(), &allowed, |stage| {
                    let (acknowledged, ack) = tokio::sync::oneshot::channel();
                    if progress
                        .send(ordinconn_app::mobile_executor::ExecutorProgress {
                            stage,
                            acknowledged,
                        })
                        .is_ok()
                    {
                        let _ = ack.blocking_recv();
                    }
                });
            ExecutorAction {
                receipt: result.receipt,
                capture: result.capture,
            }
        })
        .await
        .map_err(|_| MobileGoalError::new(MobileGoalErrorCode::ActionFailed))
    }
}
/// One cross-process advisory lock for every opt-in real AVD gate; synthetic tests remain parallel.
#[cfg(test)]
pub(crate) fn real_avd_gate_lock() -> Result<std::fs::File, String> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(std::env::temp_dir().join("ordinconn-real-avd.lock"))
        .map_err(|_| "BLOCKED: AVD lock unavailable".to_owned())?;
    file.try_lock()
        .map_err(|_| "BLOCKED: real AVD is owned by another gate".to_owned())?;
    Ok(file)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_gate_resource_lock_excludes_another_file_handle() {
        let _first = real_avd_gate_lock().unwrap();
        assert!(real_avd_gate_lock().is_err());
    }
}
