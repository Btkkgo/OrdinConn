//! SQLite repository for canonical mobile execution contracts.
pub(crate) mod executor;
use crate::{
    AppError, AppRuntime,
    events::{RuntimeEventBus, RuntimeEventEnvelope, append_event},
};
use chrono::{Duration, Utc};
use mobile_runtime::execution::*;
use serde::de::DeserializeOwned;
use sqlx::{Row, Sqlite, SqliteConnection, SqlitePool, Transaction};

#[derive(Clone)]
pub struct MobileGoalRepository {
    pool: SqlitePool,
    events: RuntimeEventBus,
}
impl AppRuntime {
    pub fn mobile_goal_repository(&self) -> MobileGoalRepository {
        MobileGoalRepository {
            pool: self.pool().clone(),
            events: self.event_bus.clone(),
        }
    }
}
fn invalid(code: MobileGoalErrorCode) -> AppError {
    MobileGoalError::new(code).into()
}
async fn read<T: DeserializeOwned>(
    conn: &mut SqliteConnection,
    table: &str,
    id: &str,
) -> Result<T, AppError> {
    let text: String = sqlx::query_scalar(&format!("SELECT domain_json FROM {table} WHERE id=?"))
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(AppError::NotFound("mobile execution object"))?;
    Ok(serde_json::from_str(&text)?)
}
async fn goal(conn: &mut SqliteConnection, id: &MobileGoalId) -> Result<MobileGoal, AppError> {
    read(conn, "mobile_goals", id.as_str()).await
}
async fn plan(conn: &mut SqliteConnection, id: &MobilePlanId) -> Result<MobilePlan, AppError> {
    read(conn, "mobile_plans", id.as_str()).await
}
async fn step(
    conn: &mut SqliteConnection,
    id: &MobilePlanStepId,
) -> Result<MobilePlanStep, AppError> {
    read(conn, "mobile_plan_steps", id.as_str()).await
}
async fn save_goal(conn: &mut SqliteConnection, g: &MobileGoal) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE mobile_goals SET status=?,active_plan_id=?,updated_at=?,domain_json=? WHERE id=?",
    )
    .bind(g.status.as_str())
    .bind(g.active_plan_id.as_ref().map(|x| x.as_str()))
    .bind(g.updated_at.to_rfc3339())
    .bind(serde_json::to_string(g)?)
    .bind(g.id.as_str())
    .execute(conn)
    .await?;
    Ok(())
}
async fn save_plan(conn: &mut SqliteConnection, p: &MobilePlan) -> Result<(), AppError> {
    sqlx::query("UPDATE mobile_plans SET status=?,domain_json=? WHERE id=?")
        .bind(p.status.as_str())
        .bind(serde_json::to_string(p)?)
        .bind(p.id.as_str())
        .execute(conn)
        .await?;
    Ok(())
}
async fn save_step(conn: &mut SqliteConnection, s: &MobilePlanStep) -> Result<(), AppError> {
    sqlx::query("UPDATE mobile_plan_steps SET status=?,observation_before_id=?,observation_after_id=?,action_id=?,evidence_id=?,domain_json=? WHERE id=?")
        .bind(s.status.as_str()).bind(&s.observation_before_id).bind(&s.observation_after_id).bind(&s.action_id).bind(&s.evidence_id).bind(serde_json::to_string(s)?).bind(s.id.as_str()).execute(conn).await?;
    Ok(())
}
async fn event(
    tx: &mut Transaction<'_, Sqlite>,
    name: &str,
    g: &MobileGoalId,
    p: Option<&MobilePlanId>,
    s: Option<&MobilePlanStepId>,
    refs: Option<MobileExecutionReferences>,
) -> Result<RuntimeEventEnvelope, AppError> {
    let mut refs = refs.unwrap_or_default();
    refs.goal_id = Some(g.clone());
    refs.plan_id = p.cloned();
    refs.step_id = s.cloned();
    append_event(
        tx,
        name,
        "mobile_goal",
        g.as_str(),
        None,
        None,
        s.map(|x| x.as_str())
            .or_else(|| p.map(|x| x.as_str()))
            .or(Some(g.as_str())),
        &serde_json::to_value(refs)?,
    )
    .await
}
impl MobileGoalRepository {
    pub(crate) async fn skip_stale_pending_step(
        &self,
        id: &MobileGoalId,
        step_id: &MobilePlanStepId,
        host_changed: bool,
    ) -> Result<bool, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let g = goal(&mut tx, id).await?;
        let mut s = step(&mut tx, step_id).await?;
        if g.status.is_terminal()
            || g.active_plan_id.as_ref() != Some(&s.plan_id)
            || s.status != MobileStepStatus::Pending
        {
            return Ok(false);
        }
        let attempts: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM mobile_executor_attempts WHERE step_id=?")
                .bind(step_id.as_str())
                .fetch_one(&mut *tx)
                .await?;
        let latest: Option<String> = sqlx::query_scalar(
            "SELECT id FROM mobile_observations ORDER BY observed_at DESC,id DESC LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await?;
        if attempts != 0 || (!host_changed && latest == s.observation_before_id) {
            return Ok(false);
        }
        change_step(
            &mut s,
            MobileStepStatus::Skipped,
            Some(&MobileGoalError::new(MobileGoalErrorCode::ObserveFailed)),
        );
        save_step(&mut tx, &s).await?;
        let e = event(
            &mut tx,
            "mobile.step_skipped",
            id,
            Some(&s.plan_id),
            Some(&s.id),
            None,
        )
        .await?;
        self.commit(tx, vec![e]).await?;
        Ok(true)
    }
    pub(crate) async fn reserve_model_call(
        &self,
        context: &mobile_runtime::planner::MobilePlannerContext,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let g = goal(&mut tx, &context.goal_id).await?;
        if g.status.is_terminal() {
            return Err(invalid(MobileGoalErrorCode::UserStopped));
        }
        if g.runtime_deadline.is_some_and(|d| d <= Utc::now()) {
            return Err(invalid(MobileGoalErrorCode::TimeLimitReached));
        }
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM mobile_model_calls WHERE goal_id=?")
                .bind(g.id.as_str())
                .fetch_one(&mut *tx)
                .await?;
        if count >= g.step_budget.max_model_calls as i64 {
            return Err(invalid(MobileGoalErrorCode::StepLimitReached));
        }
        sqlx::query(
            "INSERT INTO mobile_model_calls(id,goal_id,observation_id,created_at) VALUES (?,?,?,?)",
        )
        .bind(format!("mobile_call_{}", uuid::Uuid::now_v7()))
        .bind(g.id.as_str())
        .bind(
            context
                .current_observation
                .as_ref()
                .map(|o| o.observation_id.as_str()),
        )
        .bind(Utc::now().to_rfc3339())
        .execute(&mut *tx)
        .await?;
        let e = event(
            &mut tx,
            "mobile.model_call_reserved",
            &g.id,
            None,
            None,
            None,
        )
        .await?;
        self.commit(tx, vec![e]).await
    }
    pub(crate) async fn begin_autonomous_goal(
        &self,
        id: &MobileGoalId,
    ) -> Result<MobileGoal, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut g = goal(&mut tx, id).await?;
        if g.status.is_terminal() || g.status == MobileGoalStatus::WaitingApproval {
            return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
        }
        let now = Utc::now();
        if g.started_at.is_none() {
            g.started_at = Some(now);
            g.runtime_deadline =
                Some(now + Duration::milliseconds(g.step_budget.max_runtime_ms.into()));
        }
        if g.status == MobileGoalStatus::Pending {
            g.status = MobileGoalStatus::Planning;
        }
        g.updated_at = now;
        save_goal(&mut tx, &g).await?;
        let e = event(
            &mut tx,
            "mobile.goal_planning",
            id,
            g.active_plan_id.as_ref(),
            None,
            None,
        )
        .await?;
        self.commit(tx, vec![e]).await?;
        Ok(g)
    }
    pub async fn set_completion_target(
        &self,
        id: &MobileGoalId,
        target: &MobileCompletionTarget,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let g = goal(&mut tx, id).await?;
        let (package, activity) = match target {
            MobileCompletionTarget::PageEquals {
                package,
                activity,
                visible_text,
            } => {
                if visible_text.is_empty()
                    || visible_text.len() > 8
                    || visible_text.iter().any(|text| {
                        text.trim().is_empty()
                            || text.len() > 128
                            || mobile_runtime::planner::prompt_text(text).as_deref()
                                != Some(text.as_str())
                    })
                {
                    return Err(invalid(MobileGoalErrorCode::PolicyBlocked));
                }
                (package, activity)
            }
            MobileCompletionTarget::ActivityEquals { package, activity } => (package, activity),
            MobileCompletionTarget::InputTextEquals {
                package,
                activity,
                resource_id,
                value,
            } => {
                if resource_id.len() > 256
                    || resource_id.is_empty()
                    || mobile_runtime::planner::prompt_android_identifier(resource_id).is_none()
                    || !mobile_runtime::is_safe_mobile_input(value)
                    || mobile_runtime::planner::prompt_text(value).is_none()
                {
                    return Err(invalid(MobileGoalErrorCode::PolicyBlocked));
                }
                (package, activity)
            }
        };
        if g.status != MobileGoalStatus::Pending
            || g.active_plan_id.is_some()
            || !matches!(
                package.as_str(),
                "com.android.settings" | "com.google.android.settings.intelligence"
            )
            || activity.is_empty()
            || activity.len() > 256
            || mobile_runtime::planner::prompt_android_identifier(activity).is_none()
        {
            return Err(invalid(MobileGoalErrorCode::InvalidPlan));
        }
        sqlx::query("INSERT INTO mobile_goal_completion_targets(goal_id,target_json) VALUES (?,?)")
            .bind(id.as_str())
            .bind(serde_json::to_string(target)?)
            .execute(&mut *tx)
            .await?;
        let e = event(&mut tx, "mobile.goal_target_bound", id, None, None, None).await?;
        self.commit(tx, vec![e]).await
    }
    pub(crate) async fn bind_planned_completion_target(
        &self,
        id: &MobileGoalId,
        step_id: &MobilePlanStepId,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let g = goal(&mut tx, id).await?;
        let s = step(&mut tx, step_id).await?;
        let p = plan(&mut tx, &s.plan_id).await?;
        let target: Option<String> = sqlx::query_scalar(
            "SELECT target_json FROM mobile_goal_completion_targets WHERE goal_id=?",
        )
        .bind(id.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        let Some(target) = target else {
            return Ok(());
        };
        let target: MobileCompletionTarget = serde_json::from_str(&target)?;
        let matches = match (&target, &s.expected_result) {
            (MobileCompletionTarget::PageEquals { .. }, Some(_))
                if s.status == MobileStepStatus::Verified =>
            {
                let row: Option<String> = sqlx::query_scalar("SELECT snap.domain_json FROM mobile_observations o JOIN mobile_ui_snapshots snap ON snap.id=o.snapshot_id WHERE o.id=?")
                    .bind(&s.observation_after_id).fetch_optional(&mut *tx).await?;
                row.map(|row| serde_json::from_str::<mobile_runtime::MobileUiSnapshot>(&row))
                    .transpose()?
                    .is_some_and(|snapshot| target.matches_semantic_page(&snapshot))
            }
            (
                MobileCompletionTarget::ActivityEquals { package, activity },
                Some(ExpectedStepResult::ActivityEquals {
                    package: p,
                    activity: a,
                }),
            ) => package == p && activity == a,
            (
                MobileCompletionTarget::InputTextEquals {
                    package,
                    activity,
                    resource_id,
                    value,
                },
                Some(ExpectedStepResult::TextEquals {
                    element_ref,
                    value: v,
                }),
            ) if s.step_type == MobileStepType::InputText && value == v => {
                let row: Option<String> = sqlx::query_scalar("SELECT snap.domain_json FROM mobile_observations o JOIN mobile_ui_snapshots snap ON snap.id=o.snapshot_id WHERE o.id=COALESCE((SELECT planned_observation_id FROM mobile_executor_attempts WHERE step_id=?),?)")
                    .bind(s.id.as_str()).bind(&s.observation_before_id).fetch_optional(&mut *tx).await?;
                row.map(|row| serde_json::from_str::<mobile_runtime::MobileUiSnapshot>(&row))
                    .transpose()?
                    .is_some_and(|snapshot| {
                        snapshot.package_name == *package
                            && snapshot.activity == *activity
                            && snapshot.elements.iter().any(|e| {
                                e.element_ref == *element_ref
                                    && e.resource_id.as_ref() == Some(resource_id)
                            })
                    })
            }
            _ => false,
        };
        if !matches {
            return Ok(());
        }
        if p.goal_id != g.id
            || g.active_plan_id.as_ref() != Some(&p.id)
            || !matches!(
                s.status,
                MobileStepStatus::Pending | MobileStepStatus::Verified
            )
        {
            return Err(invalid(MobileGoalErrorCode::InvalidStep));
        }
        sqlx::query("INSERT INTO mobile_completion_criteria(goal_id,step_id,objective,expected_json) VALUES (?,?,?,?) ON CONFLICT(goal_id) DO NOTHING")
            .bind(id.as_str()).bind(step_id.as_str()).bind(g.objective).bind(serde_json::to_string(&s.expected_result)?).execute(&mut *tx).await?;
        let e = event(
            &mut tx,
            "mobile.completion_criterion_bound",
            id,
            Some(&p.id),
            Some(step_id),
            None,
        )
        .await?;
        self.commit(tx, vec![e]).await
    }
    async fn commit(
        &self,
        tx: Transaction<'_, Sqlite>,
        events: Vec<RuntimeEventEnvelope>,
    ) -> Result<(), AppError> {
        tx.commit().await?;
        for e in events {
            self.events.publish(e);
        }
        Ok(())
    }
    pub async fn create_goal(
        &self,
        objective: &str,
        budget: MobileGoalBudget,
    ) -> Result<MobileGoal, AppError> {
        if objective.trim().is_empty() || objective.chars().count() > 1000 {
            return Err(invalid(MobileGoalErrorCode::InvalidPlan));
        }
        budget.validate()?;
        let now = Utc::now();
        let g = MobileGoal {
            id: MobileGoalId::new(),
            objective: objective.into(),
            normalized_objective: None,
            status: MobileGoalStatus::Pending,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            stopped_at: None,
            failed_at: None,
            active_plan_id: None,
            step_budget: budget,
            runtime_deadline: None,
            consecutive_failure_count: 0,
            identical_observation_count: 0,
            last_error_code: None,
            last_error_message: None,
        };
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("INSERT INTO mobile_goals(id,status,created_at,updated_at,domain_json) VALUES (?,'PENDING',?,?,?)").bind(g.id.as_str()).bind(now.to_rfc3339()).bind(now.to_rfc3339()).bind(serde_json::to_string(&g)?).execute(&mut *tx).await?;
        let e = event(&mut tx, "mobile.goal_created", &g.id, None, None, None).await?;
        self.commit(tx, vec![e]).await?;
        Ok(g)
    }
    pub async fn get_goal(&self, id: &MobileGoalId) -> Result<MobileGoal, AppError> {
        goal(&mut *self.pool.acquire().await?, id).await
    }
    pub async fn list_goals(&self) -> Result<Vec<MobileGoal>, AppError> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT domain_json FROM mobile_goals ORDER BY created_at DESC,id DESC LIMIT 50",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|x| serde_json::from_str(x).map_err(AppError::from))
            .collect()
    }
    pub async fn update_goal_status(
        &self,
        id: &MobileGoalId,
        next: MobileGoalStatus,
        error: Option<MobileGoalError>,
    ) -> Result<MobileGoal, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut g = goal(&mut tx, id).await?;
        if !g.status.can_transition_to(next) {
            return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
        }
        let now = Utc::now();
        let mut events = vec![];
        if next == MobileGoalStatus::Running {
            let p = plan(
                &mut tx,
                g.active_plan_id
                    .as_ref()
                    .ok_or_else(|| invalid(MobileGoalErrorCode::InvalidPlan))?,
            )
            .await?;
            if p.status != MobilePlanStatus::Active {
                return Err(invalid(MobileGoalErrorCode::InvalidPlan));
            }
            if g.started_at.is_none() {
                g.started_at = Some(now);
                g.runtime_deadline =
                    Some(now + Duration::milliseconds(g.step_budget.max_runtime_ms.into()));
            }
        }
        if next.is_terminal() {
            if next == MobileGoalStatus::Completed {
                let active_id = g
                    .active_plan_id
                    .as_ref()
                    .ok_or_else(|| invalid(MobileGoalErrorCode::InvalidPlan))?;
                let mut p = plan(&mut tx, active_id).await?;
                let unfinished: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_plan_steps WHERE plan_id=? AND status NOT IN ('VERIFIED','SKIPPED')").bind(p.id.as_str()).fetch_one(&mut *tx).await?;
                if p.status != MobilePlanStatus::Active || unfinished != 0 {
                    return Err(invalid(MobileGoalErrorCode::InvalidPlan));
                }
                p.status = MobilePlanStatus::Completed;
                p.completed_at = Some(now);
                save_plan(&mut tx, &p).await?;
                events.push(
                    event(
                        &mut tx,
                        "mobile.plan_completed",
                        id,
                        Some(&p.id),
                        None,
                        None,
                    )
                    .await?,
                );
                g.completed_at = Some(now);
            } else {
                events.extend(
                    self.end_unfinished(
                        &mut tx,
                        id,
                        next == MobileGoalStatus::Failed,
                        error.as_ref(),
                    )
                    .await?,
                );
                if next == MobileGoalStatus::Stopped {
                    g.stopped_at = Some(now);
                } else {
                    g.failed_at = Some(now);
                }
            }
        }
        g.status = next;
        g.updated_at = now;
        if let Some(error) = error {
            let safe = MobileGoalError::new(error.code);
            g.last_error_code = Some(safe.code);
            g.last_error_message = Some(safe.message);
        }
        save_goal(&mut tx, &g).await?;
        events.push(
            event(
                &mut tx,
                next.event_type(),
                id,
                g.active_plan_id.as_ref(),
                None,
                None,
            )
            .await?,
        );
        self.commit(tx, events).await?;
        Ok(g)
    }
    pub async fn create_plan(
        &self,
        id: &MobileGoalId,
        revision: u32,
        objective: &str,
        inputs: Vec<NewMobileStep>,
    ) -> Result<MobilePlan, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let g = goal(&mut tx, id).await?;
        let previous: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(revision),0) FROM mobile_plans WHERE goal_id=?",
        )
        .bind(id.as_str())
        .fetch_one(&mut *tx)
        .await?;
        if g.status.is_terminal()
            || revision as i64 != previous + 1
            || objective.trim().is_empty()
            || objective.chars().count() > 1000
            || inputs.is_empty()
        {
            return Err(invalid(MobileGoalErrorCode::InvalidPlan));
        }
        let p = MobilePlan {
            id: MobilePlanId::new(),
            goal_id: id.clone(),
            revision,
            objective: objective.into(),
            status: MobilePlanStatus::Draft,
            created_at: Utc::now(),
            activated_at: None,
            completed_at: None,
            superseded_at: None,
        };
        sqlx::query("INSERT INTO mobile_plans(id,goal_id,revision,status,created_at,domain_json) VALUES (?,?,?,'DRAFT',?,?)").bind(p.id.as_str()).bind(id.as_str()).bind(revision).bind(p.created_at.to_rfc3339()).bind(serde_json::to_string(&p)?).execute(&mut *tx).await?;
        let (_, mut events) = self.insert_steps(&mut tx, &g, &p, inputs).await?;
        events.push(event(&mut tx, "mobile.plan_created", id, Some(&p.id), None, None).await?);
        self.commit(tx, events).await?;
        Ok(p)
    }
    pub async fn create_steps(
        &self,
        id: &MobilePlanId,
        inputs: Vec<NewMobileStep>,
    ) -> Result<Vec<MobilePlanStep>, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let p = plan(&mut tx, id).await?;
        let g = goal(&mut tx, &p.goal_id).await?;
        let (steps, events) = self.insert_steps(&mut tx, &g, &p, inputs).await?;
        self.commit(tx, events).await?;
        Ok(steps)
    }
    async fn insert_steps(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        g: &MobileGoal,
        p: &MobilePlan,
        inputs: Vec<NewMobileStep>,
    ) -> Result<(Vec<MobilePlanStep>, Vec<RuntimeEventEnvelope>), AppError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM mobile_plan_steps WHERE plan_id=?")
                .bind(p.id.as_str())
                .fetch_one(&mut **tx)
                .await?;
        if g.status.is_terminal()
            || p.status != MobilePlanStatus::Draft
            || count + inputs.len() as i64 > g.step_budget.max_steps as i64
        {
            return Err(invalid(MobileGoalErrorCode::InvalidPlan));
        }
        let mut steps = vec![];
        let mut events = vec![];
        for input in inputs {
            if input.sequence == 0
                || input.sequence > g.step_budget.max_steps
                || input.reason.trim().is_empty()
            {
                return Err(invalid(MobileGoalErrorCode::InvalidStep));
            }
            if matches!(
                input.step_type,
                MobileStepType::TapElement | MobileStepType::InputText
            ) && input
                .target_ref
                .as_ref()
                .is_none_or(|x| !x.starts_with('@') || x.len() > 64)
            {
                return Err(invalid(MobileGoalErrorCode::InvalidStep));
            }
            if input
                .input_text
                .as_ref()
                .is_some_and(|x| !mobile_runtime::is_safe_mobile_input(x))
                || matches!(&input.expected_result, Some(ExpectedStepResult::TextEquals {value, ..}) if !mobile_runtime::is_safe_mobile_input(value))
            {
                return Err(invalid(MobileGoalErrorCode::PolicyBlocked));
            }
            if (input.step_type == MobileStepType::InputText) != input.input_text.is_some() {
                return Err(invalid(MobileGoalErrorCode::InvalidStep));
            }
            let s = MobilePlanStep {
                id: MobilePlanStepId::new(),
                plan_id: p.id.clone(),
                sequence: input.sequence,
                step_type: input.step_type,
                status: MobileStepStatus::Pending,
                reason: input.reason,
                risk: input.risk,
                target_ref: input.target_ref,
                input_text: input.input_text,
                expected_result: input.expected_result,
                wait_ms: input.wait_ms,
                extraction_intent: input.extraction_intent,
                created_at: Utc::now(),
                started_at: None,
                finished_at: None,
                observation_before_id: None,
                observation_after_id: None,
                action_id: None,
                evidence_id: None,
                error_code: None,
                error_message: None,
            };
            sqlx::query("INSERT INTO mobile_plan_steps(id,plan_id,sequence,status,risk,created_at,domain_json) VALUES (?,?,?,'PENDING',?,?,?)").bind(s.id.as_str()).bind(p.id.as_str()).bind(s.sequence).bind(serde_json::to_value(s.risk)?.as_str().unwrap()).bind(s.created_at.to_rfc3339()).bind(serde_json::to_string(&s)?).execute(&mut **tx).await?;
            events.push(
                event(
                    tx,
                    "mobile.step_created",
                    &g.id,
                    Some(&p.id),
                    Some(&s.id),
                    None,
                )
                .await?,
            );
            steps.push(s);
        }
        Ok((steps, events))
    }
    pub async fn list_plans_for_goal(
        &self,
        id: &MobileGoalId,
    ) -> Result<Vec<MobilePlan>, AppError> {
        self.get_goal(id).await?;
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT domain_json FROM mobile_plans WHERE goal_id=? ORDER BY revision",
        )
        .bind(id.as_str())
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|x| serde_json::from_str(x).map_err(AppError::from))
            .collect()
    }
    pub async fn get_active_plan(&self, id: &MobileGoalId) -> Result<Option<MobilePlan>, AppError> {
        let mut conn = self.pool.acquire().await?;
        let g = goal(&mut conn, id).await?;
        match g.active_plan_id {
            Some(id) => Ok(Some(plan(&mut conn, &id).await?)),
            None => Ok(None),
        }
    }
    pub async fn get_goal_plan(
        &self,
        id: &MobileGoalId,
    ) -> Result<Option<MobileGoalPlan>, AppError> {
        let mut tx = self.pool.begin().await?;
        let g = goal(&mut tx, id).await?;
        let view = if let Some(id) = g.active_plan_id {
            let p = plan(&mut tx, &id).await?;
            let steps = steps_for(&mut tx, &id).await?;
            Some(MobileGoalPlan { plan: p, steps })
        } else {
            None
        };
        tx.commit().await?;
        Ok(view)
    }
    pub async fn activate_plan(&self, id: &MobilePlanId) -> Result<MobilePlan, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut p = plan(&mut tx, id).await?;
        let mut g = goal(&mut tx, &p.goal_id).await?;
        if p.status != MobilePlanStatus::Draft
            || !matches!(
                g.status,
                MobileGoalStatus::Planning | MobileGoalStatus::Running
            )
        {
            return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
        }
        let now = Utc::now();
        let mut events = vec![];
        if let Some(old_id) = &g.active_plan_id {
            let busy: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_plan_steps WHERE plan_id=? AND status IN ('EXECUTING','WAITING_APPROVAL')").bind(old_id.as_str()).fetch_one(&mut *tx).await?;
            if busy != 0 {
                return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
            }
            let mut old = plan(&mut tx, old_id).await?;
            if old.status != MobilePlanStatus::Active || p.revision <= old.revision {
                return Err(invalid(MobileGoalErrorCode::InvalidPlan));
            }
            old.status = MobilePlanStatus::Superseded;
            old.superseded_at = Some(now);
            save_plan(&mut tx, &old).await?;
            events.push(
                event(
                    &mut tx,
                    "mobile.plan_superseded",
                    &g.id,
                    Some(old_id),
                    None,
                    None,
                )
                .await?,
            );
        }
        p.status = MobilePlanStatus::Active;
        p.activated_at = Some(now);
        save_plan(&mut tx, &p).await?;
        g.active_plan_id = Some(p.id.clone());
        g.updated_at = now;
        save_goal(&mut tx, &g).await?;
        events.push(
            event(
                &mut tx,
                "mobile.plan_activated",
                &g.id,
                Some(id),
                None,
                None,
            )
            .await?,
        );
        self.commit(tx, events).await?;
        Ok(p)
    }
    pub async fn get_step(&self, id: &MobilePlanStepId) -> Result<MobilePlanStep, AppError> {
        step(&mut *self.pool.acquire().await?, id).await
    }
    pub async fn list_steps(&self, id: &MobilePlanId) -> Result<Vec<MobilePlanStep>, AppError> {
        steps_for(&mut *self.pool.acquire().await?, id).await
    }
    pub async fn get_next_pending_step(
        &self,
        id: &MobilePlanId,
    ) -> Result<Option<MobilePlanStep>, AppError> {
        let text: Option<String> = sqlx::query_scalar("SELECT domain_json FROM mobile_plan_steps WHERE plan_id=? AND status='PENDING' ORDER BY sequence LIMIT 1").bind(id.as_str()).fetch_optional(&self.pool).await?;
        text.map(|x| serde_json::from_str(&x).map_err(AppError::from))
            .transpose()
    }
    pub async fn update_step_status(
        &self,
        id: &MobilePlanStepId,
        next: MobileStepStatus,
        error: Option<MobileGoalError>,
    ) -> Result<MobilePlanStep, AppError> {
        // VERIFIED can only be reached with an atomic persisted result.
        if next == MobileStepStatus::Verified {
            return Err(invalid(MobileGoalErrorCode::InvalidStep));
        }
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut s = step(&mut tx, id).await?;
        let p = plan(&mut tx, &s.plan_id).await?;
        let g = goal(&mut tx, &p.goal_id).await?;
        if g.status.is_terminal()
            || p.status != MobilePlanStatus::Active
            || g.active_plan_id.as_ref() != Some(&p.id)
            || !s.status.can_transition_to(next)
        {
            return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
        }
        if matches!(
            next,
            MobileStepStatus::Executing | MobileStepStatus::WaitingApproval
        ) && g.status != MobileGoalStatus::Running
            && !(g.status == MobileGoalStatus::WaitingApproval
                && s.status == MobileStepStatus::WaitingApproval)
        {
            return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
        }
        if matches!(
            next,
            MobileStepStatus::Executing | MobileStepStatus::WaitingApproval
        ) {
            let earlier: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_plan_steps WHERE plan_id=? AND sequence<? AND status NOT IN ('VERIFIED','SKIPPED')").bind(p.id.as_str()).bind(s.sequence).fetch_one(&mut *tx).await?;
            if earlier != 0 {
                return Err(invalid(MobileGoalErrorCode::InvalidStep));
            }
        }
        if next == MobileStepStatus::Executing && s.risk == MobileStepRisk::Forbidden {
            return Err(invalid(MobileGoalErrorCode::PolicyBlocked));
        }
        change_step(&mut s, next, error.as_ref());
        save_step(&mut tx, &s).await?;
        let e = event(
            &mut tx,
            next.event_type(),
            &g.id,
            Some(&p.id),
            Some(id),
            None,
        )
        .await?;
        self.commit(tx, vec![e]).await?;
        Ok(s)
    }
    pub async fn attach_step_observations(
        &self,
        id: &MobilePlanStepId,
        before: Option<String>,
        after: Option<String>,
    ) -> Result<MobilePlanStep, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (mut s, p) = mutable_step(&mut tx, id).await?;
        if before.is_some() {
            if s.observation_before_id.is_some() && s.observation_before_id != before {
                return Err(invalid(MobileGoalErrorCode::InvalidStep));
            }
            s.observation_before_id = before;
        }
        if after.is_some() {
            if s.observation_after_id.is_some() && s.observation_after_id != after {
                return Err(invalid(MobileGoalErrorCode::InvalidStep));
            }
            s.observation_after_id = after;
        }
        if let (Some(b), Some(a)) = (&s.observation_before_id, &s.observation_after_id) {
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_observations b JOIN mobile_observations a ON a.session_id=b.session_id WHERE b.id=? AND a.id=?").bind(b).bind(a).fetch_one(&mut *tx).await?;
            if count != 1 {
                return Err(invalid(MobileGoalErrorCode::InvalidStep));
            }
        }
        save_step(&mut tx, &s).await?;
        let refs = MobileExecutionReferences {
            observation_id: s
                .observation_after_id
                .clone()
                .or(s.observation_before_id.clone()),
            ..Default::default()
        };
        let e = event(
            &mut tx,
            "mobile.step_observations_attached",
            &p.goal_id,
            Some(&p.id),
            Some(id),
            Some(refs),
        )
        .await?;
        self.commit(tx, vec![e]).await?;
        Ok(s)
    }
    pub async fn attach_step_action(
        &self,
        id: &MobilePlanStepId,
        action_id: String,
    ) -> Result<MobilePlanStep, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (mut s, p) = mutable_step(&mut tx, id).await?;
        if s.action_id.as_ref().is_some_and(|x| x != &action_id) {
            return Err(invalid(MobileGoalErrorCode::InvalidStep));
        }
        if let Some(before) = &s.observation_before_id {
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_action_receipts r JOIN mobile_observations o ON ((r.snapshot_id=o.snapshot_id AND r.session_id=o.session_id) OR (r.status='pending' AND r.snapshot_id='' AND r.session_id='')) WHERE r.id=? AND o.id=?").bind(&action_id).bind(before).fetch_one(&mut *tx).await?;
            if count != 1 {
                return Err(invalid(MobileGoalErrorCode::InvalidStep));
            }
        }
        s.action_id = Some(action_id);
        save_step(&mut tx, &s).await?;
        let refs = MobileExecutionReferences {
            action_id: s.action_id.clone(),
            ..Default::default()
        };
        let e = event(
            &mut tx,
            "mobile.step_action_attached",
            &p.goal_id,
            Some(&p.id),
            Some(id),
            Some(refs),
        )
        .await?;
        self.commit(tx, vec![e]).await?;
        Ok(s)
    }
    pub async fn attach_step_evidence(
        &self,
        id: &MobilePlanStepId,
        evidence_id: String,
        observation_id: String,
    ) -> Result<MobilePlanStep, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (mut s, p) = mutable_step(&mut tx, id).await?;
        if s.observation_before_id.as_ref() != Some(&observation_id)
            && s.observation_after_id.as_ref() != Some(&observation_id)
        {
            return Err(invalid(MobileGoalErrorCode::InvalidStep));
        }
        sqlx::query(
            "INSERT INTO mobile_step_evidence(step_id,evidence_id,observation_id) VALUES (?,?,?)",
        )
        .bind(id.as_str())
        .bind(&evidence_id)
        .bind(&observation_id)
        .execute(&mut *tx)
        .await?;
        if s.evidence_id.is_none() {
            s.evidence_id = Some(evidence_id.clone());
        }
        save_step(&mut tx, &s).await?;
        let refs = MobileExecutionReferences {
            evidence_id: Some(evidence_id),
            observation_id: Some(observation_id),
            ..Default::default()
        };
        let e = event(
            &mut tx,
            "mobile.step_evidence_attached",
            &p.goal_id,
            Some(&p.id),
            Some(id),
            Some(refs),
        )
        .await?;
        self.commit(tx, vec![e]).await?;
        Ok(s)
    }
    pub async fn get_step_evidence(
        &self,
        id: &MobilePlanStepId,
    ) -> Result<Vec<MobileStepEvidenceLink>, AppError> {
        self.get_step(id).await?;
        let rows = sqlx::query("SELECT evidence_id,observation_id FROM mobile_step_evidence WHERE step_id=? ORDER BY evidence_id").bind(id.as_str()).fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|row| MobileStepEvidenceLink {
                step_id: id.clone(),
                evidence_id: row.get("evidence_id"),
                observation_id: row.get("observation_id"),
            })
            .collect())
    }
    pub async fn record_step_result(&self, result: MobileStepResult) -> Result<(), AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (mut s, p) = mutable_step(&mut tx, &result.step_id).await?;
        let next = match result.outcome {
            MobileStepOutcome::Verified => MobileStepStatus::Verified,
            MobileStepOutcome::Failed => MobileStepStatus::Failed,
            MobileStepOutcome::Stopped => MobileStepStatus::Stopped,
        };
        if !s.status.can_transition_to(next)
            || result.verified != (next == MobileStepStatus::Verified)
            || result.observation_before_id != s.observation_before_id
            || result.observation_after_id != s.observation_after_id
            || result.action_receipt_id != s.action_id
            || (result.verified && result.error.is_some())
        {
            return Err(invalid(MobileGoalErrorCode::InvalidStep));
        }
        if result.verified
            && let Some(receipt) = &result.action_receipt_id
        {
            let executed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_action_receipts WHERE id=? AND status='executed' AND verification='VERIFIED' AND json_extract(domain_json,'$.commandSent')=1").bind(receipt).fetch_one(&mut *tx).await?;
            if executed != 1 {
                return Err(invalid(MobileGoalErrorCode::InvalidStep));
            }
        }
        for evidence in &result.evidence_ids {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM mobile_step_evidence WHERE step_id=? AND evidence_id=?",
            )
            .bind(s.id.as_str())
            .bind(evidence)
            .fetch_one(&mut *tx)
            .await?;
            if count != 1 {
                return Err(invalid(MobileGoalErrorCode::InvalidStep));
            }
        }
        let mut result = result;
        result.error = result.error.map(|e| MobileGoalError::new(e.code));
        result.created_at = Utc::now();
        sqlx::query("INSERT INTO mobile_step_results(step_id,observation_before_id,observation_after_id,action_receipt_id,created_at,domain_json) VALUES (?,?,?,?,?,?)").bind(s.id.as_str()).bind(&result.observation_before_id).bind(&result.observation_after_id).bind(&result.action_receipt_id).bind(result.created_at.to_rfc3339()).bind(serde_json::to_string(&result)?).execute(&mut *tx).await?;
        change_step(&mut s, next, result.error.as_ref());
        save_step(&mut tx, &s).await?;
        let refs = MobileExecutionReferences {
            observation_id: s.observation_after_id.clone(),
            action_id: s.action_id.clone(),
            evidence_id: s.evidence_id.clone(),
            ..Default::default()
        };
        let e = event(
            &mut tx,
            next.event_type(),
            &p.goal_id,
            Some(&p.id),
            Some(&s.id),
            Some(refs),
        )
        .await?;
        self.commit(tx, vec![e]).await
    }
    pub async fn get_step_result(
        &self,
        id: &MobilePlanStepId,
    ) -> Result<Option<MobileStepResult>, AppError> {
        let text: Option<String> =
            sqlx::query_scalar("SELECT domain_json FROM mobile_step_results WHERE step_id=?")
                .bind(id.as_str())
                .fetch_optional(&self.pool)
                .await?;
        text.map(|x| serde_json::from_str(&x).map_err(AppError::from))
            .transpose()
    }
    async fn end_unfinished(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        id: &MobileGoalId,
        failed: bool,
        error: Option<&MobileGoalError>,
    ) -> Result<Vec<RuntimeEventEnvelope>, AppError> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT domain_json FROM mobile_plans WHERE goal_id=? AND status IN ('DRAFT','ACTIVE')",
        )
        .bind(id.as_str())
        .fetch_all(&mut **tx)
        .await?;
        let mut events = vec![];
        for text in rows {
            let mut p: MobilePlan = serde_json::from_str(&text)?;
            for mut s in steps_for(tx, &p.id).await? {
                if !s.status.is_terminal() {
                    let next = if failed
                        && matches!(
                            s.status,
                            MobileStepStatus::Executing | MobileStepStatus::WaitingApproval
                        ) {
                        MobileStepStatus::Failed
                    } else {
                        MobileStepStatus::Stopped
                    };
                    change_step(&mut s, next, error);
                    save_step(tx, &s).await?;
                    events.push(
                        event(tx, next.event_type(), id, Some(&p.id), Some(&s.id), None).await?,
                    );
                }
            }
            p.status = MobilePlanStatus::Failed;
            save_plan(tx, &p).await?;
            events.push(event(tx, "mobile.plan_failed", id, Some(&p.id), None, None).await?);
        }
        Ok(events)
    }
    pub async fn recover_interrupted(&self) -> Result<u64, AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let rows: Vec<String> = sqlx::query_scalar("SELECT domain_json FROM mobile_goals WHERE status IN ('PLANNING','RUNNING','WAITING_APPROVAL')").fetch_all(&mut *tx).await?;
        let count = rows.len() as u64;
        let mut events = vec![];
        let error = MobileGoalError::new(MobileGoalErrorCode::InterruptedByRestart);
        for text in rows {
            let mut g: MobileGoal = serde_json::from_str(&text)?;
            events.extend(
                self.end_unfinished(&mut tx, &g.id, true, Some(&error))
                    .await?,
            );
            g.status = MobileGoalStatus::Failed;
            g.failed_at = Some(Utc::now());
            g.updated_at = Utc::now();
            g.last_error_code = Some(error.code);
            g.last_error_message = Some(error.message.clone());
            save_goal(&mut tx, &g).await?;
            events.push(
                event(
                    &mut tx,
                    "mobile.goal_failed",
                    &g.id,
                    g.active_plan_id.as_ref(),
                    None,
                    None,
                )
                .await?,
            );
        }
        sqlx::query("UPDATE mobile_executor_attempts SET status='interrupted',completed_at=? WHERE status='claimed'").bind(Utc::now().to_rfc3339()).execute(&mut *tx).await?;
        self.commit(tx, events).await?;
        Ok(count)
    }
}
async fn steps_for(
    conn: &mut SqliteConnection,
    id: &MobilePlanId,
) -> Result<Vec<MobilePlanStep>, AppError> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT domain_json FROM mobile_plan_steps WHERE plan_id=? ORDER BY sequence",
    )
    .bind(id.as_str())
    .fetch_all(conn)
    .await?;
    rows.iter()
        .map(|x| serde_json::from_str(x).map_err(AppError::from))
        .collect()
}
async fn mutable_step(
    conn: &mut SqliteConnection,
    id: &MobilePlanStepId,
) -> Result<(MobilePlanStep, MobilePlan), AppError> {
    let s = step(conn, id).await?;
    let p = plan(conn, &s.plan_id).await?;
    let g = goal(conn, &p.goal_id).await?;
    if s.status.is_terminal()
        || g.status.is_terminal()
        || matches!(
            p.status,
            MobilePlanStatus::Superseded | MobilePlanStatus::Failed | MobilePlanStatus::Completed
        )
    {
        return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
    }
    Ok((s, p))
}
fn change_step(s: &mut MobilePlanStep, next: MobileStepStatus, error: Option<&MobileGoalError>) {
    s.status = next;
    if next == MobileStepStatus::Executing {
        s.started_at = Some(Utc::now());
    }
    if next.is_terminal() {
        s.finished_at = Some(Utc::now());
    }
    if let Some(e) = error {
        let safe = MobileGoalError::new(e.code);
        s.error_code = Some(safe.code);
        s.error_message = Some(safe.message);
    }
}

