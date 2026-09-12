use chrono::{DateTime, Utc};
use evidence_core::{Evidence, EvidenceRelation};
use market_core::{AssetRef, new_id};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalCategory {
    Trading,
    Event,
    Demand,
    Onchain,
    Exchange,
    Social,
}

impl SignalCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trading => "trading",
            Self::Event => "event",
            Self::Demand => "demand",
            Self::Onchain => "onchain",
            Self::Exchange => "exchange",
            Self::Social => "social",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalDirection {
    Bullish,
    Bearish,
    Neutral,
    Watch,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalStatus {
    New,
    Watching,
    Approved,
    Dismissed,
    Expired,
    Invalidated,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceLink {
    pub evidence: Evidence,
    pub relation: EvidenceRelation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalCandidate {
    pub id: String,
    pub asset: AssetRef,
    pub category: SignalCategory,
    pub title: String,
    pub summary: String,
    pub direction: SignalDirection,
    pub evidence: Vec<EvidenceLink>,
    pub status: CandidateStatus,
    #[serde(default)]
    pub strategy: Option<StrategyProvenance>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyProvenance {
    pub strategy_id: String,
    pub strategy_version: String,
    pub parameter_snapshot: Value,
    pub reason_codes: Vec<String>,
    pub observation_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    PendingValidation,
    InsufficientEvidence,
    Published,
}

impl SignalCandidate {
    pub fn new(
        asset: AssetRef,
        category: SignalCategory,
        title: impl Into<String>,
        summary: impl Into<String>,
        direction: SignalDirection,
        evidence: Vec<EvidenceLink>,
    ) -> Self {
        Self {
            id: new_id("candidate"),
            asset,
            category,
            title: title.into(),
            summary: summary.into(),
            direction,
            evidence,
            status: CandidateStatus::PendingValidation,
            strategy: None,
        }
    }

    pub fn with_strategy(mut self, strategy: StrategyProvenance) -> Self {
        self.strategy = Some(strategy);
        self
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Signal {
    pub id: String,
    pub candidate_id: String,
    pub asset: AssetRef,
    pub category: SignalCategory,
    pub title: String,
    pub summary: String,
    pub direction: SignalDirection,
    pub confidence: f64,
    pub urgency: f64,
    pub time_horizon: String,
    pub evidence_quality: f64,
    pub evidence: Vec<EvidenceLink>,
    pub catalysts: Vec<String>,
    pub risks: Vec<String>,
    pub invalidation_conditions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub agent_id: String,
    pub model_id: String,
    pub status: SignalStatus,
    #[serde(default)]
    pub strategy: Option<StrategyProvenance>,
}

impl Signal {
    pub fn market(&self) -> &'static str {
        self.asset.market.as_str()
    }

    pub fn contradiction_count(&self) -> usize {
        self.evidence
            .iter()
            .filter(|link| link.relation == EvidenceRelation::Contradicting)
            .count()
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum PublishError {
    #[error("candidate does not contain qualifying source evidence")]
    InsufficientEvidence,
}

#[derive(Default)]
pub struct SignalPublisher;

impl SignalPublisher {
    pub fn publish(&self, mut candidate: SignalCandidate) -> Result<Signal, PublishError> {
        let qualifying = candidate
            .evidence
            .iter()
            .filter(|link| link.evidence.source_type.qualifies_for_publication())
            .count();
        if qualifying == 0 {
            candidate.status = CandidateStatus::InsufficientEvidence;
            return Err(PublishError::InsufficientEvidence);
        }

        let contradiction_count = candidate
            .evidence
            .iter()
            .filter(|link| link.relation == EvidenceRelation::Contradicting)
            .count();
        let scored: Vec<f64> = candidate
            .evidence
            .iter()
            .filter(|link| link.relation != EvidenceRelation::Contradicting)
            .map(|link| link.evidence.quality_score() * link.relation.weight())
            .collect();
        let base_quality = scored.iter().sum::<f64>() / scored.len().max(1) as f64;
        let evidence_quality = (base_quality - contradiction_count as f64 * 0.15).clamp(0.0, 1.0);
        let now = Utc::now();
        candidate.status = CandidateStatus::Published;

        Ok(Signal {
            id: new_id("signal"),
            candidate_id: candidate.id,
            asset: candidate.asset,
            category: candidate.category,
            title: candidate.title,
            summary: candidate.summary,
            direction: candidate.direction,
            confidence: (evidence_quality * 0.9).clamp(0.0, 1.0),
            urgency: 0.6,
            time_horizon: "1-4 weeks".into(),
            evidence_quality,
            evidence: candidate.evidence,
            catalysts: vec!["Evidence trend persists".into()],
            risks: vec!["Observed relationship may reverse".into()],
            invalidation_conditions: vec!["Primary evidence returns to baseline".into()],
            created_at: now,
            updated_at: now,
            agent_id: "signal-agent-v0-1".into(),
            model_id: "rules-plus-mock-model".into(),
            status: if contradiction_count > 0 {
                SignalStatus::Watching
            } else {
                SignalStatus::New
            },
            strategy: candidate.strategy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use evidence_core::{Evidence, EvidenceRelation, FactualLevel, SourceType};
    use market_core::{AssetRef, Market};

    fn evidence(source_type: SourceType, relation: EvidenceRelation) -> EvidenceLink {
        EvidenceLink {
            relation,
            evidence: Evidence::new(
                "fixture",
                source_type,
                AssetRef::new(Market::Crypto, "BTC"),
                "Observed exchange flow",
                "Exchange balance declined during the observation window.",
                FactualLevel::Observed,
                0.9,
                0.95,
                Utc::now(),
            ),
        }
    }

    fn candidate(links: Vec<EvidenceLink>) -> SignalCandidate {
        SignalCandidate::new(
            AssetRef::new(Market::Crypto, "BTC"),
            SignalCategory::Exchange,
            "Exchange reserve pressure",
            "Observed reserve movement may change available liquidity.",
            SignalDirection::Watch,
            links,
        )
    }

    #[test]
    fn no_evidence_candidate_is_not_published() {
        let result = SignalPublisher.publish(candidate(vec![]));
        assert_eq!(result.unwrap_err(), PublishError::InsufficientEvidence);
    }

    #[test]
    fn inference_only_candidate_is_not_published() {
        let result = SignalPublisher.publish(candidate(vec![evidence(
            SourceType::ModelInference,
            EvidenceRelation::Primary,
        )]));
        assert_eq!(result.unwrap_err(), PublishError::InsufficientEvidence);
    }

    #[test]
    fn contradicting_evidence_reduces_quality_and_is_preserved() {
        let clean = SignalPublisher
            .publish(candidate(vec![evidence(
                SourceType::ExchangeApi,
                EvidenceRelation::Primary,
            )]))
            .unwrap();
        let conflicted = SignalPublisher
            .publish(candidate(vec![
                evidence(SourceType::ExchangeApi, EvidenceRelation::Primary),
                evidence(SourceType::OfficialNews, EvidenceRelation::Contradicting),
            ]))
            .unwrap();

        assert!(conflicted.evidence_quality < clean.evidence_quality);
        assert_eq!(conflicted.contradiction_count(), 1);
        assert_eq!(conflicted.status, SignalStatus::Watching);
    }

    #[test]
    fn strategy_provenance_survives_publication() {
        let candidate = candidate(vec![evidence(
            SourceType::ExchangeApi,
            EvidenceRelation::Primary,
        )])
        .with_strategy(StrategyProvenance {
            strategy_id: "crypto.c.volume_expansion".into(),
            strategy_version: "1.0.0".into(),
            parameter_snapshot: serde_json::json!({"zScore": 2.0}),
            reason_codes: vec!["volume_zscore_high".into()],
            observation_ids: vec!["obs-1".into()],
        });
        let signal = SignalPublisher.publish(candidate).unwrap();
        assert_eq!(
            signal.strategy.unwrap().strategy_id,
            "crypto.c.volume_expansion"
        );
    }
}
