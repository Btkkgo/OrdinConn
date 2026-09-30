//! Real Model Gateway planner only. No MobileHost, execution adapter, or mock fallback.
use crate::{
    AppError, AppRuntime,
    events::{RuntimeEventBus, append_event},
    mobile_goals::MobileGoalRepository,
};
use mobile_runtime::{execution::*, planner::*};
use model_gateway::{
    ModelEvent, ModelProviderAdapter, ModelRole, OpenAiCompatibleChatAdapter, ProviderCapabilities,
    StructuredOutputRequest, UnifiedMessage, UnifiedModelRequest,
};
use serde_json::json;
use sqlx::{Row, SqlitePool};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

fn error(code: MobileGoalErrorCode) -> AppError {
    MobileGoalError::new(code).into()
}
mod retry;
/// Bounded exponential delay; production and diagnostics share this exact policy.
pub fn planner_retry_delay(error: &model_gateway::ModelError, attempt: u32) -> Option<Duration> {
    retry::Backoff::delay(&retry::ExponentialBackoff, error, attempt)
}
/// The same strict request builder is used by the production service and read-only diagnostics.
pub fn build_planner_request(
    context: &MobilePlannerContext,
    model: &str,
    timeout: Duration,
) -> Result<UnifiedModelRequest, AppError> {
    let content = serde_json::to_string(context)?;
    if content.len() > MAX_CONTEXT_BYTES {
        return Err(error(MobileGoalErrorCode::InvalidPlan));
    }
    let schema = decision_schema();
    Ok(UnifiedModelRequest {
        model: model.to_owned(),
        messages: vec![
            UnifiedMessage {
                role: ModelRole::System,
                content: format!("{SYSTEM_PROMPT}\nRequired JSON schema: {schema}"),
            },
            UnifiedMessage {
                role: ModelRole::User,
                content,
            },
        ],
        tools: vec![],
        context: Default::default(),
        temperature: 0.0,
        structured_output: Some(StructuredOutputRequest {
            name: SCHEMA_VERSION.into(),
            schema,
        }),
        max_output_tokens: Some(2048),
        timeout_ms: Some(timeout.as_millis().min(PLANNER_TIMEOUT_MS as u128) as u64),
        max_response_bytes: Some(65_536),
    })
}
struct ResolvedProvider {
    id: String,
    model: String,
    base_url: String,
    credential_required: bool,
    capabilities: ProviderCapabilities,
}
async fn resolve_provider(pool: &SqlitePool) -> Result<ResolvedProvider, AppError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_providers")
        .fetch_one(pool)
        .await?;
    if count == 0 {
        return Err(error(MobileGoalErrorCode::ModelNotConfigured));
    }
    let rows=sqlx::query("SELECT id,default_model,provider_type,base_url,credential_ref,capabilities_json FROM model_providers WHERE enabled=1 LIMIT 2").fetch_all(pool).await?;
    if rows.len() != 1 {
        return Err(error(MobileGoalErrorCode::ModelNotSelected));
    }
    let row = &rows[0];
    let model: String = row.get("default_model");
    if model.trim().is_empty() {
        return Err(error(MobileGoalErrorCode::ModelNotSelected));
    }
    let capabilities: ProviderCapabilities =
        serde_json::from_str(&row.get::<String, _>("capabilities_json"))
            .map_err(|_| error(MobileGoalErrorCode::ModelError))?;
    let base_url: String = row.get("base_url");
    if row.get::<String, _>("provider_type") != "openai_compatible_chat"
        || !capabilities.chat_completions
        || !(base_url.starts_with("https://") || base_url.starts_with("http://"))
    {
        return Err(error(MobileGoalErrorCode::ModelError));
    }
    // IDs/model are metadata, never credentials or free-form provider errors.
    let id: String = row.get("id");
    if id.len() > 128
        || model.len() > 128
        || id.chars().chain(model.chars()).any(|c| c.is_control())
    {
        return Err(error(MobileGoalErrorCode::ModelError));
    }
    Ok(ResolvedProvider {
        id,
        model,
        base_url,
        credential_required: row.get::<Option<String>, _>("credential_ref").is_some(),
        capabilities,
    })
}
pub(crate) async fn check_production_provider(pool: &SqlitePool) -> Result<(), AppError> {
    resolve_provider(pool).await.map(|_| ())
}
/// Also usable with a read-only pool for the zero-provider gate; no migrations on construction.
pub struct MobilePlanner {
    pool: SqlitePool,
    repository: MobileGoalRepository,
    events: RuntimeEventBus,
    cancellation: CancellationToken,
}

