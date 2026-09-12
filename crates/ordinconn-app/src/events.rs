use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Sqlite, Transaction};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::AppError;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventFamily {
    Agent,
    Model,
    Signal,
    Approval,
    Execution,
    System,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEventEnvelope {
    pub id: String,
    pub family: EventFamily,
    #[serde(rename = "type")]
    pub event_type: String,
    pub aggregate_id: String,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub occurred_at: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventView {
    pub id: String,
    pub event_type: String,
    pub aggregate_id: String,
    pub entity_id: Option<String>,
    pub payload_json: String,
    pub created_at: String,
    pub sequence: i64,
}

#[derive(Clone)]
pub struct RuntimeEventBus {
    sender: broadcast::Sender<RuntimeEventEnvelope>,
}

impl Default for RuntimeEventBus {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(256);
        Self { sender }
    }
}

impl RuntimeEventBus {
    pub fn subscribe(&self) -> broadcast::Receiver<RuntimeEventEnvelope> {
        self.sender.subscribe()
    }
    pub fn publish(&self, event: RuntimeEventEnvelope) {
        let _ = self.sender.send(event);
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn append_event(
    transaction: &mut Transaction<'_, Sqlite>,
    event_type: &str,
    aggregate_type: &str,
    aggregate_id: &str,
    thread_id: Option<&str>,
    turn_id: Option<&str>,
    entity_id: Option<&str>,
    payload: &Value,
) -> Result<RuntimeEventEnvelope, AppError> {
    let id = format!("event_{}", Uuid::now_v7());
    let created_at = Utc::now().to_rfc3339();
    let payload_json = serde_json::to_string(payload)?;
    sqlx::query("INSERT INTO runtime_events (id,event_type,aggregate_type,aggregate_id,thread_id,turn_id,entity_id,payload_json,created_at,sequence) VALUES (?,?,?,?,?,?,?,?,?,(SELECT COALESCE(MAX(sequence),0)+1 FROM runtime_events))")
        .bind(&id).bind(event_type).bind(aggregate_type).bind(aggregate_id).bind(thread_id)
        .bind(turn_id).bind(entity_id).bind(&payload_json).bind(&created_at)
        .execute(&mut **transaction).await?;
    Ok(RuntimeEventEnvelope {
        id,
        family: family_for(event_type),
        event_type: event_type.into(),
        aggregate_id: aggregate_id.into(),
        thread_id: thread_id.map(str::to_owned),
        turn_id: turn_id.map(str::to_owned),
        occurred_at: created_at,
        payload: payload.clone(),
    })
}

fn family_for(event_type: &str) -> EventFamily {
    match event_type.split('.').next().unwrap_or_default() {
        "agent" | "runtime" => EventFamily::Agent,
        "model" => EventFamily::Model,
        "signal" | "proposal" | "report" => EventFamily::Signal,
        "approval" => EventFamily::Approval,
        "execution" => EventFamily::Execution,
        _ => EventFamily::System,
    }
}
