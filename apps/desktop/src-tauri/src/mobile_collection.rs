use mobile_runtime::collection::*;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileInteractionInput {
    pub action_type: MobileActionType,
    pub action: Option<crate::commands::MobileActionInput>,
}
pub fn action_type(target: &mobile_runtime::MobileActionTarget) -> MobileActionType {
    match target {
        mobile_runtime::MobileActionTarget::Tap { .. } => MobileActionType::Tap,
        mobile_runtime::MobileActionTarget::Swipe { direction } => match direction {
            mobile_runtime::SwipeDirection::Up => MobileActionType::ScrollUp,
            mobile_runtime::SwipeDirection::Down => MobileActionType::ScrollDown,
            mobile_runtime::SwipeDirection::Left => MobileActionType::ScrollLeft,
            mobile_runtime::SwipeDirection::Right => MobileActionType::ScrollRight,
        },
        mobile_runtime::MobileActionTarget::Type { .. } => MobileActionType::InputText,
        mobile_runtime::MobileActionTarget::Back => MobileActionType::Back,
        mobile_runtime::MobileActionTarget::Home => MobileActionType::Home,
        mobile_runtime::MobileActionTarget::OpenApp { .. } => MobileActionType::OpenApp,
    }
}
fn sanitize_capture(capture: &mut mobile_runtime::MobileCapture) {
    let projection = observation_from_capture(capture, None);
    for (element, safe) in capture
        .snapshot
        .elements
        .iter_mut()
        .zip(&projection.elements)
    {
        element.text = if safe.redacted {
            Some("[REDACTED]".into())
        } else {
            safe.text.clone()
        };
        element.content_description = safe.content_description.clone();
        element.resource_id = safe.resource_id.clone();
        if safe.redacted {
            capture.snapshot.redactions.push(format!(
                "element:{}:REDACTED_SENSITIVE_ELEMENT",
                element.element_ref
            ));
        }
    }
    if !projection.redactions.is_empty() {
        capture.snapshot.sensitive_state =
            Some(mobile_runtime::VerificationResult::SensitiveFieldBlocked);
        capture.frame.data_url.clear();
        capture.frame.png_bytes.clear();
    }
    let id = capture.observation.id.clone();
    capture.observation = mobile_runtime::MobileObservation::from_snapshot(
        &capture.snapshot,
        &capture.frame,
        None,
        "generic-android",
        mobile_runtime::PrivacyClass::UserAllowed,
    );
    capture.observation.id = id;
}
use crate::{commands::IpcError, mobile::MobileHost, state::AppState};
use chrono::Utc;
fn host_error(error: crate::mobile::MobileHostError) -> InteractionError {
    match error {
        crate::mobile::MobileHostError::EmptyAllowlist
        | crate::mobile::MobileHostError::PackageNotAllowed(_) => InteractionError::PolicyBlocked,
        crate::mobile::MobileHostError::CommandFailed(ref message)
            if message.contains("timed out") =>
        {
            InteractionError::Timeout
        }
        crate::mobile::MobileHostError::FocusUnavailable => InteractionError::Timeout,
        crate::mobile::MobileHostError::InvalidUiTree(_)
        | crate::mobile::MobileHostError::InvalidScreenSize => InteractionError::ActionFailed,
        _ => InteractionError::DeviceDisconnected,
    }
}

use mobile_runtime::{
    MobileActionDecision, MobileActionRequest, MobileActionStatus, MobileActionTarget,
    MobileCapture,
};
use ordinconn_app::{AppRuntime, MobileWorkspaceData};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

