use agent_runtime::PageContext;
use model_gateway::{
    ModelProviderAdapter, OpenAiCompatibleChatAdapter, ProviderCapabilities, UnifiedModelRequest,
};
use ordinconn_app::{
    AgentItemView, AgentReportView, AgentTaskStarted, AppSnapshot, ApprovalView, ExecutionView,
    MobileResearchBudget, MobileResearchTaskView, MobileRuntimeSettings, MobileWorkspaceData,
    ModelProviderConfig, ModelProviderView, WarehouseEntryView,
};
use serde::{Deserialize, Serialize};
use serde_json::to_value;
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
    workspace.adb_status = if state
        .mobile_host
        .adb_available_with_sdk(workspace.settings.android_sdk.as_deref())
    {
        "ready"
    } else {
        "missing"
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
    let capture = state
        .mobile_host
        .observe_with_sdk(
            current.settings.android_sdk.as_deref(),
            &current.settings.allowed_apps,
        )
        .map_err(IpcError::internal)?;
    state
        .runtime
        .record_mobile_capture(&capture)
        .await
        .map_err(IpcError::internal)?;
    let mut workspace = state
        .runtime
        .mobile_workspace_data()
        .await
        .map_err(IpcError::internal)?;
    workspace.runtime_status = "observing".into();
    workspace.adb_status = "ready".into();
    workspace.session = Some(capture.session);
    workspace.ui_snapshot = Some(capture.snapshot);
    workspace.frame = Some(capture.frame);
    Ok(workspace)
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
