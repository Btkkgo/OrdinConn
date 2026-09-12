use chrono::{DateTime, Utc};
use model_gateway::{ModelRole, UnifiedMessage, UnifiedModelRequest};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageContext {
    pub page: String,
    pub market: Option<String>,
    pub asset: Option<String>,
    pub signal_id: Option<String>,
    pub evidence_ids: Vec<String>,
}

pub fn build_contextual_request(
    text: impl Into<String>,
    context: &PageContext,
) -> UnifiedModelRequest {
    let mut values = BTreeMap::new();
    values.insert("page".into(), context.page.clone());
    if let Some(market) = &context.market {
        values.insert("market".into(), market.clone());
    }
    if let Some(asset) = &context.asset {
        values.insert("asset".into(), asset.clone());
    }
    if let Some(signal_id) = &context.signal_id {
        values.insert("signalId".into(), signal_id.clone());
    }
    if !context.evidence_ids.is_empty() {
        values.insert("evidenceIds".into(), context.evidence_ids.join(","));
    }
    UnifiedModelRequest {
        model: "mock-model".into(),
        messages: vec![
            UnifiedMessage {
                role: ModelRole::System,
                content: "Use supplied OrdinConn context. Separate Evidence from inference.".into(),
            },
            UnifiedMessage {
                role: ModelRole::User,
                content: text.into(),
            },
        ],
        tools: Vec::new(),
        context: values,
        temperature: 0.2,
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnStatus {
    Pending,
    Running,
    WaitingTool,
    Completed,
    Failed,
    Interrupted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentThread {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTurn {
    pub id: String,
    pub thread_id: String,
    pub status: TurnStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentItem {
    pub id: String,
    pub turn_id: String,
    pub item_type: String,
    pub status: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnHandle {
    pub thread_id: String,
    pub turn_id: String,
}

struct ActiveTurn {
    status: TurnStatus,
    cancellation: CancellationToken,
}

#[derive(Default)]
pub struct AgentRuntime {
    turns: Mutex<HashMap<String, ActiveTurn>>,
    active_by_thread: Mutex<HashMap<String, String>>,
}

impl AgentRuntime {
    pub async fn start_turn(
        &self,
        thread_id: impl Into<String>,
    ) -> Result<TurnHandle, &'static str> {
        let thread_id = thread_id.into();
        let mut active_by_thread = self.active_by_thread.lock().await;
        if active_by_thread.contains_key(&thread_id) {
            return Err("thread already has an active turn");
        }
        let turn_id = format!("turn_{}", Uuid::now_v7());
        active_by_thread.insert(thread_id.clone(), turn_id.clone());
        self.turns.lock().await.insert(
            turn_id.clone(),
            ActiveTurn {
                status: TurnStatus::Running,
                cancellation: CancellationToken::new(),
            },
        );
        Ok(TurnHandle { thread_id, turn_id })
    }

    pub async fn cancel_turn(&self, turn_id: &str) -> bool {
        let mut turns = self.turns.lock().await;
        let Some(turn) = turns.get_mut(turn_id) else {
            return false;
        };
        turn.cancellation.cancel();
        turn.status = TurnStatus::Interrupted;
        let turn_id_owned = turn_id.to_owned();
        drop(turns);
        self.active_by_thread
            .lock()
            .await
            .retain(|_, active| active != &turn_id_owned);
        true
    }

    pub async fn complete_turn(&self, turn_id: &str) -> bool {
        let mut turns = self.turns.lock().await;
        let Some(turn) = turns.get_mut(turn_id) else {
            return false;
        };
        if turn.status != TurnStatus::Running && turn.status != TurnStatus::WaitingTool {
            return false;
        }
        turn.status = TurnStatus::Completed;
        let turn_id_owned = turn_id.to_owned();
        drop(turns);
        self.active_by_thread
            .lock()
            .await
            .retain(|_, active| active != &turn_id_owned);
        true
    }

    pub async fn turn_status(&self, turn_id: &str) -> Option<TurnStatus> {
        self.turns.lock().await.get(turn_id).map(|turn| turn.status)
    }

    pub async fn cancellation_token(&self, turn_id: &str) -> Option<CancellationToken> {
        self.turns
            .lock()
            .await
            .get(turn_id)
            .map(|turn| turn.cancellation.clone())
    }

    pub async fn cancel_all(&self) -> Vec<String> {
        let mut turns = self.turns.lock().await;
        let mut cancelled = Vec::new();
        for (turn_id, turn) in turns.iter_mut() {
            if matches!(turn.status, TurnStatus::Running | TurnStatus::WaitingTool) {
                turn.cancellation.cancel();
                turn.status = TurnStatus::Interrupted;
                cancelled.push(turn_id.clone());
            }
        }
        self.active_by_thread.lock().await.clear();
        cancelled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_context_is_injected_without_vendor_message_types() {
        let context = PageContext {
            page: "signals".into(),
            market: Some("crypto".into()),
            asset: Some("BTC".into()),
            signal_id: Some("signal-1".into()),
            evidence_ids: vec!["evidence-1".into()],
        };
        let request = build_contextual_request("Why this signal?", &context);
        assert_eq!(request.context.get("page").unwrap(), "signals");
        assert_eq!(request.context.get("asset").unwrap(), "BTC");
        assert_eq!(request.messages.last().unwrap().content, "Why this signal?");
    }

    #[tokio::test]
    async fn active_turn_can_be_cancelled() {
        let runtime = AgentRuntime::default();
        let handle = runtime.start_turn("thread-1").await.unwrap();
        assert!(runtime.cancel_turn(&handle.turn_id).await);
        assert_eq!(
            runtime.turn_status(&handle.turn_id).await,
            Some(TurnStatus::Interrupted)
        );
    }

    #[tokio::test]
    async fn completed_turn_releases_thread_for_next_turn() {
        let runtime = AgentRuntime::default();
        let first = runtime.start_turn("thread-1").await.unwrap();
        assert!(runtime.complete_turn(&first.turn_id).await);
        assert_eq!(
            runtime.turn_status(&first.turn_id).await,
            Some(TurnStatus::Completed)
        );
        assert!(runtime.start_turn("thread-1").await.is_ok());
    }

    #[tokio::test]
    async fn shutdown_cancels_every_active_turn() {
        let runtime = AgentRuntime::default();
        let first = runtime.start_turn("thread-1").await.unwrap();
        let second = runtime.start_turn("thread-2").await.unwrap();

        let cancelled = runtime.cancel_all().await;

        assert_eq!(cancelled.len(), 2);
        assert_eq!(
            runtime.turn_status(&first.turn_id).await,
            Some(TurnStatus::Interrupted)
        );
        assert_eq!(
            runtime.turn_status(&second.turn_id).await,
            Some(TurnStatus::Interrupted)
        );
        assert!(runtime.start_turn("thread-1").await.is_ok());
    }
}
