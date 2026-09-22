use agent_runtime::PageContext;
use chrono::Utc;
use mobile_runtime::{
    AndroidEnvironmentDiagnostics, MobileActionReceipt, MobileActionRequest, MobileActionTarget,
    MobileCapture, SensitiveText,
};
use model_gateway::{
    ModelProviderAdapter, OpenAiCompatibleChatAdapter, ProviderCapabilities, UnifiedModelRequest,
};
use ordinconn_app::{
    AgentItemView, AgentReportView, AgentTaskStarted, AppRuntime, AppSnapshot, ApprovalView,
    ExecutionView, MobileResearchBudget, MobileResearchTaskView, MobileRuntimeSettings,
    MobileWorkspaceData, ModelProviderConfig, ModelProviderView, WarehouseEntryView,
};
use serde::{Deserialize, Serialize};
use serde_json::to_value;
use std::{sync::Arc, time::Duration};
use tauri::State;
use uuid::Uuid;

use crate::state::{AppState, redact_error};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    code: String,
    message: String,
    retryable: bool,
}

impl IpcError {
    pub(crate) fn internal(error: impl std::fmt::Display) -> Self {
        Self {
            code: "internal_error".into(),
            message: redact_error(&error.to_string()),
            retryable: false,
        }
    }
}

#[tauri::command]
pub async fn get_mobile_workspace(
    state: State<'_, AppState>,
) -> Result<MobileWorkspaceData, IpcError> {
    let mut workspace = state
        .runtime
        .mobile_workspace_data()
        .await
        .map_err(IpcError::internal)?;
    let mobile_host = Arc::clone(&state.mobile_host);
    let configured_sdk = workspace.settings.android_sdk.clone();
    workspace.android_environment = tauri::async_runtime::spawn_blocking(move || {
        mobile_host.environment_diagnostics(configured_sdk.as_deref())
    })
    .await
    .map_err(IpcError::internal)?;
    workspace.adb_status = workspace.android_environment.adb_status.clone();
    workspace.runtime_status = if state.mobile_host.is_session_active() {
        "observing"
    } else {
        "disconnected"
    }
    .into();
    Ok(workspace)
}

