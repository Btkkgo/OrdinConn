use chrono::{DateTime, Utc};
use market_core::{AssetRef, new_id};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceType {
    OfficialApi,
    ExchangeApi,
    Blockchain,
    OfficialNews,
    CompanyDisclosure,
    Browser,
    ComputerUse,
    Ocr,
    Social,
    UserInput,
    ModelInference,
}

impl SourceType {
    pub const fn qualifies_for_publication(self) -> bool {
        !matches!(self, Self::ModelInference)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FactualLevel {
    Official,
    Observed,
    Secondary,
    Rumor,
    Inference,
}

impl FactualLevel {
    pub const fn weight(self) -> f64 {
        match self {
            Self::Official => 1.0,
            Self::Observed => 0.9,
            Self::Secondary => 0.7,
            Self::Rumor => 0.35,
            Self::Inference => 0.25,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRelation {
    Primary,
    Supporting,
    Contradicting,
    Context,
}

impl EvidenceRelation {
    pub const fn weight(self) -> f64 {
        match self {
            Self::Primary => 1.0,
            Self::Supporting => 0.9,
            Self::Context => 0.75,
            Self::Contradicting => 0.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub id: String,
    pub source: String,
    pub source_type: SourceType,
    pub asset: AssetRef,
    pub title: String,
    pub content: String,
    pub raw_reference: Option<String>,
    pub captured_at: DateTime<Utc>,
    pub freshness: f64,
    pub reliability: f64,
    pub factual_level: FactualLevel,
    pub confidence: f64,
    pub metadata: Value,
}

impl Evidence {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: impl Into<String>,
        source_type: SourceType,
        asset: AssetRef,
        title: impl Into<String>,
        content: impl Into<String>,
        factual_level: FactualLevel,
        reliability: f64,
        confidence: f64,
        captured_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: new_id("evidence"),
            source: source.into(),
            source_type,
            asset,
            title: title.into(),
            content: content.into(),
            raw_reference: None,
            captured_at,
            freshness: 1.0,
            reliability: reliability.clamp(0.0, 1.0),
            factual_level,
            confidence: confidence.clamp(0.0, 1.0),
            metadata: Value::Object(Default::default()),
        }
    }

    pub fn quality_score(&self) -> f64 {
        (self.reliability * 0.4
            + self.freshness.clamp(0.0, 1.0) * 0.25
            + self.confidence * 0.2
            + self.factual_level.weight() * 0.15)
            .clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_inference_is_not_source_evidence() {
        assert!(!SourceType::ModelInference.qualifies_for_publication());
        assert!(SourceType::ExchangeApi.qualifies_for_publication());
    }
}
