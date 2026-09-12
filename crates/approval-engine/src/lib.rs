use chrono::{DateTime, Duration, Utc};
use execution_core::{ExecutionPermit, TradeProposal, proposal_hash};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Requested,
    Approved,
    Rejected,
    Expired,
    Cancelled,
    Revoked,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenState {
    NotIssued,
    Issued,
    Consumed,
    Expired,
    Revoked,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub id: String,
    pub proposal_id: String,
    pub proposal_version: u32,
    pub proposal_hash: String,
    pub proposal_hash_version: u32,
    pub allowed_action: String,
    pub status: ApprovalStatus,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub nonce: String,
    pub token_digest: Option<String>,
    pub token_state: TokenState,
    pub issuer_session_id: String,
}

impl std::fmt::Debug for ApprovalRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApprovalRequest")
            .field("id", &self.id)
            .field("proposal_id", &self.proposal_id)
            .field("status", &self.status)
            .field("token_digest", &"[REDACTED]")
            .finish()
    }
}

pub struct ApprovalCapability {
    request_id: String,
    proposal_id: String,
    token: String,
}

impl std::fmt::Debug for ApprovalCapability {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApprovalCapability")
            .field("request_id", &self.request_id)
            .field("proposal_id", &self.proposal_id)
            .field("token", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ApprovalError {
    #[error("approval request was not found")]
    NotFound,
    #[error("approval request is in an invalid state")]
    InvalidState,
    #[error("approval capability expired")]
    Expired,
    #[error("approval capability was already consumed")]
    AlreadyConsumed,
    #[error("approval capability does not match the proposal")]
    ProposalMismatch,
    #[error("approval capability is invalid")]
    TokenMismatch,
    #[error("approval capability was invalidated by runtime restart")]
    RestartInvalidated,
    #[error("proposal canonicalization failed")]
    Canonicalization,
}

pub struct ApprovalEngine {
    session_id: String,
    requests: Mutex<BTreeMap<String, ApprovalRequest>>,
}

impl Default for ApprovalEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ApprovalEngine {
    pub fn new() -> Self {
        Self {
            session_id: format!("approval-session-{}", Uuid::now_v7()),
            requests: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn from_requests(requests: Vec<ApprovalRequest>) -> Self {
        Self {
            session_id: format!("approval-session-{}", Uuid::now_v7()),
            requests: Mutex::new(
                requests
                    .into_iter()
                    .map(|request| (request.id.clone(), request))
                    .collect(),
            ),
        }
    }

    pub async fn request(
        &self,
        proposal: &TradeProposal,
        ttl: Duration,
    ) -> Result<ApprovalRequest, ApprovalError> {
        let canonical = proposal_hash(proposal, 1).map_err(|_| ApprovalError::Canonicalization)?;
        let now = Utc::now();
        let request = ApprovalRequest {
            id: format!("approval_{}", Uuid::now_v7()),
            proposal_id: proposal.id.clone(),
            proposal_version: proposal.version,
            proposal_hash: canonical,
            proposal_hash_version: 1,
            allowed_action: proposal.action.as_str().into(),
            status: ApprovalStatus::Requested,
            issued_at: now,
            expires_at: now + ttl,
            nonce: random_hex(16),
            token_digest: None,
            token_state: TokenState::NotIssued,
            issuer_session_id: self.session_id.clone(),
        };
        self.requests
            .lock()
            .await
            .insert(request.id.clone(), request.clone());
        Ok(request)
    }

    pub async fn approve(
        &self,
        request_id: &str,
        proposal: &TradeProposal,
    ) -> Result<ApprovalCapability, ApprovalError> {
        let mut requests = self.requests.lock().await;
        let request = requests
            .get_mut(request_id)
            .ok_or(ApprovalError::NotFound)?;
        if request.status != ApprovalStatus::Requested
            || request.token_state != TokenState::NotIssued
        {
            return Err(ApprovalError::InvalidState);
        }
        verify_proposal(request, proposal)?;
        let token = random_hex(32);
        request.token_digest = Some(digest(&token));
        request.token_state = TokenState::Issued;
        request.status = ApprovalStatus::Approved;
        Ok(ApprovalCapability {
            request_id: request.id.clone(),
            proposal_id: proposal.id.clone(),
            token,
        })
    }

    pub async fn reject(&self, request_id: &str) -> Result<(), ApprovalError> {
        self.transition_requested(request_id, ApprovalStatus::Rejected)
            .await
    }

    pub async fn cancel(&self, request_id: &str) -> Result<(), ApprovalError> {
        self.transition_requested(request_id, ApprovalStatus::Cancelled)
            .await
    }

    async fn transition_requested(
        &self,
        request_id: &str,
        status: ApprovalStatus,
    ) -> Result<(), ApprovalError> {
        let mut requests = self.requests.lock().await;
        let request = requests
            .get_mut(request_id)
            .ok_or(ApprovalError::NotFound)?;
        if request.status != ApprovalStatus::Requested {
            return Err(ApprovalError::InvalidState);
        }
        request.status = status;
        Ok(())
    }

    pub async fn consume(
        &self,
        capability: &ApprovalCapability,
        proposal: &TradeProposal,
        now: DateTime<Utc>,
    ) -> Result<ExecutionPermit, ApprovalError> {
        let mut requests = self.requests.lock().await;
        let request = requests
            .get_mut(&capability.request_id)
            .ok_or(ApprovalError::NotFound)?;
        if request.issuer_session_id != self.session_id {
            return Err(ApprovalError::RestartInvalidated);
        }
        if request.status != ApprovalStatus::Approved {
            return Err(ApprovalError::InvalidState);
        }
        if now >= request.expires_at {
            request.token_state = TokenState::Expired;
            request.status = ApprovalStatus::Expired;
            return Err(ApprovalError::Expired);
        }
        verify_proposal(request, proposal)?;
        if capability.proposal_id != proposal.id
            || request.token_digest.as_deref() != Some(digest(&capability.token).as_str())
        {
            return Err(ApprovalError::TokenMismatch);
        }
        if request.token_state == TokenState::Consumed {
            return Err(ApprovalError::AlreadyConsumed);
        }
        if request.token_state != TokenState::Issued {
            return Err(ApprovalError::InvalidState);
        }
        request.token_state = TokenState::Consumed;
        Ok(ExecutionPermit::from_validated_approval(
            proposal.id.clone(),
            request.id.clone(),
        ))
    }

    pub async fn export_requests(&self) -> Vec<ApprovalRequest> {
        self.requests.lock().await.values().cloned().collect()
    }
}

fn verify_proposal(
    request: &ApprovalRequest,
    proposal: &TradeProposal,
) -> Result<(), ApprovalError> {
    if request.proposal_id != proposal.id
        || request.proposal_version != proposal.version
        || request.allowed_action != proposal.action.as_str()
    {
        return Err(ApprovalError::ProposalMismatch);
    }
    let hash = proposal_hash(proposal, request.proposal_hash_version)
        .map_err(|_| ApprovalError::Canonicalization)?;
    if hash != request.proposal_hash {
        return Err(ApprovalError::ProposalMismatch);
    }
    Ok(())
}

fn random_hex(bytes: usize) -> String {
    let mut value = vec![0_u8; bytes];
    rand::rng().fill(value.as_mut_slice());
    hex::encode(value)
}

fn digest(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use execution_core::{OrderType, TradeAction, TradeProposal};
    use market_core::Market;
    use rust_decimal_macros::dec;
    use std::sync::Arc;
    use tokio::sync::Barrier;

    fn proposal() -> TradeProposal {
        TradeProposal::new(
            "signal-1",
            Market::Crypto,
            "BTC-USDT",
            TradeAction::OpenLong,
            OrderType::Limit,
            dec!(0.01),
            Some(dec!(65000)),
            Some(dec!(62000)),
            Some(dec!(71000)),
            "Evidence-backed paper proposal",
            "Limited to paper execution",
        )
    }

    async fn approved(engine: &ApprovalEngine, proposal: &TradeProposal) -> ApprovalCapability {
        let request = engine
            .request(proposal, Duration::minutes(5))
            .await
            .unwrap();
        engine.approve(&request.id, proposal).await.unwrap()
    }

    #[tokio::test]
    async fn exact_approved_proposal_consumes_once() {
        let engine = ApprovalEngine::new();
        let proposal = proposal();
        let capability = approved(&engine, &proposal).await;
        let permit = engine
            .consume(&capability, &proposal, Utc::now())
            .await
            .unwrap();
        assert_eq!(permit.proposal_id(), proposal.id);
        assert_eq!(
            engine
                .consume(&capability, &proposal, Utc::now())
                .await
                .unwrap_err(),
            ApprovalError::AlreadyConsumed
        );
    }

    #[tokio::test]
    async fn expired_capability_is_rejected() {
        let engine = ApprovalEngine::new();
        let proposal = proposal();
        let capability = approved(&engine, &proposal).await;
        assert_eq!(
            engine
                .consume(&capability, &proposal, Utc::now() + Duration::minutes(6))
                .await
                .unwrap_err(),
            ApprovalError::Expired
        );
    }

    #[tokio::test]
    async fn rejected_or_cancelled_request_cannot_issue_capability() {
        let engine = ApprovalEngine::new();
        let proposal = proposal();
        let rejected = engine
            .request(&proposal, Duration::minutes(5))
            .await
            .unwrap();
        engine.reject(&rejected.id).await.unwrap();
        assert_eq!(
            engine.approve(&rejected.id, &proposal).await.unwrap_err(),
            ApprovalError::InvalidState
        );
        let cancelled = engine
            .request(&proposal, Duration::minutes(5))
            .await
            .unwrap();
        engine.cancel(&cancelled.id).await.unwrap();
        assert_eq!(
            engine.approve(&cancelled.id, &proposal).await.unwrap_err(),
            ApprovalError::InvalidState
        );
    }

    #[tokio::test]
    async fn every_material_proposal_change_invalidates_capability() {
        let engine = ApprovalEngine::new();
        let original = proposal();
        let capability = approved(&engine, &original).await;
        let mut variants = Vec::new();
        let mut changed = original.clone();
        changed.instrument = "ETH-USDT".into();
        variants.push(changed);
        let mut changed = original.clone();
        changed.quantity = dec!(0.02);
        variants.push(changed);
        let mut changed = original.clone();
        changed.price = Some(dec!(65001));
        variants.push(changed);
        let mut changed = original.clone();
        changed.stop_loss = Some(dec!(61000));
        variants.push(changed);
        let mut changed = original.clone();
        changed.take_profit = Some(dec!(72000));
        variants.push(changed);
        for variant in variants {
            assert_eq!(
                engine
                    .consume(&capability, &variant, Utc::now())
                    .await
                    .unwrap_err(),
                ApprovalError::ProposalMismatch
            );
        }
    }

    #[tokio::test]
    async fn restart_invalidates_outstanding_ephemeral_capability() {
        let first = ApprovalEngine::new();
        let proposal = proposal();
        let capability = approved(&first, &proposal).await;
        let stored = first.export_requests().await;
        let restarted = ApprovalEngine::from_requests(stored);
        assert_eq!(
            restarted
                .consume(&capability, &proposal, Utc::now())
                .await
                .unwrap_err(),
            ApprovalError::RestartInvalidated
        );
    }

    #[tokio::test]
    async fn concurrent_double_execution_yields_one_permit() {
        let engine = Arc::new(ApprovalEngine::new());
        let proposal = Arc::new(proposal());
        let capability = Arc::new(approved(&engine, &proposal).await);
        let barrier = Arc::new(Barrier::new(3));
        let mut tasks = Vec::new();
        for _ in 0..2 {
            let engine = engine.clone();
            let proposal = proposal.clone();
            let capability = capability.clone();
            let barrier = barrier.clone();
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                engine.consume(&capability, &proposal, Utc::now()).await
            }));
        }
        barrier.wait().await;
        let mut successes = 0;
        for task in tasks {
            if task.await.unwrap().is_ok() {
                successes += 1;
            }
        }
        assert_eq!(successes, 1);
    }
}