#[tauri::command]
pub async fn observe_mobile_device(
    state: State<'_, AppState>,
) -> Result<MobileWorkspaceData, IpcError> {
    let current = state
        .runtime
        .mobile_workspace_data()
        .await
        .map_err(IpcError::internal)?;
    let mobile_host = Arc::clone(&state.mobile_host);
    let configured_sdk = current.settings.android_sdk.clone();
    let allowed_apps = current.settings.allowed_apps.clone();
    let (capture, environment) = tauri::async_runtime::spawn_blocking(move || {
        let capture = mobile_host.observe_with_sdk(configured_sdk.as_deref(), &allowed_apps)?;
        let environment = mobile_host.environment_diagnostics(configured_sdk.as_deref());
        Ok::<_, crate::mobile::MobileHostError>((capture, environment))
    })
    .await
    .map_err(IpcError::internal)?
    .map_err(IpcError::internal)?;
    record_mobile_capture_workspace(state.runtime.as_ref(), environment, capture).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileActionInput {
    session_id: String,
    snapshot_id: String,
    expected_package: String,
    target: MobileActionTarget,
    text: Option<SensitiveText>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileActionResult {
    receipt: MobileActionReceipt,
    workspace: MobileWorkspaceData,
}

#[tauri::command]
pub async fn execute_mobile_action(
    input: MobileActionInput,
    state: State<'_, AppState>,
) -> Result<MobileActionResult, IpcError> {
    let current = state
        .runtime
        .mobile_workspace_data()
        .await
        .map_err(IpcError::internal)?;
    let configured_sdk = current.settings.android_sdk.clone();
    let allowed_apps = current.settings.allowed_apps.clone();
    let mobile_host = Arc::clone(&state.mobile_host);
    let request = MobileActionRequest {
        action_id: format!("mobile_action_{}", Uuid::now_v7()),
        session_id: input.session_id,
        snapshot_id: input.snapshot_id,
        expected_package: input.expected_package,
        requested_at: Utc::now(),
        target: input.target,
        text: input.text,
    };
    state
        .runtime
        .record_mobile_action_intent(&request)
        .await
        .map_err(IpcError::internal)?;
    let (execution, environment, current_capture) =
        tauri::async_runtime::spawn_blocking(move || {
            let execution =
                mobile_host.execute_action(request, configured_sdk.as_deref(), &allowed_apps);
            let environment = mobile_host.environment_diagnostics(configured_sdk.as_deref());
            let current_capture = mobile_host.current_capture();
            (execution, environment, current_capture)
        })
        .await
        .map_err(IpcError::internal)?;
    if let Err(error) = state
        .runtime
        .record_mobile_action(&execution.receipt, execution.capture.as_ref())
        .await
    {
        let session_id = state.mobile_host.session_id();
        if state.mobile_host.stop_session() {
            let _ = state.runtime.end_mobile_session(&session_id).await;
        }
        return Err(IpcError::internal(error));
    }
    let mut workspace = state
        .runtime
        .mobile_workspace_data()
        .await
        .map_err(IpcError::internal)?;
    workspace.runtime_status = if state.mobile_host.is_session_active() {
        "observing"
    } else {
        "disconnected"
    }
    .into();
    workspace.adb_status = environment.adb_status.clone();
    workspace.android_environment = environment;
    if let Some(capture) = current_capture {
        workspace.session = Some(capture.session);
        workspace.ui_snapshot = Some(capture.snapshot);
        workspace.frame = Some(capture.frame);
    }
    Ok(MobileActionResult {
        receipt: execution.receipt,
        workspace,
    })
}

pub(crate) async fn record_mobile_capture_workspace(
    runtime: &AppRuntime,
    environment: AndroidEnvironmentDiagnostics,
    capture: MobileCapture,
) -> Result<MobileWorkspaceData, IpcError> {
    runtime
        .record_mobile_capture(&capture)
        .await
        .map_err(IpcError::internal)?;
    let mut workspace = runtime
        .mobile_workspace_data()
        .await
        .map_err(IpcError::internal)?;
    workspace.runtime_status = "observing".into();
    workspace.adb_status = environment.adb_status.clone();
    workspace.android_environment = environment;
    workspace.session = Some(capture.session);
    workspace.ui_snapshot = Some(capture.snapshot);
    workspace.frame = Some(capture.frame);
    Ok(workspace)
}

pub(crate) async fn stop_mobile_workspace(
    runtime: &AppRuntime,
    mobile_host: &crate::mobile::MobileHost,
) -> Result<MobileWorkspaceData, IpcError> {
    let session_id = mobile_host.session_id();
    if mobile_host.stop_session() {
        runtime
            .end_mobile_session(&session_id)
            .await
            .map_err(IpcError::internal)?;
    }
    let mut workspace = runtime
        .mobile_workspace_data()
        .await
        .map_err(IpcError::internal)?;
    workspace.runtime_status = "disconnected".into();
    workspace.session = None;
    workspace.ui_snapshot = None;
    workspace.frame = None;
    Ok(workspace)
}

#[tauri::command]
pub async fn stop_mobile_session(
    state: State<'_, AppState>,
) -> Result<MobileWorkspaceData, IpcError> {
    let configured_sdk = state
        .runtime
        .mobile_workspace_data()
        .await
        .map_err(IpcError::internal)?
        .settings
        .android_sdk;
    let mut workspace =
        stop_mobile_workspace(state.runtime.as_ref(), state.mobile_host.as_ref()).await?;
    let mobile_host = Arc::clone(&state.mobile_host);
    workspace.android_environment = tauri::async_runtime::spawn_blocking(move || {
        mobile_host.environment_diagnostics(configured_sdk.as_deref())
    })
    .await
    .map_err(IpcError::internal)?;
    workspace.adb_status = workspace.android_environment.adb_status.clone();
    Ok(workspace)
}

#[tauri::command]
pub async fn start_mobile_avd(
    name: String,
    state: State<'_, AppState>,
) -> Result<MobileWorkspaceData, IpcError> {
    let current = state
        .runtime
        .mobile_workspace_data()
        .await
        .map_err(IpcError::internal)?;
    let mobile_host = Arc::clone(&state.mobile_host);
    let configured_sdk = current.settings.android_sdk.clone();
    tauri::async_runtime::spawn_blocking(move || {
        mobile_host.start_avd(configured_sdk.as_deref(), &name, Duration::from_secs(120))
    })
    .await
    .map_err(IpcError::internal)?
    .map_err(IpcError::internal)?;
    get_mobile_workspace(state).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn set_warehouse_entry(
    item_id: String,
    favorite: bool,
    saved: bool,
    tags: Vec<String>,
    state: State<'_, AppState>,
) -> Result<WarehouseEntryView, IpcError> {
    state
        .runtime
        .set_warehouse_entry(&item_id, favorite, saved, tags)
        .await
        .map_err(IpcError::internal)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn create_mobile_research_task(
    query: String,
    allowed_apps: Vec<String>,
    budget: MobileResearchBudget,
    state: State<'_, AppState>,
) -> Result<MobileResearchTaskView, IpcError> {
    state
        .runtime
        .create_mobile_research_task(&query, allowed_apps, budget)
        .await
        .map_err(IpcError::internal)
}

#[tauri::command]
pub async fn save_mobile_settings(
    settings: MobileRuntimeSettings,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    state
        .runtime
        .save_mobile_settings(&settings)
        .await
        .map_err(IpcError::internal)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn set_strategy_enabled(
    strategy_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    state
        .runtime
        .set_strategy_enabled(&strategy_id, enabled)
        .await
        .map_err(IpcError::internal)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    pub id: Option<String>,
    pub name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model_id: String,
    pub temperature: f32,
    pub context_window: u32,
    pub enabled: bool,
    pub capabilities: ProviderCapabilities,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    pub ok: bool,
    pub message: String,
}

#[tauri::command]
pub async fn get_snapshot(state: State<'_, AppState>) -> Result<AppSnapshot, IpcError> {
    state.runtime.snapshot().await.map_err(IpcError::internal)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn start_agent_turn(
    thread_id: Option<String>,
    question: String,
    context: PageContext,
    state: State<'_, AppState>,
) -> Result<AgentTaskStarted, IpcError> {
    state
        .runtime
        .start_agent_turn(thread_id, &question, context)
        .await
        .map_err(IpcError::internal)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_thread_items(
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<AgentItemView>, IpcError> {
    state
        .runtime
        .thread_items(&thread_id)
        .await
        .map_err(IpcError::internal)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn cancel_agent_turn(
    turn_id: String,
    state: State<'_, AppState>,
) -> Result<bool, IpcError> {
    state
        .runtime
        .cancel_agent_turn(&turn_id)
        .await
        .map_err(IpcError::internal)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn create_report(
    signal_id: String,
    state: State<'_, AppState>,
) -> Result<AgentReportView, IpcError> {
    state
        .runtime
        .create_report(&signal_id)
        .await
        .map_err(IpcError::internal)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn create_trade_proposal(
    signal_id: String,
    state: State<'_, AppState>,
) -> Result<execution_core::TradeProposal, IpcError> {
    state
        .runtime
        .create_trade_proposal(&signal_id)
        .await
        .map_err(IpcError::internal)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn request_approval(
    proposal_id: String,
    state: State<'_, AppState>,
) -> Result<ApprovalView, IpcError> {
    state
        .runtime
        .request_approval(&proposal_id)
        .await
        .map_err(IpcError::internal)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn approve_and_execute_paper(
    approval_id: String,
    state: State<'_, AppState>,
) -> Result<ExecutionView, IpcError> {
    state
        .runtime
        .approve_and_execute_paper(&approval_id)
        .await
        .map_err(IpcError::internal)
}

#[tauri::command]
pub async fn save_model_provider(
    input: ProviderInput,
    state: State<'_, AppState>,
) -> Result<ModelProviderView, IpcError> {
    validate_provider(&input)?;
    let id = input
        .id
        .unwrap_or_else(|| format!("provider_{}", Uuid::now_v7()));
    let credential_ref = if let Some(key) = input.api_key.as_deref().filter(|key| !key.is_empty()) {
        state
            .credentials
            .set(&id, key)
            .map_err(IpcError::internal)?;
        Some(format!("keyring:{id}"))
    } else if state
        .credentials
        .get(&id)
        .map_err(IpcError::internal)?
        .is_some()
    {
        Some(format!("keyring:{id}"))
    } else {
        None
    };
    state
        .runtime
        .upsert_model_provider(ModelProviderConfig {
            id,
            name: input.name,
            provider_type: "openai_compatible_chat".into(),
            base_url: input.base_url,
            credential_ref,
            default_model: input.model_id,
            temperature: input.temperature,
            context_window: input.context_window,
            enabled: input.enabled,
            capabilities: to_value(input.capabilities).map_err(IpcError::internal)?,
        })
        .await
        .map_err(IpcError::internal)
}

#[tauri::command]
pub async fn test_model_provider(
    input: ProviderInput,
    state: State<'_, AppState>,
) -> Result<ConnectionTestResult, IpcError> {
    validate_provider(&input)?;
    let key = match input.api_key {
        Some(key) if !key.is_empty() => Some(key),
        _ => match &input.id {
            Some(id) => state.credentials.get(id).map_err(IpcError::internal)?,
            None => None,
        },
    };
    let adapter = OpenAiCompatibleChatAdapter::new(&input.base_url, input.capabilities);
    let mut request = UnifiedModelRequest::with_user_text("Reply with the single word OK.");
    request.model = input.model_id;
    request.temperature = input.temperature;
    match adapter.complete(&request, key.as_deref()).await {
        Ok(_) => Ok(ConnectionTestResult {
            ok: true,
            message: "Connection succeeded".into(),
        }),
        Err(error) => Ok(ConnectionTestResult {
            ok: false,
            message: redact_error(&error.to_string()),
        }),
    }
}

fn validate_provider(input: &ProviderInput) -> Result<(), IpcError> {
    if input.name.trim().is_empty()
        || input.model_id.trim().is_empty()
        || !(input.base_url.starts_with("https://") || input.base_url.starts_with("http://"))
        || !input.capabilities.chat_completions
        || !(0.0..=2.0).contains(&input.temperature)
        || input.context_window == 0
    {
        return Err(IpcError { code: "invalid_provider".into(), message: "Provider name, HTTP(S) base URL, model, and chatCompletions capability are required".into(), retryable: false });
    }
    Ok(())
}

#[cfg(test)]
mod mobile_action_ipc_tests {
    use super::MobileActionInput;

    #[test]
    fn mobile_action_ipc_rejects_raw_coordinates_commands_and_extra_fields() {
        let safe = serde_json::json!({
            "sessionId": "session-1", "snapshotId": "snapshot-1", "expectedPackage": "com.android.settings",
            "target": { "kind": "tap", "elementRef": "@e1" }
        });
        assert!(serde_json::from_value::<MobileActionInput>(safe.clone()).is_ok());
        let mut with_coordinates = safe.clone();
        with_coordinates["target"]["x"] = serde_json::json!(10);
        assert!(serde_json::from_value::<MobileActionInput>(with_coordinates).is_err());
        let mut with_shell = safe;
        with_shell["command"] = serde_json::json!("shell input tap 1 2");
        assert!(serde_json::from_value::<MobileActionInput>(with_shell).is_err());
    }
}