struct DesktopManualDriver {
    runtime: Arc<AppRuntime>,
    host: Arc<MobileHost>,
    sdk: Option<String>,
    allowed: Vec<String>,
    input: Option<crate::commands::MobileActionInput>,
    selected_capture: Option<MobileCapture>,
    capture: Option<MobileCapture>,
    post_capture: Option<MobileCapture>,
    before_id: Option<String>,
    previous_id: Option<String>,
    token: Arc<AtomicBool>,
    started: Instant,
    action_id: String,
    receipt: Option<mobile_runtime::MobileActionReceipt>,
}
impl ManualInteractionDriver for DesktopManualDriver {
    fn cancelled(&self) -> bool {
        self.token.load(Ordering::Acquire)
    }
    fn elapsed_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }
    fn poll_interval(&mut self) {
        std::thread::sleep(std::time::Duration::from_millis(120));
    }
    fn observe(&mut self) -> Result<MobileObservation, InteractionError> {
        if self.cancelled() {
            return Err(InteractionError::Cancelled);
        }
        let mut capture = if let Some(post) = self.post_capture.take() {
            post
        } else {
            self.host
                .observe_with_sdk(self.sdk.as_deref(), &self.allowed)
                .map_err(host_error)?
        };
        sanitize_capture(&mut capture);
        let observation = observation_from_capture(&capture, self.previous_id.clone());
        tauri::async_runtime::block_on(self.runtime.collect_mobile_observation(&observation))
            .map_err(|_| InteractionError::PersistenceFailed)?;
        self.previous_id = Some(observation.id.clone());
        self.capture = Some(capture);
        Ok(observation)
    }
    fn checkpoint(&mut self, result: &MobileActionResult) -> Result<(), InteractionError> {
        self.before_id = result.before_observation_id.clone();
        let package = self
            .capture
            .as_ref()
            .map(|c| c.snapshot.package_name.as_str())
            .unwrap_or_default();
        tauri::async_runtime::block_on(self.runtime.record_mobile_interaction(result, package))
            .map_err(|_| InteractionError::PersistenceFailed)
    }
    fn act(&mut self) -> Result<(), InteractionError> {
        if self.cancelled() {
            return Err(InteractionError::Cancelled);
        }
        let input = self.input.as_ref().ok_or(InteractionError::PolicyBlocked)?;
        let before = self
            .capture
            .as_ref()
            .ok_or(InteractionError::DeviceDisconnected)?;
        let selected = self
            .selected_capture
            .as_ref()
            .ok_or(InteractionError::TargetChanged)?;
        if selected.session.session_id != input.session_id
            || selected.snapshot.snapshot_id != input.snapshot_id
            || selected.snapshot.package_name != input.expected_package
            || before.snapshot.package_name != input.expected_package
            || before.snapshot.activity != selected.snapshot.activity
            || before.session.device_id != selected.session.device_id
        {
            return Err(InteractionError::TargetChanged);
        }
        if !before.snapshot.redactions.is_empty()
            && !matches!(
                input.target,
                MobileActionTarget::Back | MobileActionTarget::Home
            )
        {
            return Err(InteractionError::PolicyBlocked);
        }
        if input
            .text
            .as_ref()
            .is_some_and(|v| prohibited_manual_input(v.as_str()))
        {
            return Err(InteractionError::PolicyBlocked);
        }
        let target = match &input.target {
            MobileActionTarget::Tap { element_ref } | MobileActionTarget::Type { element_ref } => {
                let source = selected
                    .snapshot
                    .elements
                    .iter()
                    .find(|e| &e.element_ref == element_ref)
                    .ok_or(InteractionError::TargetChanged)?;
                let element = mobile_runtime::executor::matching_element(source, &before.snapshot)
                    .ok_or(InteractionError::TargetChanged)?;
                if prohibited_manual_target(&format!(
                    "{} {} {}",
                    element.text.as_deref().unwrap_or_default(),
                    element.content_description.as_deref().unwrap_or_default(),
                    element.resource_id.as_deref().unwrap_or_default()
                )) || element.text.as_deref() == Some("[REDACTED]")
                {
                    return Err(InteractionError::PolicyBlocked);
                }
                if matches!(input.target, MobileActionTarget::Tap { .. }) {
                    MobileActionTarget::Tap {
                        element_ref: element.element_ref.clone(),
                    }
                } else {
                    MobileActionTarget::Type {
                        element_ref: element.element_ref.clone(),
                    }
                }
            }
            target => target.clone(),
        };
        let request = MobileActionRequest {
            action_id: self.action_id.clone(),
            session_id: before.session.session_id.clone(),
            snapshot_id: before.snapshot.snapshot_id.clone(),
            expected_package: before.snapshot.package_name.clone(),
            requested_at: Utc::now(),
            target,
            text: input.text.clone(),
        };
        if self.cancelled() {
            return Err(InteractionError::Cancelled);
        }
        let execution = self
            .host
            .execute_action(request, self.sdk.as_deref(), &self.allowed);
        let mut post = execution.capture;
        if let Some(c) = &mut post {
            sanitize_capture(c);
        }
        let receipt = execution.receipt;
        tauri::async_runtime::block_on(
            self.runtime
                .record_mobile_manual_receipt(&receipt, &before.session.device_id),
        )
        .map_err(|_| InteractionError::PersistenceFailed)?;
        let sent = receipt.command_sent
            && receipt.status == MobileActionStatus::Executed
            && receipt.decision == MobileActionDecision::Allowed;
        self.receipt = Some(receipt);
        self.post_capture = post;
        if sent {
            Ok(())
        } else {
            Err(InteractionError::ActionFailed)
        }
    }
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileInteractionResponse {
    pub result: MobileActionResult,
    pub workspace: MobileWorkspaceData,
}

