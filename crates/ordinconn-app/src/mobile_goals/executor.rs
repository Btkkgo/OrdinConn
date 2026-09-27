//! Transaction boundaries for one executor attempt. External actions never run in a DB transaction.
use super::*;
use crate::mobile_executor::{MobileExtractedObject, MobileStepExecutionOutcome};
use mobile_runtime::{
    MobileCapture, MobileUiSnapshot,
    executor::{MobileVerificationEngine, MobileVerificationOutcome, same_app_ui},
};
#[derive(Clone)]
pub(crate) struct ExecutorSnapshot {
    pub goal: MobileGoal,
    pub plan: MobilePlan,
    pub step: MobilePlanStep,
    pub planned: Option<MobileUiSnapshot>,
    pub device_id: Option<String>,
    pub approval_authorized: bool,
}
fn approval_subject(
    g: &MobileGoal,
    p: &MobilePlan,
    s: &MobilePlanStep,
) -> Result<approval_engine::ApprovalSubject, AppError> {
    let canonical = serde_json::to_string(&(
        "mobile-step-v1",
        &g.id,
        &g.objective,
        &p.id,
        p.revision,
        &s.id,
        s.step_type,
        s.risk,
        &s.target_ref,
        &s.input_text,
        &s.expected_result,
        &s.wait_ms,
        &s.extraction_intent,
        &s.observation_before_id,
    ))?;
    let mobile = MobileApprovalSubject {
        goal_id: g.id.clone(),
        plan_id: p.id.clone(),
        step_id: s.id.clone(),
        action_type: s.step_type,
        target_hash: Some(mobile_runtime::SensitiveText::new(canonical).sha256()),
        observation_id: s.observation_before_id.clone(),
    };
    Ok(approval_engine::ApprovalSubject {
        object_id: s.id.as_str().into(),
        revision: p.revision,
        canonical_hash: mobile
            .canonical_hash()?
            .trim_start_matches("sha256:")
            .into(),
        action: format!("mobile:{:?}", s.step_type),
    })
}
async fn snapshot_in(
    conn: &mut SqliteConnection,
    id: &MobileGoalId,
) -> Result<ExecutorSnapshot, AppError> {
    let g = goal(conn, id).await?;
    if !matches!(
        g.status,
        MobileGoalStatus::Planning | MobileGoalStatus::Running
    ) {
        return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
    }
    g.step_budget.validate()?;
    let p = plan(
        conn,
        g.active_plan_id
            .as_ref()
            .ok_or_else(|| invalid(MobileGoalErrorCode::InvalidPlan))?,
    )
    .await?;
    if p.goal_id != g.id || p.status != MobilePlanStatus::Active {
        return Err(invalid(MobileGoalErrorCode::InvalidPlan));
    }
    let pending:Option<String>=sqlx::query_scalar("SELECT domain_json FROM mobile_plan_steps WHERE plan_id=? AND status='PENDING' ORDER BY sequence LIMIT 1").bind(p.id.as_str()).fetch_optional(&mut *conn).await?;
    let s: MobilePlanStep = serde_json::from_str(
        &pending.ok_or_else(|| invalid(MobileGoalErrorCode::InvalidStateTransition))?,
    )?;
    let blocked:i64=sqlx::query_scalar("SELECT COUNT(*) FROM mobile_plan_steps s JOIN mobile_plans p ON p.id=s.plan_id WHERE p.goal_id=? AND (s.status IN ('EXECUTING','WAITING_APPROVAL') OR (p.id=? AND s.sequence<? AND s.status NOT IN ('VERIFIED','SKIPPED')))").bind(id.as_str()).bind(p.id.as_str()).bind(s.sequence).fetch_one(&mut *conn).await?;
    if blocked != 0 {
        return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
    }
    let attempts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM mobile_executor_attempts WHERE goal_id=?")
            .bind(id.as_str())
            .fetch_one(&mut *conn)
            .await?;
    if s.sequence > g.step_budget.max_steps
        || attempts >= g.step_budget.max_steps as i64
        || attempts >= g.step_budget.max_execution_actions as i64
    {
        return Err(invalid(MobileGoalErrorCode::StepLimitReached));
    }
    if g.runtime_deadline.is_some_and(|d| d <= Utc::now()) {
        return Err(invalid(MobileGoalErrorCode::TimeLimitReached));
    }
    if g.consecutive_failure_count >= g.step_budget.max_consecutive_failures
        || g.identical_observation_count >= g.step_budget.max_identical_observations
    {
        return Err(invalid(MobileGoalErrorCode::Stalled));
    }
    let mut planned = None;
    let mut device_id = None;
    if let Some(o) = &s.observation_before_id {
        let row=sqlx::query("SELECT snap.domain_json AS snapshot,session.device_id,ob.domain_json AS observation FROM mobile_observations ob JOIN mobile_ui_snapshots snap ON snap.id=ob.snapshot_id JOIN mobile_device_sessions session ON session.id=ob.session_id WHERE ob.id=?").bind(o).fetch_optional(&mut *conn).await?.ok_or_else(||invalid(MobileGoalErrorCode::InvalidStep))?;
        let ob: mobile_runtime::MobileObservation =
            serde_json::from_str(&row.get::<String, _>("observation"))?;
        if ob.privacy_class == mobile_runtime::PrivacyClass::Sensitive {
            return Err(invalid(MobileGoalErrorCode::PolicyBlocked));
        }
        planned = Some(serde_json::from_str(&row.get::<String, _>("snapshot"))?);
        device_id = Some(row.get("device_id"));
    }
    let approval: Option<String> = sqlx::query_scalar(
        "SELECT request_json FROM mobile_step_approvals WHERE step_id=? AND status='authorized'",
    )
    .bind(s.id.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    let subject = approval_subject(&g, &p, &s)?;
    let approval_authorized = approval
        .map(|row| serde_json::from_str::<approval_engine::ApprovalRequest>(&row))
        .transpose()?
        .is_some_and(|a| {
            a.token_state == approval_engine::TokenState::Consumed
                && a.expires_at > Utc::now()
                && a.proposal_id == subject.object_id
                && a.proposal_version == subject.revision
                && a.proposal_hash == subject.canonical_hash
                && a.allowed_action == subject.action
        });
    Ok(ExecutorSnapshot {
        goal: g,
        plan: p,
        step: s,
        planned,
        device_id,
        approval_authorized,
    })
}
impl MobileGoalRepository {
    pub(crate) async fn request_step_approval(
        &self,
        snapshot: &ExecutorSnapshot,
        engine: &approval_engine::ApprovalEngine,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let current = snapshot_in(&mut tx, &snapshot.goal.id).await?;
        if current.step.id != snapshot.step.id
            || current.step.risk != MobileStepRisk::ApprovalRequired
        {
            return Err(invalid(MobileGoalErrorCode::InvalidStep));
        }
        let request = engine
            .request_subject(
                &approval_subject(&current.goal, &current.plan, &current.step)?,
                Duration::minutes(5),
            )
            .await?;
        sqlx::query("INSERT INTO mobile_step_approvals(id,goal_id,plan_id,step_id,status,request_json) VALUES (?,?,?,?,'requested',?)")
            .bind(&request.id).bind(current.goal.id.as_str()).bind(current.plan.id.as_str()).bind(current.step.id.as_str())
            .bind(serde_json::to_string(&request)?).execute(&mut *tx).await?;
        let e = event(
            &mut tx,
            "mobile.approval_requested",
            &current.goal.id,
            Some(&current.plan.id),
            Some(&current.step.id),
            None,
        )
        .await?;
        self.commit(tx, vec![e]).await
    }
    pub async fn get_step_approval(
        &self,
        id: &MobilePlanStepId,
    ) -> Result<Option<approval_engine::ApprovalRequest>, AppError> {
        let row: Option<String> =
            sqlx::query_scalar("SELECT request_json FROM mobile_step_approvals WHERE step_id=?")
                .bind(id.as_str())
                .fetch_optional(&self.pool)
                .await?;
        row.map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(AppError::from)
    }
    pub(crate) async fn authorize_step(
        &self,
        goal_id: &MobileGoalId,
        step_id: &MobilePlanStepId,
        approval_id: &str,
        engine: &approval_engine::ApprovalEngine,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut g = goal(&mut tx, goal_id).await?;
        let mut s = step(&mut tx, step_id).await?;
        let p = plan(&mut tx, &s.plan_id).await?;
        if g.status != MobileGoalStatus::WaitingApproval
            || s.status != MobileStepStatus::WaitingApproval
            || s.risk != MobileStepRisk::ApprovalRequired
            || p.goal_id != g.id
            || g.active_plan_id.as_ref() != Some(&p.id)
            || p.status != MobilePlanStatus::Active
            || g.runtime_deadline.is_some_and(|d| d <= Utc::now())
        {
            return Err(invalid(MobileGoalErrorCode::ApprovalRejected));
        }
        let row: Option<String> = sqlx::query_scalar("SELECT request_json FROM mobile_step_approvals WHERE id=? AND goal_id=? AND plan_id=? AND step_id=? AND status='requested'")
            .bind(approval_id).bind(goal_id.as_str()).bind(p.id.as_str()).bind(step_id.as_str()).fetch_optional(&mut *tx).await?;
        let stored: approval_engine::ApprovalRequest = serde_json::from_str(
            &row.ok_or_else(|| invalid(MobileGoalErrorCode::ApprovalRejected))?,
        )?;
        // Approval is bound to its original observation; any subsequent Observe invalidates it.
        let latest: Option<String> = sqlx::query_scalar(
            "SELECT id FROM mobile_observations ORDER BY observed_at DESC,id DESC LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await?;
        if stored.expires_at <= Utc::now() || latest != s.observation_before_id {
            return Err(invalid(MobileGoalErrorCode::ApprovalRejected));
        }
        let subject = approval_subject(&g, &p, &s)?;
        let capability = engine.approve_subject(approval_id, &subject).await?;
        engine
            .consume_subject(&capability, &subject, Utc::now())
            .await?;
        let consumed = engine
            .export_requests()
            .await
            .into_iter()
            .find(|a| a.id == approval_id)
            .ok_or_else(|| invalid(MobileGoalErrorCode::ApprovalRejected))?;
        sqlx::query("UPDATE mobile_step_approvals SET status='authorized',request_json=? WHERE id=? AND status='requested'")
            .bind(serde_json::to_string(&consumed)?).bind(approval_id).execute(&mut *tx).await?;
        // Durable, single-use authorization intent is created atomically with token consumption.
        s.status = MobileStepStatus::Pending;
        s.error_code = None;
        s.error_message = None;
        g.status = MobileGoalStatus::Running;
        g.updated_at = Utc::now();
        if g.started_at.is_none() {
            g.started_at = Some(Utc::now());
            g.runtime_deadline =
                Some(Utc::now() + Duration::milliseconds(g.step_budget.max_runtime_ms.into()));
        }
        save_step(&mut tx, &s).await?;
        save_goal(&mut tx, &g).await?;
        let e = event(
            &mut tx,
            "mobile.approval_consumed",
            goal_id,
            Some(&p.id),
            Some(step_id),
            None,
        )
        .await?;
        self.commit(tx, vec![e]).await
    }
    pub(crate) async fn executor_snapshot(
        &self,
        id: &MobileGoalId,
    ) -> Result<ExecutorSnapshot, AppError> {
        snapshot_in(&mut *self.pool.acquire().await?, id).await
    }
    pub(crate) async fn executor_wait_approval(
        &self,
        snapshot: &ExecutorSnapshot,
    ) -> Result<MobileStepExecutionOutcome, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let current = snapshot_in(&mut tx, &snapshot.goal.id).await?;
        if current.step.id != snapshot.step.id
            || current.step.risk != MobileStepRisk::ApprovalRequired
        {
            return Err(invalid(MobileGoalErrorCode::InvalidStep));
        }
        let mut s = current.step;
        change_step(
            &mut s,
            MobileStepStatus::WaitingApproval,
            Some(&MobileGoalError::new(MobileGoalErrorCode::ApprovalRequired)),
        );
        save_step(&mut tx, &s).await?;
        let mut g = current.goal.clone();
        g.status = MobileGoalStatus::WaitingApproval;
        g.updated_at = Utc::now();
        save_goal(&mut tx, &g).await?;
        let e = event(
            &mut tx,
            "mobile.step_waiting_approval",
            &current.goal.id,
            Some(&current.plan.id),
            Some(&s.id),
            None,
        )
        .await?;
        self.commit(tx, vec![e]).await?;
        Ok(outcome(
            &current.goal,
            &s,
            None,
            false,
            Some(MobileGoalError::new(MobileGoalErrorCode::ApprovalRequired)),
            None,
            vec![],
        ))
    }
    pub(crate) async fn executor_claim(
        &self,
        snapshot: &ExecutorSnapshot,
        current: &MobileCapture,
        execution: &str,
    ) -> Result<ExecutorSnapshot, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut fresh = snapshot_in(&mut tx, &snapshot.goal.id).await?;
        let latest: Option<String> = sqlx::query_scalar(
            "SELECT id FROM mobile_observations ORDER BY observed_at DESC,id DESC LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await?;
        if fresh.planned.is_some() && latest != fresh.step.observation_before_id {
            return Err(invalid(MobileGoalErrorCode::ObserveFailed));
        }
        if fresh.step.id != snapshot.step.id
            || fresh.plan.id != snapshot.plan.id
            || fresh.step.risk != snapshot.step.risk
            || fresh
                .device_id
                .as_ref()
                .is_some_and(|d| d != &current.session.device_id)
            || fresh
                .planned
                .as_ref()
                .is_some_and(|s| s.session_id != current.session.session_id)
        {
            return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
        }
        if !fresh.approval_authorized
            && !matches!(
                fresh.step.risk,
                MobileStepRisk::ReadOnly | MobileStepRisk::Reversible
            )
        {
            return Err(invalid(MobileGoalErrorCode::PolicyBlocked));
        }
        if fresh.approval_authorized {
            let consumed = sqlx::query("UPDATE mobile_step_approvals SET status='claimed' WHERE step_id=? AND status='authorized'")
                .bind(fresh.step.id.as_str()).execute(&mut *tx).await?;
            if consumed.rows_affected() != 1 {
                return Err(invalid(MobileGoalErrorCode::ApprovalRejected));
            }
        }
        if fresh.goal.status == MobileGoalStatus::Planning {
            fresh.goal.status = MobileGoalStatus::Running;
            fresh.goal.updated_at = Utc::now();
            if fresh.goal.started_at.is_none() {
                fresh.goal.started_at = Some(Utc::now());
                fresh.goal.runtime_deadline = Some(
                    Utc::now()
                        + Duration::milliseconds(fresh.goal.step_budget.max_runtime_ms.into()),
                );
            }
            save_goal(&mut tx, &fresh.goal).await?;
        }
        sqlx::query("INSERT INTO mobile_executor_attempts(id,goal_id,plan_id,step_id,device_id,session_id,planned_observation_id,acquired_at,status) VALUES (?,?,?,?,?,?,?,?,'claimed')").bind(execution).bind(fresh.goal.id.as_str()).bind(fresh.plan.id.as_str()).bind(fresh.step.id.as_str()).bind(&current.session.device_id).bind(&current.session.session_id).bind(&fresh.step.observation_before_id).bind(Utc::now().to_rfc3339()).execute(&mut *tx).await?;
        change_step(&mut fresh.step, MobileStepStatus::Executing, None);
        save_step(&mut tx, &fresh.step).await?;
        let events = vec![
            event(
                &mut tx,
                "mobile.goal_started",
                &fresh.goal.id,
                Some(&fresh.plan.id),
                Some(&fresh.step.id),
                None,
            )
            .await?,
            event(
                &mut tx,
                "mobile.step_started",
                &fresh.goal.id,
                Some(&fresh.plan.id),
                Some(&fresh.step.id),
                None,
            )
            .await?,
            executor_event(
                &mut tx,
                "mobile.executor_step_claimed",
                &fresh,
                execution,
                None,
                None,
            )
            .await?,
        ];
        self.commit(tx, events).await?;
        Ok(fresh)
    }
    pub(crate) async fn executor_event(
        &self,
        s: &ExecutorSnapshot,
        id: &str,
        name: &str,
        observation: Option<&str>,
        action: Option<&str>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        let e = executor_event(&mut tx, name, s, id, observation, action).await?;
        self.commit(tx, vec![e]).await
    }
    pub(crate) async fn executor_validate_dispatch(
        &self,
        claimed: &ExecutorSnapshot,
        before: &MobileCapture,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        let g = goal(&mut tx, &claimed.goal.id).await?;
        let latest: Option<String> = sqlx::query_scalar(
            "SELECT id FROM mobile_observations ORDER BY observed_at DESC,id DESC LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await?;
        if g.status != MobileGoalStatus::Running
            || latest.as_deref() != Some(before.observation.id.as_str())
        {
            return Err(invalid(MobileGoalErrorCode::ObserveFailed));
        }
        if g.runtime_deadline.is_some_and(|d| d <= Utc::now()) {
            return Err(invalid(MobileGoalErrorCode::TimeLimitReached));
        }
        if claimed.approval_authorized {
            let row: String = sqlx::query_scalar("SELECT request_json FROM mobile_step_approvals WHERE step_id=? AND status='claimed'").bind(claimed.step.id.as_str()).fetch_one(&mut *tx).await?;
            let request: approval_engine::ApprovalRequest = serde_json::from_str(&row)?;
            if request.expires_at <= Utc::now() {
                return Err(invalid(MobileGoalErrorCode::ApprovalRejected));
            }
        }
        tx.commit().await?;
        Ok(())
    }
    pub(crate) async fn executor_bind_before(
        &self,
        s: &ExecutorSnapshot,
        id: &str,
        before: &MobileCapture,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM mobile_executor_attempts WHERE id=? AND step_id=? AND status='claimed' AND device_id=? AND session_id=?").bind(id).bind(s.step.id.as_str()).bind(&before.session.device_id).bind(&before.session.session_id).fetch_one(&mut *tx).await?;
        if count != 1 {
            return Err(invalid(MobileGoalErrorCode::InvalidStep));
        }
        let mut step = step(&mut tx, &s.step.id).await?;
        if step.status != MobileStepStatus::Executing {
            return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
        }
        step.observation_before_id = Some(before.observation.id.clone());
        save_step(&mut tx, &step).await?;
        let e = executor_event(
            &mut tx,
            "mobile.executor_observe_before_completed",
            s,
            id,
            Some(&before.observation.id),
            None,
        )
        .await?;
        self.commit(tx, vec![e]).await
    }
    pub(crate) async fn executor_finish(
        &self,
        snapshot: &ExecutorSnapshot,
        execution: &str,
        before: Option<&MobileCapture>,
        after: Option<&MobileCapture>,
        action: Option<&str>,
        expected: Option<&ExpectedStepResult>,
        error: Option<MobileGoalError>,
        extract: Option<MobileExtractedObject>,
    ) -> Result<MobileStepExecutionOutcome, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut s = step(&mut tx, &snapshot.step.id).await?;
        let mut p = plan(&mut tx, &s.plan_id).await?;
        let mut g = goal(&mut tx, &p.goal_id).await?;
        let attempt:i64=sqlx::query_scalar("SELECT COUNT(*) FROM mobile_executor_attempts WHERE id=? AND step_id=? AND status='claimed'").bind(execution).bind(s.id.as_str()).fetch_one(&mut *tx).await?;
        if attempt != 1
            || s.status != MobileStepStatus::Executing
            || g.status != MobileGoalStatus::Running
            || g.active_plan_id.as_ref() != Some(&p.id)
            || p.status != MobilePlanStatus::Active
        {
            return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
        }
        let mut receipt = None;
        if let Some(id) = action {
            let json: String =
                sqlx::query_scalar("SELECT domain_json FROM mobile_action_receipts WHERE id=?")
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
            receipt = Some(serde_json::from_str::<mobile_runtime::MobileActionReceipt>(
                &json,
            )?);
        }
        let mut object_id = None;
        let mut evidence_ids = vec![];
        if let Some(object) = extract.as_ref() {
            if error.is_some()
                || s.step_type != MobileStepType::Extract
                || before.is_none_or(|b| object.observation_id != b.observation.id)
                || object.category != "mobile_observation_object"
            {
                return Err(invalid(MobileGoalErrorCode::InvalidStep));
            }
            let capture = before.ok_or_else(|| invalid(MobileGoalErrorCode::InvalidStep))?;
            let canonical = crate::mobile_executor::extract_object(capture)?;
            if object.source_id != canonical.source_id
                || object.source_locator != canonical.source_locator
                || object.captured_at != canonical.captured_at
                || object.facts != canonical.facts
            {
                return Err(invalid(MobileGoalErrorCode::InvalidStep));
            }
            insert_extract(&mut tx, &s, object).await?;
            object_id = Some(object.id.clone());
            evidence_ids.push(object.evidence_id.clone());
        }
        let mutation = matches!(
            s.step_type,
            MobileStepType::ScrollDown
                | MobileStepType::ScrollUp
                | MobileStepType::TapElement
                | MobileStepType::InputText
                | MobileStepType::Back
        );
        if let Some(id) = action {
            if id != format!("mobile_action_{execution}") || s.action_id.as_deref() != Some(id) {
                return Err(invalid(MobileGoalErrorCode::InvalidStep));
            }
        }
        let canonical = before.and_then(|b| {
            // Freshness was enforced before action; finish only rebinds the stored semantic contract.
            let mut b = b.clone();
            b.snapshot.captured_at = Utc::now();
            let mut validation_step = s.clone();
            if snapshot.approval_authorized {
                validation_step.risk = if matches!(
                    s.step_type,
                    MobileStepType::Observe | MobileStepType::Wait | MobileStepType::Extract
                ) {
                    MobileStepRisk::ReadOnly
                } else {
                    MobileStepRisk::Reversible
                };
            }
            mobile_runtime::executor::resolve_semantic_step(
                &validation_step,
                &b,
                snapshot.planned.as_ref(),
                execution,
                &[
                    "com.android.settings".into(),
                    "com.google.android.settings.intelligence".into(),
                ],
            )
            .ok()
        });
        let expected_bound = canonical
            .as_ref()
            .is_some_and(|r| Some(&r.expected_result) == expected);
        let observed = match (expected, before, after) {
            (Some(expected), Some(b), Some(a))
                if expected_bound && (!mutation || receipt.is_some()) =>
            {
                MobileVerificationEngine::verify(
                    expected,
                    b,
                    a,
                    receipt.as_ref(),
                    object_id.is_some(),
                )
            }
            _ => MobileVerificationOutcome::Inconclusive,
        };
        let error = error.or_else(|| {
            if g.runtime_deadline.is_some_and(|d| d <= Utc::now()) {
                Some(MobileGoalError::new(MobileGoalErrorCode::TimeLimitReached))
            } else if observed != MobileVerificationOutcome::Verified {
                Some(MobileGoalError::new(MobileGoalErrorCode::VerifyFailed))
            } else {
                None
            }
        });
        let verified = error.is_none();
        if !verified && object_id.is_some() {
            return Err(invalid(MobileGoalErrorCode::VerifyFailed));
        }
        s.observation_before_id = before
            .map(|b| b.observation.id.clone())
            .or(s.observation_before_id);
        s.observation_after_id = after.map(|a| a.observation.id.clone());
        s.action_id = action.map(str::to_owned);
        s.evidence_id = evidence_ids.first().cloned();
        change_step(
            &mut s,
            if verified {
                MobileStepStatus::Verified
            } else {
                MobileStepStatus::Failed
            },
            error.as_ref(),
        );
        save_step(&mut tx, &s).await?;
        let result = MobileStepResult {
            step_id: s.id.clone(),
            outcome: if verified {
                MobileStepOutcome::Verified
            } else {
                MobileStepOutcome::Failed
            },
            verified,
            observation_before_id: s.observation_before_id.clone(),
            observation_after_id: s.observation_after_id.clone(),
            action_receipt_id: s.action_id.clone(),
            evidence_ids: evidence_ids.clone(),
            error: error.clone(),
            created_at: Utc::now(),
        };
        sqlx::query("INSERT INTO mobile_step_results(step_id,observation_before_id,observation_after_id,action_receipt_id,created_at,domain_json) VALUES (?,?,?,?,?,?)").bind(s.id.as_str()).bind(&result.observation_before_id).bind(&result.observation_after_id).bind(&result.action_receipt_id).bind(result.created_at.to_rfc3339()).bind(serde_json::to_string(&result)?).execute(&mut *tx).await?;
        sqlx::query("UPDATE mobile_executor_attempts SET status=?,completed_at=? WHERE id=? AND status='claimed'").bind(if verified{"verified"}else{"failed"}).bind(Utc::now().to_rfc3339()).bind(execution).execute(&mut *tx).await?;
        g.identical_observation_count = if before
            .zip(after)
            .is_some_and(|(b, a)| same_app_ui(&b.snapshot, &a.snapshot))
        {
            g.identical_observation_count.saturating_add(1)
        } else {
            0
        };
        g.consecutive_failure_count = if verified {
            0
        } else {
            g.consecutive_failure_count.saturating_add(1)
        };
        let mut events = vec![
            event(
                &mut tx,
                s.status.event_type(),
                &g.id,
                Some(&p.id),
                Some(&s.id),
                Some(MobileExecutionReferences {
                    observation_id: s.observation_after_id.clone(),
                    action_id: s.action_id.clone(),
                    evidence_id: s.evidence_id.clone(),
                    ..Default::default()
                }),
            )
            .await?,
        ];
        if !verified {
            g.status = MobileGoalStatus::Failed;
            g.failed_at = Some(Utc::now());
            g.last_error_code = error.as_ref().map(|e| e.code);
            g.last_error_message = error.as_ref().map(|e| e.message.clone());
            events.extend(
                self.end_unfinished(&mut tx, &g.id, true, error.as_ref())
                    .await?,
            );
            p.status = MobilePlanStatus::Failed;
            save_plan(&mut tx, &p).await?;
            events.push(
                event(
                    &mut tx,
                    "mobile.plan_failed",
                    &g.id,
                    Some(&p.id),
                    Some(&s.id),
                    None,
                )
                .await?,
            );
            events.push(
                event(
                    &mut tx,
                    "mobile.goal_failed",
                    &g.id,
                    Some(&p.id),
                    Some(&s.id),
                    None,
                )
                .await?,
            );
        }
        g.updated_at = Utc::now();
        save_goal(&mut tx, &g).await?;
        for name in [
            if verified {
                "mobile.executor_verify_succeeded"
            } else {
                "mobile.executor_verify_failed"
            },
            if verified {
                "mobile.executor_step_completed"
            } else {
                "mobile.executor_step_failed"
            },
        ] {
            events.push(
                executor_event(
                    &mut tx,
                    name,
                    snapshot,
                    execution,
                    s.observation_after_id.as_deref(),
                    s.action_id.as_deref(),
                )
                .await?,
            );
        }
        if object_id.is_some() {
            events.push(
                executor_event(
                    &mut tx,
                    "mobile.data_updated",
                    snapshot,
                    execution,
                    s.observation_after_id.as_deref(),
                    None,
                )
                .await?,
            );
        }
        self.commit(tx, events).await?;
        Ok(outcome(
            &g,
            &s,
            Some(execution.into()),
            verified,
            error,
            object_id,
            evidence_ids,
        ))
    }
}
fn outcome(
    g: &MobileGoal,
    s: &MobilePlanStep,
    execution_id: Option<String>,
    verified: bool,
    error: Option<MobileGoalError>,
    data_object_id: Option<String>,
    evidence_ids: Vec<String>,
) -> MobileStepExecutionOutcome {
    MobileStepExecutionOutcome {
        goal_id: g.id.clone(),
        plan_id: s.plan_id.clone(),
        step_id: s.id.clone(),
        execution_id,
        status: s.status,
        verified,
        error,
        observation_before_id: s.observation_before_id.clone(),
        observation_after_id: s.observation_after_id.clone(),
        action_receipt_id: s.action_id.clone(),
        evidence_ids,
        data_object_id,
    }
}
async fn executor_event(
    tx: &mut Transaction<'_, Sqlite>,
    name: &str,
    s: &ExecutorSnapshot,
    execution: &str,
    observation: Option<&str>,
    action: Option<&str>,
) -> Result<RuntimeEventEnvelope, AppError> {
    append_event(tx,name,"mobile_goal",s.goal.id.as_str(),None,None,Some(s.step.id.as_str()),&serde_json::json!({"goalId":s.goal.id,"planId":s.plan.id,"stepId":s.step.id,"executionId":execution,"observationId":observation,"actionId":action,"receiptId":action})).await
}
async fn insert_extract(
    tx: &mut Transaction<'_, Sqlite>,
    s: &MobilePlanStep,
    o: &MobileExtractedObject,
) -> Result<(), AppError> {
    use evidence_core::{DataOrigin, Evidence, FactualLevel, SourceType};
    use market_core::{AssetRef, Market};
    let source = crate::mobile_executor::mobile_source_definition();
    let mut registry = collector_runtime::SourceRegistry::default();
    registry.register(source.clone())?;
    let source = registry.get(&source.id).ok_or(AppError::InvalidData)?;
    sqlx::query("INSERT OR IGNORE INTO sources(id,name,endpoint,collector_type,classification,capabilities_json,reliability_tier,poll_policy_json,rate_limit_json,auth_requirement,retention_policy_json,enabled,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,1,?,?)").bind(&source.id).bind(&source.name).bind(&source.endpoint).bind("computer").bind("company").bind(serde_json::to_string(&source.capabilities)?).bind("tier4_unverified").bind(serde_json::to_string(&source.poll)?).bind(serde_json::to_string(&source.rate_limit)?).bind("none").bind(serde_json::to_string(&source.retention)?).bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339()).execute(&mut **tx).await?;
    let mut e = Evidence::new(
        &o.source_id,
        SourceType::ComputerUse,
        AssetRef::new(Market::Traditional, "MOBILE_UI"),
        "Observed Android Settings labels",
        o.facts.join(" · "),
        FactualLevel::Observed,
        0.35,
        1.0,
        o.captured_at,
    );
    e.id = o.evidence_id.clone();
    e.raw_reference = Some(o.source_locator.clone());
    e.data_origin = DataOrigin::Real;
    e.metadata = serde_json::json!({"category":o.category,"observationId":o.observation_id,"dataObjectId":o.id,"sourceId":o.source_id,"marketApplicable":false});
    crate::services::insert_evidence(tx, &e).await?;
    sqlx::query("INSERT INTO mobile_extracted_objects(id,step_id,observation_id,evidence_id,source_id,captured_at,domain_json) VALUES (?,?,?,?,?,?,?)").bind(&o.id).bind(s.id.as_str()).bind(&o.observation_id).bind(&o.evidence_id).bind(&o.source_id).bind(o.captured_at.to_rfc3339()).bind(serde_json::to_string(o)?).execute(&mut **tx).await?;
    sqlx::query(
        "INSERT INTO mobile_step_evidence(step_id,evidence_id,observation_id) VALUES (?,?,?)",
    )
    .bind(s.id.as_str())
    .bind(&o.evidence_id)
    .bind(&o.observation_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
impl MobileGoalRepository {
    pub async fn set_mobile_completion_criterion(
        &self,
        goal_id: &MobileGoalId,
        step_id: &MobilePlanStepId,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let g = goal(&mut tx, goal_id).await?;
        let s = step(&mut tx, step_id).await?;
        let p = plan(&mut tx, &s.plan_id).await?;
        let known_objective = matches!(
            g.objective.as_str(),
            "Scroll down the current Settings page once" | "向下滚动当前设置页面一次"
        );
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM mobile_plan_steps WHERE plan_id=?")
                .bind(p.id.as_str())
                .fetch_one(&mut *tx)
                .await?;
        if !known_objective
            || g.status != MobileGoalStatus::Planning
            || g.active_plan_id.as_ref() != Some(&p.id)
            || p.goal_id != g.id
            || p.status != MobilePlanStatus::Active
            || s.step_type != MobileStepType::ScrollDown
            || s.status != MobileStepStatus::Pending
            || s.expected_result != Some(ExpectedStepResult::UiChanged)
            || count != 1
        {
            return Err(invalid(MobileGoalErrorCode::VerifyFailed));
        }
        sqlx::query("INSERT INTO mobile_completion_criteria(goal_id,step_id,objective,expected_json) VALUES (?,?,?,?)").bind(g.id.as_str()).bind(s.id.as_str()).bind(&g.objective).bind(serde_json::to_string(&s.expected_result)?).execute(&mut *tx).await?;
        let e = event(
            &mut tx,
            "mobile.completion_criterion_bound",
            &g.id,
            Some(&p.id),
            Some(&s.id),
            None,
        )
        .await?;
        self.commit(tx, vec![e]).await
    }
    pub(crate) async fn executor_verify_completion(
        &self,
        goal_id: &MobileGoalId,
        proposal: &mobile_runtime::planner::MobileCompletionProposal,
    ) -> Result<MobileGoal, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut g = goal(&mut tx, goal_id).await?;
        if g.status != MobileGoalStatus::Running
            || proposal.supporting_observation_ids.is_empty()
            || proposal.supporting_observation_ids.len() > 6
            || proposal.reason.trim().is_empty()
            || proposal.reason.len() > 512
            || mobile_runtime::planner::unsafe_text(&proposal.reason)
        {
            return Err(invalid(MobileGoalErrorCode::VerifyFailed));
        }
        if g.runtime_deadline.is_some_and(|d| d <= Utc::now()) {
            return Err(invalid(MobileGoalErrorCode::TimeLimitReached));
        }
        let mut p = plan(
            &mut tx,
            g.active_plan_id
                .as_ref()
                .ok_or_else(|| invalid(MobileGoalErrorCode::InvalidPlan))?,
        )
        .await?;
        let row=sqlx::query("SELECT step_id,objective,expected_json FROM mobile_completion_criteria WHERE goal_id=?").bind(g.id.as_str()).fetch_optional(&mut *tx).await?.ok_or_else(||invalid(MobileGoalErrorCode::VerifyFailed))?;
        if row.get::<String, _>("objective") != g.objective
            || p.status != MobilePlanStatus::Active
            || p.goal_id != g.id
        {
            return Err(invalid(MobileGoalErrorCode::VerifyFailed));
        }
        let criterion_id: MobilePlanStepId =
            serde_json::from_value(serde_json::json!(row.get::<String, _>("step_id")))?;
        let s = step(&mut tx, &criterion_id).await?;
        let expected: Option<ExpectedStepResult> =
            serde_json::from_str(&row.get::<String, _>("expected_json"))?;
        let steps = steps_for(&mut tx, &p.id).await?;
        if steps.is_empty()
            || steps.iter().any(|s| s.status != MobileStepStatus::Verified)
            || s.plan_id != p.id
            || s.expected_result != expected
        {
            return Err(invalid(MobileGoalErrorCode::VerifyFailed));
        }
        let verified:i64=sqlx::query_scalar("SELECT COUNT(*) FROM mobile_step_results r JOIN mobile_executor_attempts a ON a.step_id=r.step_id JOIN mobile_action_receipts receipt ON receipt.id=r.action_receipt_id WHERE r.step_id=? AND a.status='verified' AND json_extract(r.domain_json,'$.verified')=1 AND receipt.status='executed' AND json_extract(receipt.domain_json,'$.commandSent')=1").bind(s.id.as_str()).fetch_one(&mut *tx).await?;
        if verified != 1
            || s.observation_after_id
                .as_ref()
                .is_none_or(|id| !proposal.supporting_observation_ids.contains(id))
        {
            return Err(invalid(MobileGoalErrorCode::VerifyFailed));
        }
        let latest: Option<String> = sqlx::query_scalar(
            "SELECT id FROM mobile_observations ORDER BY observed_at DESC,id DESC LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await?;
        if latest != s.observation_after_id {
            return Err(invalid(MobileGoalErrorCode::VerifyFailed));
        }
        let target: Option<String> = sqlx::query_scalar(
            "SELECT target_json FROM mobile_goal_completion_targets WHERE goal_id=?",
        )
        .bind(g.id.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(target) = target {
            let target: MobileCompletionTarget = serde_json::from_str(&target)?;
            let row: String=sqlx::query_scalar("SELECT snap.domain_json FROM mobile_observations o JOIN mobile_ui_snapshots snap ON snap.id=o.snapshot_id WHERE o.id=?").bind(&s.observation_after_id).fetch_one(&mut *tx).await?;
            let after: MobileUiSnapshot = serde_json::from_str(&row)?;
            let target_passes = match target {
                MobileCompletionTarget::ActivityEquals { package, activity } => {
                    after.package_name == package && after.activity == activity
                }
                MobileCompletionTarget::InputTextEquals {
                    package,
                    activity,
                    resource_id,
                    value,
                } => {
                    let receipt: String = sqlx::query_scalar(
                        "SELECT domain_json FROM mobile_action_receipts WHERE id=?",
                    )
                    .bind(&s.action_id)
                    .fetch_one(&mut *tx)
                    .await?;
                    let receipt: mobile_runtime::MobileActionReceipt =
                        serde_json::from_str(&receipt)?;
                    after.package_name == package
                        && after.activity == activity
                        && after.elements.iter().any(|e| {
                            e.resource_id.as_ref() == Some(&resource_id)
                                && e.class_name.ends_with("EditText")
                        })
                        && receipt.input_value_verified == Some(true)
                        && receipt.text_sha256
                            == Some(mobile_runtime::SensitiveText::new(value).sha256())
                }
            };
            if !target_passes {
                return Err(invalid(MobileGoalErrorCode::VerifyFailed));
            }
        } else if s.step_type != MobileStepType::ScrollDown
            || expected != Some(ExpectedStepResult::UiChanged)
        {
            return Err(invalid(MobileGoalErrorCode::VerifyFailed));
        }
        for id in &proposal.supporting_observation_ids {
            let owned:i64=sqlx::query_scalar("SELECT COUNT(*) FROM mobile_plan_steps s JOIN mobile_plans p ON p.id=s.plan_id JOIN mobile_step_results r ON r.step_id=s.id JOIN mobile_observations o ON o.id=? WHERE p.goal_id=? AND s.status='VERIFIED' AND json_extract(r.domain_json,'$.verified')=1 AND (r.observation_before_id=o.id OR r.observation_after_id=o.id) AND json_extract(o.domain_json,'$.privacyClass') IN ('public','user_allowed')").bind(id).bind(g.id.as_str()).fetch_one(&mut *tx).await?;
            if owned == 0 {
                return Err(invalid(MobileGoalErrorCode::VerifyFailed));
            }
        }
        let now = Utc::now();
        g.status = MobileGoalStatus::Completed;
        g.completed_at = Some(now);
        g.updated_at = now;
        p.status = MobilePlanStatus::Completed;
        p.completed_at = Some(now);
        save_plan(&mut tx, &p).await?;
        save_goal(&mut tx, &g).await?;
        let events = vec![
            event(
                &mut tx,
                "mobile.completion_verified",
                &g.id,
                Some(&p.id),
                Some(&s.id),
                None,
            )
            .await?,
            event(
                &mut tx,
                "mobile.plan_completed",
                &g.id,
                Some(&p.id),
                None,
                None,
            )
            .await?,
            event(
                &mut tx,
                "mobile.goal_completed",
                &g.id,
                Some(&p.id),
                Some(&s.id),
                None,
            )
            .await?,
        ];
        self.commit(tx, events).await?;
        Ok(g)
    }
}
