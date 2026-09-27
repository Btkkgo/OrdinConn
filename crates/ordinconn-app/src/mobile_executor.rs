//! One persisted step per explicit invocation. No planner calls, retries, approval consumption or loop.
use crate::{AppError, AppRuntime};
use chrono::{DateTime, Utc};
use mobile_runtime::{
    MobileActionReceipt, MobileActionRequest, MobileActionStatus, MobileCapture, execution::*,
    executor::*,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
#[derive(Clone, Default)]
pub struct DeviceExecutionLeases {
    owners: Arc<Mutex<HashMap<String, String>>>,
    idle: Arc<tokio::sync::Notify>,
}
/// RAII ownership persists through blocking host work; errors/panics drop the lease.
pub struct DeviceExecutionLease {
    registry: DeviceExecutionLeases,
    pub device_id: String,
    pub goal_id: Option<MobileGoalId>,
    pub executor_id: String,
    pub acquired_at: DateTime<Utc>,
}
impl DeviceExecutionLeases {
    pub async fn wait_idle(&self) {
        loop {
            let notified = self.idle.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self
                .owners
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .is_empty()
            {
                return;
            }
            notified.await;
        }
    }
    pub fn acquire(
        &self,
        device: &str,
        goal: Option<&MobileGoalId>,
        executor: &str,
    ) -> Result<Arc<DeviceExecutionLease>, AppError> {
        let mut owners = self.owners.lock().unwrap_or_else(|e| e.into_inner());
        if owners.contains_key(device) {
            return Err(MobileGoalError::new(MobileGoalErrorCode::InvalidStateTransition).into());
        }
        owners.insert(device.into(), executor.into());
        Ok(Arc::new(DeviceExecutionLease {
            registry: self.clone(),
            device_id: device.into(),
            goal_id: goal.cloned(),
            executor_id: executor.into(),
            acquired_at: Utc::now(),
        }))
    }
}
impl Drop for DeviceExecutionLease {
    fn drop(&mut self) {
        let mut owners = self
            .registry
            .owners
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if owners.get(&self.device_id) == Some(&self.executor_id) {
            owners.remove(&self.device_id);
            self.registry.idle.notify_waiters();
        }
    }
}
#[derive(Clone, Copy)]
pub enum ExecutorActionStage {
    ActionCompleted,
    ObserveAfterStarted,
}
pub struct ExecutorProgress {
    pub stage: ExecutorActionStage,
    pub acknowledged: tokio::sync::oneshot::Sender<()>,
}
pub type ExecutorProgressSender = tokio::sync::mpsc::UnboundedSender<ExecutorProgress>;
pub struct ExecutorAction {
    pub receipt: MobileActionReceipt,
    pub capture: Option<MobileCapture>,
}
#[async_trait::async_trait]
pub trait MobileExecutorRuntime: Send + Sync {
    fn current_capture(&self) -> Option<MobileCapture>;
    async fn observe(&self) -> Result<MobileCapture, MobileGoalError>;
    async fn action(
        &self,
        request: MobileActionRequest,
        lease: Arc<DeviceExecutionLease>,
        progress: ExecutorProgressSender,
    ) -> Result<ExecutorAction, MobileGoalError>;
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileStepExecutionOutcome {
    pub goal_id: MobileGoalId,
    pub plan_id: MobilePlanId,
    pub step_id: MobilePlanStepId,
    pub execution_id: Option<String>,
    pub status: MobileStepStatus,
    pub verified: bool,
    pub error: Option<MobileGoalError>,
    pub observation_before_id: Option<String>,
    pub observation_after_id: Option<String>,
    pub action_receipt_id: Option<String>,
    pub evidence_ids: Vec<String>,
    pub data_object_id: Option<String>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileExtractedObject {
    pub id: String,
    pub category: String,
    pub source_id: String,
    pub source_locator: String,
    pub observation_id: String,
    pub captured_at: DateTime<Utc>,
    pub evidence_id: String,
    pub facts: Vec<String>,
}
pub(crate) fn mobile_source_definition() -> collector_runtime::SourceDefinition {
    use collector_runtime::*;
    SourceDefinition {
        id: "android-settings-ui".into(),
        name: "Android Settings UI observation".into(),
        endpoint: "android://com.android.settings".into(),
        collector_kind: CollectorKind::Computer,
        classification: SourceClass::Company,
        capabilities: vec!["sanitized_accessibility_observation".into()],
        reliability_tier: evidence_core::ReliabilityTier::Tier4Unverified,
        poll: PollPolicy {
            interval_seconds: 60,
            timeout_seconds: 30,
            max_retries: 0,
        },
        rate_limit: RateLimitPolicy {
            requests: 20,
            per_seconds: 60,
        },
        auth: AuthRequirement::None,
        retention: RetentionPolicy {
            raw_days: 7,
            observation_days: 30,
            audit_days: None,
        },
        enabled: true,
    }
}
pub(crate) fn extract_object(
    before: &MobileCapture,
) -> Result<MobileExtractedObject, MobileGoalError> {
    if !valid_capture(before) || before.snapshot.package_name != "com.android.settings" {
        return Err(MobileGoalError::new(MobileGoalErrorCode::PolicyBlocked));
    }
    let mut facts = vec![];
    for e in &before.snapshot.elements {
        if e.class_name.ends_with("EditText") {
            continue;
        }
        for text in [e.text.as_deref(), e.content_description.as_deref()]
            .into_iter()
            .flatten()
        {
            if let Some(value) = mobile_runtime::planner::prompt_text(text)
                .filter(|s| !s.trim().is_empty() && s != "[REDACTED]")
            {
                if !facts.contains(&value) {
                    facts.push(value);
                }
            }
            if facts.len() >= 80 {
                break;
            }
        }
        if facts.len() >= 80 {
            break;
        }
    }
    if facts.is_empty() {
        return Err(MobileGoalError::new(MobileGoalErrorCode::VerifyFailed));
    }
    Ok(MobileExtractedObject {
        id: format!("mobile_object_{}", uuid::Uuid::now_v7()),
        category: "mobile_observation_object".into(),
        source_id: "android-settings-ui".into(),
        source_locator: before.observation.source_locator.clone(),
        observation_id: before.observation.id.clone(),
        captured_at: before.observation.observed_at,
        evidence_id: format!("evidence_{}", uuid::Uuid::now_v7()),
        facts,
    })
}
impl AppRuntime {
    pub fn device_execution_leases(&self) -> DeviceExecutionLeases {
        self.device_execution_leases.clone()
    }
    pub async fn execute_mobile_goal_step(
        self: &Arc<Self>,
        id: &MobileGoalId,
        runtime: Arc<dyn MobileExecutorRuntime>,
    ) -> Result<MobileStepExecutionOutcome, AppError> {
        MobileGoalExecutor {
            app: self.clone(),
            runtime,
        }
        .execute(id)
        .await
    }
}
pub struct MobileGoalExecutor {
    app: Arc<AppRuntime>,
    runtime: Arc<dyn MobileExecutorRuntime>,
}
impl MobileGoalExecutor {
    async fn execute(&self, id: &MobileGoalId) -> Result<MobileStepExecutionOutcome, AppError> {
        let cancellation = self.app.mobile_cancellation_token();
        let repo = self.app.mobile_goal_repository();
        let snapshot = repo.executor_snapshot(id).await?;
        if snapshot.step.risk == MobileStepRisk::ApprovalRequired && !snapshot.approval_authorized {
            repo.request_step_approval(&snapshot, &self.app.approval_engine)
                .await?;
            return repo.executor_wait_approval(&snapshot).await;
        }
        if snapshot.step.risk == MobileStepRisk::Forbidden {
            return Err(MobileGoalError::new(MobileGoalErrorCode::PolicyBlocked).into());
        }
        if self.app.mobile_is_stopped() {
            return Err(MobileGoalError::new(MobileGoalErrorCode::UserStopped).into());
        }
        let current = self
            .runtime
            .current_capture()
            .filter(valid_capture)
            .ok_or_else(|| MobileGoalError::new(MobileGoalErrorCode::DeviceDisconnected))?;
        let settings = self.app.load_mobile_settings().await?;
        if !settings
            .allowed_apps
            .contains(&current.snapshot.package_name)
        {
            return Err(MobileGoalError::new(MobileGoalErrorCode::PolicyBlocked).into());
        }
        let execution = format!("mobile_execution_{}", uuid::Uuid::now_v7());
        let lease = self.app.device_execution_leases.acquire(
            &current.session.device_id,
            Some(id),
            &execution,
        )?;
        let claimed = repo.executor_claim(&snapshot, &current, &execution).await?;
        repo.executor_event(
            &claimed,
            &execution,
            "mobile.executor_observe_before_started",
            None,
            None,
        )
        .await?;
        let before = match self.runtime.observe().await {
            Ok(c)
                if valid_capture(&c)
                    && c.session.device_id == current.session.device_id
                    && c.session.session_id == current.session.session_id =>
            {
                c
            }
            _ => {
                return repo
                    .executor_finish(
                        &claimed,
                        &execution,
                        None,
                        None,
                        None,
                        None,
                        Some(MobileGoalError::new(MobileGoalErrorCode::ObserveFailed)),
                        None,
                    )
                    .await;
            }
        };
        self.app.record_mobile_capture(&before).await?;
        repo.executor_bind_before(&claimed, &execution, &before)
            .await?;
        let mut dispatch_step = claimed.step.clone();
        if claimed.approval_authorized {
            dispatch_step.risk = if matches!(
                dispatch_step.step_type,
                MobileStepType::Observe | MobileStepType::Wait | MobileStepType::Extract
            ) {
                MobileStepRisk::ReadOnly
            } else {
                MobileStepRisk::Reversible
            };
        }
        let resolved = match resolve_semantic_step(
            &dispatch_step,
            &before,
            claimed.planned.as_ref(),
            &execution,
            &settings.allowed_apps,
        ) {
            Ok(s) => s,
            Err(e) => {
                return repo
                    .executor_finish(
                        &claimed,
                        &execution,
                        Some(&before),
                        None,
                        None,
                        None,
                        Some(e),
                        None,
                    )
                    .await;
            }
        };
        if self.app.mobile_is_stopped() {
            return repo
                .executor_finish(
                    &claimed,
                    &execution,
                    Some(&before),
                    None,
                    None,
                    None,
                    Some(MobileGoalError::new(MobileGoalErrorCode::UserStopped)),
                    None,
                )
                .await;
        }
        if claimed
            .goal
            .runtime_deadline
            .is_some_and(|d| d <= Utc::now())
        {
            return repo
                .executor_finish(
                    &claimed,
                    &execution,
                    Some(&before),
                    None,
                    None,
                    None,
                    Some(MobileGoalError::new(MobileGoalErrorCode::TimeLimitReached)),
                    None,
                )
                .await;
        }
        let mut action_id = None;
        let mut extract = None;
        if let Err(e) = repo.executor_validate_dispatch(&claimed, &before).await {
            let code = match e {
                AppError::MobileGoal(e) => e.code,
                _ => MobileGoalErrorCode::InvalidStep,
            };
            return repo
                .executor_finish(
                    &claimed,
                    &execution,
                    Some(&before),
                    None,
                    None,
                    None,
                    Some(MobileGoalError::new(code)),
                    None,
                )
                .await;
        }
        let after = if let Some(request) = resolved.request {
            action_id = Some(request.action_id.clone());
            self.app.record_mobile_action_intent(&request).await?;
            repo.attach_step_action(&claimed.step.id, request.action_id.clone())
                .await?;
            repo.executor_event(
                &claimed,
                &execution,
                "mobile.executor_action_started",
                Some(&before.observation.id),
                action_id.as_deref(),
            )
            .await?;
            if self.app.mobile_is_stopped() {
                return repo
                    .executor_finish(
                        &claimed,
                        &execution,
                        Some(&before),
                        None,
                        None,
                        None,
                        Some(MobileGoalError::new(MobileGoalErrorCode::UserStopped)),
                        None,
                    )
                    .await;
            }
            let (progress, mut stages) = tokio::sync::mpsc::unbounded_channel();
            let action = self.runtime.action(request, lease.clone(), progress);
            tokio::pin!(action);
            let mut audit_error = None;
            let result = loop {
                tokio::select! {
                    result = &mut action => break result,
                    Some(stage) = stages.recv() => {
                        let name = match stage.stage {
                            ExecutorActionStage::ActionCompleted => "mobile.executor_action_completed",
                            ExecutorActionStage::ObserveAfterStarted => "mobile.executor_observe_after_started",
                        };
                        if let Err(error) = repo.executor_event(&claimed,&execution,name,
                            Some(&before.observation.id),action_id.as_deref()).await {
                            audit_error = Some(error);
                        }
                        let _ = stage.acknowledged.send(());
                    }
                }
            };
            // Never drop an external action or its device lease on an audit failure.
            if let Some(error) = audit_error {
                return Err(error);
            }
            match result {
                Ok(result) => {
                    self.app
                        .record_mobile_action(&result.receipt, result.capture.as_ref())
                        .await?;
                    if result.receipt.status != MobileActionStatus::Executed
                        || !result.receipt.command_sent
                    {
                        let after_failed = result.receipt.command_sent && result.capture.is_none();
                        repo.executor_event(
                            &claimed,
                            &execution,
                            if after_failed {
                                "mobile.executor_observe_after_failed"
                            } else {
                                "mobile.executor_action_failed"
                            },
                            Some(&before.observation.id),
                            action_id.as_deref(),
                        )
                        .await?;
                        return repo
                            .executor_finish(
                                &claimed,
                                &execution,
                                Some(&before),
                                None,
                                action_id.as_deref(),
                                Some(&resolved.expected_result),
                                Some(MobileGoalError::new(if after_failed {
                                    MobileGoalErrorCode::ObserveFailed
                                } else {
                                    MobileGoalErrorCode::ActionFailed
                                })),
                                None,
                            )
                            .await;
                    }
                    match result.capture {
                        Some(c) => c,
                        None => {
                            return repo
                                .executor_finish(
                                    &claimed,
                                    &execution,
                                    Some(&before),
                                    None,
                                    action_id.as_deref(),
                                    Some(&resolved.expected_result),
                                    Some(MobileGoalError::new(MobileGoalErrorCode::ObserveFailed)),
                                    None,
                                )
                                .await;
                        }
                    }
                }
                Err(e) => {
                    repo.executor_event(
                        &claimed,
                        &execution,
                        "mobile.executor_action_failed",
                        Some(&before.observation.id),
                        action_id.as_deref(),
                    )
                    .await?;
                    return repo
                        .executor_finish(
                            &claimed,
                            &execution,
                            Some(&before),
                            None,
                            action_id.as_deref(),
                            Some(&resolved.expected_result),
                            Some(e),
                            None,
                        )
                        .await;
                }
            }
        } else if claimed.step.step_type == MobileStepType::Wait {
            let delay = std::time::Duration::from_millis(claimed.step.wait_ms.unwrap_or(0) as u64);
            if claimed.goal.runtime_deadline.is_some_and(|deadline| {
                deadline <= Utc::now() + chrono::Duration::milliseconds(delay.as_millis() as i64)
            }) {
                return repo
                    .executor_finish(
                        &claimed,
                        &execution,
                        Some(&before),
                        None,
                        None,
                        None,
                        Some(MobileGoalError::new(MobileGoalErrorCode::TimeLimitReached)),
                        None,
                    )
                    .await;
            }
            tokio::select! {_ = tokio::time::sleep(delay)=>{},_ = cancellation.cancelled()=>return repo.executor_finish(&claimed,&execution,Some(&before),None,None,None,Some(MobileGoalError::new(MobileGoalErrorCode::UserStopped)),None).await}
            repo.executor_event(
                &claimed,
                &execution,
                "mobile.executor_observe_after_started",
                Some(&before.observation.id),
                None,
            )
            .await?;
            match self.runtime.observe().await {
                Ok(c) => {
                    self.app.record_mobile_capture(&c).await?;
                    c
                }
                Err(e) => {
                    return repo
                        .executor_finish(
                            &claimed,
                            &execution,
                            Some(&before),
                            None,
                            None,
                            Some(&resolved.expected_result),
                            Some(e),
                            None,
                        )
                        .await;
                }
            }
        } else {
            if claimed.step.step_type == MobileStepType::Extract {
                match extract_object(&before) {
                    Ok(o) => extract = Some(o),
                    Err(e) => {
                        return repo
                            .executor_finish(
                                &claimed,
                                &execution,
                                Some(&before),
                                Some(&before),
                                None,
                                Some(&resolved.expected_result),
                                Some(e),
                                None,
                            )
                            .await;
                    }
                }
            }
            before.clone()
        };
        repo.executor_event(
            &claimed,
            &execution,
            "mobile.executor_observe_after_completed",
            Some(&after.observation.id),
            action_id.as_deref(),
        )
        .await?;
        repo.executor_event(
            &claimed,
            &execution,
            "mobile.executor_verify_started",
            Some(&after.observation.id),
            action_id.as_deref(),
        )
        .await?;
        repo.executor_finish(
            &claimed,
            &execution,
            Some(&before),
            Some(&after),
            action_id.as_deref(),
            Some(&resolved.expected_result),
            None,
            extract,
        )
        .await
    }
}
#[cfg(test)]
mod tests;
impl AppRuntime {
    pub async fn verify_mobile_goal_completion(
        &self,
        id: &MobileGoalId,
        proposal: &mobile_runtime::planner::MobileCompletionProposal,
    ) -> Result<MobileGoal, AppError> {
        self.mobile_goal_repository()
            .executor_verify_completion(id, proposal)
            .await
    }
}
