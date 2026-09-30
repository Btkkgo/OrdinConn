//! Private, opt-in acceptance controller inside the signed application process.
//! No Tauri command, credential input, MobileHost or Executor is exposed here.
use crate::state::AppState;
use mobile_runtime::execution::{MobileGoalError, MobileGoalErrorCode, MobileGoalId};
use ordinconn_app::AppError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::Path, sync::Arc, time::Duration};
use tauri::Manager;

const SOURCE_GOAL: &str = "mobile_goal_01a0f199-9c53-7275-b7f0-f370fc19d723";

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Action {
    CredentialCheck,
    PlannerFive,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Control {
    process_id: u32,
    started_at: String,
    request_id: String,
    action: Action,
}

fn enabled(value: Option<&str>) -> bool {
    value == Some("1")
}
fn can_start_batch(credential_ready: bool, batch_started: bool) -> bool {
    credential_ready && !batch_started
}
fn compiled_retry_supported() -> bool {
    let error = model_gateway::ModelError::HttpRejected {
        status: 503,
        code: "503".into(),
        message: "[REDACTED]".into(),
        retry_after_ms: None,
    };
    let first = ordinconn_app::mobile_planner::planner_retry_delay(&error, 0);
    first.is_some_and(|d| (Duration::from_secs(1)..=Duration::from_millis(1250)).contains(&d))
        && ordinconn_app::mobile_planner::planner_retry_delay(&error, 2).is_some()
        && ordinconn_app::mobile_planner::planner_retry_delay(&error, 3).is_none()
}

fn private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    if !path.exists() {
        fs::DirBuilder::new().mode(0o700).create(path)?;
    }
    let m = fs::symlink_metadata(path)?;
    if !m.is_dir() || m.file_type().is_symlink() || m.uid() != unsafe { libc::geteuid() } {
        return Err(std::io::Error::other("unsafe acceptance directory"));
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

fn private_file(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let m = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    if !m.is_file()
        || m.file_type().is_symlink()
        || m.uid() != unsafe { libc::geteuid() }
        || m.permissions().mode() & 0o077 != 0
        || m.len() > 16_384
    {
        return Err(std::io::Error::other("unsafe acceptance control file"));
    }
    fs::read(path).map(Some)
}

fn write_status(dir: &Path, value: &serde_json::Value) -> std::io::Result<()> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let path = dir.join("status.json");
    // Verify the destination as well as the private directory before replacement.
    let _ = private_file(&path)?;
    let temp = dir.join(format!("status-{}.tmp", uuid::Uuid::now_v7()));
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)?;
    output.write_all(&serde_json::to_vec_pretty(value)?)?;
    output.sync_all()?;
    fs::rename(temp, path)
}

