use chrono::{DateTime, Utc};
use market_core::{Market, new_id};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeAction {
    Buy,
    Sell,
    OpenLong,
    OpenShort,
    Close,
}

impl TradeAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
            Self::OpenLong => "open_long",
            Self::OpenShort => "open_short",
            Self::Close => "close",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    Market,
    Limit,
}

impl OrderType {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Market => "market",
            Self::Limit => "limit",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Draft,
    WaitingApproval,
    Approved,
    Rejected,
    Expired,
    Executed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeProposal {
    pub id: String,
    pub signal_id: String,
    pub version: u32,
    pub market: Market,
    pub instrument: String,
    pub action: TradeAction,
    pub order_type: OrderType,
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub price: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub stop_loss: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub take_profit: Option<Decimal>,
    pub reason: String,
    pub risk_summary: String,
    pub status: ProposalStatus,
    pub created_at: DateTime<Utc>,
}

impl TradeProposal {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        signal_id: impl Into<String>,
        market: Market,
        instrument: impl Into<String>,
        action: TradeAction,
        order_type: OrderType,
        quantity: Decimal,
        price: Option<Decimal>,
        stop_loss: Option<Decimal>,
        take_profit: Option<Decimal>,
        reason: impl Into<String>,
        risk_summary: impl Into<String>,
    ) -> Self {
        Self {
            id: new_id("proposal"),
            signal_id: signal_id.into(),
            version: 1,
            market,
            instrument: instrument.into(),
            action,
            order_type,
            quantity,
            price,
            stop_loss,
            take_profit,
            reason: reason.into(),
            risk_summary: risk_summary.into(),
            status: ProposalStatus::Draft,
            created_at: Utc::now(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalTradeProposalV1 {
    pub hash_version: u32,
    pub bytes: Vec<u8>,
    pub hash: String,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum CanonicalizationError {
    #[error("unknown proposal hash version {0}")]
    UnknownHashVersion(u32),
    #[error("quantity must be positive")]
    InvalidQuantity,
}

pub fn canonicalize_proposal_v1(
    proposal: &TradeProposal,
) -> Result<CanonicalTradeProposalV1, CanonicalizationError> {
    if proposal.quantity <= Decimal::ZERO {
        return Err(CanonicalizationError::InvalidQuantity);
    }
    let fields = [
        "v1".to_owned(),
        proposal.market.as_str().to_owned(),
        escape_field(&proposal.instrument),
        proposal.action.as_str().to_owned(),
        proposal.order_type.as_str().to_owned(),
        decimal_field(Some(proposal.quantity)),
        decimal_field(proposal.price),
        decimal_field(proposal.stop_loss),
        decimal_field(proposal.take_profit),
    ];
    let bytes = fields.join("\n").into_bytes();
    let hash = hex::encode(Sha256::digest(&bytes));
    Ok(CanonicalTradeProposalV1 {
        hash_version: 1,
        bytes,
        hash,
    })
}

pub fn proposal_hash(
    proposal: &TradeProposal,
    version: u32,
) -> Result<String, CanonicalizationError> {
    match version {
        1 => Ok(canonicalize_proposal_v1(proposal)?.hash),
        unknown => Err(CanonicalizationError::UnknownHashVersion(unknown)),
    }
}

fn escape_field(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\n', "\\n")
}

fn decimal_field(value: Option<Decimal>) -> String {
    value
        .map(|decimal| decimal.normalize().to_string())
        .unwrap_or_else(|| "~".into())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionPermit {
    proposal_id: String,
    approval_request_id: String,
}

impl ExecutionPermit {
    pub fn from_validated_approval(
        proposal_id: impl Into<String>,
        approval_request_id: impl Into<String>,
    ) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            approval_request_id: approval_request_id.into(),
        }
    }

    pub fn proposal_id(&self) -> &str {
        &self.proposal_id
    }

    pub fn approval_request_id(&self) -> &str {
        &self.approval_request_id
    }

    #[cfg(test)]
    fn for_test_only(proposal_id: String) -> Self {
        Self::from_validated_approval(proposal_id, "approval-test")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Starting,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRecord {
    pub id: String,
    pub proposal_id: String,
    pub approval_request_id: String,
    pub adapter_id: String,
    pub status: ExecutionStatus,
    pub result_summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ExecutionError {
    #[error("execution permit does not match proposal")]
    PermitMismatch,
}

#[derive(Default)]
pub struct PaperExecutionAdapter;

impl PaperExecutionAdapter {
    pub fn execute(
        &self,
        proposal: &TradeProposal,
        permit: ExecutionPermit,
    ) -> Result<ExecutionRecord, ExecutionError> {
        if permit.proposal_id() != proposal.id {
            return Err(ExecutionError::PermitMismatch);
        }
        let now = Utc::now();
        Ok(ExecutionRecord {
            id: new_id("execution"),
            proposal_id: proposal.id.clone(),
            approval_request_id: permit.approval_request_id().to_owned(),
            adapter_id: "paper".into(),
            status: ExecutionStatus::Completed,
            result_summary: Some(format!(
                "Paper {} {} {}",
                proposal.action.as_str(),
                proposal.quantity.normalize(),
                proposal.instrument
            )),
            created_at: now,
            completed_at: Some(now),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use market_core::Market;
    use rust_decimal_macros::dec;

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
            "Limited to a paper adapter",
        )
    }

    #[test]
    fn canonical_hash_is_deterministic_and_versioned() {
        let first = canonicalize_proposal_v1(&proposal()).unwrap();
        let second = canonicalize_proposal_v1(&proposal()).unwrap();
        assert_eq!(first.hash_version, 1);
        assert_eq!(first.hash, second.hash);
        assert_eq!(first.bytes, second.bytes);
    }

    #[test]
    fn every_material_risk_field_changes_the_hash() {
        let original = proposal_hash(&proposal(), 1).unwrap();
        let mut variants = Vec::new();
        let mut changed = proposal();
        changed.instrument = "ETH-USDT".into();
        variants.push(changed);
        let mut changed = proposal();
        changed.action = TradeAction::OpenShort;
        variants.push(changed);
        let mut changed = proposal();
        changed.order_type = OrderType::Market;
        variants.push(changed);
        let mut changed = proposal();
        changed.quantity = dec!(0.02);
        variants.push(changed);
        let mut changed = proposal();
        changed.price = Some(dec!(65001));
        variants.push(changed);
        let mut changed = proposal();
        changed.stop_loss = Some(dec!(61000));
        variants.push(changed);
        let mut changed = proposal();
        changed.take_profit = Some(dec!(72000));
        variants.push(changed);
        assert!(
            variants
                .into_iter()
                .all(|variant| proposal_hash(&variant, 1).unwrap() != original)
        );
    }

    #[test]
    fn unknown_hash_version_is_rejected() {
        assert_eq!(
            proposal_hash(&proposal(), 2).unwrap_err(),
            CanonicalizationError::UnknownHashVersion(2)
        );
    }

    #[test]
    fn paper_adapter_requires_validated_authorization() {
        let proposal = proposal();
        let permit = ExecutionPermit::for_test_only(proposal.id.clone());
        let record = PaperExecutionAdapter.execute(&proposal, permit).unwrap();
        assert_eq!(record.status, ExecutionStatus::Completed);
        assert_eq!(record.adapter_id, "paper");
    }
}
