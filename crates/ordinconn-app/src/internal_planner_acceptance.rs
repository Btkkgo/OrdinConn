//! Opt-in desktop acceptance support. This module does not register IPC or start work.
//! Only the desktop's default-off internal controller may invoke these methods.
//! No executor, device host, caller-supplied credential, or production write is used.
use crate::{
    AppError, AppRuntime,
    mobile_planner::{PlannerAttemptObserver, PlannerHttpAttempt},
};
use mobile_runtime::execution::{MobileGoal, MobileGoalErrorCode, MobileGoalId, MobileGoalStatus};
use model_gateway::ProviderCapabilities;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{Column, Row, SqlitePool, TypeInfo};
use std::{sync::Arc, time::Duration};

const LOGICAL_CALLS: usize = 5;

#[derive(Clone, Debug, Serialize)]
pub struct PlannerAcceptanceReadiness {
    pub provider_count: usize,
    pub provider_id: String,
    pub model: String,
    pub provider_metadata_sha256: String,
    pub original_input_sha256: String,
    pub source_goal_id: String,
    pub android_actions_executed: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct PlannerAcceptanceLogicalCall {
    pub logical_call: usize,
    pub started_at: String,
    pub wait_before_call_ms: u64,
    pub failure_class: Option<&'static str>,
    pub passed: bool,
    pub decision: Option<mobile_runtime::planner::MobilePlannerDecisionDto>,
    pub error_code: Option<MobileGoalErrorCode>,
    pub attempts: Vec<PlannerHttpAttempt>,
    pub budgeted_model_calls: u64,
    pub android_actions_executed: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct PlannerAcceptanceReport {
    pub readiness: PlannerAcceptanceReadiness,
    pub logical_calls: Vec<PlannerAcceptanceLogicalCall>,
    pub passed: usize,
    pub http_attempts: usize,
    pub http_503_retries: usize,
    pub http_429_count: usize,
    pub http_503_count: usize,
    pub stop_reason: Option<&'static str>,
    pub production_data_unchanged: bool,
    pub android_actions_executed: u64,
}

/// The only state accepted by `run_internal_planner_acceptance` is built from saved data.
/// Fields are private; there is no API key, endpoint override, or model override input.
pub struct PlannerAcceptanceSession {
    readiness: PlannerAcceptanceReadiness,
    source_goal_json: String,
    source_settings_json: String,
    source_provider: SavedRow,
    source_captures: Vec<SavedRow>,
    source_capture_ids: Vec<(&'static str, String)>,
    source_observation_id: String,
    production_actions_before: i64,
    isolated: Vec<Arc<AppRuntime>>,
    _directory: tempfile::TempDir,
    used: bool,
}
impl PlannerAcceptanceSession {
    pub fn readiness(&self) -> &PlannerAcceptanceReadiness {
        &self.readiness
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
enum Cell {
    Text(Option<String>),
    Integer(Option<i64>),
    Real(Option<f64>),
}
#[derive(Clone, Debug, PartialEq, Serialize)]
struct SavedRow {
    table: &'static str,
    cells: Vec<(String, Cell)>,
}
impl SavedRow {
    async fn read(pool: &SqlitePool, table: &'static str, id: &str) -> Result<Self, AppError> {
        // SQL identifiers are a fixed, internal allowlist, never request-controlled.
        if !matches!(
            table,
            "model_providers"
                | "mobile_device_sessions"
                | "mobile_ui_snapshots"
                | "mobile_observations"
        ) {
            return Err(AppError::InvalidData);
        }
        let row = sqlx::query(&format!("SELECT * FROM {table} WHERE id=?"))
            .bind(id)
            .fetch_one(pool)
            .await?;
        let mut cells = Vec::new();
        for column in row.columns() {
            let name = column.name();
            let value = match column.type_info().name() {
                "TEXT" | "NULL" => Cell::Text(row.try_get(name)?),
                "INTEGER" => Cell::Integer(row.try_get(name)?),
                "REAL" => Cell::Real(row.try_get(name)?),
                _ => return Err(AppError::InvalidData),
            };
            cells.push((name.to_owned(), value));
        }
        Ok(Self { table, cells })
    }
    async fn insert(&self, pool: &SqlitePool) -> Result<(), AppError> {
        let names = self
            .cells
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>();
        let statement = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.table,
            names.join(","),
            vec!["?"; names.len()].join(",")
        );
        let mut query = sqlx::query(&statement);
        for (_, cell) in &self.cells {
            query = match cell {
                Cell::Text(value) => query.bind(value),
                Cell::Integer(value) => query.bind(value),
                Cell::Real(value) => query.bind(value),
            };
        }
        query.execute(pool).await?;
        Ok(())
    }
}
fn fingerprint(value: &impl Serialize) -> Result<String, AppError> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}
async fn action_count(pool: &SqlitePool) -> Result<i64, AppError> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*) FROM mobile_action_receipts")
            .fetch_one(pool)
            .await?,
    )
}

