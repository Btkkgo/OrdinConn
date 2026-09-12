use agent_runtime::{AgentRuntime, PageContext, build_contextual_request};
use approval_engine::{ApprovalEngine, ApprovalError, ApprovalRequest};
use chrono::{Duration, Utc};
use connector_runtime::{ConnectorDescriptor, MockConnectorSet};
use evidence_core::Evidence;
use execution_core::{
    OrderType, PaperExecutionAdapter, ProposalStatus, TradeAction, TradeProposal,
};
use model_gateway::{MockModelAdapter, ModelEvent, ModelProviderAdapter};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use signal_core::{Signal, SignalDirection};
use sqlx::{Row, SqlitePool};
use std::{
    path::Path,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use thiserror::Error;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    db::open_database,
    events::{AuditEventView, RuntimeEventBus, RuntimeEventEnvelope, append_event},
};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("database migration error")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("serialization error")]
    Serialization(#[from] serde_json::Error),
    #[error("approval rejected: {0}")]
    Approval(#[from] ApprovalError),
    #[error("execution rejected")]
    Execution,
    #[error("record not found: {0}")]
    NotFound(&'static str),
    #[error("invalid stored data")]
    InvalidData,
    #[error("runtime is shutting down")]
    ShuttingDown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceView {
    pub id: String,
    pub source: String,
    pub source_type: String,
    pub market: String,
    pub asset: String,
    pub title: String,
    pub content: String,
    pub captured_at: String,
    pub freshness: f64,
    pub reliability: f64,
    pub factual_level: String,
    pub confidence: f64,
    pub relation: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalView {
    pub id: String,
    pub market: String,
    pub category: String,
    pub asset: String,
    pub title: String,
    pub summary: String,
    pub direction: String,
    pub confidence: f64,
    pub urgency: f64,
    pub time_horizon: String,
    pub evidence_quality: f64,
    pub evidence: Vec<EvidenceView>,
    pub catalysts: Vec<String>,
    pub risks: Vec<String>,
    pub invalidation_conditions: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub agent_id: String,
    pub model_id: String,
    pub status: String,
}

impl From<Signal> for SignalView {
    fn from(signal: Signal) -> Self {
        let evidence = signal
            .evidence
            .iter()
            .map(EvidenceView::from_link)
            .collect();
        Self {
            id: signal.id,
            market: signal.asset.market.as_str().into(),
            category: signal.category.as_str().into(),
            asset: signal.asset.symbol,
            title: signal.title,
            summary: signal.summary,
            direction: enum_json(&signal.direction),
            confidence: signal.confidence,
            urgency: signal.urgency,
            time_horizon: signal.time_horizon,
            evidence_quality: signal.evidence_quality,
            evidence,
            catalysts: signal.catalysts,
            risks: signal.risks,
            invalidation_conditions: signal.invalidation_conditions,
            created_at: signal.created_at.to_rfc3339(),
            updated_at: signal.updated_at.to_rfc3339(),
            agent_id: signal.agent_id,
            model_id: signal.model_id,
            status: enum_json(&signal.status),
        }
    }
}

impl EvidenceView {
    fn from_link(link: &signal_core::EvidenceLink) -> Self {
        let item = &link.evidence;
        Self {
            id: item.id.clone(),
            source: item.source.clone(),
            source_type: enum_json(&item.source_type),
            market: item.asset.market.as_str().into(),
            asset: item.asset.symbol.clone(),
            title: item.title.clone(),
            content: item.content.clone(),
            captured_at: item.captured_at.to_rfc3339(),
            freshness: item.freshness,
            reliability: item.reliability,
            factual_level: enum_json(&item.factual_level),
            confidence: item.confidence,
            relation: enum_json(&link.relation),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentReportView {
    pub id: String,
    pub signal_id: String,
    pub executive_summary: String,
    pub what_happened: String,
    pub why_it_matters: String,
    pub evidence: String,
    pub inference: String,
    pub market_impact: String,
    pub bull_case: String,
    pub bear_case: String,
    pub risk: String,
    pub invalidation: String,
    pub time_horizon: String,
    pub possible_actions: String,
    pub watch_conditions: String,
    pub conclusion: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalView {
    pub id: String,
    pub proposal_id: String,
    pub proposal_version: u32,
    pub proposal_hash: String,
    pub proposal_hash_version: u32,
    pub allowed_action: String,
    pub status: String,
    pub issued_at: String,
    pub expires_at: String,
}

impl From<&ApprovalRequest> for ApprovalView {
    fn from(item: &ApprovalRequest) -> Self {
        Self {
            id: item.id.clone(),
            proposal_id: item.proposal_id.clone(),
            proposal_version: item.proposal_version,
            proposal_hash: item.proposal_hash.clone(),
            proposal_hash_version: item.proposal_hash_version,
            allowed_action: item.allowed_action.clone(),
            status: enum_json(&item.status),
            issued_at: item.issued_at.to_rfc3339(),
            expires_at: item.expires_at.to_rfc3339(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionView {
    pub id: String,
    pub proposal_id: String,
    pub approval_request_id: String,
    pub adapter_id: String,
    pub status: String,
    pub result_summary: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProviderView {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub default_model: String,
    pub enabled: bool,
    pub credential_configured: bool,
    pub temperature: f32,
    pub context_window: u32,
    pub capabilities: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProviderConfig {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub credential_ref: Option<String>,
    pub default_model: String,
    pub temperature: f32,
    pub context_window: u32,
    pub enabled: bool,
    pub capabilities: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub signals: Vec<SignalView>,
    pub connectors: Vec<ConnectorDescriptor>,
    pub providers: Vec<ModelProviderView>,
    pub pending_approvals: Vec<ApprovalView>,
    pub recent_executions: Vec<ExecutionView>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTaskStarted {
    pub thread_id: String,
    pub turn_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentItemView {
    pub id: String,
    pub turn_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub model_id: Option<String>,
    pub mock: bool,
}

pub struct AppRuntime {
    pool: SqlitePool,
    agent_runtime: AgentRuntime,
    approval_engine: ApprovalEngine,
    execution_guard: Mutex<()>,
    event_bus: RuntimeEventBus,
    accepting_tasks: AtomicBool,
}

impl AppRuntime {
    pub async fn initialize(path: &Path) -> Result<Arc<Self>, AppError> {
        let pool = open_database(path).await?;
        let stored = load_approval_requests(&pool).await?;
        let runtime = Arc::new(Self {
            pool,
            agent_runtime: AgentRuntime::default(),
            approval_engine: ApprovalEngine::from_requests(stored),
            execution_guard: Mutex::new(()),
            event_bus: RuntimeEventBus::default(),
            accepting_tasks: AtomicBool::new(true),
        });
        runtime.recover_interrupted_tasks().await?;
        Ok(runtime)
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<RuntimeEventEnvelope> {
        self.event_bus.subscribe()
    }

    pub async fn shutdown(&self) -> Result<(), AppError> {
        self.accepting_tasks.store(false, Ordering::Release);
        self.agent_runtime.cancel_all().await;
        let mut transaction = self.pool.begin().await?;
        let rows = sqlx::query(
            "SELECT id,thread_id FROM agent_turns WHERE status IN ('running','waiting_tool')",
        )
        .fetch_all(&mut *transaction)
        .await?;
        let mut events = Vec::new();
        for row in rows {
            let turn_id: String = row.get("id");
            let thread_id: String = row.get("thread_id");
            sqlx::query("UPDATE agent_turns SET status='interrupted',completed_at=? WHERE id=?")
                .bind(Utc::now().to_rfc3339())
                .bind(&turn_id)
                .execute(&mut *transaction)
                .await?;
            events.push(
                append_event(
                    &mut transaction,
                    "runtime.task_interrupted",
                    "thread",
                    &thread_id,
                    Some(&thread_id),
                    Some(&turn_id),
                    Some(&turn_id),
                    &json!({"reason": "orderly_shutdown"}),
                )
                .await?,
            );
        }
        transaction.commit().await?;
        for event in events {
            self.event_bus.publish(event);
        }
        self.pool.close().await;
        Ok(())
    }

    pub async fn seed_demo_data(&self) -> Result<(), AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM signals")
            .fetch_one(&self.pool)
            .await?;
        if count > 0 {
            return Ok(());
        }
        let output = MockConnectorSet
            .run()
            .await
            .map_err(|_| AppError::InvalidData)?;
        let mut transaction = self.pool.begin().await?;
        for connector in &output.connectors {
            sqlx::query("INSERT INTO connectors (id,name,market,connector_type,capabilities_json,auth_type,status,reliability,last_update,is_mock) VALUES (?,?,?,?,?,?,?,?,?,1)")
                .bind(&connector.id).bind(&connector.name).bind(&connector.market).bind(&connector.connector_type)
                .bind(serde_json::to_string(&connector.capabilities)?).bind(&connector.auth_type)
                .bind(&connector.status).bind(connector.reliability).bind(&connector.last_update)
                .execute(&mut *transaction).await?;
        }
        for evidence in &output.evidence {
            insert_evidence(&mut transaction, evidence).await?;
        }
        let mut emitted = Vec::new();
        for signal in &output.signals {
            sqlx::query("INSERT INTO signal_candidates (id,market,category,asset,title,summary,status,created_at) VALUES (?,?,?,?,?,?,?,?)")
                .bind(&signal.candidate_id).bind(signal.market()).bind(signal.category.as_str())
                .bind(&signal.asset.symbol).bind(&signal.title).bind(&signal.summary)
                .bind("published").bind(signal.created_at.to_rfc3339())
                .execute(&mut *transaction).await?;
            sqlx::query("INSERT INTO signals (id,candidate_id,market,category,asset,title,summary,direction,confidence,urgency,time_horizon,evidence_quality,agent_id,model_id,status,created_at,updated_at,domain_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
                .bind(&signal.id).bind(&signal.candidate_id).bind(signal.market()).bind(signal.category.as_str())
                .bind(&signal.asset.symbol).bind(&signal.title).bind(&signal.summary).bind(enum_json(&signal.direction))
                .bind(signal.confidence).bind(signal.urgency).bind(&signal.time_horizon).bind(signal.evidence_quality)
                .bind(&signal.agent_id).bind(&signal.model_id).bind(enum_json(&signal.status))
                .bind(signal.created_at.to_rfc3339()).bind(signal.updated_at.to_rfc3339())
                .bind(serde_json::to_string(signal)?).execute(&mut *transaction).await?;
            for link in &signal.evidence {
                sqlx::query(
                    "INSERT INTO signal_evidence (signal_id,evidence_id,relation) VALUES (?,?,?)",
                )
                .bind(&signal.id)
                .bind(&link.evidence.id)
                .bind(enum_json(&link.relation))
                .execute(&mut *transaction)
                .await?;
            }
            emitted.push(
                append_event(
                    &mut transaction,
                    "signal.created",
                    "signal",
                    &signal.id,
                    None,
                    None,
                    Some(&signal.id),
                    &json!({"market": signal.market(), "category": signal.category.as_str()}),
                )
                .await?,
            );
        }
        transaction.commit().await?;
        for event in emitted {
            self.event_bus.publish(event);
        }
        Ok(())
    }

    pub async fn snapshot(&self) -> Result<AppSnapshot, AppError> {
        let rows = sqlx::query("SELECT domain_json FROM signals ORDER BY created_at,id")
            .fetch_all(&self.pool)
            .await?;
        let signals = rows
            .into_iter()
            .map(|row| serde_json::from_str::<Signal>(row.get("domain_json")).map(SignalView::from))
            .collect::<Result<Vec<_>, _>>()?;
        let rows = sqlx::query("SELECT id,name,market,connector_type,capabilities_json,auth_type,status,reliability,last_update,is_mock FROM connectors ORDER BY id").fetch_all(&self.pool).await?;
        let connectors = rows
            .into_iter()
            .map(|row| {
                Ok(ConnectorDescriptor {
                    id: row.get("id"),
                    name: row.get("name"),
                    market: row.get("market"),
                    connector_type: row.get("connector_type"),
                    capabilities: serde_json::from_str(row.get("capabilities_json"))?,
                    auth_type: row.get("auth_type"),
                    status: row.get("status"),
                    reliability: row.get("reliability"),
                    last_update: row.get("last_update"),
                    mock: row.get::<i64, _>("is_mock") != 0,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let rows = sqlx::query("SELECT id,name,provider_type,base_url,credential_ref,default_model,temperature,context_window,enabled,capabilities_json FROM model_providers ORDER BY name").fetch_all(&self.pool).await?;
        let providers = rows
            .into_iter()
            .map(|row| {
                Ok(ModelProviderView {
                    id: row.get("id"),
                    name: row.get("name"),
                    provider_type: row.get("provider_type"),
                    base_url: row.get("base_url"),
                    default_model: row.get("default_model"),
                    enabled: row.get::<i64, _>("enabled") != 0,
                    credential_configured: row.get::<Option<String>, _>("credential_ref").is_some(),
                    temperature: row.get("temperature"),
                    context_window: row.get::<i64, _>("context_window") as u32,
                    capabilities: serde_json::from_str(row.get("capabilities_json"))?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let pending_approvals = load_approval_requests(&self.pool)
            .await?
            .iter()
            .filter(|item| matches!(item.status, approval_engine::ApprovalStatus::Requested))
            .map(ApprovalView::from)
            .collect();
        let rows = sqlx::query("SELECT id,proposal_id,approval_request_id,adapter_id,status,result_summary,created_at,completed_at FROM execution_records ORDER BY created_at DESC LIMIT 20").fetch_all(&self.pool).await?;
        let recent_executions = rows.into_iter().map(execution_view).collect();
        Ok(AppSnapshot {
            signals,
            connectors,
            providers,
            pending_approvals,
            recent_executions,
        })
    }

    pub async fn upsert_model_provider(
        &self,
        config: ModelProviderConfig,
    ) -> Result<ModelProviderView, AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO model_providers (id,name,provider_type,base_url,credential_ref,default_model,temperature,context_window,enabled,capabilities_json,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,provider_type=excluded.provider_type,base_url=excluded.base_url,credential_ref=excluded.credential_ref,default_model=excluded.default_model,temperature=excluded.temperature,context_window=excluded.context_window,enabled=excluded.enabled,capabilities_json=excluded.capabilities_json,updated_at=excluded.updated_at")
            .bind(&config.id).bind(&config.name).bind(&config.provider_type).bind(&config.base_url)
            .bind(&config.credential_ref).bind(&config.default_model).bind(config.temperature).bind(config.context_window as i64).bind(i64::from(config.enabled))
            .bind(serde_json::to_string(&config.capabilities)?).bind(&now).bind(&now)
            .execute(&self.pool).await?;
        Ok(ModelProviderView {
            id: config.id,
            name: config.name,
            provider_type: config.provider_type,
            base_url: config.base_url,
            default_model: config.default_model,
            enabled: config.enabled,
            credential_configured: config.credential_ref.is_some(),
            temperature: config.temperature,
            context_window: config.context_window,
            capabilities: config.capabilities,
        })
    }

    pub async fn start_agent_turn(
        self: &Arc<Self>,
        thread_id: Option<String>,
        question: &str,
        context: PageContext,
    ) -> Result<AgentTaskStarted, AppError> {
        if !self.accepting_tasks.load(Ordering::Acquire) {
            return Err(AppError::ShuttingDown);
        }
        let thread_id = thread_id.unwrap_or_else(|| format!("thread_{}", Uuid::now_v7()));
        let handle = self
            .agent_runtime
            .start_turn(thread_id.clone())
            .await
            .map_err(|_| AppError::InvalidData)?;
        let user_item_id = format!("item_{}", Uuid::now_v7());
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool.begin().await?;
        sqlx::query("INSERT OR IGNORE INTO agent_threads (id,title,status,created_at,updated_at) VALUES (?,?,'active',?,?)")
            .bind(&thread_id).bind("Contextual analysis").bind(&now).bind(&now)
            .execute(&mut *transaction).await?;
        sqlx::query(
            "INSERT INTO agent_turns (id,thread_id,status,created_at) VALUES (?,?,'running',?)",
        )
        .bind(&handle.turn_id)
        .bind(&thread_id)
        .bind(&now)
        .execute(&mut *transaction)
        .await?;
        sqlx::query("INSERT INTO agent_items (id,turn_id,item_type,status,content,created_at) VALUES (?,?,'user','completed',?,?)")
            .bind(&user_item_id).bind(&handle.turn_id).bind(question).bind(&now)
            .execute(&mut *transaction).await?;
        let event = append_event(
            &mut transaction,
            "agent.turn_started",
            "thread",
            &thread_id,
            Some(&thread_id),
            Some(&handle.turn_id),
            Some(&user_item_id),
            &json!({"page": context.page}),
        )
        .await?;
        transaction.commit().await?;
        self.event_bus.publish(event);

        let started = AgentTaskStarted {
            thread_id: thread_id.clone(),
            turn_id: handle.turn_id.clone(),
        };
        let runtime = Arc::clone(self);
        let question = question.to_owned();
        tokio::spawn(async move {
            if runtime
                .finish_agent_turn(&thread_id, &handle.turn_id, &question, &context)
                .await
                .is_err()
            {
                let _ = runtime.fail_agent_turn(&thread_id, &handle.turn_id).await;
            }
        });
        Ok(started)
    }

    async fn finish_agent_turn(
        &self,
        thread_id: &str,
        turn_id: &str,
        question: &str,
        context: &PageContext,
    ) -> Result<(), AppError> {
        let cancellation = self
            .agent_runtime
            .cancellation_token(turn_id)
            .await
            .ok_or(AppError::InvalidData)?;
        let request = build_contextual_request(question, context);
        let events = MockModelAdapter::default()
            .complete(&request, None)
            .await
            .map_err(|_| AppError::InvalidData)?;
        let content = events
            .into_iter()
            .filter_map(|event| match event {
                ModelEvent::MessageDelta { text } => Some(text),
                _ => None,
            })
            .collect::<String>();
        if cancellation.is_cancelled() {
            return Ok(());
        }
        let item_id = format!("item_{}", Uuid::now_v7());
        let completed_at = Utc::now().to_rfc3339();
        let mut transaction = self.pool.begin().await?;
        sqlx::query("INSERT INTO agent_items (id,turn_id,item_type,status,content,model_id,created_at) VALUES (?,?,'assistant','completed',?,'mock-model',?)")
            .bind(&item_id).bind(turn_id).bind(&content).bind(&completed_at).execute(&mut *transaction).await?;
        sqlx::query("UPDATE agent_turns SET status='completed',completed_at=? WHERE id=?")
            .bind(&completed_at)
            .bind(turn_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("UPDATE agent_threads SET updated_at=? WHERE id=?")
            .bind(&completed_at)
            .bind(thread_id)
            .execute(&mut *transaction)
            .await?;
        let event = append_event(
            &mut transaction,
            "agent.message_completed",
            "thread",
            thread_id,
            Some(thread_id),
            Some(turn_id),
            Some(&item_id),
            &json!({"itemId": item_id, "content": content, "modelId": "mock-model", "mock": true}),
        )
        .await?;
        transaction.commit().await?;
        self.agent_runtime.complete_turn(turn_id).await;
        self.event_bus.publish(event);
        Ok(())
    }

    async fn fail_agent_turn(&self, thread_id: &str, turn_id: &str) -> Result<(), AppError> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query("UPDATE agent_turns SET status='failed',completed_at=? WHERE id=?")
            .bind(Utc::now().to_rfc3339())
            .bind(turn_id)
            .execute(&mut *transaction)
            .await?;
        let event = append_event(
            &mut transaction,
            "agent.turn_failed",
            "thread",
            thread_id,
            Some(thread_id),
            Some(turn_id),
            Some(turn_id),
            &json!({"code": "agent_turn_failed"}),
        )
        .await?;
        transaction.commit().await?;
        self.event_bus.publish(event);
        Ok(())
    }

    pub async fn cancel_agent_turn(&self, turn_id: &str) -> Result<bool, AppError> {
        if !self.agent_runtime.cancel_turn(turn_id).await {
            return Ok(false);
        }
        let row = sqlx::query("SELECT thread_id FROM agent_turns WHERE id=?")
            .bind(turn_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::NotFound("turn"))?;
        let thread_id: String = row.get("thread_id");
        let mut transaction = self.pool.begin().await?;
        sqlx::query("UPDATE agent_turns SET status='interrupted',completed_at=? WHERE id=?")
            .bind(Utc::now().to_rfc3339())
            .bind(turn_id)
            .execute(&mut *transaction)
            .await?;
        let event = append_event(
            &mut transaction,
            "agent.turn_interrupted",
            "thread",
            &thread_id,
            Some(&thread_id),
            Some(turn_id),
            Some(turn_id),
            &json!({"reason": "user_cancelled"}),
        )
        .await?;
        transaction.commit().await?;
        self.event_bus.publish(event);
        Ok(true)
    }

    pub async fn thread_items(&self, thread_id: &str) -> Result<Vec<AgentItemView>, AppError> {
        let rows = sqlx::query("SELECT i.id,i.turn_id,i.item_type,i.content,i.created_at,i.model_id FROM agent_items i JOIN agent_turns t ON t.id=i.turn_id WHERE t.thread_id=? ORDER BY i.created_at,i.id")
            .bind(thread_id).fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let model_id: Option<String> = row.get("model_id");
                AgentItemView {
                    id: row.get("id"),
                    turn_id: row.get("turn_id"),
                    role: row.get("item_type"),
                    content: row.get("content"),
                    created_at: row.get("created_at"),
                    mock: model_id.as_deref() == Some("mock-model"),
                    model_id,
                }
            })
            .collect())
    }

    pub async fn create_report(&self, signal_id: &str) -> Result<AgentReportView, AppError> {
        let signal = self.load_signal(signal_id).await?;
        let report = AgentReportView {
            id: format!("report_{}", Uuid::now_v7()),
            signal_id: signal.id.clone(),
            executive_summary: signal.summary.clone(),
            what_happened: format!("{} evidence records produced this signal.", signal.evidence.len()),
            why_it_matters: format!("The signal may affect {} over {}.", signal.asset.symbol, signal.time_horizon),
            evidence: signal.evidence.iter().map(|item| item.evidence.title.as_str()).collect::<Vec<_>>().join("; "),
            inference: "Inference: the observed evidence may persist, but persistence is not established as fact.".into(),
            market_impact: format!("Current direction: {}.", enum_json(&signal.direction)),
            bull_case: "Primary evidence strengthens and broadens.".into(),
            bear_case: "The observed relationship reverses or contradictory evidence grows.".into(),
            risk: signal.risks.join("; "),
            invalidation: signal.invalidation_conditions.join("; "),
            time_horizon: signal.time_horizon.clone(),
            possible_actions: "Observe, compare evidence, or create a paper-only proposal.".into(),
            watch_conditions: signal.catalysts.join("; "),
            conclusion: "Maintain an evidence-led watch posture.".into(),
            created_at: Utc::now().to_rfc3339(),
        };
        let mut transaction = self.pool.begin().await?;
        sqlx::query("INSERT INTO agent_reports (id,signal_id,executive_summary,evidence_summary,inference_summary,bull_case,bear_case,risk,invalidation,time_horizon,possible_actions,watch_conditions,conclusion,created_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&report.id).bind(&report.signal_id).bind(&report.executive_summary).bind(&report.evidence)
            .bind(&report.inference).bind(&report.bull_case).bind(&report.bear_case).bind(&report.risk)
            .bind(&report.invalidation).bind(&report.time_horizon).bind(&report.possible_actions)
            .bind(&report.watch_conditions).bind(&report.conclusion).bind(&report.created_at)
            .execute(&mut *transaction).await?;
        let event = append_event(
            &mut transaction,
            "report.created",
            "signal",
            signal_id,
            None,
            None,
            Some(&report.id),
            &json!({"reportId": report.id}),
        )
        .await?;
        transaction.commit().await?;
        self.event_bus.publish(event);
        Ok(report)
    }

    pub async fn create_trade_proposal(&self, signal_id: &str) -> Result<TradeProposal, AppError> {
        let signal = self.load_signal(signal_id).await?;
        let action = match signal.direction {
            SignalDirection::Bearish => TradeAction::OpenShort,
            _ => TradeAction::OpenLong,
        };
        let proposal = TradeProposal::new(
            signal.id.clone(),
            signal.asset.market,
            format!("{}-PAPER", signal.asset.symbol),
            action,
            OrderType::Market,
            Decimal::from_str("1").map_err(|_| AppError::InvalidData)?,
            None,
            None,
            None,
            format!("Paper proposal derived from signal {}", signal.id),
            "Paper execution only. No real funds or external account connection.",
        );
        let mut transaction = self.pool.begin().await?;
        sqlx::query("INSERT INTO trade_proposals (id,signal_id,version,status,domain_json,created_at) VALUES (?,?,?,?,?,?)")
            .bind(&proposal.id).bind(&proposal.signal_id).bind(proposal.version as i64)
            .bind(enum_json(&proposal.status)).bind(serde_json::to_string(&proposal)?)
            .bind(proposal.created_at.to_rfc3339()).execute(&mut *transaction).await?;
        let event = append_event(
            &mut transaction,
            "proposal.created",
            "proposal",
            &proposal.id,
            None,
            None,
            Some(&proposal.id),
            &json!({"signalId": signal_id}),
        )
        .await?;
        transaction.commit().await?;
        self.event_bus.publish(event);
        Ok(proposal)
    }

    pub async fn request_approval(&self, proposal_id: &str) -> Result<ApprovalView, AppError> {
        let mut proposal = self.load_proposal(proposal_id).await?;
        let request = self
            .approval_engine
            .request(&proposal, Duration::minutes(5))
            .await?;
        proposal.status = ProposalStatus::WaitingApproval;
        let mut transaction = self.pool.begin().await?;
        sqlx::query("UPDATE trade_proposals SET status=?,proposal_hash=?,proposal_hash_version=?,domain_json=? WHERE id=?")
            .bind(enum_json(&proposal.status)).bind(&request.proposal_hash).bind(request.proposal_hash_version as i64)
            .bind(serde_json::to_string(&proposal)?).bind(&proposal.id).execute(&mut *transaction).await?;
        insert_approval(&mut transaction, &request).await?;
        let event = append_event(
            &mut transaction,
            "approval.requested",
            "proposal",
            proposal_id,
            None,
            None,
            Some(&request.id),
            &json!({"approvalRequestId": request.id, "expiresAt": request.expires_at.to_rfc3339()}),
        )
        .await?;
        transaction.commit().await?;
        self.event_bus.publish(event);
        Ok(ApprovalView::from(&request))
    }

    pub async fn approve_and_execute_paper(
        &self,
        approval_id: &str,
    ) -> Result<ExecutionView, AppError> {
        let _guard = self.execution_guard.lock().await;
        let stored = load_approval_request(&self.pool, approval_id).await?;
        if stored.token_state != approval_engine::TokenState::NotIssued
            || stored.status != approval_engine::ApprovalStatus::Requested
        {
            return Err(AppError::Approval(ApprovalError::InvalidState));
        }
        let proposal = self.load_proposal(&stored.proposal_id).await?;
        let capability = self.approval_engine.approve(approval_id, &proposal).await?;
        let permit = self
            .approval_engine
            .consume(&capability, &proposal, Utc::now())
            .await?;
        let approved = self
            .approval_engine
            .export_requests()
            .await
            .into_iter()
            .find(|item| item.id == approval_id)
            .ok_or(AppError::InvalidData)?;
        let execution_id = format!("execution_{}", Uuid::now_v7());
        let created_at = Utc::now().to_rfc3339();
        let mut transaction = self.pool.begin().await?;
        let updated = sqlx::query("UPDATE approval_requests SET status='approved',token_digest=?,token_state='consumed' WHERE id=? AND token_state='not_issued'")
            .bind(&approved.token_digest).bind(approval_id).execute(&mut *transaction).await?;
        if updated.rows_affected() != 1 {
            return Err(AppError::Approval(ApprovalError::AlreadyConsumed));
        }
        sqlx::query("INSERT INTO execution_records (id,proposal_id,approval_request_id,adapter_id,status,created_at) VALUES (?,?,?,'paper','starting',?)")
            .bind(&execution_id).bind(&proposal.id).bind(approval_id).bind(&created_at)
            .execute(&mut *transaction).await?;
        let events = vec![
            append_event(
                &mut transaction,
                "approval.approved",
                "proposal",
                &proposal.id,
                None,
                None,
                Some(approval_id),
                &json!({"approvalRequestId": approval_id}),
            )
            .await?,
            append_event(
                &mut transaction,
                "approval.token_issued",
                "proposal",
                &proposal.id,
                None,
                None,
                Some(approval_id),
                &json!({"expiresAt": approved.expires_at.to_rfc3339()}),
            )
            .await?,
            append_event(
                &mut transaction,
                "approval.token_consumed",
                "proposal",
                &proposal.id,
                None,
                None,
                Some(approval_id),
                &json!({}),
            )
            .await?,
            append_event(
                &mut transaction,
                "execution.created",
                "proposal",
                &proposal.id,
                None,
                None,
                Some(&execution_id),
                &json!({"adapterId": "paper"}),
            )
            .await?,
        ];
        transaction.commit().await?;
        for event in events {
            self.event_bus.publish(event);
        }

        let adapter_record = PaperExecutionAdapter
            .execute(&proposal, permit)
            .map_err(|_| AppError::Execution)?;
        let completed_at = Utc::now().to_rfc3339();
        let result_summary = adapter_record
            .result_summary
            .unwrap_or_else(|| "Paper execution completed".into());
        let mut transaction = self.pool.begin().await?;
        sqlx::query("UPDATE execution_records SET status='completed',result_summary=?,completed_at=? WHERE id=?")
            .bind(&result_summary).bind(&completed_at).bind(&execution_id).execute(&mut *transaction).await?;
        sqlx::query("UPDATE trade_proposals SET status='executed' WHERE id=?")
            .bind(&proposal.id)
            .execute(&mut *transaction)
            .await?;
        let event = append_event(
            &mut transaction,
            "execution.completed",
            "proposal",
            &proposal.id,
            None,
            None,
            Some(&execution_id),
            &json!({"adapterId": "paper"}),
        )
        .await?;
        transaction.commit().await?;
        self.event_bus.publish(event);
        Ok(ExecutionView {
            id: execution_id,
            proposal_id: proposal.id,
            approval_request_id: approval_id.into(),
            adapter_id: "paper".into(),
            status: "completed".into(),
            result_summary: Some(result_summary),
            created_at,
            completed_at: Some(completed_at),
        })
    }

    pub async fn audit_timeline(
        &self,
        aggregate_id: &str,
    ) -> Result<Vec<AuditEventView>, AppError> {
        let rows = sqlx::query("SELECT id,event_type,aggregate_id,entity_id,payload_json,created_at,sequence FROM runtime_events WHERE aggregate_id=? ORDER BY sequence")
            .bind(aggregate_id).fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|row| AuditEventView {
                id: row.get("id"),
                event_type: row.get("event_type"),
                aggregate_id: row.get("aggregate_id"),
                entity_id: row.get("entity_id"),
                payload_json: row.get("payload_json"),
                created_at: row.get("created_at"),
                sequence: row.get("sequence"),
            })
            .collect())
    }

    pub async fn recover_interrupted_tasks(&self) -> Result<u64, AppError> {
        let mut transaction = self.pool.begin().await?;
        let rows = sqlx::query(
            "SELECT id,thread_id FROM agent_turns WHERE status IN ('running','waiting_tool')",
        )
        .fetch_all(&mut *transaction)
        .await?;
        let mut events = Vec::new();
        for row in &rows {
            let turn_id: String = row.get("id");
            let thread_id: String = row.get("thread_id");
            sqlx::query("UPDATE agent_turns SET status='interrupted',completed_at=? WHERE id=?")
                .bind(Utc::now().to_rfc3339())
                .bind(&turn_id)
                .execute(&mut *transaction)
                .await?;
            events.push(
                append_event(
                    &mut transaction,
                    "runtime.task_interrupted",
                    "thread",
                    &thread_id,
                    Some(&thread_id),
                    Some(&turn_id),
                    Some(&turn_id),
                    &json!({"reason": "application_restart"}),
                )
                .await?,
            );
        }
        transaction.commit().await?;
        for event in events {
            self.event_bus.publish(event);
        }
        Ok(rows.len() as u64)
    }

    async fn load_signal(&self, id: &str) -> Result<Signal, AppError> {
        let data: Option<String> = sqlx::query_scalar("SELECT domain_json FROM signals WHERE id=?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(serde_json::from_str(
            &data.ok_or(AppError::NotFound("signal"))?,
        )?)
    }

    async fn load_proposal(&self, id: &str) -> Result<TradeProposal, AppError> {
        let data: Option<String> =
            sqlx::query_scalar("SELECT domain_json FROM trade_proposals WHERE id=?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(serde_json::from_str(
            &data.ok_or(AppError::NotFound("proposal"))?,
        )?)
    }
}

async fn insert_evidence(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    item: &Evidence,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO evidence (id,source,source_type,market,asset,title,content,raw_reference,captured_at,freshness,reliability,factual_level,confidence,metadata_json,domain_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&item.id).bind(&item.source).bind(enum_json(&item.source_type)).bind(item.asset.market.as_str())
        .bind(&item.asset.symbol).bind(&item.title).bind(&item.content).bind(&item.raw_reference)
        .bind(item.captured_at.to_rfc3339()).bind(item.freshness).bind(item.reliability)
        .bind(enum_json(&item.factual_level)).bind(item.confidence).bind(serde_json::to_string(&item.metadata)?)
        .bind(serde_json::to_string(item)?).execute(&mut **transaction).await?;
    Ok(())
}

async fn insert_approval(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    item: &ApprovalRequest,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO approval_requests (id,proposal_id,proposal_version,proposal_hash,proposal_hash_version,allowed_action,status,issued_at,expires_at,nonce,token_digest,token_state,issuer_session_id) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&item.id).bind(&item.proposal_id).bind(item.proposal_version as i64).bind(&item.proposal_hash)
        .bind(item.proposal_hash_version as i64).bind(&item.allowed_action).bind(enum_json(&item.status))
        .bind(item.issued_at.to_rfc3339()).bind(item.expires_at.to_rfc3339()).bind(&item.nonce)
        .bind(&item.token_digest).bind(enum_json(&item.token_state)).bind(&item.issuer_session_id)
        .execute(&mut **transaction).await?;
    Ok(())
}

async fn load_approval_requests(pool: &SqlitePool) -> Result<Vec<ApprovalRequest>, AppError> {
    let rows = sqlx::query("SELECT id,proposal_id,proposal_version,proposal_hash,proposal_hash_version,allowed_action,status,issued_at,expires_at,nonce,token_digest,token_state,issuer_session_id FROM approval_requests")
        .fetch_all(pool).await?;
    rows.into_iter().map(approval_from_row).collect()
}

async fn load_approval_request(pool: &SqlitePool, id: &str) -> Result<ApprovalRequest, AppError> {
    let row = sqlx::query("SELECT id,proposal_id,proposal_version,proposal_hash,proposal_hash_version,allowed_action,status,issued_at,expires_at,nonce,token_digest,token_state,issuer_session_id FROM approval_requests WHERE id=?")
        .bind(id).fetch_optional(pool).await?.ok_or(AppError::NotFound("approval"))?;
    approval_from_row(row)
}

fn approval_from_row(row: sqlx::sqlite::SqliteRow) -> Result<ApprovalRequest, AppError> {
    Ok(ApprovalRequest {
        id: row.get("id"),
        proposal_id: row.get("proposal_id"),
        proposal_version: row.get::<i64, _>("proposal_version") as u32,
        proposal_hash: row.get("proposal_hash"),
        proposal_hash_version: row.get::<i64, _>("proposal_hash_version") as u32,
        allowed_action: row.get("allowed_action"),
        status: parse_enum(row.get("status"))?,
        issued_at: chrono::DateTime::parse_from_rfc3339(row.get("issued_at"))
            .map_err(|_| AppError::InvalidData)?
            .with_timezone(&Utc),
        expires_at: chrono::DateTime::parse_from_rfc3339(row.get("expires_at"))
            .map_err(|_| AppError::InvalidData)?
            .with_timezone(&Utc),
        nonce: row.get("nonce"),
        token_digest: row.get("token_digest"),
        token_state: parse_enum(row.get("token_state"))?,
        issuer_session_id: row.get("issuer_session_id"),
    })
}

fn execution_view(row: sqlx::sqlite::SqliteRow) -> ExecutionView {
    ExecutionView {
        id: row.get("id"),
        proposal_id: row.get("proposal_id"),
        approval_request_id: row.get("approval_request_id"),
        adapter_id: row.get("adapter_id"),
        status: row.get("status"),
        result_summary: row.get("result_summary"),
        created_at: row.get("created_at"),
        completed_at: row.get("completed_at"),
    }
}

fn enum_json<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|item| item.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn parse_enum<T: for<'de> Deserialize<'de>>(value: String) -> Result<T, AppError> {
    serde_json::from_value(Value::String(value)).map_err(AppError::Serialization)
}
