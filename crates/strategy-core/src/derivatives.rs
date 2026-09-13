use crate::{BaselineSnapshot, Readiness, StrategyConfig, StrategyDefinition, StrategyResult};
use chrono::Utc;
use collector_runtime::ObservationKind;
use market_core::Market;
use market_core::new_id;
use serde_json::json;
use signal_core::SignalDirection;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct DerivativesStrategyEngine {
    config: StrategyConfig,
}

struct ZScoreRule<'a> {
    id: &'a str,
    readiness_prefix: &'a str,
    metric_name: &'a str,
    threshold: f64,
    absolute_threshold: Option<f64>,
    reason: &'a str,
}

impl DerivativesStrategyEngine {
    pub fn new(config: StrategyConfig) -> Self {
        Self { config }
    }

    pub fn funding(&self, baseline: &BaselineSnapshot) -> StrategyResult {
        let mut result = self.result_from_baseline("crypto.exchange.funding_anomaly", baseline);
        if !ready(baseline, &mut result, "funding_history") {
            return result;
        }
        let current = baseline.current_value.unwrap_or_default();
        let z_score = baseline.z_score.unwrap_or_default();
        result
            .trigger_metrics
            .insert("funding_rate".into(), current);
        result
            .trigger_metrics
            .insert("funding_z_score".into(), z_score);
        result.triggered = current.abs() >= self.config.funding_abs && z_score.abs() >= 2.0;
        finish_threshold(&mut result, "funding_anomaly", current.signum());
        result
    }

    pub fn open_interest(&self, baseline: &BaselineSnapshot) -> StrategyResult {
        let mut result = self.result_from_baseline("crypto.exchange.oi-expansion", baseline);
        if !ready(baseline, &mut result, "open_interest_history") {
            return result;
        }
        let change = baseline.change.unwrap_or_default();
        let percentage = baseline.percentage_change.unwrap_or_default();
        result.trigger_metrics.insert("oi_change".into(), change);
        result
            .trigger_metrics
            .insert("oi_percentage_change".into(), percentage);
        result.triggered = percentage.abs() >= self.config.open_interest_growth;
        finish_threshold(&mut result, "open_interest_expansion", percentage.signum());
        result
    }

    pub fn price_oi_divergence(
        &self,
        price: &BaselineSnapshot,
        open_interest: &BaselineSnapshot,
    ) -> StrategyResult {
        let mut result = self.result_from_baseline("crypto.exchange.price_oi_divergence", price);
        if !ready(price, &mut result, "price_history") {
            return result;
        }
        if open_interest.readiness != Readiness::Ready {
            apply_not_ready(
                &mut result,
                open_interest.readiness,
                "open_interest_history",
            );
            return result;
        }
        let price_change = price.percentage_change.unwrap_or_default();
        let oi_change = open_interest.percentage_change.unwrap_or_default();
        result
            .trigger_metrics
            .insert("price_change".into(), price_change);
        result.trigger_metrics.insert("oi_change".into(), oi_change);
        let reason = match (price_change >= 0.0, oi_change >= 0.0) {
            (true, true) => "price_up_oi_up",
            (true, false) => "price_up_oi_down",
            (false, true) => "price_down_oi_up",
            (false, false) => "price_down_oi_down",
        };
        result.reason_codes.push(reason.into());
        result.triggered = price_change.abs() >= self.config.price_move_abs
            && oi_change.abs() >= self.config.open_interest_growth;
        result.direction = if price_change >= 0.0 {
            SignalDirection::Bullish
        } else {
            SignalDirection::Bearish
        };
        result.score = ((price_change.abs() / self.config.price_move_abs
            + oi_change.abs() / self.config.open_interest_growth)
            / 4.0)
            .clamp(0.0, 1.0);
        if !result.triggered {
            result.rejected_reason = Some("combined_threshold_not_met".into());
        }
        result
    }

    pub fn volume(&self, baseline: &BaselineSnapshot) -> StrategyResult {
        self.zscore_strategy(
            baseline,
            ZScoreRule {
                id: "crypto.exchange.volume_expansion",
                readiness_prefix: "volume_history",
                metric_name: "volume_z_score",
                threshold: self.config.volume_zscore,
                absolute_threshold: None,
                reason: "volume_expansion",
            },
        )
    }

    pub fn orderbook_imbalance(&self, baseline: &BaselineSnapshot) -> StrategyResult {
        self.zscore_strategy(
            baseline,
            ZScoreRule {
                id: "crypto.exchange.orderbook_imbalance",
                readiness_prefix: "orderbook_history",
                metric_name: "book_imbalance_z_score",
                threshold: 2.0,
                absolute_threshold: Some(self.config.book_imbalance),
                reason: "orderbook_imbalance",
            },
        )
    }

