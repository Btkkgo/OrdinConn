//! Bounded production orchestration. All model calls use the existing Gateway Planner;
//! all device side effects use the accepted single-step Executor.
use crate::{AppError, AppRuntime, mobile_executor::MobileExecutorRuntime};
use mobile_runtime::{execution::*, planner::*};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

impl AppRuntime {
    pub async fn approve_mobile_step(
        self: &Arc<Self>,
        id: &MobileGoalId,
        step: &MobilePlanStepId,
        approval: &str,
        runtime: Arc<dyn MobileExecutorRuntime>,
    ) -> Result<crate::mobile_executor::MobileStepExecutionOutcome, AppError> {
        let _run = self
            .mobile_run_guard
            .try_lock()
            .map_err(|_| MobileGoalError::new(MobileGoalErrorCode::InvalidStateTransition))?;
        if self.mobile_is_stopped() {
            return Err(MobileGoalError::new(MobileGoalErrorCode::UserStopped).into());
        }
        self.mobile_goal_repository()
            .authorize_step(id, step, approval, &self.approval_engine)
            .await?;
        let executed = self.execute_mobile_goal_step(id, runtime).await?;
        if self.mobile_is_stopped() {
            let repo = self.mobile_goal_repository();
            if !repo.get_goal(id).await?.status.is_terminal() {
                repo.update_goal_status(
                    id,
                    MobileGoalStatus::Stopped,
                    Some(MobileGoalError::new(MobileGoalErrorCode::UserStopped)),
                )
                .await?;
            }
        } else if executed.verified {
            self.complete_mobile_owner_target(id, &executed).await?;
        }
        Ok(executed)
    }
    async fn complete_mobile_owner_target(
        &self,
        id: &MobileGoalId,
        executed: &crate::mobile_executor::MobileStepExecutionOutcome,
    ) -> Result<Option<MobileGoal>, AppError> {
        self.mobile_goal_repository()
            .bind_planned_completion_target(id, &executed.step_id)
            .await?;
        let bound: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_goal_completion_targets t JOIN mobile_completion_criteria c ON c.goal_id=t.goal_id WHERE t.goal_id=? AND c.step_id=?")
            .bind(id.as_str()).bind(executed.step_id.as_str()).fetch_one(self.pool()).await?;
        if bound != 1 {
            return Ok(None);
        }
        let support = [
            executed.observation_before_id.clone(),
            executed.observation_after_id.clone(),
        ]
        .into_iter()
        .flatten()
        .collect();
        self.verify_mobile_goal_completion(
            id,
            &MobileCompletionProposal {
                reason: "Owner target satisfied by verified step".into(),
                supporting_observation_ids: support,
            },
        )
        .await
        .map(Some)
    }
    pub(crate) fn mobile_cancellation_token(&self) -> CancellationToken {
        self.mobile_cancellation
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub(crate) fn mobile_is_stopped(&self) -> bool {
        self.planner_cancellation.is_cancelled() || self.mobile_cancellation_token().is_cancelled()
    }
    /// Signal immediately, then wait for the in-flight receipt and runner to settle.
    pub async fn stop_mobile_goals(&self) -> Result<(), AppError> {
        self.mobile_cancellation_token().cancel();
        let _settled = self.mobile_run_guard.lock().await;
        self.device_execution_leases.wait_idle().await;
        let rows: Vec<String> = sqlx::query_scalar("SELECT domain_json FROM mobile_goals WHERE status NOT IN ('COMPLETED','FAILED','STOPPED')").fetch_all(self.pool()).await?;
        for row in rows {
            let g: MobileGoal = serde_json::from_str(&row)?;
            if !g.status.is_terminal() {
                self.mobile_goal_repository()
                    .update_goal_status(
                        &g.id,
                        MobileGoalStatus::Stopped,
                        Some(MobileGoalError::new(MobileGoalErrorCode::UserStopped)),
                    )
                    .await?;
            }
        }
        Ok(())
    }
    pub async fn run_mobile_goal<F>(
        self: &Arc<Self>,
        id: &MobileGoalId,
        runtime: Arc<dyn MobileExecutorRuntime>,
        credentials: F,
    ) -> Result<MobileGoal, AppError>
    where
        F: Fn(&str) -> Result<Option<String>, AppError>,
    {
        let _run = self
            .mobile_run_guard
            .try_lock()
            .map_err(|_| MobileGoalError::new(MobileGoalErrorCode::InvalidStateTransition))?;
        // Provider gate before Observe, state changes or credentials; zero-provider Goals stay PENDING.
        *self
            .mobile_cancellation
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = self.planner_cancellation.child_token();
        let token = self.mobile_cancellation_token();
        crate::mobile_planner::check_production_provider(self.pool()).await?;
        if token.is_cancelled() {
            return Err(MobileGoalError::new(MobileGoalErrorCode::UserStopped).into());
        }
        let repo = self.mobile_goal_repository();
        let g = repo.begin_autonomous_goal(id).await?;
        let deadline = g
            .runtime_deadline
            .ok_or_else(|| MobileGoalError::new(MobileGoalErrorCode::InvalidPlan))?;
        // Canonical step/model budgets persist below. This bound also limits stale replanning.
        let result = async {
            let mut post_capture = if g.status==MobileGoalStatus::Running {runtime.current_capture()} else {None};
            for _ in 0..=g.step_budget.max_model_calls {
                if token.is_cancelled() { return Err(MobileGoalError::new(MobileGoalErrorCode::UserStopped).into()); }
                let remaining = (deadline - chrono::Utc::now()).to_std().map_err(|_|MobileGoalError::new(MobileGoalErrorCode::TimeLimitReached))?;
                let existing_post = post_capture.take();
                let is_new_capture = existing_post.is_none();
                let capture = if let Some(capture) = existing_post { capture } else { tokio::select! {
                    _=token.cancelled()=>return Err(MobileGoalError::new(MobileGoalErrorCode::UserStopped).into()),
                    result=tokio::time::timeout(remaining,runtime.observe())=>result.map_err(|_|MobileGoalError::new(MobileGoalErrorCode::TimeLimitReached))??,
                }};
                // Action post-observations were already persisted by the Executor.
                if is_new_capture {
                    self.record_mobile_capture(&capture).await?;
                }
                let remaining = (deadline - chrono::Utc::now()).to_std().map_err(|_|MobileGoalError::new(MobileGoalErrorCode::TimeLimitReached))?;
                let planned = tokio::time::timeout(remaining,self.plan_mobile_goal(id,Some(&capture.observation.id),&credentials)).await
                    .map_err(|_|MobileGoalError::new(MobileGoalErrorCode::TimeLimitReached))?;
                let planned = match planned {
                    Err(AppError::MobileGoal(e)) if e.code==MobileGoalErrorCode::ObserveFailed => continue,
                    other => other?,
                };
                if token.is_cancelled() { return Err(MobileGoalError::new(MobileGoalErrorCode::UserStopped).into()); }
                match planned.decision {
                    MobilePlannerDecisionDto::CompletionProposal => {
                        return self.verify_mobile_goal_completion(id,planned.completion.as_ref().ok_or_else(||MobileGoalError::new(MobileGoalErrorCode::VerifyFailed))?).await;
                    },
                    MobilePlannerDecisionDto::CannotProceed => return Err(MobileGoalError::new(MobileGoalErrorCode::InvalidPlan).into()),
                    MobilePlannerDecisionDto::NextAction => {
                        let step_id=planned.step_id.as_ref().ok_or_else(||MobileGoalError::new(MobileGoalErrorCode::InvalidStep))?;
                        let host_changed=runtime.current_capture().is_none_or(|c|c.observation.id!=capture.observation.id);
                        if repo.skip_stale_pending_step(id,step_id,host_changed).await? {continue;}
                        // Never cancel an emitted device action. The Executor settles receipt/Observe/Verify.
                        let executed = match self.execute_mobile_goal_step(id,runtime.clone()).await {
                            Err(AppError::MobileGoal(e)) if e.code==MobileGoalErrorCode::ObserveFailed => {
                                if repo.skip_stale_pending_step(id,step_id,false).await? {continue;}
                                return Err(e.into());
                            },
                            other => other?,
                        };
                        if token.is_cancelled() { return Err(MobileGoalError::new(MobileGoalErrorCode::UserStopped).into()); }
                        if executed.status==MobileStepStatus::WaitingApproval { return repo.get_goal(id).await; }
                        post_capture = runtime.current_capture();
                        if !executed.verified { return Err(executed.error.unwrap_or_else(||MobileGoalError::new(MobileGoalErrorCode::VerifyFailed)).into()); }
                        if let Some(completed) = self.complete_mobile_owner_target(id, &executed).await? {
                            return Ok(completed);
                        }
                    }
                }
            }
            Err(MobileGoalError::new(MobileGoalErrorCode::StepLimitReached).into())
        }.await;
        match result {
            Ok(g) => Ok(g),
            Err(e) => {
                let code = match &e {
                    AppError::MobileGoal(e) => e.code,
                    _ => MobileGoalErrorCode::InvalidPlan,
                };
                let current = repo.get_goal(id).await?;
                if !current.status.is_terminal() {
                    repo.update_goal_status(
                        id,
                        if code == MobileGoalErrorCode::UserStopped {
                            MobileGoalStatus::Stopped
                        } else {
                            MobileGoalStatus::Failed
                        },
                        Some(MobileGoalError::new(code)),
                    )
                    .await?;
                }
                Err(e)
            }
        }
    }
}
