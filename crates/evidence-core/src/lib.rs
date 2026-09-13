use chrono::{DateTime, Utc};
use market_core::{AssetRef, new_id};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReliabilityTier {
    Tier1Official,
    Tier2Primary,
    Tier3Secondary,
    Tier4Unverified,
}

impl ReliabilityTier {
    pub const fn score(self) -> f64 {
        match self {
            Self::Tier1Official => 0.95,
            Self::Tier2Primary => 0.85,
            Self::Tier3Secondary => 0.65,
            Self::Tier4Unverified => 0.35,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterRelation {
    Original,
    Syndication,
    Independent,
    Contradicting,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataOrigin {
    Real,
    Mock,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceFingerprint {
    pub source_id: String,
    pub canonical_url: Option<String>,
    pub original_url: Option<String>,
    pub content: String,
}

pub fn classify_cluster_relation(
    original: &EvidenceFingerprint,
    candidate: &EvidenceFingerprint,
    contradicting: bool,
) -> ClusterRelation {
    if contradicting {
        return ClusterRelation::Contradicting;
    }
    let shared_url = candidate.canonical_url.as_ref().is_some_and(|url| {
        original.canonical_url.as_ref() == Some(url) || original.original_url.as_ref() == Some(url)
    }) || candidate.original_url.as_ref().is_some_and(|url| {
        original.canonical_url.as_ref() == Some(url) || original.original_url.as_ref() == Some(url)
    });
    if candidate.source_id == original.source_id
        || shared_url
        || content_similarity(&original.content, &candidate.content) >= 0.8
    {
        ClusterRelation::Syndication
    } else {
        ClusterRelation::Independent
    }
}

fn content_similarity(left: &str, right: &str) -> f64 {
    let tokens = |value: &str| {
        value
            .to_lowercase()
            .split(|character: char| !character.is_alphanumeric())
            .filter(|token| token.len() > 2)
            .map(str::to_owned)
            .collect::<std::collections::HashSet<_>>()
    };
    let left = tokens(left);
    let right = tokens(right);
    let union = left.union(&right).count();
    if union == 0 {
        0.0
    } else {
        left.intersection(&right).count() as f64 / union as f64
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceClusterMember {
    pub evidence_id: String,
    pub source_id: String,
    pub relation: ClusterRelation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceCluster {
    pub id: String,
    pub members: Vec<EvidenceClusterMember>,
    pub contradiction_count: u32,
}

impl EvidenceCluster {
    pub fn independent_confirmations(&self) -> usize {
        self.members
            .iter()
            .filter(|member| {
                matches!(
                    member.relation,
                    ClusterRelation::Original | ClusterRelation::Independent
                )
            })
            .map(|member| member.source_id.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len()
            .saturating_sub(1)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FreshnessPolicy {
    pub fresh_for_seconds: i64,
    pub stale_after_seconds: i64,
}

impl FreshnessPolicy {
    pub fn score(&self, captured_at: DateTime<Utc>, now: DateTime<Utc>) -> f64 {
        let age = (now - captured_at).num_seconds().max(0);
        if age <= self.fresh_for_seconds {
            1.0
        } else if age >= self.stale_after_seconds {
            0.0
        } else {
            1.0 - (age - self.fresh_for_seconds) as f64
                / (self.stale_after_seconds - self.fresh_for_seconds).max(1) as f64
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceAssessment {
    pub quality: f64,
    pub reliability: f64,
    pub freshness: f64,
    pub completeness: f64,
    pub independent_confirmations: usize,
    pub contradictions: u32,
}

pub fn assess_evidence(
    evidence: &Evidence,
    completeness: f64,
    cluster: Option<&EvidenceCluster>,
) -> EvidenceAssessment {
    let independent_confirmations = cluster.map_or(0, EvidenceCluster::independent_confirmations);
    let contradictions = cluster.map_or(0, |item| item.contradiction_count);
    let confirmation_bonus = (independent_confirmations.min(3) as f64) * 0.05;
    let contradiction_penalty = (contradictions.min(3) as f64) * 0.12;
    let quality = (evidence.reliability * 0.35
        + evidence.freshness.clamp(0.0, 1.0) * 0.25
        + completeness.clamp(0.0, 1.0) * 0.20
        + evidence.factual_level.weight() * 0.20
        + confirmation_bonus
        - contradiction_penalty)
        .clamp(0.0, 1.0);
    EvidenceAssessment {
        quality,
        reliability: evidence.reliability,
        freshness: evidence.freshness.clamp(0.0, 1.0),
        completeness: completeness.clamp(0.0, 1.0),
        independent_confirmations,
        contradictions,
    }
}

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
    #[serde(default)]
    pub data_origin: DataOrigin,
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
            data_origin: DataOrigin::Unknown,
        }
    }

    pub fn quality_score(&self) -> f64 {
        assess_evidence(self, self.confidence, None).quality
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

    #[test]
    fn syndication_does_not_count_as_independent_confirmation() {
        let cluster = EvidenceCluster {
            id: "cluster-1".into(),
            members: vec![
                EvidenceClusterMember {
                    evidence_id: "a".into(),
                    source_id: "official-a".into(),
                    relation: ClusterRelation::Original,
                },
                EvidenceClusterMember {
                    evidence_id: "b".into(),
                    source_id: "copy-b".into(),
                    relation: ClusterRelation::Syndication,
                },
                EvidenceClusterMember {
                    evidence_id: "c".into(),
                    source_id: "official-c".into(),
                    relation: ClusterRelation::Independent,
                },
            ],
            contradiction_count: 0,
        };
        assert_eq!(cluster.independent_confirmations(), 1);
    }

    #[test]
    fn contradictions_reduce_deterministic_quality() {
        let item = Evidence::new(
            "official",
            SourceType::OfficialApi,
            AssetRef::new(market_core::Market::Traditional, "USD"),
            "title",
            "body",
            FactualLevel::Official,
            0.95,
            1.0,
            Utc::now(),
        );
        let clean = EvidenceCluster {
            id: "a".into(),
            members: vec![],
            contradiction_count: 0,
        };
        let conflicted = EvidenceCluster {
            id: "b".into(),
            members: vec![],
            contradiction_count: 2,
        };
        assert!(
            assess_evidence(&item, 1.0, Some(&conflicted)).quality
                < assess_evidence(&item, 1.0, Some(&clean)).quality
        );
    }

    #[test]
    fn freshness_policy_degrades_to_zero() {
        let now = Utc::now();
        let policy = FreshnessPolicy {
            fresh_for_seconds: 60,
            stale_after_seconds: 120,
        };
        assert_eq!(policy.score(now - chrono::Duration::seconds(30), now), 1.0);
        assert_eq!(policy.score(now - chrono::Duration::seconds(121), now), 0.0);
    }

    #[test]
    fn canonical_url_and_similar_content_are_syndication_not_confirmation() {
        let original = EvidenceFingerprint {
            source_id: "official".into(),
            canonical_url: Some("https://source.test/a".into()),
            original_url: None,
            content: "Federal Reserve policy statement holds rates steady today".into(),
        };
        let linked = EvidenceFingerprint {
            source_id: "wire-copy".into(),
            canonical_url: Some("https://copy.test/a".into()),
            original_url: Some("https://source.test/a".into()),
            content: "Federal Reserve policy statement holds rates steady today".into(),
        };
        assert_eq!(
            classify_cluster_relation(&original, &linked, false),
            ClusterRelation::Syndication
        );
        let independent = EvidenceFingerprint {
            source_id: "independent".into(),
            canonical_url: Some("https://independent.test/b".into()),
            original_url: None,
            content: "Bond yields moved after the central bank announcement".into(),
        };
        assert_eq!(
            classify_cluster_relation(&original, &independent, false),
            ClusterRelation::Independent
        );
        assert_eq!(
            classify_cluster_relation(&original, &independent, true),
            ClusterRelation::Contradicting
        );
    }
}