impl AppRuntime {
    /// Prepare isolated replays using the exact saved provider and original observation.
    /// This method performs no credential resolution, model request, or device operation.
    pub async fn prepare_internal_planner_acceptance(
        &self,
        source_goal_id: &MobileGoalId,
    ) -> Result<PlannerAcceptanceSession, AppError> {
        let provider_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_providers")
            .fetch_one(self.pool())
            .await?;
        if provider_count != 1 {
            return Err(AppError::InvalidData);
        }
        let provider = sqlx::query("SELECT id,provider_type,default_model,base_url,capabilities_json,credential_ref,enabled FROM model_providers")
            .fetch_one(self.pool()).await?;
        let provider_id: String = provider.get("id");
        let model: String = provider.get("default_model");
        let capabilities: ProviderCapabilities =
            serde_json::from_str(&provider.get::<String, _>("capabilities_json"))?;
        if provider.get::<i64, _>("enabled") != 1
            || provider.get::<String, _>("provider_type") != "openai_compatible_chat"
            || provider.get::<String, _>("base_url").trim_end_matches('/')
                != "https://generativelanguage.googleapis.com/v1beta/openai"
            || model != "gemini-3.6-flash"
            || provider
                .get::<Option<String>, _>("credential_ref")
                .as_deref()
                != Some(format!("keyring:{provider_id}").as_str())
            || !capabilities.chat_completions
        {
            return Err(AppError::InvalidData);
        }
        let source_provider = SavedRow::read(self.pool(), "model_providers", &provider_id).await?;
        let source_goal_json: String =
            sqlx::query_scalar("SELECT domain_json FROM mobile_goals WHERE id=?")
                .bind(source_goal_id.as_str())
                .fetch_one(self.pool())
                .await?;
        let failed: MobileGoal = serde_json::from_str(&source_goal_json)?;
        if failed.status != MobileGoalStatus::Failed {
            return Err(AppError::InvalidData);
        }
        let target: String = sqlx::query_scalar(
            "SELECT target_json FROM mobile_goal_completion_targets WHERE goal_id=?",
        )
        .bind(source_goal_id.as_str())
        .fetch_one(self.pool())
        .await?;
        let source_observation_id: String = sqlx::query_scalar("SELECT observation_id FROM mobile_model_calls WHERE goal_id=? ORDER BY created_at,id LIMIT 1")
            .bind(source_goal_id.as_str()).fetch_one(self.pool()).await?;
        let observation =
            sqlx::query("SELECT session_id,snapshot_id FROM mobile_observations WHERE id=?")
                .bind(&source_observation_id)
                .fetch_one(self.pool())
                .await?;
        let source_capture_ids = vec![
            (
                "mobile_device_sessions",
                observation.get::<String, _>("session_id"),
            ),
            (
                "mobile_ui_snapshots",
                observation.get::<String, _>("snapshot_id"),
            ),
            ("mobile_observations", source_observation_id.clone()),
        ];
        let mut source_captures = Vec::new();
        for (table, id) in &source_capture_ids {
            source_captures.push(SavedRow::read(self.pool(), table, id).await?);
        }
        let source_settings_json: String =
            sqlx::query_scalar("SELECT value_json FROM settings WHERE key='mobile_runtime'")
                .fetch_one(self.pool())
                .await?;
        let directory = tempfile::tempdir().map_err(|_| AppError::InvalidData)?;
        let mut isolated = Vec::new();
        let mut original_input_sha256 = None;
        for index in 0..LOGICAL_CALLS {
            let replay =
                AppRuntime::initialize(&directory.path().join(format!("planner-{index}.sqlite3")))
                    .await?;
            source_provider.insert(replay.pool()).await?;
            for row in &source_captures {
                row.insert(replay.pool()).await?;
            }
            sqlx::query("INSERT OR REPLACE INTO settings(key,value_json,updated_at) VALUES ('mobile_runtime',?,?)")
                .bind(&source_settings_json).bind(chrono::Utc::now().to_rfc3339()).execute(replay.pool()).await?;
            // Historical source is never resumed. Its input is replayed in a fresh private DB.
            let mut replay_goal = failed.clone();
            replay_goal.status = MobileGoalStatus::Pending;
            replay_goal.started_at = None;
            replay_goal.failed_at = None;
            replay_goal.last_error_code = None;
            replay_goal.last_error_message = None;
            replay_goal.runtime_deadline = None;
            replay_goal.active_plan_id = None;
            replay_goal.updated_at = chrono::Utc::now();
            sqlx::query("INSERT INTO mobile_goals(id,status,created_at,updated_at,domain_json) VALUES (?,'PENDING',?,?,?)")
                .bind(replay_goal.id.as_str()).bind(replay_goal.created_at.to_rfc3339()).bind(replay_goal.updated_at.to_rfc3339())
                .bind(serde_json::to_string(&replay_goal)?).execute(replay.pool()).await?;
            replay
                .mobile_goal_repository()
                .set_completion_target(&replay_goal.id, &serde_json::from_str(&target)?)
                .await?;
            let mut context = replay
                .mobile_goal_repository()
                .planner_context(&replay_goal.id, Some(&source_observation_id))
                .await?;
            context.goal_status = MobileGoalStatus::Planning;
            let input = fingerprint(&context)?;
            if original_input_sha256
                .as_ref()
                .is_some_and(|first| first != &input)
            {
                return Err(AppError::InvalidData);
            }
            original_input_sha256 = Some(input);
            isolated.push(replay);
        }
        let session = PlannerAcceptanceSession {
            readiness: PlannerAcceptanceReadiness {
                provider_count: 1,
                provider_id,
                model,
                provider_metadata_sha256: fingerprint(&source_provider)?,
                original_input_sha256: original_input_sha256.ok_or(AppError::InvalidData)?,
                source_goal_id: source_goal_id.as_str().to_owned(),
                android_actions_executed: 0,
            },
            source_goal_json,
            source_settings_json,
            source_provider,
            source_captures,
            source_capture_ids,
            source_observation_id,
            production_actions_before: action_count(self.pool()).await?,
            isolated,
            _directory: directory,
            used: false,
        };
        self.verify_acceptance_source(&session).await?;
        Ok(session)
    }
    async fn verify_acceptance_source(
        &self,
        acceptance_session: &PlannerAcceptanceSession,
    ) -> Result<(), AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_providers")
            .fetch_one(self.pool())
            .await?;
        let goal: String = sqlx::query_scalar("SELECT domain_json FROM mobile_goals WHERE id=?")
            .bind(&acceptance_session.readiness.source_goal_id)
            .fetch_one(self.pool())
            .await?;
        let settings: String =
            sqlx::query_scalar("SELECT value_json FROM settings WHERE key='mobile_runtime'")
                .fetch_one(self.pool())
                .await?;
        if count != 1
            || goal != acceptance_session.source_goal_json
            || settings != acceptance_session.source_settings_json
            || SavedRow::read(
                self.pool(),
                "model_providers",
                &acceptance_session.readiness.provider_id,
            )
            .await?
                != acceptance_session.source_provider
            || action_count(self.pool()).await? != acceptance_session.production_actions_before
        {
            return Err(AppError::InvalidData);
        }
        for ((table, id), expected) in acceptance_session
            .source_capture_ids
            .iter()
            .zip(&acceptance_session.source_captures)
        {
            if SavedRow::read(self.pool(), table, id).await? != *expected {
                return Err(AppError::InvalidData);
            }
        }
        Ok(())
    }
    /// Exactly five new logical calls through the production Planner Service.
    /// The desktop supplies its internal SystemCredentialStore callback, never user key input.
    pub async fn run_internal_planner_acceptance<F>(
        &self,
        acceptance_session: &mut PlannerAcceptanceSession,
        credentials: F,
    ) -> Result<PlannerAcceptanceReport, AppError>
    where
        F: Fn(&str) -> Result<Option<String>, AppError>,
    {
        self.run_internal_planner_acceptance_with_pacing(
            acceptance_session,
            credentials,
            Duration::from_secs(60),
            Duration::from_secs(5),
        )
        .await
    }
    async fn run_internal_planner_acceptance_with_pacing<F>(
        &self,
        acceptance_session: &mut PlannerAcceptanceSession,
        credentials: F,
        cooldown: Duration,
        pacing: Duration,
    ) -> Result<PlannerAcceptanceReport, AppError>
    where
        F: Fn(&str) -> Result<Option<String>, AppError>,
    {
        let _planner_guard = self
            .planner_guard
            .try_lock()
            .map_err(|_| AppError::InvalidData)?;
        let _runner_guard = self
            .mobile_run_guard
            .try_lock()
            .map_err(|_| AppError::InvalidData)?;
        if acceptance_session.used {
            return Err(AppError::InvalidData);
        }
        self.verify_acceptance_source(acceptance_session).await?;
        acceptance_session.used = true;
        let goal_id: MobileGoalId = serde_json::from_value(serde_json::Value::String(
            acceptance_session.readiness.source_goal_id.clone(),
        ))?;
        let mut logical_calls = Vec::new();
        let mut next_wait = cooldown;
        let mut stop_reason = None;
        for (index, replay) in acceptance_session.isolated.iter().enumerate() {
            tokio::select! {
                biased;
                _ = self.planner_cancellation.cancelled() => return Err(mobile_runtime::execution::MobileGoalError::new(MobileGoalErrorCode::UserStopped).into()),
                _ = tokio::time::sleep(next_wait) => {},
            }
            self.verify_acceptance_source(acceptance_session).await?;
            let started_at = chrono::Utc::now().to_rfc3339();
            let wait_before_call_ms = next_wait.as_millis() as u64;
            let repo = replay.mobile_goal_repository();
            repo.begin_autonomous_goal(&goal_id).await?;
            let context = repo
                .planner_context(&goal_id, Some(&acceptance_session.source_observation_id))
                .await?;
            if fingerprint(&context)? != acceptance_session.readiness.original_input_sha256 {
                return Err(AppError::InvalidData);
            }
            let observer = PlannerAttemptObserver::default();
            let outcome = replay
                .plan_mobile_goal_observed(
                    &goal_id,
                    Some(&acceptance_session.source_observation_id),
                    &credentials,
                    Some(&observer),
                )
                .await;
            let attempts = observer.snapshot();
            let budgeted: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM mobile_model_calls WHERE goal_id=?")
                    .bind(goal_id.as_str())
                    .fetch_one(replay.pool())
                    .await?;
            let actions = action_count(replay.pool()).await?;
            if actions != 0 || budgeted != attempts.len() as i64 {
                return Err(AppError::InvalidData);
            }
            let error_code = match &outcome {
                Ok(_) => None,
                Err(AppError::MobileGoal(error)) => Some(error.code),
                Err(_) => Some(MobileGoalErrorCode::InvalidPlan),
            };
            let failure_class = match error_code {
                Some(MobileGoalErrorCode::InvalidModelOutput) => Some("INVALID_RESPONSE"),
                Some(MobileGoalErrorCode::ModelError) => attempts
                    .last()
                    .and_then(|a| a.failure_class)
                    .or(Some("TIMEOUT")),
                _ => None,
            };
            next_wait = pacing.max(Duration::from_millis(
                attempts
                    .iter()
                    .filter_map(|a| a.transport.retry_after_ms)
                    .max()
                    .unwrap_or(0),
            ));
            let rate_limited = outcome.is_err() && failure_class == Some("RATE_LIMITED");
            logical_calls.push(PlannerAcceptanceLogicalCall {
                logical_call: index + 1,
                started_at,
                wait_before_call_ms,
                failure_class,
                passed: outcome.is_ok(),
                decision: outcome.as_ref().ok().map(|result| result.decision),
                error_code,
                attempts,
                budgeted_model_calls: budgeted as u64,
                android_actions_executed: 0,
            });
            if rate_limited {
                stop_reason = Some("FREE_TIER_RATE_LIMIT_BLOCKED");
                break;
            }
        }
        self.verify_acceptance_source(acceptance_session).await?;
        let passed = logical_calls.iter().filter(|call| call.passed).count();
        let http_attempts = logical_calls.iter().map(|call| call.attempts.len()).sum();
        let http_503_retries = logical_calls
            .iter()
            .map(|call| {
                call.attempts
                    .windows(2)
                    .filter(|pair| pair[0].transport.http_status == Some(503))
                    .count()
            })
            .sum();
        let http_429_count = logical_calls
            .iter()
            .flat_map(|call| &call.attempts)
            .filter(|a| a.transport.http_status == Some(429))
            .count();
        let http_503_count = logical_calls
            .iter()
            .flat_map(|call| &call.attempts)
            .filter(|a| a.transport.http_status == Some(503))
            .count();
        Ok(PlannerAcceptanceReport {
            readiness: acceptance_session.readiness.clone(),
            logical_calls,
            passed,
            http_attempts,
            http_503_retries,
            http_429_count,
            http_503_count,
            stop_reason,
            production_data_unchanged: true,
            android_actions_executed: 0,
        })
    }
}

#[cfg(test)]
mod tests;