    pub fn spread(&self, baseline: &BaselineSnapshot) -> StrategyResult {
        self.zscore_strategy(
            baseline,
            ZScoreRule {
                id: "crypto.exchange.spread_anomaly",
                readiness_prefix: "spread_history",
                metric_name: "spread_z_score",
                threshold: 2.0,
                absolute_threshold: Some(self.config.spread_bps),
                reason: "spread_anomaly",
            },
        )
    }

    fn zscore_strategy(&self, baseline: &BaselineSnapshot, rule: ZScoreRule<'_>) -> StrategyResult {
        let mut result = self.result_from_baseline(rule.id, baseline);
        if !ready(baseline, &mut result, rule.readiness_prefix) {
            return result;
        }
        let current = baseline.current_value.unwrap_or_default();
        let z_score = baseline.z_score.unwrap_or_default();
        result
            .trigger_metrics
            .insert(rule.metric_name.into(), z_score);
        result
            .trigger_metrics
            .insert("current_value".into(), current);
        result.triggered = z_score.abs() >= rule.threshold
            && rule
                .absolute_threshold
                .is_none_or(|minimum| current.abs() >= minimum);
        finish_threshold(&mut result, rule.reason, current.signum());
        result
    }

    fn result_from_baseline(
        &self,
        strategy_id: &str,
        baseline: &BaselineSnapshot,
    ) -> StrategyResult {
        let now = Utc::now();
        StrategyResult {
            id: new_id("strategy_run"),
            strategy_id: strategy_id.into(),
            strategy_version: "2.0.0".into(),
            triggered: false,
            direction: SignalDirection::Watch,
            score: 0.0,
            reason_codes: vec![],
            observation_ids: vec![],
            parameter_snapshot: json!({
                "priceMoveAbs": self.config.price_move_abs,
                "volumeZScore": self.config.volume_zscore,
                "fundingAbs": self.config.funding_abs,
                "openInterestGrowth": self.config.open_interest_growth,
                "bookImbalance": self.config.book_imbalance,
                "spreadBps": self.config.spread_bps
            }),
            rejected_reason: None,
            readiness: baseline.readiness,
            instrument_id: Some(baseline.instrument_id.clone()),
            baseline_window: Some(baseline.window.as_str().into()),
            input_snapshot: serde_json::to_value(baseline).unwrap_or_default(),
            trigger_metrics: HashMap::new(),
            started_at: Some(now),
            completed_at: Some(now),
        }
    }
}

pub fn derivatives_strategy_definitions(config: &StrategyConfig) -> Vec<StrategyDefinition> {
    let definitions = [
        (
            "crypto.exchange.funding_anomaly",
            vec![ObservationKind::Funding],
        ),
        (
            "crypto.exchange.oi-expansion",
            vec![ObservationKind::OpenInterest],
        ),
        (
            "crypto.exchange.price_oi_divergence",
            vec![ObservationKind::PriceVolume, ObservationKind::OpenInterest],
        ),
        (
            "crypto.exchange.volume_expansion",
            vec![ObservationKind::PriceVolume],
        ),
        (
            "crypto.exchange.orderbook_imbalance",
            vec![ObservationKind::OrderBook],
        ),
        (
            "crypto.exchange.spread_anomaly",
            vec![ObservationKind::OrderBook],
        ),
    ];
    definitions.into_iter().map(|(id, kinds)| StrategyDefinition {
        id:id.into(), version:"2.0.0".into(), market:Market::Crypto,
        category:signal_core::SignalCategory::Exchange,
        required_observation_kinds:kinds,
        rolling_windows:crate::ROLLING_WINDOWS.iter().map(|value|(*value).into()).collect(),
        parameters:json!({"fundingAbs":config.funding_abs,"openInterestGrowth":config.open_interest_growth,
            "volumeZScore":config.volume_zscore,"bookImbalance":config.book_imbalance,"spreadBps":config.spread_bps}),
    }).collect()
}

fn ready(baseline: &BaselineSnapshot, result: &mut StrategyResult, prefix: &str) -> bool {
    if baseline.readiness == Readiness::Ready
        && baseline.current_value.is_some()
        && baseline.mean.is_some()
        && baseline.stddev.is_some()
    {
        return true;
    }
    apply_not_ready(result, baseline.readiness, prefix);
    false
}

fn apply_not_ready(result: &mut StrategyResult, readiness: Readiness, prefix: &str) {
    result.readiness = readiness;
    let suffix = match readiness {
        Readiness::WarmingUp => "warming_up",
        Readiness::MissingInput => "missing_input",
        Readiness::StaleInput => "stale_input",
        Readiness::SchemaError => "schema_error",
        Readiness::InsufficientHistory => "insufficient_history",
        Readiness::Ready => "missing_baseline_metric",
    };
    result.reason_codes.push(format!("{prefix}_{suffix}"));
    result.rejected_reason = Some(suffix.into());
}

