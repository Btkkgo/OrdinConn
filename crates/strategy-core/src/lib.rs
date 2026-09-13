use chrono::{DateTime, Utc};
use collector_runtime::{NormalizedObservation, ObservationKind};
use market_core::{AssetRef, Market, new_id};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use signal_core::{
    EvidenceLink, SignalCandidate, SignalCategory, SignalDirection, StrategyProvenance,
};
use std::collections::HashMap;
use thiserror::Error;

pub mod rolling;
pub use rolling::*;
pub mod derivatives;
pub use derivatives::*;

pub const ROLLING_WINDOWS: &[&str] = &["1m", "5m", "15m", "1h", "4h", "24h"];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyDefinition {
    pub id: String,
    pub version: String,
    pub market: Market,
    pub category: SignalCategory,
    pub required_observation_kinds: Vec<ObservationKind>,
    pub rolling_windows: Vec<String>,
    pub parameters: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyContext {
    pub now: DateTime<Utc>,
    pub observations: Vec<NormalizedObservation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyResult {
    pub id: String,
    pub strategy_id: String,
    pub strategy_version: String,
    pub triggered: bool,
    pub direction: SignalDirection,
    pub score: f64,
    pub reason_codes: Vec<String>,
    pub observation_ids: Vec<String>,
    pub parameter_snapshot: Value,
    pub rejected_reason: Option<String>,
    #[serde(default)]
    pub readiness: Readiness,
    #[serde(default)]
    pub instrument_id: Option<String>,
    #[serde(default)]
    pub baseline_window: Option<String>,
    #[serde(default)]
    pub input_snapshot: Value,
    #[serde(default)]
    pub trigger_metrics: HashMap<String, f64>,
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
}

impl StrategyResult {
    pub fn to_candidate(
        &self,
        asset: AssetRef,
        category: SignalCategory,
        title: impl Into<String>,
        summary: impl Into<String>,
        evidence: Vec<EvidenceLink>,
    ) -> Option<SignalCandidate> {
        (self.triggered && self.readiness == Readiness::Ready).then(|| {
            let evidence_ids = evidence
                .iter()
                .map(|link| link.evidence.id.clone())
                .collect();
            SignalCandidate::new(asset, category, title, summary, self.direction, evidence)
                .with_strategy(StrategyProvenance {
                    strategy_id: self.strategy_id.clone(),
                    strategy_version: self.strategy_version.clone(),
                    parameter_snapshot: self.parameter_snapshot.clone(),
                    reason_codes: self.reason_codes.clone(),
                    observation_ids: self.observation_ids.clone(),
                    baseline_window: self.baseline_window.clone(),
                    trigger_metrics: self.trigger_metrics.clone(),
                    source_ids: vec![],
                    evidence_ids,
                    input_snapshot: self.input_snapshot.clone(),
                })
        })
    }
}

#[derive(Clone, Debug)]
pub struct StrategyConfig {
    pub price_move_abs: f64,
    pub volume_zscore: f64,
    pub volatility_zscore: f64,
    pub breadth_ratio: f64,
    pub relative_strength: f64,
    pub macro_rate_change_bps: f64,
    pub demand_score: f64,
    pub funding_abs: f64,
    pub open_interest_growth: f64,
    pub book_imbalance: f64,
    pub spread_bps: f64,
    pub max_age_seconds: i64,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            price_move_abs: 0.03,
            volume_zscore: 2.0,
            volatility_zscore: 2.0,
            breadth_ratio: 0.65,
            relative_strength: 0.02,
            macro_rate_change_bps: 15.0,
            demand_score: 0.65,
            funding_abs: 0.0005,
            open_interest_growth: 0.10,
            book_imbalance: 0.20,
            spread_bps: 10.0,
            max_age_seconds: 86_400,
        }
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum StrategyError {
    #[error("missing required observation")]
    MissingObservation,
    #[error("required metric is missing: {0}")]
    MissingMetric(String),
    #[error("strategy is not registered: {0}")]
    UnknownStrategy(String),
}

pub struct StrategyEngine {
    definitions: HashMap<String, StrategyDefinition>,
    config: StrategyConfig,
}

impl Default for StrategyEngine {
    fn default() -> Self {
        Self::new(StrategyConfig::default())
    }
}

impl StrategyEngine {
    pub fn new(config: StrategyConfig) -> Self {
        Self {
            definitions: builtin_strategies(&config)
                .into_iter()
                .map(|d| (d.id.clone(), d))
                .collect(),
            config,
        }
    }
    pub fn definitions(&self) -> Vec<&StrategyDefinition> {
        let mut items: Vec<_> = self.definitions.values().collect();
        items.sort_by_key(|d| &d.id);
        items
    }
    pub fn evaluate(
        &self,
        strategy_id: &str,
        context: &StrategyContext,
    ) -> Result<StrategyResult, StrategyError> {
        let definition = self
            .definitions
            .get(strategy_id)
            .ok_or_else(|| StrategyError::UnknownStrategy(strategy_id.into()))?;
        let relevant: Vec<_> = context
            .observations
            .iter()
            .filter(|o| definition.required_observation_kinds.contains(&o.kind))
            .collect();
        if relevant.is_empty() {
            return Err(StrategyError::MissingObservation);
        }
        let stale = relevant.iter().any(|o| {
            o.stale || (context.now - o.observed_at).num_seconds() > self.config.max_age_seconds
        });
        let mut result = match strategy_id {
            "traditional.a.price_volume_expansion" => threshold_pair(
                definition,
                &relevant,
                "return",
                self.config.price_move_abs,
                "volume_zscore",
                self.config.volume_zscore,
                "price_volume_expansion",
            ),
            "traditional.a.volatility_expansion" => threshold(
                definition,
                &relevant,
                "volatility_zscore",
                self.config.volatility_zscore,
                "volatility_expansion",
            ),
            "traditional.a.breadth_relative_strength" => threshold_pair(
                definition,
                &relevant,
                "breadth_ratio",
                self.config.breadth_ratio,
                "relative_strength",
                self.config.relative_strength,
                "breadth_relative_strength",
            ),
            "traditional.a.macro_repricing" => threshold(
                definition,
                &relevant,
                "rate_change_bps_abs",
                self.config.macro_rate_change_bps,
                "macro_repricing",
            ),
            "traditional.b.official_event" => evaluate_event(definition, &relevant),
            "traditional.c.demand_score" => {
                evaluate_demand(definition, &relevant, self.config.demand_score)
            }
            "crypto.a.onchain_pressure" => evaluate_onchain(definition, &relevant),
            "crypto.b.official_event" => evaluate_event(definition, &relevant),
            "crypto.b.narrative_observation" => evaluate_narrative(definition, &relevant),
            "crypto.c.funding_anomaly" => threshold_abs(
                definition,
                &relevant,
                "funding_rate",
                self.config.funding_abs,
                "funding_anomaly",
            ),
            "crypto.c.open_interest_expansion" => threshold(
                definition,
                &relevant,
                "open_interest_growth",
                self.config.open_interest_growth,
                "open_interest_expansion",
            ),
            "crypto.c.price_oi_divergence" => evaluate_divergence(definition, &relevant),
            "crypto.c.volume_expansion" => threshold(
                definition,
                &relevant,
                "volume_zscore",
                self.config.volume_zscore,
                "volume_expansion",
            ),
            "crypto.c.book_imbalance" => threshold_abs(
                definition,
                &relevant,
                "book_imbalance",
                self.config.book_imbalance,
                "book_imbalance",
            ),
            "crypto.c.spread_anomaly" => threshold(
                definition,
                &relevant,
                "spread_bps",
                self.config.spread_bps,
                "spread_anomaly",
            ),
            _ => return Err(StrategyError::UnknownStrategy(strategy_id.into())),
        }?;
        if stale {
            result.triggered = false;
            result.score = result.score.min(0.35);
            result.reason_codes.push("stale_input".into());
            result.rejected_reason = Some("stale_observation".into());
            result.readiness = Readiness::StaleInput;
        }
        Ok(result)
    }
}

fn base_result(
    definition: &StrategyDefinition,
    observations: &[&NormalizedObservation],
) -> StrategyResult {
    StrategyResult {
        id: new_id("strategy_run"),
        strategy_id: definition.id.clone(),
        strategy_version: definition.version.clone(),
        triggered: false,
        direction: SignalDirection::Watch,
        score: 0.0,
        reason_codes: vec![],
        observation_ids: observations.iter().map(|o| o.id.clone()).collect(),
        parameter_snapshot: definition.parameters.clone(),
        rejected_reason: None,
        readiness: Readiness::Ready,
        instrument_id: observations.first().map(|item| item.asset.symbol.clone()),
        baseline_window: None,
        input_snapshot: Value::Null,
        trigger_metrics: HashMap::new(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
    }
}
fn first_metric(observations: &[&NormalizedObservation], key: &str) -> Result<f64, StrategyError> {
    observations
        .iter()
        .find_map(|o| o.metrics.get(key).copied())
        .ok_or_else(|| StrategyError::MissingMetric(key.into()))
}
fn threshold(
    def: &StrategyDefinition,
    obs: &[&NormalizedObservation],
    key: &str,
    limit: f64,
    reason: &str,
) -> Result<StrategyResult, StrategyError> {
    let value = first_metric(obs, key)?;
    let mut r = base_result(def, obs);
    r.triggered = value >= limit;
    r.score = (value / limit.max(f64::EPSILON) / 2.0).clamp(0.0, 1.0);
    if r.triggered {
        r.reason_codes.push(reason.into());
        r.direction = SignalDirection::Bullish;
    } else {
        r.rejected_reason = Some("threshold_not_met".into());
    }
    Ok(r)
}
fn threshold_abs(
    def: &StrategyDefinition,
    obs: &[&NormalizedObservation],
    key: &str,
    limit: f64,
    reason: &str,
) -> Result<StrategyResult, StrategyError> {
    let value = first_metric(obs, key)?;
    let mut r = base_result(def, obs);
    r.triggered = value.abs() >= limit;
    r.score = (value.abs() / limit.max(f64::EPSILON) / 2.0).clamp(0.0, 1.0);
    if r.triggered {
        r.reason_codes.push(reason.into());
        r.direction = if value > 0.0 {
            SignalDirection::Bearish
        } else {
            SignalDirection::Bullish
        };
    } else {
        r.rejected_reason = Some("threshold_not_met".into());
    }
    Ok(r)
}
fn threshold_pair(
    def: &StrategyDefinition,
    obs: &[&NormalizedObservation],
    a: &str,
    a_limit: f64,
    b: &str,
    b_limit: f64,
    reason: &str,
) -> Result<StrategyResult, StrategyError> {
    let av = first_metric(obs, a)?;
    let bv = first_metric(obs, b)?;
    let mut r = base_result(def, obs);
    r.triggered = av.abs() >= a_limit && bv >= b_limit;
    r.score = ((av.abs() / a_limit + bv / b_limit) / 4.0).clamp(0.0, 1.0);
    if r.triggered {
        r.reason_codes.push(reason.into());
        r.direction = if av >= 0.0 {
            SignalDirection::Bullish
        } else {
            SignalDirection::Bearish
        };
    } else {
        r.rejected_reason = Some("combined_threshold_not_met".into());
    }
    Ok(r)
}

fn evaluate_event(
    def: &StrategyDefinition,
    obs: &[&NormalizedObservation],
) -> Result<StrategyResult, StrategyError> {
    let facts = obs
        .iter()
        .flat_map(|o| &o.facts)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let classes = [
        ("rate", "monetary_policy"),
        ("inflation", "inflation"),
        ("regulat", "regulation"),
        ("earnings", "company_results"),
        ("launch", "product_release"),
    ];
    let mut r = base_result(def, obs);
    for (needle, class) in classes {
        if facts.contains(needle) {
            r.triggered = true;
            r.reason_codes.push(format!("event_class:{class}"));
        }
    }
    if !r.triggered {
        r.rejected_reason = Some("unclassified_event".into());
    }
    r.score = if r.triggered { 0.7 } else { 0.0 };
    r.direction = SignalDirection::Watch;
    Ok(r)
}
fn evaluate_demand(
    def: &StrategyDefinition,
    obs: &[&NormalizedObservation],
    limit: f64,
) -> Result<StrategyResult, StrategyError> {
    let order = first_metric(obs, "order_growth")?;
    let backlog = first_metric(obs, "backlog_growth")?;
    let utilization = first_metric(obs, "utilization")?;
    let lead = first_metric(obs, "lead_time_growth")?;
    let capex = first_metric(obs, "capex_growth")?;
    let inventory = first_metric(obs, "inventory_growth")?;
    let score = (0.22 * order + 0.18 * backlog + 0.18 * utilization + 0.14 * lead + 0.16 * capex
        - 0.12 * inventory)
        .clamp(0.0, 1.0);
    let mut r = base_result(def, obs);
    r.score = score;
    r.triggered = score >= limit;
    r.direction = SignalDirection::Bullish;
    if r.triggered {
        r.reason_codes.push("demand_score_high".into())
    } else {
        r.rejected_reason = Some("demand_score_below_threshold".into())
    };
    Ok(r)
}
fn evaluate_onchain(
    def: &StrategyDefinition,
    obs: &[&NormalizedObservation],
) -> Result<StrategyResult, StrategyError> {
    let pressure = first_metric(obs, "mempool_pressure")?;
    let fee = first_metric(obs, "fee_zscore")?;
    let mut r = base_result(def, obs);
    r.triggered = pressure >= 0.75 && fee >= 2.0;
    r.score = ((pressure + fee / 4.0) / 2.0).clamp(0.0, 1.0);
    r.direction = SignalDirection::Watch;
    if r.triggered {
        r.reason_codes.push("btc_mempool_pressure".into())
    } else {
        r.rejected_reason = Some("onchain_threshold_not_met".into())
    };
    Ok(r)
}
fn evaluate_narrative(
    def: &StrategyDefinition,
    obs: &[&NormalizedObservation],
) -> Result<StrategyResult, StrategyError> {
    let mentions = first_metric(obs, "mention_zscore")?;
    let mut r = base_result(def, obs);
    r.triggered = mentions >= 2.0;
    r.score = (mentions / 4.0).clamp(0.0, 1.0);
    r.direction = SignalDirection::Watch;
    if r.triggered {
        r.reason_codes
            .extend(["narrative_acceleration".into(), "not_a_buy_signal".into()])
    } else {
        r.rejected_reason = Some("narrative_threshold_not_met".into())
    };
    Ok(r)
}
fn evaluate_divergence(
    def: &StrategyDefinition,
    obs: &[&NormalizedObservation],
) -> Result<StrategyResult, StrategyError> {
    let price = first_metric(obs, "price_return")?;
    let oi = first_metric(obs, "open_interest_growth")?;
    let mut r = base_result(def, obs);
    r.triggered = price.signum() != oi.signum() && price.abs() >= 0.02 && oi.abs() >= 0.05;
    r.score = ((price.abs() / 0.04 + oi.abs() / 0.10) / 2.0).clamp(0.0, 1.0);
    r.direction = SignalDirection::Watch;
    if r.triggered {
        r.reason_codes.push("price_oi_divergence".into())
    } else {
        r.rejected_reason = Some("divergence_not_present".into())
    };
    Ok(r)
}

pub fn builtin_strategies(config: &StrategyConfig) -> Vec<StrategyDefinition> {
    let def = |id: &str, market, category, kinds: Vec<_>, parameters| StrategyDefinition {
        id: id.into(),
        version: "1.0.0".into(),
        market,
        category,
        required_observation_kinds: kinds,
        rolling_windows: ROLLING_WINDOWS.iter().map(|v| (*v).into()).collect(),
        parameters,
    };
    vec![
        def(
            "traditional.a.price_volume_expansion",
            Market::Traditional,
            SignalCategory::Trading,
            vec![ObservationKind::PriceVolume],
            json!({"priceMoveAbs":config.price_move_abs,"volumeZscore":config.volume_zscore}),
        ),
        def(
            "traditional.a.volatility_expansion",
            Market::Traditional,
            SignalCategory::Trading,
            vec![ObservationKind::Volatility],
            json!({"volatilityZscore":config.volatility_zscore}),
        ),
        def(
            "traditional.a.breadth_relative_strength",
            Market::Traditional,
            SignalCategory::Trading,
            vec![ObservationKind::Breadth],
            json!({"breadthRatio":config.breadth_ratio,"relativeStrength":config.relative_strength}),
        ),
        def(
            "traditional.a.macro_repricing",
            Market::Traditional,
            SignalCategory::Trading,
            vec![ObservationKind::Macro],
            json!({"rateChangeBps":config.macro_rate_change_bps}),
        ),
        def(
            "traditional.b.official_event",
            Market::Traditional,
            SignalCategory::Event,
            vec![ObservationKind::Event],
            json!({"factOnly":true}),
        ),
        def(
            "traditional.c.demand_score",
            Market::Traditional,
            SignalCategory::Demand,
            vec![ObservationKind::Demand],
            json!({"minimum":config.demand_score,"formulaVersion":"1"}),
        ),
        def(
            "crypto.a.onchain_pressure",
            Market::Crypto,
            SignalCategory::Onchain,
            vec![ObservationKind::Onchain],
            json!({"mempoolPressure":0.75,"feeZscore":2.0}),
        ),
        def(
            "crypto.b.official_event",
            Market::Crypto,
            SignalCategory::Event,
            vec![ObservationKind::Event],
            json!({"officialOnly":true}),
        ),
        def(
            "crypto.b.narrative_observation",
            Market::Crypto,
            SignalCategory::Social,
            vec![ObservationKind::Narrative],
            json!({"mentionZscore":2.0,"buySignal":false}),
        ),
        def(
            "crypto.c.funding_anomaly",
            Market::Crypto,
            SignalCategory::Exchange,
            vec![ObservationKind::Funding],
            json!({"fundingAbs":config.funding_abs}),
        ),
        def(
            "crypto.c.open_interest_expansion",
            Market::Crypto,
            SignalCategory::Exchange,
            vec![ObservationKind::OpenInterest],
            json!({"growth":config.open_interest_growth}),
        ),
        def(
            "crypto.c.price_oi_divergence",
            Market::Crypto,
            SignalCategory::Exchange,
            vec![ObservationKind::PriceVolume, ObservationKind::OpenInterest],
            json!({"priceAbs":0.02,"oiAbs":0.05}),
        ),
        def(
            "crypto.c.volume_expansion",
            Market::Crypto,
            SignalCategory::Exchange,
            vec![ObservationKind::PriceVolume],
            json!({"volumeZscore":config.volume_zscore}),
        ),
        def(
            "crypto.c.book_imbalance",
            Market::Crypto,
            SignalCategory::Exchange,
            vec![ObservationKind::OrderBook],
            json!({"imbalanceAbs":config.book_imbalance}),
        ),
        def(
            "crypto.c.spread_anomaly",
            Market::Crypto,
            SignalCategory::Exchange,
            vec![ObservationKind::Spread],
            json!({"spreadBps":config.spread_bps}),
        ),
    ]
}

#[derive(Default)]
pub struct InstrumentRegistry {
    aliases: HashMap<String, AssetRef>,
}
impl InstrumentRegistry {
    pub fn with_defaults() -> Self {
        let mut r = Self::default();
        for base in ["BTC", "ETH", "SOL"] {
            let asset = AssetRef::new(Market::Crypto, format!("{base}-USDT-SPOT"));
            for alias in [
                format!("{base}USDT"),
                format!("{base}/USDT"),
                format!("{base}-USDT"),
                format!("{base}-USDT-SPOT"),
            ] {
                r.aliases.insert(alias, asset.clone());
            }
        }
        r
    }
    pub fn resolve_perpetual(&self, value: &str) -> Option<AssetRef> {
        let compact = value.to_ascii_uppercase().replace(['-', '/'], "");
        ["BTC", "ETH", "SOL"]
            .into_iter()
            .find(|base| compact == format!("{base}USDT") || compact == format!("{base}USDTPERP"))
            .map(|base| AssetRef::new(Market::Crypto, format!("{base}-USDT-PERP")))
    }
    pub fn resolve(&self, value: &str) -> Option<&AssetRef> {
        self.aliases.get(&value.to_ascii_uppercase())
    }
}

#[derive(Default)]
pub struct EntityRegistry {
    aliases: HashMap<String, String>,
}
impl EntityRegistry {
    pub fn with_defaults() -> Self {
        let mut r = Self::default();
        for alias in ["NVIDIA", "NVDA", "英伟达"] {
            r.aliases
                .insert(alias.to_lowercase(), "entity:nvidia".into());
        }
        r
    }
    pub fn resolve(&self, value: &str) -> Option<&str> {
        self.aliases.get(&value.to_lowercase()).map(String::as_str)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationConfidence {
    Verified,
    Inferred,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplyChainRelation {
    pub id: String,
    pub upstream_entity_id: String,
    pub downstream_entity_id: String,
    pub relation_type: String,
    pub confidence: RelationConfidence,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransmissionPathProposal {
    pub event_class: String,
    pub source_entity_id: String,
    pub path: Vec<String>,
    pub factual_level: TransmissionFactualLevel,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransmissionFactualLevel {
    Inference,
}

pub fn fed_transmission_paths(event_class: &str) -> Vec<TransmissionPathProposal> {
    if !matches!(event_class, "monetary_policy" | "inflation") {
        return vec![];
    }
    [
        vec!["rates", "usd"],
        vec!["rates", "gold"],
        vec!["rates", "equity"],
        vec!["rates", "crypto"],
    ]
    .into_iter()
    .map(|path| TransmissionPathProposal {
        event_class: event_class.into(),
        source_entity_id: "entity:federal-reserve".into(),
        path: path.into_iter().map(str::to_owned).collect(),
        factual_level: TransmissionFactualLevel::Inference,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observation(kind: ObservationKind, metrics: &[(&str, f64)]) -> NormalizedObservation {
        NormalizedObservation {
            id: "obs-1".into(),
            raw_record_id: "raw-1".into(),
            source_id: "fixture".into(),
            kind,
            asset: AssetRef::new(Market::Crypto, "BTC-USDT"),
            entity_id: None,
            observed_at: Utc::now(),
            schema_version: "fixture.v1".into(),
            metrics: metrics.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            facts: vec![],
            completeness: 1.0,
            stale: false,
        }
    }
    fn context(obs: NormalizedObservation) -> StrategyContext {
        StrategyContext {
            now: Utc::now(),
            observations: vec![obs],
        }
    }
    #[test]
    fn definitions_have_versions_windows_and_parameters() {
        let e = StrategyEngine::default();
        assert_eq!(e.definitions().len(), 15);
        assert!(e.definitions().iter().all(|d| d.version == "1.0.0"
            && d.rolling_windows.len() == 6
            && !d.parameters.is_null()));
    }

    #[test]
    fn fed_transmission_paths_are_never_promoted_to_fact() {
        let paths = fed_transmission_paths("monetary_policy");
        assert_eq!(paths.len(), 4);
        assert!(
            paths
                .iter()
                .all(|path| path.factual_level == TransmissionFactualLevel::Inference)
        );
        assert!(
            paths
                .iter()
                .any(|path| path.path == vec!["rates", "crypto"])
        );
    }
    #[test]
    fn traditional_price_volume_requires_both_thresholds() {
        let e = StrategyEngine::default();
        let r = e
            .evaluate(
                "traditional.a.price_volume_expansion",
                &context(observation(
                    ObservationKind::PriceVolume,
                    &[("return", 0.04), ("volume_zscore", 2.5)],
                )),
            )
            .unwrap();
        assert!(r.triggered);
        assert_eq!(r.direction, SignalDirection::Bullish);
    }
    #[test]
    fn event_classification_separates_fact_from_inference() {
        let e = StrategyEngine::default();
        let mut o = observation(ObservationKind::Event, &[]);
        o.facts = vec!["Federal Reserve rate decision".into()];
        let r = e
            .evaluate("traditional.b.official_event", &context(o))
            .unwrap();
        assert!(r.triggered);
        assert!(
            r.reason_codes
                .contains(&"event_class:monetary_policy".into())
        );
    }
    #[test]
    fn demand_score_is_deterministic() {
        let e = StrategyEngine::default();
        let metrics = [
            ("order_growth", 1.0),
            ("backlog_growth", 1.0),
            ("utilization", 1.0),
            ("lead_time_growth", 1.0),
            ("capex_growth", 1.0),
            ("inventory_growth", 0.0),
        ];
        let r = e
            .evaluate(
                "traditional.c.demand_score",
                &context(observation(ObservationKind::Demand, &metrics)),
            )
            .unwrap();
        assert!(r.triggered);
        assert!((r.score - 0.88).abs() < 1e-9);
    }
    #[test]
    fn crypto_exchange_strategies_cover_six_conditions() {
        let e = StrategyEngine::default();
        let cases = [
            (
                "crypto.c.funding_anomaly",
                ObservationKind::Funding,
                vec![("funding_rate", 0.001)],
            ),
            (
                "crypto.c.open_interest_expansion",
                ObservationKind::OpenInterest,
                vec![("open_interest_growth", 0.2)],
            ),
            (
                "crypto.c.volume_expansion",
                ObservationKind::PriceVolume,
                vec![("volume_zscore", 3.0)],
            ),
            (
                "crypto.c.book_imbalance",
                ObservationKind::OrderBook,
                vec![("book_imbalance", 0.4)],
            ),
            (
                "crypto.c.spread_anomaly",
                ObservationKind::Spread,
                vec![("spread_bps", 20.0)],
            ),
        ];
        for (id, kind, m) in cases {
            assert!(
                e.evaluate(id, &context(observation(kind, &m)))
                    .unwrap()
                    .triggered
            )
        }
        let c = StrategyContext {
            now: Utc::now(),
            observations: vec![
                observation(ObservationKind::PriceVolume, &[("price_return", 0.03)]),
                observation(
                    ObservationKind::OpenInterest,
                    &[("open_interest_growth", -0.08)],
                ),
            ],
        };
        assert!(
            e.evaluate("crypto.c.price_oi_divergence", &c)
                .unwrap()
                .triggered
        );
    }
    #[test]
    fn stale_data_cannot_trigger_high_confidence() {
        let e = StrategyEngine::default();
        let mut o = observation(ObservationKind::Funding, &[("funding_rate", 0.01)]);
        o.stale = true;
        let r = e.evaluate("crypto.c.funding_anomaly", &context(o)).unwrap();
        assert!(!r.triggered);
        assert_eq!(r.rejected_reason.as_deref(), Some("stale_observation"));
        assert!(r.score <= 0.35);
    }
    #[test]
    fn missing_metric_fails_closed() {
        let e = StrategyEngine::default();
        assert_eq!(
            e.evaluate(
                "crypto.c.funding_anomaly",
                &context(observation(ObservationKind::Funding, &[]))
            )
            .unwrap_err(),
            StrategyError::MissingMetric("funding_rate".into())
        );
    }
    #[test]
    fn aliases_are_canonical_and_bilingual() {
        let instruments = InstrumentRegistry::with_defaults();
        assert_eq!(
            instruments.resolve("btc/usdt").unwrap().symbol,
            "BTC-USDT-SPOT"
        );
        assert_eq!(
            instruments.resolve_perpetual("BTCUSDT").unwrap().symbol,
            "BTC-USDT-PERP"
        );
        let entities = EntityRegistry::with_defaults();
        assert_eq!(entities.resolve("英伟达"), Some("entity:nvidia"));
    }
    #[test]
    fn narrative_is_explicitly_not_a_buy_signal() {
        let e = StrategyEngine::default();
        let r = e
            .evaluate(
                "crypto.b.narrative_observation",
                &context(observation(
                    ObservationKind::Narrative,
                    &[("mention_zscore", 3.0)],
                )),
            )
            .unwrap();
        assert_eq!(r.direction, SignalDirection::Watch);
        assert!(r.reason_codes.contains(&"not_a_buy_signal".into()));
    }
}