#[tauri::command]
pub async fn interact_mobile_device(
    input: MobileInteractionInput,
    state: tauri::State<'_, AppState>,
) -> Result<MobileInteractionResponse, IpcError> {
    let mut current = state
        .runtime
        .mobile_workspace_data()
        .await
        .map_err(IpcError::internal)?;
    let mut capture = state.mobile_host.current_capture();
    if let Some(c) = &mut capture {
        sanitize_capture(c);
    }
    let device = capture
        .as_ref()
        .map(|c| c.session.device_id.clone())
        .unwrap_or_default();
    let mut result = MobileActionResult {
        action_id: format!("manual_action_{}", uuid::Uuid::now_v7()),
        action_type: input.action_type,
        started_at: Utc::now(),
        completed_at: None,
        status: "running".into(),
        device_id: device,
        before_observation_id: None,
        after_observation_id: None,
        error_code: None,
        error_message: None,
        triggered_by: "owner".into(),
    };
    if input.action_type == MobileActionType::Stop {
        if let Some(token) = state
            .manual_mobile
            .lock()
            .map_err(IpcError::internal)?
            .as_ref()
        {
            token.store(true, Ordering::Release);
        }
        let host = Arc::clone(&state.mobile_host);
        tauri::async_runtime::spawn_blocking(move || host.stop_session())
            .await
            .map_err(IpcError::internal)?;
        result.status = "completed".into();
        result.completed_at = Some(Utc::now());
        state
            .runtime
            .record_mobile_interaction(&result, "")
            .await
            .map_err(IpcError::internal)?;
        current = state
            .runtime
            .mobile_workspace_data()
            .await
            .map_err(IpcError::internal)?;
        current.runtime_status = "disconnected".into();
        return Ok(MobileInteractionResponse {
            result,
            workspace: current,
        });
    }
    let token = Arc::new(AtomicBool::new(false));
    let valid = match &input.action {
        Some(a) => action_type(&a.target) == input.action_type,
        None => input.action_type == MobileActionType::Observe,
    };
    let busy = {
        let mut active = state.manual_mobile.lock().map_err(IpcError::internal)?;
        if active.is_some() {
            true
        } else if valid {
            *active = Some(Arc::clone(&token));
            false
        } else {
            false
        }
    };
    if busy || !valid {
        result.status = "failed".into();
        result.error_code = Some(
            if busy {
                "MOBILE_BUSY"
            } else {
                "INVALID_ACTION"
            }
            .into(),
        );
        result.completed_at = Some(Utc::now());
        state
            .runtime
            .record_mobile_interaction(&result, "")
            .await
            .map_err(IpcError::internal)?;
        return Ok(MobileInteractionResponse {
            result,
            workspace: state
                .runtime
                .mobile_workspace_data()
                .await
                .map_err(IpcError::internal)?,
        });
    }
    let runtime = Arc::clone(&state.runtime);
    let host = Arc::clone(&state.mobile_host);
    let initial = runtime.record_mobile_interaction(&result, "").await;
    if let Err(e) = initial {
        *state.manual_mobile.lock().map_err(IpcError::internal)? = None;
        return Err(IpcError::internal(e));
    }
    let sdk = current.settings.android_sdk.clone();
    let allowed = current.settings.allowed_apps.clone();
    let previous = capture
        .as_ref()
        .and_then(|c| {
            current
                .collection
                .observations
                .iter()
                .find(|o| o.device_id == c.session.device_id)
        })
        .map(|o| o.id.clone());
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let lease = if let Some(c) = &capture {
            Some(
                runtime
                    .device_execution_leases()
                    .acquire(&c.session.device_id, None, &result.action_id)
                    .map_err(|_| InteractionError::PolicyBlocked),
            )
        } else {
            None
        };
        if lease.as_ref().is_some_and(|l| l.is_err()) {
            result.status = "failed".into();
            result.completed_at = Some(Utc::now());
            result.error_code = Some("DEVICE_BUSY".into());
            tauri::async_runtime::block_on(runtime.record_mobile_interaction(&result, ""))
                .map_err(|_| InteractionError::PersistenceFailed)?;
            return Ok((result, None, None));
        }
        let _lease = lease;
        let mut driver = DesktopManualDriver {
            runtime,
            host,
            sdk,
            allowed,
            input: input.action,
            selected_capture: capture,
            capture: None,
            post_capture: None,
            before_id: None,
            previous_id: previous,
            token,
            started: Instant::now(),
            action_id: result.action_id.clone(),
            receipt: None,
        };
        let result = run_manual_interaction(&mut driver, result);
        Ok::<_, InteractionError>((result, driver.capture, driver.receipt))
    })
    .await;
    *state.manual_mobile.lock().map_err(IpcError::internal)? = None;
    let (result, capture, receipt) = outcome
        .map_err(IpcError::internal)?
        .map_err(|e| IpcError::internal(e.code()))?;
    let mut workspace = state
        .runtime
        .mobile_workspace_data()
        .await
        .map_err(IpcError::internal)?;
    if let Some(c) = capture {
        workspace.runtime_status = if state.mobile_host.is_session_active() {
            "observing"
        } else {
            "disconnected"
        }
        .into();
        workspace.session = Some(c.session);
        workspace.ui_snapshot = Some(c.snapshot);
        if !c.frame.data_url.is_empty() {
            workspace.frame = Some(c.frame);
        }
    }
    if let Some(r) = receipt {
        workspace.latest_action_receipt = Some(r);
    }
    fill_environment(&mut workspace, Arc::clone(&state.mobile_host)).await?;
    Ok(MobileInteractionResponse { result, workspace })
}
#[tauri::command]
pub async fn extract_mobile_page(
    observation_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<MobileWorkspaceData, IpcError> {
    state
        .runtime
        .extract_mobile_observation(&observation_id)
        .await
        .map_err(IpcError::internal)?;
    let mut workspace = state
        .runtime
        .mobile_workspace_data()
        .await
        .map_err(IpcError::internal)?;
    if let Some(mut c) = state.mobile_host.current_capture() {
        sanitize_capture(&mut c);
        workspace.session = Some(c.session);
        workspace.ui_snapshot = Some(c.snapshot);
        workspace.runtime_status = if state.mobile_host.is_session_active() {
            "observing"
        } else {
            "disconnected"
        }
        .into();
        if !c.frame.data_url.is_empty() {
            workspace.frame = Some(c.frame);
        }
    }
    fill_environment(&mut workspace, Arc::clone(&state.mobile_host)).await?;
    Ok(workspace)
}
async fn fill_environment(
    workspace: &mut MobileWorkspaceData,
    host: Arc<MobileHost>,
) -> Result<(), IpcError> {
    let sdk = workspace.settings.android_sdk.clone();
    let environment =
        tauri::async_runtime::spawn_blocking(move || host.environment_diagnostics(sdk.as_deref()))
            .await
            .map_err(IpcError::internal)?;
    workspace.adb_status = environment.adb_status.clone();
    workspace.android_environment = environment;
    Ok(())
}
#[tauri::command]
pub async fn get_mobile_data_provenance(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<ordinconn_app::mobile_collection::MobileDataProvenance, IpcError> {
    state
        .runtime
        .mobile_data_provenance(&id)
        .await
        .map_err(IpcError::internal)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_redacts_pin_from_desktop_and_hides_sensitive_frame() {
        let (_dir, sdk, _, _, host) = crate::mobile::tests::mobile_action_fixture();
        let mut c = host
            .observe_with_sdk(sdk.to_str(), &["com.example.news".into()])
            .unwrap();
        c.snapshot.elements[1].resource_id = Some("id/pin".into());
        c.snapshot.elements[1].text = Some("123456".into());
        sanitize_capture(&mut c);
        let j = serde_json::to_string(&c).unwrap();
        assert!(!j.contains("123456"));
        assert!(c.frame.data_url.is_empty());
        assert!(c.snapshot.sensitive_state.is_some());
    }
    #[test]
    fn fixture_manual_loop_reuses_adb_host_and_persists_traceable_data() {
        let (_dir, sdk, _, log, host) = crate::mobile::tests::mobile_action_fixture();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let database = tempfile::tempdir().unwrap();
        let app = runtime
            .block_on(AppRuntime::initialize(
                &database.path().join("collection.sqlite"),
            ))
            .unwrap();
        runtime
            .block_on(
                app.save_mobile_settings(&ordinconn_app::MobileRuntimeSettings {
                    allowed_apps: vec!["com.example.news".into()],
                    ..Default::default()
                }),
            )
            .unwrap();
        let host = Arc::new(host);
        let capture = host
            .observe_with_sdk(sdk.to_str(), &["com.example.news".into()])
            .unwrap();
        let input = crate::commands::MobileActionInput {
            session_id: capture.session.session_id.clone(),
            snapshot_id: capture.snapshot.snapshot_id.clone(),
            expected_package: capture.snapshot.package_name.clone(),
            target: MobileActionTarget::Tap {
                element_ref: "@e1".into(),
            },
            text: None,
        };
        let result = MobileActionResult {
            action_id: "fixture-manual-loop".into(),
            action_type: MobileActionType::Tap,
            started_at: Utc::now(),
            completed_at: None,
            status: "running".into(),
            device_id: capture.session.device_id.clone(),
            before_observation_id: None,
            after_observation_id: None,
            error_code: None,
            error_message: None,
            triggered_by: "owner".into(),
        };
        let mut driver = DesktopManualDriver {
            runtime: Arc::clone(&app),
            host,
            sdk: Some(sdk.to_string_lossy().into_owned()),
            allowed: vec!["com.example.news".into()],
            input: Some(input),
            selected_capture: Some(capture),
            capture: None,
            post_capture: None,
            before_id: None,
            previous_id: None,
            token: Arc::new(AtomicBool::new(false)),
            started: Instant::now(),
            action_id: result.action_id.clone(),
            receipt: None,
        };
        let result = run_manual_interaction(&mut driver, result);
        assert_eq!(result.status, "completed", "{:?}", result.error_code);
        assert_eq!(std::fs::read_to_string(log).unwrap().lines().count(), 1);
        let workspace = runtime
            .block_on(
                app.extract_mobile_observation(result.after_observation_id.as_deref().unwrap()),
            )
            .unwrap();
        assert!(!workspace.data_objects.is_empty());
        let source = runtime
            .block_on(app.mobile_data_provenance(&workspace.data_objects[0].id))
            .unwrap();
        assert_eq!(source.observation.device_id, "emulator-5554");
        assert!(!source.extracted_data.is_empty());
        assert!(!source.object.provenance.element_ids.is_empty());
        assert_eq!(
            workspace.actions[0].before_observation_id,
            result.before_observation_id
        );
    }
}