fn finish_threshold(result: &mut StrategyResult, reason: &str, sign: f64) {
    let z_score = result
        .trigger_metrics
        .iter()
        .find(|(key, _)| key.ends_with("z_score"))
        .map_or(0.0, |(_, value)| value.abs());
    result.score = (z_score / 4.0).clamp(0.0, 1.0);
    if result.triggered {
        result.reason_codes.push(reason.into());
        result.direction = if sign >= 0.0 {
            SignalDirection::Bullish
        } else {
            SignalDirection::Bearish
        };
    } else {
        result.rejected_reason = Some("threshold_not_met".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BaselineSnapshot, HistoryWindow, Readiness};
    use chrono::{TimeZone, Utc};

    fn baseline(
        metric: &str,
        current: f64,
        mean: f64,
        stddev: f64,
        change: f64,
        pct: f64,
    ) -> BaselineSnapshot {
        BaselineSnapshot {
            instrument_id: "BTC-USDT-PERP".into(),
            metric: metric.into(),
            window: HistoryWindow::OneHour,
            readiness: Readiness::Ready,
            current_value: Some(current),
            mean: Some(mean),
            stddev: Some(stddev),
            z_score: Some((current - mean) / stddev),
            change: Some(change),
            percentage_change: Some(pct),
            sample_count: 12,
            first_event_at: Some(Utc.timestamp_opt(1_700_000_000, 0).unwrap()),
            last_event_at: Some(Utc.timestamp_opt(1_700_003_600, 0).unwrap()),
        }
    }

    #[test]
    fn readiness_failure_does_not_trigger() {
        let mut warm = baseline("funding_rate", 0.001, 0.0, 0.0001, 0.0, 0.0);
        warm.readiness = Readiness::WarmingUp;
        warm.mean = None;
        let result = DerivativesStrategyEngine::default().funding(&warm);
        assert_eq!(result.readiness, Readiness::WarmingUp);
        assert!(!result.triggered);
        assert_eq!(result.reason_codes, vec!["funding_history_warming_up"]);
    }

    #[test]
    fn funding_anomaly_requires_real_zscore_baseline() {
        let result = DerivativesStrategyEngine::default().funding(&baseline(
            "funding_rate",
            0.001,
            0.0001,
            0.0002,
            0.0,
            0.0,
        ));
        assert_eq!(result.readiness, Readiness::Ready);
        assert!(result.triggered);
        assert_eq!(result.trigger_metrics["funding_z_score"], 4.5);
        assert_eq!(result.baseline_window.as_deref(), Some("1h"));
    }

    #[test]
    fn oi_expansion_uses_historical_percentage_change() {
        let result = DerivativesStrategyEngine::default().open_interest(&baseline(
            "open_interest",
            120.0,
            105.0,
            5.0,
            20.0,
            0.20,
        ));
        assert!(result.triggered);
        assert_eq!(result.trigger_metrics["oi_change"], 20.0);
        assert_eq!(result.trigger_metrics["oi_percentage_change"], 0.20);
    }

    #[test]
    fn price_oi_divergence_classifies_all_four_patterns() {
        let engine = DerivativesStrategyEngine::default();
        let cases = [
            (0.03, 0.12, "price_up_oi_up"),
            (0.03, -0.12, "price_up_oi_down"),
            (-0.03, 0.12, "price_down_oi_up"),
            (-0.03, -0.12, "price_down_oi_down"),
        ];
        for (price, oi, reason) in cases {
            let result = engine.price_oi_divergence(
                &baseline("price", 100.0, 99.0, 1.0, price, price),
                &baseline("open_interest", 100.0, 99.0, 1.0, oi, oi),
            );
            assert_eq!(result.reason_codes, vec![reason]);
            assert_eq!(result.trigger_metrics["price_change"], price);
            assert_eq!(result.trigger_metrics["oi_change"], oi);
        }
    }

    #[test]
    fn volume_imbalance_and_spread_use_baselines() {
        let engine = DerivativesStrategyEngine::default();
        assert!(
            engine
                .volume(&baseline("volume", 300.0, 100.0, 50.0, 200.0, 2.0))
                .triggered
        );
        assert!(
            engine
                .orderbook_imbalance(&baseline("book_imbalance", 0.5, 0.0, 0.1, 0.5, 0.0))
                .triggered
        );
        assert!(
            engine
                .spread(&baseline("spread_bps", 20.0, 5.0, 3.0, 15.0, 3.0))
                .triggered
        );
    }
}