impl MobileGoalRepository {
    pub(crate) fn from_pool(pool: SqlitePool, events: RuntimeEventBus) -> Self {
        Self { pool, events }
    }
    pub async fn planner_context(
        &self,
        id: &MobileGoalId,
        observation_id: Option<&str>,
    ) -> Result<mobile_runtime::planner::MobilePlannerContext, AppError> {
        let mut tx = self.pool.begin().await?;
        let context = planner_context_in(&mut tx, id, observation_id).await?;
        tx.commit().await?;
        Ok(context)
    }
    pub(crate) async fn persist_planner_decision(
        &self,
        context: &mobile_runtime::planner::MobilePlannerContext,
        decision: mobile_runtime::planner::MobilePlannerDecision,
        provider: &str,
        model: &str,
        duration: u64,
    ) -> Result<mobile_runtime::planner::MobilePlannerOutcome, AppError> {
        use mobile_runtime::planner::*;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let fresh = planner_context_in(
            &mut tx,
            &context.goal_id,
            context
                .current_observation
                .as_ref()
                .map(|o| o.observation_id.as_str()),
        )
        .await?;
        // Recheck state/budget/history under the write lock; no stale response can append twice.
        if fresh.steps_used != context.steps_used
            || fresh.goal_status != context.goal_status
            || fresh.plan_revision != context.plan_revision
            || serde_json::to_value(&fresh.current_observation)?
                != serde_json::to_value(&context.current_observation)?
            || fresh.allowed_apps != context.allowed_apps
        {
            return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
        }
        let mut g = goal(&mut tx, &context.goal_id).await?;
        let mut events = vec![];
        let mut outcome = MobilePlannerOutcome {
            decision: MobilePlannerDecisionDto::CannotProceed,
            goal_id: g.id.clone(),
            plan_id: None,
            step_id: None,
            waiting_executor: false,
            completion: None,
            failure: None,
        };
        match decision {
            MobilePlannerDecision::NextAction { action } => {
                let input = action.validated_step(&fresh, fresh.steps_used + 1)?;
                if let Some(old_id) = &g.active_plan_id {
                    let mut old = plan(&mut tx, old_id).await?;
                    if old.status != MobilePlanStatus::Active {
                        return Err(invalid(MobileGoalErrorCode::InvalidPlan));
                    }
                    old.status = MobilePlanStatus::Superseded;
                    old.superseded_at = Some(Utc::now());
                    save_plan(&mut tx, &old).await?;
                    events.push(
                        event(
                            &mut tx,
                            "mobile.plan_superseded",
                            &g.id,
                            Some(old_id),
                            None,
                            None,
                        )
                        .await?,
                    );
                }
                let mut p = MobilePlan {
                    id: MobilePlanId::new(),
                    goal_id: g.id.clone(),
                    revision: fresh.plan_revision.unwrap_or(0) + 1,
                    objective: g.objective.clone(),
                    status: MobilePlanStatus::Draft,
                    created_at: Utc::now(),
                    activated_at: None,
                    completed_at: None,
                    superseded_at: None,
                };
                sqlx::query("INSERT INTO mobile_plans(id,goal_id,revision,status,created_at,domain_json) VALUES (?,?,?,'DRAFT',?,?)").bind(p.id.as_str()).bind(g.id.as_str()).bind(p.revision).bind(p.created_at.to_rfc3339()).bind(serde_json::to_string(&p)?).execute(&mut *tx).await?;
                events.push(
                    event(
                        &mut tx,
                        "mobile.plan_created",
                        &g.id,
                        Some(&p.id),
                        None,
                        None,
                    )
                    .await?,
                );
                let (mut steps, step_events) =
                    self.insert_steps(&mut tx, &g, &p, vec![input]).await?;
                events.extend(step_events);
                let step = &mut steps[0];
                step.observation_before_id = context
                    .current_observation
                    .as_ref()
                    .map(|o| o.observation_id.clone());
                save_step(&mut tx, step).await?;
                if g.status == MobileGoalStatus::Pending {
                    g.status = MobileGoalStatus::Planning;
                    events.push(
                        event(
                            &mut tx,
                            "mobile.goal_planning",
                            &g.id,
                            Some(&p.id),
                            None,
                            None,
                        )
                        .await?,
                    );
                }
                p.status = MobilePlanStatus::Active;
                p.activated_at = Some(Utc::now());
                save_plan(&mut tx, &p).await?;
                g.active_plan_id = Some(p.id.clone());
                g.updated_at = Utc::now();
                save_goal(&mut tx, &g).await?;
                events.push(
                    event(
                        &mut tx,
                        "mobile.plan_activated",
                        &g.id,
                        Some(&p.id),
                        None,
                        None,
                    )
                    .await?,
                );
                outcome.decision = MobilePlannerDecisionDto::NextAction;
                outcome.plan_id = Some(p.id);
                outcome.step_id = Some(step.id.clone());
                outcome.waiting_executor = true;
            }
            MobilePlannerDecision::Complete { completion } => {
                outcome.decision = MobilePlannerDecisionDto::CompletionProposal;
                outcome.completion = Some(completion);
            }
            MobilePlannerDecision::CannotProceed { failure } => {
                outcome.failure = Some(failure);
            }
        }
        let observation_id = context
            .current_observation
            .as_ref()
            .map(|o| o.observation_id.as_str());
        sqlx::query("INSERT INTO mobile_planner_decisions(id,goal_id,plan_id,step_id,observation_id,planner_version,schema_version,model_provider_id,model_name,created_at,outcome_json) VALUES (?,?,?,?,?,?,?,?,?,?,?)")
            .bind(format!("mobile_planner_{}",uuid::Uuid::now_v7())).bind(g.id.as_str()).bind(outcome.plan_id.as_ref().map(|p|p.as_str())).bind(outcome.step_id.as_ref().map(|s|s.as_str())).bind(observation_id).bind(PLANNER_VERSION).bind(SCHEMA_VERSION).bind(provider).bind(model).bind(Utc::now().to_rfc3339()).bind(serde_json::to_string(&outcome)?).execute(&mut *tx).await?;
        events.push(append_event(&mut tx,"mobile.planner_succeeded","mobile_goal",g.id.as_str(),None,None,outcome.step_id.as_ref().map(|s|s.as_str()),&serde_json::json!({"goalId":g.id,"planId":outcome.plan_id,"stepId":outcome.step_id,"observationId":observation_id,"plannerVersion":PLANNER_VERSION,"schemaVersion":SCHEMA_VERSION,"modelProviderId":provider,"modelName":model,"durationMs":duration,"decision":outcome.decision,"waitingExecutor":outcome.waiting_executor})).await?);
        self.commit(tx, events).await?;
        Ok(outcome)
    }
}
async fn planner_context_in(
    conn: &mut SqliteConnection,
    id: &MobileGoalId,
    observation_id: Option<&str>,
) -> Result<mobile_runtime::planner::MobilePlannerContext, AppError> {
    use mobile_runtime::planner::*;
    let g = goal(conn, id).await?;
    if g.status.is_terminal() || g.status == MobileGoalStatus::WaitingApproval {
        return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
    }
    let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM mobile_plan_steps s JOIN mobile_plans p ON s.plan_id=p.id WHERE p.goal_id=?").bind(id.as_str()).fetch_one(&mut *conn).await?;
    if count >= g.step_budget.max_steps as i64 {
        return Err(invalid(MobileGoalErrorCode::StepLimitReached));
    }
    let busy:i64=sqlx::query_scalar("SELECT COUNT(*) FROM mobile_plan_steps s JOIN mobile_plans p ON s.plan_id=p.id WHERE p.goal_id=? AND s.status IN ('PENDING','EXECUTING','WAITING_APPROVAL')").bind(id.as_str()).fetch_one(&mut *conn).await?;
    if busy != 0 {
        return Err(invalid(MobileGoalErrorCode::InvalidStateTransition));
    }
    if g.runtime_deadline
        .is_some_and(|deadline| deadline <= Utc::now())
    {
        return Err(invalid(MobileGoalErrorCode::TimeLimitReached));
    }
    let settings: Option<String> =
        sqlx::query_scalar("SELECT value_json FROM settings WHERE key='mobile_runtime'")
            .fetch_optional(&mut *conn)
            .await?;
    let settings: crate::MobileRuntimeSettings = settings
        .map(|s| serde_json::from_str(&s))
        .transpose()?
        .unwrap_or_default();
    let observation=sqlx::query("SELECT o.id,s.domain_json,json_extract(o.domain_json,'$.privacyClass') AS privacy_class FROM mobile_observations o JOIN mobile_ui_snapshots s ON o.snapshot_id=s.id ORDER BY o.observed_at DESC,o.id DESC LIMIT 1").fetch_optional(&mut *conn).await?;
    // A requested reference is an optimistic binding to the latest observation,
    // never permission to select a historical snapshot after a concurrent Observe.
    if observation_id.is_some_and(|id| {
        observation
            .as_ref()
            .is_none_or(|row| row.get::<String, _>("id") != id)
    }) {
        return Err(invalid(MobileGoalErrorCode::ObserveFailed));
    }
    let current_observation = observation
        .map(|row| -> Result<MobilePlannerObservation, AppError> {
            let privacy_class: Option<String> = row.get("privacy_class");
            if !matches!(privacy_class.as_deref(), Some("public" | "user_allowed")) {
                return Err(invalid(MobileGoalErrorCode::PolicyBlocked));
            }
            let s: mobile_runtime::MobileUiSnapshot =
                serde_json::from_str(&row.get::<String, _>("domain_json"))?;
            let safe_meta = |value: &str| {
                prompt_android_identifier(value).unwrap_or_else(|| "[REDACTED]".into())
            };
            let elements = s
                .elements
                .iter()
                .take(MAX_ELEMENTS)
                .map(|e| {
                    let sensitive = s.sensitive_state.is_some()
                        || s.redactions
                            .iter()
                            .any(|r| r.starts_with(&format!("element:{}:", e.element_ref)))
                        || e.text.as_deref() == Some("[REDACTED]")
                        || e.text.as_deref().is_some_and(unsafe_text)
                        || e.content_description.as_deref().is_some_and(unsafe_text)
                        || e.resource_id
                            .as_deref()
                            .is_some_and(|id| prompt_android_identifier(id).is_none());
                    let editable = e.class_name.ends_with("EditText");
                    MobilePlannerElement {
                        element_ref: e.element_ref.clone(),
                        resource_id: e.resource_id.as_deref().and_then(prompt_android_identifier),
                        text: if sensitive || editable {
                            None
                        } else {
                            e.text
                                .as_deref()
                                .or(e.content_description.as_deref())
                                .and_then(prompt_text)
                        },
                        clickable: e.clickable,
                        editable,
                        enabled: e.enabled,
                        sensitive,
                    }
                })
                .collect();
            Ok(MobilePlannerObservation {
                observation_id: row.get("id"),
                snapshot_id: s.snapshot_id,
                device_session_id: s.session_id,
                captured_at: s.captured_at.to_rfc3339(),
                package: safe_meta(&s.package_name),
                activity: safe_meta(&s.activity),
                screen_width: s.screen_width,
                screen_height: s.screen_height,
                sensitive_screen: s.sensitive_state.is_some(),
                elements,
            })
        })
        .transpose()?;
    let rows:Vec<String>=sqlx::query_scalar("SELECT s.domain_json FROM mobile_plan_steps s JOIN mobile_plans p ON s.plan_id=p.id WHERE p.goal_id=? ORDER BY p.revision DESC,s.sequence DESC LIMIT ?").bind(id.as_str()).bind(MAX_RECENT_STEPS as i64).fetch_all(&mut *conn).await?;
    let mut recent_steps = vec![];
    for row in rows {
        let s: MobilePlanStep = serde_json::from_str(&row)?;
        recent_steps.push(MobilePlannerHistory {
            step_id: s.id,
            sequence: s.sequence,
            step_type: s.step_type,
            status: s.status,
            error_code: s.error_code,
            observation_before_id: s.observation_before_id,
            observation_after_id: s.observation_after_id,
        });
    }
    recent_steps.reverse();
    let revision: Option<i64> =
        sqlx::query_scalar("SELECT MAX(revision) FROM mobile_plans WHERE goal_id=?")
            .bind(id.as_str())
            .fetch_one(&mut *conn)
            .await?;
    let target: Option<String> = sqlx::query_scalar(
        "SELECT target_json FROM mobile_goal_completion_targets WHERE goal_id=?",
    )
    .bind(id.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    let context = MobilePlannerContext {
        completion_target: target.map(|t| serde_json::from_str(&t)).transpose()?,
        goal_id: g.id,
        objective: prompt_objective(&g.objective)
            .unwrap_or_else(|| "[REDACTED: sensitive objective]".into()),
        goal_status: g.status,
        plan_revision: revision.map(|r| r as u32),
        step_budget: g.step_budget.max_steps,
        steps_used: count as u32,
        remaining_steps: g.step_budget.max_steps - (count as u32),
        observation_missing: current_observation.is_none(),
        current_observation,
        recent_failures: recent_steps
            .iter()
            .filter_map(|s| s.error_code)
            .take(MAX_RECENT_STEPS)
            .collect(),
        recent_steps,
        allowed_action_types: [
            "observe",
            "scroll_down",
            "scroll_up",
            "tap_element",
            "input_text",
            "back",
            "wait",
            "extract",
            "complete",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        forbidden_capabilities: [
            "shell",
            "adb",
            "filesystem",
            "sqlite",
            "tauri",
            "coordinates",
            "payment",
            "wallet",
            "password",
            "otp",
            "private_key",
            "seed_phrase",
            "delete_account",
            "irreversible_submission",
            "public_publishing",
            "send_message",
            "financial_action",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        allowed_apps: settings
            .allowed_apps
            .into_iter()
            .take(MAX_ELEMENTS)
            .collect(),
    };
    if serde_json::to_vec(&context)?.len() > MAX_CONTEXT_BYTES {
        return Err(invalid(MobileGoalErrorCode::InvalidPlan));
    }
    Ok(context)
}