fn valid_control(control: &Control, pid: u32, started: &str) -> bool {
    control.process_id == pid
        && control.started_at == started
        && !control.request_id.is_empty()
        && control.request_id.len() <= 64
        && control
            .request_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn safe_error(error: &AppError) -> serde_json::Value {
    match error {
        AppError::MobileGoal(e) => json!(e.code),
        _ => json!("INTERNAL_ACCEPTANCE_ERROR"),
    }
}

pub(crate) fn start_if_enabled(
    handle: &tauri::AppHandle,
    state: &AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    if !enabled(
        std::env::var("ORDINCONN_INTERNAL_M3_ACCEPTANCE")
            .ok()
            .as_deref(),
    ) {
        return Ok(());
    }
    let cache = handle.path().app_cache_dir()?;
    fs::create_dir_all(&cache)?;
    let dir = cache.join("final-m3-acceptance");
    private_directory(&dir)?;
    let runtime = Arc::clone(&state.runtime);
    let credentials = Arc::clone(&state.credentials);
    let pid = std::process::id();
    let started = chrono::Utc::now().to_rfc3339();
    let source_goal: MobileGoalId = serde_json::from_value(json!(SOURCE_GOAL))?;
    let native = keyring::Entry::new("ai.ordinconn.desktop.model-provider", "backend-check")?;
    let native_backend = !native
        .get_credential()
        .is::<keyring::mock::MockCredential>();
    let mut status = json!({"processId":pid,"startedAt":started,"phase":"PREPARING","androidActionsExecuted":0,
        "bounded503RetryIncluded":compiled_retry_supported(),"nativeCredentialBackend":native_backend,
        "publicDiagnosticIpcAdded":false});
    if !native_backend || !compiled_retry_supported() {
        status["phase"] = json!("COMPILED_SUPPORT_NOT_READY");
        write_status(&dir, &status)?;
        return Ok(());
    }
    write_status(&dir, &status)?;
    tauri::async_runtime::spawn(async move {
        let mut session = match runtime
            .prepare_internal_planner_acceptance(&source_goal)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                status["phase"] = json!("NOT_READY");
                status["errorCode"] = safe_error(&e);
                let _ = write_status(&dir, &status);
                return;
            }
        };
        status["readiness"] = serde_json::to_value(session.readiness()).unwrap_or(json!(null));
        status["phase"] = json!("READY_FOR_CREDENTIAL_CHECK");
        if write_status(&dir, &status).is_err() {
            return;
        }
        let mut checked = false;
        let mut batch_started = false;
        let mut last_request = String::new();
        loop {
            tokio::time::sleep(Duration::from_millis(250)).await;
            let bytes = match private_file(&dir.join("control.json")) {
                Ok(Some(b)) => b,
                Ok(None) => continue,
                Err(_) => {
                    status["phase"] = json!("INVALID_PRIVATE_CONTROL");
                    let _ = write_status(&dir, &status);
                    return;
                }
            };
            let control: Control = match serde_json::from_slice(&bytes) {
                Ok(c) => c,
                Err(_) => {
                    status["phase"] = json!("INVALID_CONTROL");
                    let _ = write_status(&dir, &status);
                    return;
                }
            };
            if !valid_control(&control, pid, &started) || control.request_id == last_request {
                continue;
            }
            last_request = control.request_id;
            match control.action {
                Action::CredentialCheck => {
                    // This phase can open an OS prompt, but never sends a model request.
                    status["phase"] = json!("KEYCHAIN_CHECK_PENDING");
                    if write_status(&dir, &status).is_err() {
                        return;
                    }
                    let store = Arc::clone(&credentials);
                    let provider = session.readiness().provider_id.clone();
                    let result =
                        tauri::async_runtime::spawn_blocking(move || store.get(&provider)).await;
                    checked = matches!(result,Ok(Ok(Some(ref key))) if !key.is_empty());
                    drop(result);
                    status["phase"] = json!(if checked {
                        "KEYCHAIN_READY"
                    } else {
                        "KEYCHAIN_UNAVAILABLE"
                    });
                    if write_status(&dir, &status).is_err() {
                        return;
                    }
                }
                Action::PlannerFive => {
                    if !can_start_batch(checked, batch_started) {
                        status["phase"] = json!(if batch_started {
                            "BATCH_ALREADY_STARTED"
                        } else {
                            "CREDENTIAL_CHECK_REQUIRED"
                        });
                        let _ = write_status(&dir, &status);
                        continue;
                    }
                    batch_started = true;
                    status["phase"] = json!("PLANNER_FIVE_RUNNING");
                    if write_status(&dir, &status).is_err() {
                        return;
                    }
                    let store = Arc::clone(&credentials);
                    let result = runtime
                        .run_internal_planner_acceptance(&mut session, move |id| {
                            store.get(id).map_err(|_| {
                                MobileGoalError::new(MobileGoalErrorCode::ModelError).into()
                            })
                        })
                        .await;
                    match result {
                        Ok(report) => {
                            status["phase"] = json!("PLANNER_FIVE_FINISHED");
                            status["report"] = serde_json::to_value(report).unwrap_or(json!(null));
                        }
                        Err(e) => {
                            status["phase"] = json!("PLANNER_FIVE_ERROR");
                            status["errorCode"] = safe_error(&e);
                        }
                    }
                    if write_status(&dir, &status).is_err() {
                        return;
                    }
                }
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acceptance_is_off_by_default_and_controls_cannot_carry_credentials() {
        for value in [None, Some(""), Some("0"), Some("true")] {
            assert!(!enabled(value));
        }
        assert!(enabled(Some("1")));
        assert!(compiled_retry_supported());
        assert!(!can_start_batch(false, false));
        assert!(!can_start_batch(false, true));
        assert!(can_start_batch(true, false));
        assert!(!can_start_batch(true, true));
        let mut value = json!({"processId":1,"startedAt":"start","requestId":"request-1","action":"PLANNER_FIVE"});
        let control: Control = serde_json::from_value(value.clone()).unwrap();
        assert!(valid_control(&control, 1, "start"));
        assert!(!valid_control(&control, 2, "start"));
        assert!(!valid_control(&control, 1, "other-start"));
        for field in [
            "apiKey",
            "provider",
            "capabilities",
            "execute",
            "actionOverride",
        ] {
            value[field] = json!("forbidden");
            assert!(serde_json::from_value::<Control>(value.clone()).is_err());
            value.as_object_mut().unwrap().remove(field);
        }
    }
    #[test]
    fn acceptance_private_files_reject_symlinks_and_world_readable_controls() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("control.json");
        fs::write(&path, b"{}").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(private_file(&path).is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(private_file(&path).unwrap(), Some(b"{}".to_vec()));
        let link = dir.path().join("link.json");
        symlink(&path, &link).unwrap();
        assert!(private_file(&link).is_err());
    }
}
