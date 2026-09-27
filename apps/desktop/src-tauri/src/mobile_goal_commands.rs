//! Explicit typed mobile goal commands; orchestration remains in ordinconn-app.
use crate::{commands::IpcError, state::AppState};
use mobile_runtime::execution::{MobileGoal, MobileGoalBudget, MobileGoalId, MobileGoalPlan};
use mobile_runtime::planner::MobilePlannerOutcome;
use ordinconn_app::{AppError, AppRuntime};
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateMobileGoalInput {
    pub objective: String,
    pub budget: Option<MobileGoalBudget>,
    pub completion_target: Option<mobile_runtime::execution::MobileCompletionTarget>,
}
fn error(error: AppError) -> IpcError {
    IpcError::mobile_goal(error)
}
pub(crate) async fn create_goal_record(
    runtime: &AppRuntime,
    input: CreateMobileGoalInput,
) -> Result<MobileGoal, IpcError> {
    let goal = runtime
        .mobile_goal_repository()
        .create_goal(&input.objective, input.budget.unwrap_or_default())
        .await
        .map_err(error)?;
    if let Some(target) = input.completion_target {
        runtime
            .mobile_goal_repository()
            .set_completion_target(&goal.id, &target)
            .await
            .map_err(error)?;
    }
    Ok(goal)
}
#[tauri::command]
pub async fn create_mobile_goal(
    state: State<'_, AppState>,
    input: CreateMobileGoalInput,
) -> Result<MobileGoal, IpcError> {
    create_goal_record(state.runtime.as_ref(), input).await
}
#[tauri::command]
pub async fn get_mobile_goal(
    state: State<'_, AppState>,
    goal_id: MobileGoalId,
) -> Result<MobileGoal, IpcError> {
    state
        .runtime
        .mobile_goal_repository()
        .get_goal(&goal_id)
        .await
        .map_err(error)
}
#[tauri::command]
pub async fn list_mobile_goals(state: State<'_, AppState>) -> Result<Vec<MobileGoal>, IpcError> {
    state
        .runtime
        .mobile_goal_repository()
        .list_goals()
        .await
        .map_err(error)
}
#[tauri::command]
pub async fn get_mobile_goal_plan(
    state: State<'_, AppState>,
    goal_id: MobileGoalId,
) -> Result<Option<MobileGoalPlan>, IpcError> {
    state
        .runtime
        .mobile_goal_repository()
        .get_goal_plan(&goal_id)
        .await
        .map_err(error)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanMobileGoalInput {
    pub goal_id: MobileGoalId,
    pub observation_id: Option<String>,
}
pub(crate) async fn plan_goal_record(
    runtime: &AppRuntime,
    input: PlanMobileGoalInput,
    credentials: &dyn crate::credential_store::CredentialStore,
) -> Result<MobilePlannerOutcome, IpcError> {
    runtime
        .plan_mobile_goal(&input.goal_id, input.observation_id.as_deref(), |id| {
            credentials.get(id).map_err(|_| {
                mobile_runtime::execution::MobileGoalError::new(
                    mobile_runtime::execution::MobileGoalErrorCode::ModelError,
                )
                .into()
            })
        })
        .await
        .map_err(error)
}
#[tauri::command]
pub async fn plan_mobile_goal(
    state: State<'_, AppState>,
    input: PlanMobileGoalInput,
) -> Result<MobilePlannerOutcome, IpcError> {
    plan_goal_record(state.runtime.as_ref(), input, state.credentials.as_ref()).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecuteMobileGoalStepInput {
    pub goal_id: MobileGoalId,
}
#[tauri::command]
pub async fn execute_mobile_goal_step(
    state: State<'_, AppState>,
    input: ExecuteMobileGoalStepInput,
) -> Result<ordinconn_app::mobile_executor::MobileStepExecutionOutcome, IpcError> {
    let settings = state
        .runtime
        .mobile_workspace_data()
        .await
        .map_err(error)?
        .settings;
    let runtime = std::sync::Arc::new(crate::mobile_executor::DesktopMobileExecutorRuntime::new(
        state.mobile_host.clone(),
        settings.android_sdk,
        settings.allowed_apps,
    ));
    state
        .runtime
        .execute_mobile_goal_step(&input.goal_id, runtime)
        .await
        .map_err(error)
}

#[tauri::command]
pub async fn run_mobile_goal(
    state: State<'_, AppState>,
    input: ExecuteMobileGoalStepInput,
) -> Result<MobileGoal, IpcError> {
    let settings = state
        .runtime
        .mobile_workspace_data()
        .await
        .map_err(error)?
        .settings;
    let executor = std::sync::Arc::new(crate::mobile_executor::DesktopMobileExecutorRuntime::new(
        state.mobile_host.clone(),
        settings.android_sdk,
        settings.allowed_apps,
    ));
    state
        .runtime
        .run_mobile_goal(&input.goal_id, executor, |id| {
            state.credentials.get(id).map_err(|_| {
                mobile_runtime::execution::MobileGoalError::new(
                    mobile_runtime::execution::MobileGoalErrorCode::ModelError,
                )
                .into()
            })
        })
        .await
        .map_err(error)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApproveMobileStepInput {
    pub goal_id: MobileGoalId,
    pub step_id: mobile_runtime::execution::MobilePlanStepId,
    pub approval_id: String,
}
#[tauri::command]
pub async fn get_mobile_step_approval(
    state: State<'_, AppState>,
    step_id: mobile_runtime::execution::MobilePlanStepId,
) -> Result<Option<ordinconn_app::ApprovalView>, IpcError> {
    Ok(state
        .runtime
        .mobile_goal_repository()
        .get_step_approval(&step_id)
        .await
        .map_err(error)?
        .as_ref()
        .map(ordinconn_app::ApprovalView::from))
}
#[tauri::command]
pub async fn approve_mobile_goal_step(
    state: State<'_, AppState>,
    input: ApproveMobileStepInput,
) -> Result<MobileGoal, IpcError> {
    let settings = state
        .runtime
        .mobile_workspace_data()
        .await
        .map_err(error)?
        .settings;
    let executor = std::sync::Arc::new(crate::mobile_executor::DesktopMobileExecutorRuntime::new(
        state.mobile_host.clone(),
        settings.android_sdk,
        settings.allowed_apps,
    ));
    let step = state
        .runtime
        .approve_mobile_step(
            &input.goal_id,
            &input.step_id,
            &input.approval_id,
            executor.clone(),
        )
        .await
        .map_err(error)?;
    let goal = state
        .runtime
        .mobile_goal_repository()
        .get_goal(&input.goal_id)
        .await
        .map_err(error)?;
    if !step.verified || goal.status.is_terminal() {
        return Ok(goal);
    }
    state
        .runtime
        .run_mobile_goal(&input.goal_id, executor, |id| {
            state.credentials.get(id).map_err(|_| {
                mobile_runtime::execution::MobileGoalError::new(
                    mobile_runtime::execution::MobileGoalErrorCode::ModelError,
                )
                .into()
            })
        })
        .await
        .map_err(error)
}

#[cfg(test)]
mod tests {
    #[test]
    fn executor_ipc_accepts_only_goal_identity() {
        let id = MobileGoalId::new();
        assert!(
            serde_json::from_value::<super::ExecuteMobileGoalStepInput>(
                serde_json::json!({"goalId":id})
            )
            .is_ok()
        );
        for field in [
            "coordinates",
            "command",
            "adb",
            "action",
            "planId",
            "stepId",
            "inputText",
        ] {
            let mut value = serde_json::json!({"goalId":id});
            value[field] = serde_json::json!("forbidden");
            assert!(serde_json::from_value::<super::ExecuteMobileGoalStepInput>(value).is_err());
        }
        assert!(
            serde_json::from_value::<super::ExecuteMobileGoalStepInput>(serde_json::json!({}))
                .is_err()
        );
    }

    use super::*;
    #[tokio::test]
    async fn typed_ipc_creates_only_a_persisted_pending_goal() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = AppRuntime::initialize(&dir.path().join("ipc.sqlite3"))
            .await
            .unwrap();
        let input: CreateMobileGoalInput =
            serde_json::from_value(serde_json::json!({"objective":" original objective "}))
                .unwrap();
        let goal = create_goal_record(&runtime, input).await.unwrap();
        let wire = serde_json::to_value(&goal).unwrap();
        assert_eq!(wire["objective"], " original objective ");
        assert_eq!(wire["status"], "PENDING");
        assert_eq!(wire["stepBudget"]["maxSteps"], 8);
        assert_eq!(
            runtime
                .mobile_goal_repository()
                .get_goal(&goal.id)
                .await
                .unwrap()
                .objective,
            " original objective "
        );
        assert!(
            runtime
                .mobile_goal_repository()
                .get_goal_plan(&goal.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(runtime.snapshot().await.unwrap().providers.is_empty());
        assert!(
            runtime
                .mobile_workspace_data()
                .await
                .unwrap()
                .latest_action_receipt
                .is_none()
        );
    }
    #[tokio::test]
    async fn explicit_planner_ipc_returns_model_not_configured_without_mutating_goal() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = AppRuntime::initialize(&dir.path().join("planner-ipc.sqlite3"))
            .await
            .unwrap();
        let goal = create_goal_record(
            &runtime,
            CreateMobileGoalInput {
                objective: "Read public UI".into(),
                budget: None,
                completion_target: None,
            },
        )
        .await
        .unwrap();
        let e = plan_goal_record(
            &runtime,
            PlanMobileGoalInput {
                goal_id: goal.id.clone(),
                observation_id: None,
            },
            &crate::credential_store::MemoryCredentialStore::default(),
        )
        .await
        .unwrap_err();
        let wire = serde_json::to_value(e).unwrap();
        assert_eq!(wire["code"], "MODEL_NOT_CONFIGURED");
        assert_eq!(
            runtime
                .mobile_goal_repository()
                .get_goal(&goal.id)
                .await
                .unwrap()
                .status,
            mobile_runtime::execution::MobileGoalStatus::Pending
        );
        assert!(
            runtime
                .mobile_goal_repository()
                .get_goal_plan(&goal.id)
                .await
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn typed_ipc_rejects_actions_and_budget_unknown_fields() {
        assert!(
            serde_json::from_value::<CreateMobileGoalInput>(
                serde_json::json!({"objective":"read","execute":true})
            )
            .is_err()
        );
        assert!(serde_json::from_value::<CreateMobileGoalInput>(serde_json::json!({"objective":"read","budget":{"maxSteps":8,"maxRuntimeMs":120000,"maxConsecutiveFailures":2,"maxIdenticalObservations":3,"command":"adb"}})).is_err());
    }
}
