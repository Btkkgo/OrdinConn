use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryWindow {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    FourHours,
    TwentyFourHours,
}

impl HistoryWindow {
    pub const fn seconds(self) -> i64 {
        match self {
            Self::OneMinute => 60,
            Self::FiveMinutes => 300,
            Self::FifteenMinutes => 900,
            Self::OneHour => 3_600,
            Self::FourHours => 14_400,
            Self::TwentyFourHours => 86_400,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OneMinute => "1m",
            Self::FiveMinutes => "5m",
            Self::FifteenMinutes => "15m",
            Self::OneHour => "1h",
            Self::FourHours => "4h",
            Self::TwentyFourHours => "24h",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Readiness {
    Ready,
    WarmingUp,
    #[default]
    MissingInput,
    StaleInput,
    SchemaError,
    InsufficientHistory,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricSample {
    pub event_time: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub value: f64,
    pub volume: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IngestResult {
    Accepted,
    AcceptedOutOfOrder,
    Replaced,
    DiscardedOutOfOrder,
    RejectedNonFinite,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollingMetrics {
    pub count: usize,
    pub sum: f64,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
    pub first_value: f64,
    pub last_value: f64,
    pub stddev: f64,
    pub change: f64,
    pub percentage_change: Option<f64>,
    pub z_score: Option<f64>,
    pub ema: f64,
    pub volume_sum: f64,
    pub first_event_at: DateTime<Utc>,
    pub last_event_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineSnapshot {
    pub instrument_id: String,
    pub metric: String,
    pub window: HistoryWindow,
    pub readiness: Readiness,
    pub current_value: Option<f64>,
    pub mean: Option<f64>,
    pub stddev: Option<f64>,
    pub z_score: Option<f64>,
    pub change: Option<f64>,
    pub percentage_change: Option<f64>,
    pub sample_count: usize,
    pub first_event_at: Option<DateTime<Utc>>,
    pub last_event_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregateBucket {
    pub instrument_id: String,
    pub metric: String,
    pub window: HistoryWindow,
    pub bucket_time: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub sum: f64,
    pub count: usize,
    pub mean: f64,
    pub stddev: f64,
    pub volume_sum: f64,
    pub first_event_at: DateTime<Utc>,
    pub last_event_at: DateTime<Utc>,
}

impl AggregateBucket {
    pub fn from_samples(
        instrument_id: &str,
        metric: &str,
        window: HistoryWindow,
        bucket_time: DateTime<Utc>,
        samples: &[MetricSample],
    ) -> Option<Self> {
        let metrics = calculate_metrics(samples)?;
        Some(Self {
            instrument_id: instrument_id.into(),
            metric: metric.into(),
            window,
            bucket_time,
            open: samples.first()?.value,
            high: metrics.max,
            low: metrics.min,
            close: samples.last()?.value,
            sum: metrics.sum,
            count: metrics.count,
            mean: metrics.mean,
            stddev: metrics.stddev,
            volume_sum: metrics.volume_sum,
            first_event_at: metrics.first_event_at,
            last_event_at: metrics.last_event_at,
        })
    }
}

pub struct RollingHistoryEngine {
    capacity_per_series: usize,
    out_of_order_tolerance: Duration,
    series: HashMap<(String, String), VecDeque<MetricSample>>,
    discarded_out_of_order: u64,
}

impl RollingHistoryEngine {
    pub fn new(capacity_per_series: usize, out_of_order_tolerance: Duration) -> Self {
        Self {
            capacity_per_series: capacity_per_series.max(1),
            out_of_order_tolerance,
            series: HashMap::new(),
            discarded_out_of_order: 0,
        }
    }

    pub fn ingest(
        &mut self,
        instrument_id: &str,
        metric: &str,
        sample: MetricSample,
    ) -> IngestResult {
        if !sample.value.is_finite() || !sample.volume.is_finite() {
            return IngestResult::RejectedNonFinite;
        }
        let values = self
            .series
            .entry((instrument_id.into(), metric.into()))
            .or_default();
        let result = match values.back() {
            Some(last) if sample.event_time == last.event_time => {
                values.pop_back();
                values.push_back(sample);
                IngestResult::Replaced
            }
            Some(last) if sample.event_time < last.event_time => {
                if last.event_time - sample.event_time > self.out_of_order_tolerance {
                    self.discarded_out_of_order += 1;
                    return IngestResult::DiscardedOutOfOrder;
                }
                let position = values
                    .iter()
                    .position(|item| item.event_time > sample.event_time)
                    .unwrap_or(values.len());
                values.insert(position, sample);
                IngestResult::AcceptedOutOfOrder
            }
            _ => {
                values.push_back(sample);
                IngestResult::Accepted
            }
        };
        while values.len() > self.capacity_per_series {
            values.pop_front();
        }
        result
    }

    pub fn metrics(
        &self,
        instrument_id: &str,
        metric: &str,
        window: HistoryWindow,
        now: DateTime<Utc>,
    ) -> Option<RollingMetrics> {
        let cutoff = now - Duration::seconds(window.seconds());
        let samples: Vec<_> = self
            .series
            .get(&(instrument_id.into(), metric.into()))?
            .iter()
            .filter(|sample| sample.event_time >= cutoff && sample.event_time <= now)
            .cloned()
            .collect();
        calculate_metrics(&samples)
    }

    pub fn baseline(
        &self,
        instrument_id: &str,
        metric: &str,
        window: HistoryWindow,
        now: DateTime<Utc>,
        minimum_samples: usize,
        stale_after: Duration,
    ) -> BaselineSnapshot {
        let metrics = self.metrics(instrument_id, metric, window, now);
        let Some(metrics) = metrics else {
            return empty_baseline(
                instrument_id,
                metric,
                window,
                Readiness::InsufficientHistory,
            );
        };
        if now - metrics.last_event_at > stale_after {
            return BaselineSnapshot {
                instrument_id: instrument_id.into(),
                metric: metric.into(),
                window,
                readiness: Readiness::StaleInput,
                current_value: Some(metrics.last_value),
                mean: None,
                stddev: None,
                z_score: None,
                change: None,
                percentage_change: None,
                sample_count: metrics.count,
                first_event_at: Some(metrics.first_event_at),
                last_event_at: Some(metrics.last_event_at),
            };
        }
        if metrics.count < minimum_samples {
            return BaselineSnapshot {
                instrument_id: instrument_id.into(),
                metric: metric.into(),
                window,
                readiness: Readiness::WarmingUp,
                current_value: Some(metrics.last_value),
                mean: None,
                stddev: None,
                z_score: None,
                change: None,
                percentage_change: None,
                sample_count: metrics.count,
                first_event_at: Some(metrics.first_event_at),
                last_event_at: Some(metrics.last_event_at),
            };
        }
        BaselineSnapshot {
            instrument_id: instrument_id.into(),
            metric: metric.into(),
            window,
            readiness: Readiness::Ready,
            current_value: Some(metrics.last_value),
            mean: Some(metrics.mean),
            stddev: Some(metrics.stddev),
            z_score: metrics.z_score,
            change: Some(metrics.change),
            percentage_change: metrics.percentage_change,
            sample_count: metrics.count,
            first_event_at: Some(metrics.first_event_at),
            last_event_at: Some(metrics.last_event_at),
        }
    }

    pub fn bucket(
        &self,
        instrument_id: &str,
        metric: &str,
        window: HistoryWindow,
        bucket_time: DateTime<Utc>,
    ) -> Option<AggregateBucket> {
        let end = bucket_time + Duration::seconds(window.seconds());
        let samples: Vec<_> = self
            .series
            .get(&(instrument_id.into(), metric.into()))?
            .iter()
            .filter(|sample| sample.event_time >= bucket_time && sample.event_time < end)
            .cloned()
            .collect();
        AggregateBucket::from_samples(instrument_id, metric, window, bucket_time, &samples)
    }

    pub fn restore_buckets(&mut self, buckets: &[AggregateBucket]) {
        for bucket in buckets {
            let _ = self.ingest(
                &bucket.instrument_id,
                &bucket.metric,
                MetricSample {
                    event_time: bucket.last_event_at,
                    received_at: bucket.last_event_at,
                    value: bucket.close,
                    volume: bucket.volume_sum,
                },
            );
        }
    }
    pub fn current_buckets(
        &self,
        now: DateTime<Utc>,
        windows: &[HistoryWindow],
    ) -> Vec<AggregateBucket> {
        let mut buckets = Vec::new();
        for (instrument_id, metric) in self.series.keys() {
            for window in windows {
                let seconds = now.timestamp().div_euclid(window.seconds()) * window.seconds();
                if let Some(bucket_time) = DateTime::from_timestamp(seconds, 0)
                    && let Some(bucket) = self.bucket(instrument_id, metric, *window, bucket_time)
                {
                    buckets.push(bucket);
                }
            }
        }
        buckets
    }
    pub fn sample_count(&self, instrument_id: &str, metric: &str) -> usize {
        self.series
            .get(&(instrument_id.into(), metric.into()))
            .map_or(0, VecDeque::len)
    }
    pub fn total_samples(&self) -> usize {
        self.series.values().map(VecDeque::len).sum()
    }
    pub fn series_count(&self) -> usize {
        self.series.len()
    }
    pub const fn capacity_per_series(&self) -> usize {
        self.capacity_per_series
    }
    pub const fn discarded_out_of_order(&self) -> u64 {
        self.discarded_out_of_order
    }
}

fn empty_baseline(
    instrument_id: &str,
    metric: &str,
    window: HistoryWindow,
    readiness: Readiness,
) -> BaselineSnapshot {
    BaselineSnapshot {
        instrument_id: instrument_id.into(),
        metric: metric.into(),
        window,
        readiness,
        current_value: None,
        mean: None,
        stddev: None,
        z_score: None,
        change: None,
        percentage_change: None,
        sample_count: 0,
        first_event_at: None,
        last_event_at: None,
    }
}

fn calculate_metrics(samples: &[MetricSample]) -> Option<RollingMetrics> {
    let first = samples.first()?;
    let last = samples.last()?;
    let count = samples.len();
    let sum = samples.iter().map(|sample| sample.value).sum::<f64>();
    let mean = sum / count as f64;
    let variance = samples
        .iter()
        .map(|sample| (sample.value - mean).powi(2))
        .sum::<f64>()
        / count as f64;
    let stddev = variance.sqrt();
    let min = samples
        .iter()
        .map(|sample| sample.value)
        .fold(f64::INFINITY, f64::min);
    let max = samples
        .iter()
        .map(|sample| sample.value)
        .fold(f64::NEG_INFINITY, f64::max);
    let change = last.value - first.value;
    let percentage_change = (first.value != 0.0).then_some(change / first.value);
    let z_score = (stddev > 0.0).then_some((last.value - mean) / stddev);
    let alpha = 2.0 / (count as f64 + 1.0);
    let ema = samples.iter().skip(1).fold(first.value, |value, sample| {
        alpha * sample.value + (1.0 - alpha) * value
    });
    Some(RollingMetrics {
        count,
        sum,
        mean,
        min,
        max,
        first_value: first.value,
        last_value: last.value,
        stddev,
        change,
        percentage_change,
        z_score,
        ema,
        volume_sum: samples.iter().map(|sample| sample.volume).sum(),
        first_event_at: first.event_time,
        last_event_at: last.event_time,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn at(second: i64) -> chrono::DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000 + second, 0).unwrap()
    }

    fn sample(second: i64, value: f64, volume: f64) -> MetricSample {
        MetricSample {
            event_time: at(second),
            received_at: at(second + 1),
            value,
            volume,
        }
    }

    #[test]
    fn rolling_metrics_are_deterministic() {
        let mut history = RollingHistoryEngine::new(100, chrono::Duration::seconds(5));
        for item in [
            sample(0, 10.0, 1.0),
            sample(10, 20.0, 2.0),
            sample(20, 30.0, 3.0),
        ] {
            assert_eq!(
                history.ingest("BTC-USDT-PERP", "funding", item),
                IngestResult::Accepted
            );
        }
        let metrics = history
            .metrics("BTC-USDT-PERP", "funding", HistoryWindow::OneMinute, at(30))
            .unwrap();
        assert_eq!(metrics.count, 3);
        assert_eq!(metrics.sum, 60.0);
        assert_eq!(metrics.mean, 20.0);
        assert_eq!(metrics.min, 10.0);
        assert_eq!(metrics.max, 30.0);
        assert_eq!(metrics.first_value, 10.0);
        assert_eq!(metrics.last_value, 30.0);
        assert!((metrics.stddev - 8.164_965_809).abs() < 1e-9);
        assert_eq!(metrics.change, 20.0);
        assert_eq!(metrics.percentage_change, Some(2.0));
        assert!((metrics.z_score.unwrap() - 1.224_744_871).abs() < 1e-9);
        assert_eq!(metrics.ema, 22.5);
        assert_eq!(metrics.volume_sum, 6.0);
    }

    #[test]
    fn baseline_reports_warmup_ready_and_stale_without_default_values() {
        let mut history = RollingHistoryEngine::new(10, chrono::Duration::seconds(5));
        history.ingest("BTC-USDT-PERP", "oi", sample(0, 100.0, 0.0));
        let warm = history.baseline(
            "BTC-USDT-PERP",
            "oi",
            HistoryWindow::OneHour,
            at(10),
            3,
            chrono::Duration::minutes(2),
        );
        assert_eq!(warm.readiness, Readiness::WarmingUp);
        assert!(warm.mean.is_none());
        history.ingest("BTC-USDT-PERP", "oi", sample(20, 110.0, 0.0));
        history.ingest("BTC-USDT-PERP", "oi", sample(40, 121.0, 0.0));
        assert_eq!(
            history
                .baseline(
                    "BTC-USDT-PERP",
                    "oi",
                    HistoryWindow::OneHour,
                    at(50),
                    3,
                    chrono::Duration::minutes(2)
                )
                .readiness,
            Readiness::Ready
        );
        assert_eq!(
            history
                .baseline(
                    "BTC-USDT-PERP",
                    "oi",
                    HistoryWindow::OneHour,
                    at(500),
                    3,
                    chrono::Duration::minutes(2)
                )
                .readiness,
            Readiness::StaleInput
        );
    }

    #[test]
    fn aggregate_bucket_has_literal_ohlc_and_statistics() {
        let bucket = AggregateBucket::from_samples(
            "BTC-USDT-PERP",
            "price",
            HistoryWindow::OneMinute,
            at(0),
            &[
                sample(0, 10.0, 1.0),
                sample(10, 30.0, 2.0),
                sample(20, 20.0, 3.0),
            ],
        )
        .unwrap();
        assert_eq!(
            (bucket.open, bucket.high, bucket.low, bucket.close),
            (10.0, 30.0, 10.0, 20.0)
        );
        assert_eq!(bucket.sum, 60.0);
        assert_eq!(bucket.count, 3);
        assert_eq!(bucket.mean, 20.0);
        assert_eq!(bucket.volume_sum, 6.0);
    }

    #[test]
    fn out_of_order_within_tolerance_is_sorted_and_older_data_is_discarded() {
        let mut history = RollingHistoryEngine::new(10, chrono::Duration::seconds(5));
        assert_eq!(
            history.ingest("BTC-USDT-PERP", "price", sample(10, 10.0, 0.0)),
            IngestResult::Accepted
        );
        assert_eq!(
            history.ingest("BTC-USDT-PERP", "price", sample(7, 7.0, 0.0)),
            IngestResult::AcceptedOutOfOrder
        );
        assert_eq!(
            history.ingest("BTC-USDT-PERP", "price", sample(0, 0.0, 0.0)),
            IngestResult::DiscardedOutOfOrder
        );
        assert_eq!(
            history
                .metrics("BTC-USDT-PERP", "price", HistoryWindow::OneMinute, at(11))
                .unwrap()
                .change,
            3.0
        );
    }

    #[test]
    fn series_capacity_is_strictly_bounded() {
        let mut history = RollingHistoryEngine::new(3, chrono::Duration::seconds(5));
        for second in 0..100 {
            history.ingest("BTC-USDT-PERP", "trade", sample(second, second as f64, 1.0));
        }
        assert_eq!(history.sample_count("BTC-USDT-PERP", "trade"), 3);
        assert_eq!(history.total_samples(), 3);
    }

    #[test]
    fn equal_timestamp_replaces_aggregate_without_growing_series() {
        let mut history = RollingHistoryEngine::new(10, chrono::Duration::seconds(5));
        assert_eq!(
            history.ingest("BTC-USDT-PERP", "volume_5m", sample(0, 10.0, 10.0)),
            IngestResult::Accepted
        );
        assert_eq!(
            history.ingest("BTC-USDT-PERP", "volume_5m", sample(0, 25.0, 25.0)),
            IngestResult::Replaced
        );
        assert_eq!(history.sample_count("BTC-USDT-PERP", "volume_5m"), 1);
        assert_eq!(
            history
                .metrics("BTC-USDT-PERP", "volume_5m", HistoryWindow::OneHour, at(1))
                .unwrap()
                .last_value,
            25.0
        );
    }

    #[test]
    fn restored_buckets_rebuild_long_window_baseline() {
        let buckets = vec![
            AggregateBucket::from_samples(
                "BTC-USDT-PERP",
                "funding",
                HistoryWindow::OneHour,
                at(0),
                &[sample(0, 0.001, 0.0), sample(10, 0.002, 0.0)],
            )
            .unwrap(),
            AggregateBucket::from_samples(
                "BTC-USDT-PERP",
                "funding",
                HistoryWindow::OneHour,
                at(3600),
                &[sample(3600, 0.003, 0.0), sample(3610, 0.004, 0.0)],
            )
            .unwrap(),
        ];
        let mut history = RollingHistoryEngine::new(100, chrono::Duration::seconds(5));
        history.restore_buckets(&buckets);
        let baseline = history.baseline(
            "BTC-USDT-PERP",
            "funding",
            HistoryWindow::TwentyFourHours,
            at(3620),
            2,
            chrono::Duration::hours(2),
        );
        assert_eq!(baseline.readiness, Readiness::Ready);
        assert_eq!(baseline.sample_count, 2);
        assert_eq!(baseline.mean, Some(0.003));
    }
}