/// Safe transport metadata for the opt-in, in-process acceptance harness only.
#[derive(Clone, Debug, serde::Serialize)]
pub struct PlannerHttpAttempt {
    pub attempt: u32,
    pub transport: model_gateway::ChatCompletionDiagnostics,
    pub gateway_result: &'static str,
    pub retry_delay_ms: Option<u64>,
    pub failure_class: Option<&'static str>,
}
#[derive(Default)]
pub(crate) struct PlannerAttemptObserver(Mutex<Vec<PlannerHttpAttempt>>);
impl PlannerAttemptObserver {
    pub(crate) fn snapshot(&self) -> Vec<PlannerHttpAttempt> {
        self.0.lock().expect("acceptance observer poisoned").clone()
    }
}
struct ObservedGatewayAdapter<'a> {
    inner: OpenAiCompatibleChatAdapter,
    observer: &'a PlannerAttemptObserver,
}
#[async_trait::async_trait]
impl ModelProviderAdapter for ObservedGatewayAdapter<'_> {
    fn capabilities(&self) -> &ProviderCapabilities {
        self.inner.capabilities()
    }
    async fn complete(
        &self,
        request: &UnifiedModelRequest,
        key: Option<&str>,
    ) -> Result<Vec<ModelEvent>, model_gateway::ModelError> {
        let index = {
            let mut attempts = self
                .observer
                .0
                .lock()
                .expect("acceptance observer poisoned");
            let index = attempts.len();
            attempts.push(PlannerHttpAttempt {
                attempt: index as u32 + 1,
                transport: Default::default(),
                gateway_result: "INCOMPLETE",
                retry_delay_ms: None,
                failure_class: None,
            });
            index
        };
        let (result, transport) = self.inner.complete_with_diagnostics(request, key).await;
        let gateway_result = match &result {
            Ok(_) => "PASS",
            Err(model_gateway::ModelError::HttpRejected { .. }) => "HTTP_REJECTED",
            Err(model_gateway::ModelError::MalformedResponse(_)) => "MALFORMED_RESPONSE",
            Err(model_gateway::ModelError::Timeout) => "TIMEOUT",
            Err(model_gateway::ModelError::Network(_)) => "NETWORK_ERROR",
            Err(_) => "GATEWAY_ERROR",
        };
        self.observer
            .0
            .lock()
            .expect("acceptance observer poisoned")[index] = PlannerHttpAttempt {
            attempt: index as u32 + 1,
            transport,
            gateway_result,
            retry_delay_ms: None,
            failure_class: result
                .as_ref()
                .err()
                .map(model_gateway::ModelError::diagnostic_class),
        };
        result
    }
    async fn stream(
        &self,
        _: &UnifiedModelRequest,
        _: Option<&str>,
        _: tokio::sync::mpsc::Sender<ModelEvent>,
    ) -> Result<(), model_gateway::ModelError> {
        Err(model_gateway::ModelError::UnsupportedCapability(
            "acceptance streaming",
        ))
    }
}
impl AppRuntime {
    pub async fn plan_mobile_goal<F>(
        &self,
        id: &MobileGoalId,
        observation_id: Option<&str>,
        credentials: F,
    ) -> Result<MobilePlannerOutcome, AppError>
    where
        F: Fn(&str) -> Result<Option<String>, AppError>,
    {
        self.plan_mobile_goal_observed(id, observation_id, credentials, None)
            .await
    }
    pub(crate) async fn plan_mobile_goal_observed<F>(
        &self,
        id: &MobileGoalId,
        observation_id: Option<&str>,
        credentials: F,
        observer: Option<&PlannerAttemptObserver>,
    ) -> Result<MobilePlannerOutcome, AppError>
    where
        F: Fn(&str) -> Result<Option<String>, AppError>,
    {
        let _guard = self
            .planner_guard
            .try_lock()
            .map_err(|_| error(MobileGoalErrorCode::InvalidStateTransition))?;
        MobilePlanner::new(
            self.pool().clone(),
            self.event_bus.clone(),
            self.mobile_cancellation_token(),
        )
        .plan_mobile_goal_observed(id, observation_id, credentials, observer)
        .await
    }
}
impl MobilePlanner {
    pub fn new(pool: SqlitePool, events: RuntimeEventBus, cancellation: CancellationToken) -> Self {
        Self {
            repository: MobileGoalRepository::from_pool(pool.clone(), events.clone()),
            pool,
            events,
            cancellation,
        }
    }
    pub async fn plan_mobile_goal<F>(
        &self,
        id: &MobileGoalId,
        observation_id: Option<&str>,
        credentials: F,
    ) -> Result<MobilePlannerOutcome, AppError>
    where
        F: Fn(&str) -> Result<Option<String>, AppError>,
    {
        self.plan_mobile_goal_observed(id, observation_id, credentials, None)
            .await
    }
    pub(crate) async fn plan_mobile_goal_observed<F>(
        &self,
        id: &MobileGoalId,
        observation_id: Option<&str>,
        credentials: F,
        observer: Option<&PlannerAttemptObserver>,
    ) -> Result<MobilePlannerOutcome, AppError>
    where
        F: Fn(&str) -> Result<Option<String>, AppError>,
    {
        // This gate precedes Goal lookup/writes so an unmigrated production DB stays read-only.
        if self.cancellation.is_cancelled() {
            return Err(error(MobileGoalErrorCode::UserStopped));
        }
        let provider = resolve_provider(&self.pool).await?;
        let context = self.repository.planner_context(id, observation_id).await?;
        let key = if provider.credential_required {
            Some(
                credentials(&provider.id)
                    .map_err(|_| error(MobileGoalErrorCode::ModelError))?
                    .filter(|key| !key.is_empty())
                    .ok_or_else(|| error(MobileGoalErrorCode::ModelNotConfigured))?,
            )
        } else {
            None
        };
        let adapter =
            OpenAiCompatibleChatAdapter::new(&provider.base_url, provider.capabilities.clone());
        if let Some(observer) = observer {
            let observed = ObservedGatewayAdapter {
                inner: adapter,
                observer,
            };
            return self
                .run_with_backoff(
                    &context,
                    &provider,
                    &observed,
                    key.as_deref(),
                    Duration::from_millis(PLANNER_TIMEOUT_MS),
                    &retry::ExponentialBackoff,
                    Some(observer),
                )
                .await;
        }
        self.run(
            &context,
            &provider,
            &adapter,
            key.as_deref(),
            Duration::from_millis(PLANNER_TIMEOUT_MS),
        )
        .await
    }
    async fn audit(
        &self,
        name: &str,
        context: &MobilePlannerContext,
        provider: &ResolvedProvider,
        code: Option<MobileGoalErrorCode>,
        duration_ms: u128,
        failure_class: Option<&str>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let event=append_event(&mut tx,name,"mobile_goal",context.goal_id.as_str(),None,None,Some(context.goal_id.as_str()),&json!({"goalId":context.goal_id,"observationId":context.current_observation.as_ref().map(|o|&o.observation_id),"plannerVersion":PLANNER_VERSION,"schemaVersion":SCHEMA_VERSION,"modelProviderId":provider.id,"modelName":provider.model,"errorCode":code,"providerFailure":failure_class,"durationMs":duration_ms.min(u64::MAX as u128) as u64})).await?;
        tx.commit().await?;
        self.events.publish(event);
        Ok(())
    }
    async fn run(
        &self,
        context: &MobilePlannerContext,
        provider: &ResolvedProvider,
        adapter: &dyn ModelProviderAdapter,
        key: Option<&str>,
        timeout: Duration,
    ) -> Result<MobilePlannerOutcome, AppError> {
        self.run_with_backoff(
            context,
            provider,
            adapter,
            key,
            timeout,
            &retry::ExponentialBackoff,
            None,
        )
        .await
    }
    #[allow(clippy::too_many_arguments)]
    async fn run_with_backoff(
        &self,
        context: &MobilePlannerContext,
        provider: &ResolvedProvider,
        adapter: &dyn ModelProviderAdapter,
        key: Option<&str>,
        timeout: Duration,
        backoff: &dyn retry::Backoff,
        observer: Option<&PlannerAttemptObserver>,
    ) -> Result<MobilePlannerOutcome, AppError> {
        let request = build_planner_request(context, &provider.model, timeout)?;
        let start = Instant::now();
        self.audit("mobile.planner_started", context, provider, None, 0, None)
            .await?;
        let deadline =
            tokio::time::Instant::now() + timeout.min(Duration::from_millis(PLANNER_TIMEOUT_MS));
        let mut failure_class = None;
        let attempt_result = tokio::select! {
            biased;
            _=self.cancellation.cancelled()=>Err(error(MobileGoalErrorCode::UserStopped)),
            result=tokio::time::timeout_at(deadline,async {
                for attempt in 0..MAX_PLANNER_ATTEMPTS {
                    if self.cancellation.is_cancelled() { return Err(error(MobileGoalErrorCode::UserStopped)); }
                    if tokio::time::Instant::now() >= deadline { failure_class=Some("TIMEOUT"); return Err(error(MobileGoalErrorCode::ModelError)); }
                    self.repository.reserve_model_call(context).await?;
                    let events=match adapter.complete(&request,key).await {
                        Ok(events)=>events,
                        Err(e)=>{
                            failure_class = Some(e.diagnostic_class());
                            let delay = backoff.delay(&e, attempt);
                            if let Some(observer) = observer {
                                if let Some(last) = observer.0.lock().expect("acceptance observer poisoned").last_mut() {
                                    last.retry_delay_ms = delay.map(|d| d.as_millis() as u64);
                                }
                            }
                            let status = match &e { model_gateway::ModelError::HttpRejected { status, .. } => Some(*status), _ => None };
                            let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
                            let event = append_event(&mut tx, "mobile.provider_attempt", "mobile_goal", context.goal_id.as_str(), None, None, Some(context.goal_id.as_str()), &json!({"attempt":attempt+1,"httpStatus":status,"providerFailure":failure_class,"retryDelayMs":delay.map(|d|d.as_millis() as u64)})).await?;
                            tx.commit().await?;
                            self.events.publish(event);
                            if let Some(delay) = delay {
                                backoff.wait(delay).await;
                                continue;
                            }
                            return Err(error(MobileGoalErrorCode::ModelError));
                        },
                    };
                    failure_class = Some("INVALID_RESPONSE");
                    let mut raw=String::new();
                    for event in events {
                        match event {
                            ModelEvent::MessageDelta{text}=>{
                                if text.len()>MAX_PLANNER_OUTPUT_BYTES.saturating_sub(raw.len()) {return Err(error(MobileGoalErrorCode::InvalidModelOutput));}
                                raw.push_str(&text);
                            },
                            ModelEvent::Usage{..}|ModelEvent::Completed=>{},
                            _=>return Err(error(MobileGoalErrorCode::InvalidModelOutput)),
                        }
                    }
                    if key.is_some_and(|secret| !secret.is_empty() && raw.contains(secret)) {return Err(error(MobileGoalErrorCode::PolicyBlocked));}
                    match decode_decision(&raw,context) {
                        Ok(decision)=>{failure_class=None; return Ok(decision)},
                        Err(e)=>return Err(e.into()),
                    }
                }
                Err(error(MobileGoalErrorCode::InvalidModelOutput))
            })=>match result { Ok(value)=>value, Err(_)=>{failure_class=Some("TIMEOUT"); Err(error(MobileGoalErrorCode::ModelError))} },
        };
        let outcome = match attempt_result {
            Ok(decision) => {
                if self.cancellation.is_cancelled() {
                    Err(error(MobileGoalErrorCode::UserStopped))
                } else {
                    self.repository
                        .persist_planner_decision(
                            context,
                            decision,
                            &provider.id,
                            &provider.model,
                            start.elapsed().as_millis() as u64,
                        )
                        .await
                }
            }
            Err(e) => Err(e),
        };
        if let Err(e) = &outcome {
            let code = if let AppError::MobileGoal(e) = e {
                e.code
            } else {
                MobileGoalErrorCode::InvalidPlan
            };
            self.audit(
                "mobile.planner_failed",
                context,
                provider,
                Some(code),
                start.elapsed().as_millis(),
                failure_class,
            )
            .await?;
        }
        outcome
    }
}

#[cfg(test)]
mod tests;
