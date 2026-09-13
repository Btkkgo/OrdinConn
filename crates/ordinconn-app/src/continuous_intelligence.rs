use crate::{AppError, AppRuntime, events::append_event};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use collector_runtime::SourceScheduleState;
use collector_runtime::{
    CollectorKind, CollectorScheduler, JobRunResult, ScheduledJob, SchedulerLifecycle,
    builtin_sources,
};
use evidence_core::{ClusterRelation, EvidenceCluster, EvidenceClusterMember};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::{
    sync::{Arc, Weak},
    time::Duration,
};
use strategy_core::{AggregateBucket, HistoryWindow};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceClusterSummary {
    pub cluster_id: String,
    pub source_count: usize,
    pub independent_source_count: usize,
    pub highest_reliability: f64,
    pub latest_captured_at: Option<DateTime<Utc>>,
    pub contradiction_count: u32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceDiagnostics {
    pub scheduler: collector_runtime::SchedulerDiagnostics,
    pub scheduler_state: String,
    pub history_samples: usize,
    pub history_capacity_per_series: usize,
    pub history_series: usize,
    pub persisted_buckets: u64,
    pub evidence_clusters: u64,
    pub demand_observations: u64,
    pub strategy_runs: u64,
    pub candidates: u64,
    pub published_signals: u64,
    pub records_per_minute: u64,
    pub observations_per_minute: u64,
    pub evidence_per_minute: u64,
    pub strategy_runs_per_minute: u64,
    pub candidates_per_minute: u64,
    pub published_per_minute: u64,
    pub parse_failures: u64,
    pub reconnects: u64,
    pub stale_sources: u64,
    pub out_of_order_discards: u64,
    pub schema_drift: u64,
}

impl AppRuntime {
    pub async fn start_continuous_intelligence(self: &Arc<Self>) -> Result<(), AppError> {
        let mut guard = self.scheduler.lock().await;
        if guard.is_some() {
            return Ok(());
        }
        let scheduler = Arc::new(CollectorScheduler::new(3));
        for state in self.load_scheduler_state().await? {
            scheduler
                .restore_state(state)
                .await
                .map_err(|_| AppError::InvalidData)?;
        }
        for source in builtin_sources()
            .into_iter()
            .filter(|source| source.enabled && source.collector_kind != CollectorKind::WebSocket)
        {
            scheduler
                .register(Arc::new(RuntimeCollectionJob {
                    source_id: source.id,
                    cadence: Duration::from_secs(source.poll.interval_seconds.max(1)),
                    runtime: Arc::downgrade(self),
                }))
                .await
                .map_err(|_| AppError::InvalidData)?;
        }
        let mut scheduler_events = scheduler.subscribe_events();
        scheduler.start().await.map_err(|_| AppError::InvalidData)?;
        let futures_stream = Arc::new(collector_runtime::BinanceFuturesWebSocket::public(
            Duration::from_secs(15),
        )?);
        let mut stream_events = futures_stream.subscribe();
        futures_stream.start().await?;
        let scheduler_runtime = Arc::downgrade(self);
        self.continuous_tasks
            .lock()
            .await
            .push(tokio::spawn(async move {
                while let Ok(event) = scheduler_events.recv().await {
                    if let Some(runtime) = scheduler_runtime.upgrade() {
                        let _ = runtime
                            .append_intelligence_event(
                                &event.event_type,
                                &event.source_id,
                                json!({"occurredAt":event.occurred_at,"detail":event.detail}),
                            )
                            .await;
                    } else {
                        break;
                    }
                    if event.event_type == "scheduler.stopped" {
                        break;
                    }
                }
            }));
        let stream_runtime = Arc::downgrade(self);
        self.continuous_tasks.lock().await.push(tokio::spawn(async move {
            while let Ok(event)=stream_events.recv().await {
                let (event_type,detail,done)=match event {
                    collector_runtime::FuturesStreamEvent::Connected=>("collector.recovered",json!({"transport":"websocket"}),false),
                    collector_runtime::FuturesStreamEvent::Subscribed=>("collector.subscribed",json!({"transport":"websocket"}),false),
                    collector_runtime::FuturesStreamEvent::Stale=>("collector.stale",json!({"transport":"websocket"}),false),
                    collector_runtime::FuturesStreamEvent::Reconnecting=>("collector.reconnecting",json!({"transport":"websocket"}),false),
                    collector_runtime::FuturesStreamEvent::Error(error)=>("collector.failed",json!({"transport":"websocket","error":error}),false),
                    collector_runtime::FuturesStreamEvent::Stopped=>("collector.stopped",json!({"transport":"websocket"}),true),
                    collector_runtime::FuturesStreamEvent::Data(payload)=>{
                        if let Some(runtime)=stream_runtime.upgrade()
                            && runtime.ingest_futures_stream_message(&payload).await.is_err(){
                            let _=sqlx::query("INSERT INTO intelligence_diagnostics (singleton_id,parse_failures,updated_at) VALUES (1,1,?) ON CONFLICT(singleton_id) DO UPDATE SET parse_failures=parse_failures+1,updated_at=excluded.updated_at")
                                .bind(Utc::now().to_rfc3339()).execute(runtime.pool()).await;
                        }
                        continue
                    },
                };
                if let Some(runtime)=stream_runtime.upgrade(){let _=runtime.append_intelligence_event(event_type,"binance-usdm-futures",detail).await;}else{break;}
                if done{break;}
            }
        }));
        let mut tx = self.pool().begin().await?;
        append_event(
            &mut tx,
            "scheduler.started",
            "scheduler",
            "collector",
            None,
            None,
            None,
            &json!({"jobs":scheduler.snapshot().await.len()}),
        )
        .await?;
        tx.commit().await?;
        *guard = Some(scheduler);
        *self.futures_stream.lock().await = Some(futures_stream);
        Ok(())
    }

    pub async fn pause_continuous_intelligence(&self) -> Result<(), AppError> {
        if let Some(scheduler) = self.scheduler.lock().await.as_ref() {
            scheduler.pause().await;
            self.persist_scheduler_state(&scheduler.snapshot().await)
                .await?;
        }
        Ok(())
    }

    pub async fn resume_continuous_intelligence(&self) -> Result<(), AppError> {
        if let Some(scheduler) = self.scheduler.lock().await.as_ref() {
            scheduler.resume().await;
        }
        Ok(())
    }

    pub async fn intelligence_diagnostics(&self) -> Result<IntelligenceDiagnostics, AppError> {
        let scheduler = self.scheduler.lock().await.as_ref().cloned();
        let (scheduler_diagnostics, scheduler_state) = if let Some(scheduler) = scheduler {
            (
                scheduler.diagnostics().await,
                scheduler_state_name(scheduler.state().await).into(),
            )
        } else {
            (Default::default(), "idle".into())
        };
        let history = self.history.lock().await;
        let history_samples = history.total_samples();
        let history_capacity_per_series = history.capacity_per_series();
        let history_series = history.series_count();
        drop(history);
        let stream_reconnects = if let Some(stream) = self.futures_stream.lock().await.as_ref() {
            stream.diagnostics().await.reconnects
        } else {
            0
        };
        let persisted_buckets = count(self, "market_metric_buckets").await?;
        let evidence_clusters = count(self, "evidence_clusters").await?;
        let demand_observations = count(self, "demand_timelines").await?;
        let strategy_runs = count(self, "strategy_runs").await?;
        let candidates = count(self, "signal_candidates").await?;
        let published_signals = count(self, "signals").await?;
        let records_per_minute = recent_count(self, "raw_records", "retrieved_at").await?;
        let observations_per_minute =
            recent_count(self, "market_observations", "observed_at").await?;
        let evidence_per_minute = recent_count(self, "evidence", "captured_at").await?;
        let strategy_runs_per_minute = recent_count(self, "strategy_runs", "created_at").await?;
        let candidates_per_minute = recent_count(self, "signal_candidates", "created_at").await?;
        let published_per_minute = recent_count(self, "signals", "published_at").await?;
        let row = sqlx::query("SELECT parse_failures,reconnects,stale_sources,out_of_order_discards,schema_drift FROM intelligence_diagnostics WHERE singleton_id=1")
            .fetch_optional(self.pool()).await?;
        Ok(IntelligenceDiagnostics {
            scheduler: scheduler_diagnostics,
            scheduler_state,
            history_samples,
            history_capacity_per_series,
            history_series,
            persisted_buckets,
            evidence_clusters,
            demand_observations,
            strategy_runs,
            candidates,
            published_signals,
            records_per_minute,
            observations_per_minute,
            evidence_per_minute,
            strategy_runs_per_minute,
            candidates_per_minute,
            published_per_minute,
            parse_failures: row
                .as_ref()
                .map_or(0, |row| row.get::<i64, _>("parse_failures") as u64),
            reconnects: row
                .as_ref()
                .map_or(0, |row| row.get::<i64, _>("reconnects") as u64)
                + stream_reconnects,
            stale_sources: row
                .as_ref()
                .map_or(0, |row| row.get::<i64, _>("stale_sources") as u64),
            out_of_order_discards: row
                .as_ref()
                .map_or(0, |row| row.get::<i64, _>("out_of_order_discards") as u64),
            schema_drift: row
                .as_ref()
                .map_or(0, |row| row.get::<i64, _>("schema_drift") as u64),
        })
    }

    async fn append_intelligence_event(
        &self,
        event_type: &str,
        source_id: &str,
        payload: serde_json::Value,
    ) -> Result<(), AppError> {
        let mut tx = self.pool().begin().await?;
        let event = append_event(
            &mut tx,
            event_type,
            "source",
            source_id,
            None,
            None,
            Some(source_id),
            &payload,
        )
        .await?;
        tx.commit().await?;
        self.event_bus.publish(event);
        Ok(())
    }

    pub async fn persist_scheduler_state(
        &self,
        states: &[SourceScheduleState],
    ) -> Result<(), AppError> {
        let mut tx = self.pool().begin().await?;
        for state in states {
            sqlx::query("INSERT INTO scheduler_state (source_id,last_scheduled_at,last_started_at,last_completed_at,next_run_at,consecutive_failures,current_backoff_seconds,last_result,updated_at) VALUES (?,?,?,?,?,?,?,?,?) ON CONFLICT(source_id) DO UPDATE SET last_scheduled_at=excluded.last_scheduled_at,last_started_at=excluded.last_started_at,last_completed_at=excluded.last_completed_at,next_run_at=excluded.next_run_at,consecutive_failures=excluded.consecutive_failures,current_backoff_seconds=excluded.current_backoff_seconds,last_result=excluded.last_result,updated_at=excluded.updated_at")
                .bind(&state.source_id)
                .bind(state.last_scheduled_at.map(|value| value.to_rfc3339()))
                .bind(state.last_started_at.map(|value| value.to_rfc3339()))
                .bind(state.last_completed_at.map(|value| value.to_rfc3339()))
                .bind(state.next_run_at.map(|value| value.to_rfc3339()))
                .bind(i64::from(state.consecutive_failures))
                .bind(state.current_backoff_seconds as i64)
                .bind(&state.last_result)
                .bind(Utc::now().to_rfc3339())
                .execute(&mut *tx).await?;
        }
        if !states.is_empty() {
            append_event(
                &mut tx,
                "scheduler.state_persisted",
                "scheduler",
                "collector",
                None,
                None,
                None,
                &json!({"sourceCount": states.len()}),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn load_scheduler_state(&self) -> Result<Vec<SourceScheduleState>, AppError> {
        let rows = sqlx::query("SELECT source_id,last_scheduled_at,last_started_at,last_completed_at,next_run_at,consecutive_failures,current_backoff_seconds,last_result FROM scheduler_state ORDER BY source_id")
            .fetch_all(self.pool()).await?;
        rows.into_iter()
            .map(|row| {
                Ok(SourceScheduleState {
                    source_id: row.get("source_id"),
                    last_scheduled_at: parse_optional_time(row.get("last_scheduled_at"))?,
                    last_started_at: parse_optional_time(row.get("last_started_at"))?,
                    last_completed_at: parse_optional_time(row.get("last_completed_at"))?,
                    next_run_at: parse_optional_time(row.get("next_run_at"))?,
                    consecutive_failures: row.get::<i64, _>("consecutive_failures") as u32,
                    current_backoff_seconds: row.get::<i64, _>("current_backoff_seconds") as u64,
                    last_result: row.get("last_result"),
                })
            })
            .collect()
    }

    pub async fn persist_metric_bucket(&self, bucket: &AggregateBucket) -> Result<(), AppError> {
        sqlx::query("INSERT INTO market_metric_buckets (instrument_id,metric,window,bucket_time,open,high,low,close,sum,sample_count,mean,stddev,volume_sum,first_event_at,last_event_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(instrument_id,metric,window,bucket_time) DO UPDATE SET open=excluded.open,high=excluded.high,low=excluded.low,close=excluded.close,sum=excluded.sum,sample_count=excluded.sample_count,mean=excluded.mean,stddev=excluded.stddev,volume_sum=excluded.volume_sum,first_event_at=excluded.first_event_at,last_event_at=excluded.last_event_at,updated_at=excluded.updated_at")
            .bind(&bucket.instrument_id).bind(&bucket.metric).bind(bucket.window.as_str())
            .bind(bucket.bucket_time.to_rfc3339()).bind(bucket.open).bind(bucket.high)
            .bind(bucket.low).bind(bucket.close).bind(bucket.sum).bind(bucket.count as i64)
            .bind(bucket.mean).bind(bucket.stddev).bind(bucket.volume_sum)
            .bind(bucket.first_event_at.to_rfc3339()).bind(bucket.last_event_at.to_rfc3339())
            .bind(Utc::now().to_rfc3339()).execute(self.pool()).await?;
        Ok(())
    }

    pub async fn flush_history_buckets(&self) -> Result<usize, AppError> {
        let buckets = self.history.lock().await.current_buckets(
            Utc::now(),
            &[
                HistoryWindow::OneMinute,
                HistoryWindow::FiveMinutes,
                HistoryWindow::FifteenMinutes,
                HistoryWindow::OneHour,
                HistoryWindow::FourHours,
                HistoryWindow::TwentyFourHours,
            ],
        );
        for bucket in &buckets {
            self.persist_metric_bucket(bucket).await?;
        }
        Ok(buckets.len())
    }

    pub async fn load_metric_buckets(&self, limit: i64) -> Result<Vec<AggregateBucket>, AppError> {
        let rows = sqlx::query("SELECT * FROM (SELECT instrument_id,metric,window,bucket_time,open,high,low,close,sum,sample_count,mean,stddev,volume_sum,first_event_at,last_event_at FROM market_metric_buckets ORDER BY last_event_at DESC LIMIT ?) ORDER BY last_event_at ASC")
            .bind(limit.max(1)).fetch_all(self.pool()).await?;
        rows.into_iter().map(bucket_from_row).collect()
    }

    pub async fn persist_evidence_cluster(
        &self,
        topic_key: &str,
        cluster: &EvidenceCluster,
    ) -> Result<EvidenceClusterSummary, AppError> {
        let now = Utc::now().to_rfc3339();
        let original = cluster
            .members
            .iter()
            .find(|member| member.relation == ClusterRelation::Original);
        let mut tx = self.pool().begin().await?;
        sqlx::query("INSERT INTO evidence_clusters (id,topic_key,original_evidence_id,members_json,independent_confirmations,contradiction_count,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET topic_key=excluded.topic_key,original_evidence_id=excluded.original_evidence_id,members_json=excluded.members_json,independent_confirmations=excluded.independent_confirmations,contradiction_count=excluded.contradiction_count,updated_at=excluded.updated_at")
            .bind(&cluster.id).bind(topic_key).bind(original.map(|item| item.evidence_id.as_str()))
            .bind(serde_json::to_string(&cluster.members)?).bind(cluster.independent_confirmations() as i64)
            .bind(i64::from(cluster.contradiction_count)).bind(&now).bind(&now).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM evidence_cluster_members WHERE cluster_id=?")
            .bind(&cluster.id)
            .execute(&mut *tx)
            .await?;
        for member in &cluster.members {
            let metadata_json: String =
                sqlx::query_scalar("SELECT metadata_json FROM evidence WHERE id=?")
                    .bind(&member.evidence_id)
                    .fetch_one(&mut *tx)
                    .await?;
            let metadata: serde_json::Value = serde_json::from_str(&metadata_json)?;
            let canonical_url = metadata
                .get("canonicalUrl")
                .and_then(serde_json::Value::as_str);
            let original_url = metadata
                .get("originalUrl")
                .and_then(serde_json::Value::as_str);
            let content_hash = metadata
                .get("contentHash")
                .and_then(serde_json::Value::as_str);
            sqlx::query("INSERT INTO evidence_cluster_members (cluster_id,evidence_id,source_id,canonical_url,original_url,content_hash,relation,created_at) VALUES (?,?,?,?,?,?,?,?)")
                .bind(&cluster.id).bind(&member.evidence_id).bind(&member.source_id)
                .bind(canonical_url).bind(original_url).bind(content_hash)
                .bind(cluster_relation(member.relation)).bind(&now).execute(&mut *tx).await?;
        }
        append_event(&mut tx, "evidence.cluster_updated", "evidence_cluster", &cluster.id, None, None, Some(&cluster.id),
            &json!({"members":cluster.members.len(),"independentConfirmations":cluster.independent_confirmations(),"contradictions":cluster.contradiction_count})).await?;
        tx.commit().await?;
        self.evidence_cluster_summary(&cluster.id).await
    }

    pub async fn evidence_cluster_summary(
        &self,
        cluster_id: &str,
    ) -> Result<EvidenceClusterSummary, AppError> {
        let row = sqlx::query("SELECT COUNT(DISTINCT m.source_id) source_count,COUNT(DISTINCT CASE WHEN m.relation IN ('original','independent') THEN m.source_id END) independent_count,MAX(e.reliability) highest_reliability,MAX(e.captured_at) latest_captured_at,SUM(CASE WHEN m.relation='contradicting' THEN 1 ELSE 0 END) contradictions FROM evidence_cluster_members m JOIN evidence e ON e.id=m.evidence_id WHERE m.cluster_id=?")
            .bind(cluster_id).fetch_one(self.pool()).await?;
        let independent_sources = row.get::<i64, _>("independent_count") as usize;
        Ok(EvidenceClusterSummary {
            cluster_id: cluster_id.into(),
            source_count: row.get::<i64, _>("source_count") as usize,
            independent_source_count: independent_sources.saturating_sub(1),
            highest_reliability: row
                .get::<Option<f64>, _>("highest_reliability")
                .unwrap_or_default(),
            latest_captured_at: parse_optional_time(row.get("latest_captured_at"))?,
            contradiction_count: row.get::<i64, _>("contradictions") as u32,
        })
    }

    pub async fn record_demand_observation(
        &self,
        entity_id: &str,
        topic: &str,
        direction: &str,
        evidence_id: &str,
        observed_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let allowed_topic = matches!(
            topic,
            "ai_infrastructure" | "gpu" | "memory" | "data_center"
        );
        let allowed_direction = matches!(direction, "positive" | "negative" | "neutral");
        if !allowed_topic || !allowed_direction {
            return Err(AppError::InvalidData);
        }
        let time_bucket = observed_at.format("%Y-%m-%d").to_string();
        sqlx::query("INSERT OR IGNORE INTO demand_timelines (id,entity_id,topic,direction,time_bucket,evidence_id,observed_at,created_at) VALUES (?,?,?,?,?,?,?,?)")
            .bind(market_core::new_id("demand_observation")).bind(entity_id).bind(topic).bind(direction)
            .bind(time_bucket).bind(evidence_id).bind(observed_at.to_rfc3339()).bind(Utc::now().to_rfc3339())
            .execute(self.pool()).await?;
        Ok(())
    }

    pub async fn demand_persistence(
        &self,
        entity_id: &str,
        topic: &str,
        direction: &str,
    ) -> Result<usize, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT time_bucket) FROM demand_timelines WHERE entity_id=? AND topic=? AND direction=?")
            .bind(entity_id).bind(topic).bind(direction).fetch_one(self.pool()).await?;
        Ok(count as usize)
    }
}

struct RuntimeCollectionJob {
    source_id: String,
    cadence: Duration,
    runtime: Weak<AppRuntime>,
}

#[async_trait]
impl ScheduledJob for RuntimeCollectionJob {
    fn id(&self) -> &str {
        &self.source_id
    }
    fn cadence(&self) -> Duration {
        self.cadence
    }
    async fn run_once(&self) -> Result<JobRunResult, String> {
        let runtime = self
            .runtime
            .upgrade()
            .ok_or_else(|| "runtime stopped".to_string())?;
        let summary = runtime
            .run_source_once(&self.source_id)
            .await
            .map_err(|error| error.to_string())?;
        if summary.sources_succeeded == 0 && !summary.errors.is_empty() {
            return Err(summary.errors.join("; "));
        }
        Ok(JobRunResult {
            records: summary.raw_records as u64,
            observations: summary.observations as u64,
        })
    }
}

fn scheduler_state_name(value: SchedulerLifecycle) -> &'static str {
    match value {
        SchedulerLifecycle::Idle => "idle",
        SchedulerLifecycle::Running => "running",
        SchedulerLifecycle::Paused => "paused",
        SchedulerLifecycle::Shutdown => "shutdown",
    }
}

async fn count(runtime: &AppRuntime, table: &str) -> Result<u64, AppError> {
    let query = format!("SELECT COUNT(*) FROM {table}");
    let count: i64 = sqlx::query_scalar(&query).fetch_one(runtime.pool()).await?;
    Ok(count as u64)
}

async fn recent_count(
    runtime: &AppRuntime,
    table: &str,
    time_column: &str,
) -> Result<u64, AppError> {
    let query = format!(
        "SELECT COUNT(*) FROM {table} WHERE julianday({time_column}) >= julianday('now','-1 minute')"
    );
    let count: i64 = sqlx::query_scalar(&query).fetch_one(runtime.pool()).await?;
    Ok(count as u64)
}

fn cluster_relation(value: ClusterRelation) -> &'static str {
    match value {
        ClusterRelation::Original => "original",
        ClusterRelation::Syndication => "syndication",
        ClusterRelation::Independent => "independent",
        ClusterRelation::Contradicting => "contradicting",
    }
}

fn parse_optional_time(value: Option<String>) -> Result<Option<DateTime<Utc>>, AppError> {
    value
        .map(|item| {
            DateTime::parse_from_rfc3339(&item)
                .map(|value| value.with_timezone(&Utc))
                .map_err(|_| AppError::InvalidData)
        })
        .transpose()
}

fn parse_time(value: String) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(&value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AppError::InvalidData)
}

fn bucket_from_row(row: sqlx::sqlite::SqliteRow) -> Result<AggregateBucket, AppError> {
    let window = match row.get::<String, _>("window").as_str() {
        "1m" => HistoryWindow::OneMinute,
        "5m" => HistoryWindow::FiveMinutes,
        "15m" => HistoryWindow::FifteenMinutes,
        "1h" => HistoryWindow::OneHour,
        "4h" => HistoryWindow::FourHours,
        "24h" => HistoryWindow::TwentyFourHours,
        _ => return Err(AppError::InvalidData),
    };
    Ok(AggregateBucket {
        instrument_id: row.get("instrument_id"),
        metric: row.get("metric"),
        window,
        bucket_time: parse_time(row.get("bucket_time"))?,
        open: row.get("open"),
        high: row.get("high"),
        low: row.get("low"),
        close: row.get("close"),
        sum: row.get("sum"),
        count: row.get::<i64, _>("sample_count") as usize,
        mean: row.get("mean"),
        stddev: row.get("stddev"),
        volume_sum: row.get("volume_sum"),
        first_event_at: parse_time(row.get("first_event_at"))?,
        last_event_at: parse_time(row.get("last_event_at"))?,
    })
}

#[allow(dead_code)]
fn _member_shape(member: &EvidenceClusterMember) -> (&str, &str) {
    (&member.evidence_id, &member.source_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::insert_evidence;
    use chrono::{Duration as ChronoDuration, TimeZone};
    use evidence_core::{DataOrigin, Evidence, FactualLevel, SourceType};
    use market_core::{AssetRef, Market};

    async fn runtime() -> (tempfile::TempDir, Arc<AppRuntime>) {
        let directory = tempfile::tempdir().unwrap();
        let runtime = AppRuntime::initialize(&directory.path().join("continuous.sqlite3"))
            .await
            .unwrap();
        (directory, runtime)
    }

    #[tokio::test]
    async fn scheduler_state_and_bucket_round_trip_and_audit() {
        let (_directory, runtime) = runtime().await;
        let timestamp = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let state = SourceScheduleState {
            source_id: "binance-usdm-futures".into(),
            last_completed_at: Some(timestamp),
            next_run_at: Some(timestamp + ChronoDuration::minutes(1)),
            consecutive_failures: 2,
            current_backoff_seconds: 120,
            last_result: Some("failed".into()),
            ..Default::default()
        };
        runtime.persist_scheduler_state(&[state]).await.unwrap();
        let restored = runtime.load_scheduler_state().await.unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].consecutive_failures, 2);
        let bucket = AggregateBucket {
            instrument_id: "BTC-USDT-PERP".into(),
            metric: "funding_rate".into(),
            window: HistoryWindow::OneHour,
            bucket_time: timestamp,
            open: 0.1,
            high: 0.3,
            low: 0.1,
            close: 0.2,
            sum: 0.6,
            count: 3,
            mean: 0.2,
            stddev: 0.081,
            volume_sum: 0.0,
            first_event_at: timestamp,
            last_event_at: timestamp + ChronoDuration::minutes(40),
        };
        runtime.persist_metric_bucket(&bucket).await.unwrap();
        let buckets = runtime.load_metric_buckets(10).await.unwrap();
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].instrument_id, "BTC-USDT-PERP");
        let audit: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM runtime_events WHERE event_type='scheduler.state_persisted'",
        )
        .fetch_one(runtime.pool())
        .await
        .unwrap();
        assert_eq!(audit, 1);
    }

    #[tokio::test]
    async fn cluster_roles_are_durable_and_syndication_does_not_confirm() {
        let (_directory, runtime) = runtime().await;
        let mut evidence = Vec::new();
        for source in ["original", "copy", "independent", "contradiction"] {
            let mut item = Evidence::new(
                source,
                SourceType::OfficialApi,
                AssetRef::new(Market::Crypto, "BTC"),
                source,
                "fact",
                FactualLevel::Observed,
                0.9,
                1.0,
                Utc::now(),
            );
            item.data_origin = DataOrigin::Real;
            let mut tx = runtime.pool().begin().await.unwrap();
            insert_evidence(&mut tx, &item).await.unwrap();
            tx.commit().await.unwrap();
            evidence.push(item);
        }
        let cluster = EvidenceCluster {
            id: "cluster-1".into(),
            contradiction_count: 1,
            members: vec![
                EvidenceClusterMember {
                    evidence_id: evidence[0].id.clone(),
                    source_id: "original".into(),
                    relation: ClusterRelation::Original,
                },
                EvidenceClusterMember {
                    evidence_id: evidence[1].id.clone(),
                    source_id: "copy".into(),
                    relation: ClusterRelation::Syndication,
                },
                EvidenceClusterMember {
                    evidence_id: evidence[2].id.clone(),
                    source_id: "independent".into(),
                    relation: ClusterRelation::Independent,
                },
                EvidenceClusterMember {
                    evidence_id: evidence[3].id.clone(),
                    source_id: "contradiction".into(),
                    relation: ClusterRelation::Contradicting,
                },
            ],
        };
        let summary = runtime
            .persist_evidence_cluster("btc-liquidity", &cluster)
            .await
            .unwrap();
        assert_eq!(summary.source_count, 4);
        assert_eq!(summary.independent_source_count, 1);
        assert_eq!(summary.contradiction_count, 1);
    }

    #[tokio::test]
    async fn demand_persistence_requires_distinct_time_buckets() {
        let (_directory, runtime) = runtime().await;
        let mut item = Evidence::new(
            "nvidia",
            SourceType::CompanyDisclosure,
            AssetRef::new(Market::Traditional, "NVDA"),
            "demand",
            "fact",
            FactualLevel::Official,
            0.95,
            1.0,
            Utc::now(),
        );
        item.data_origin = DataOrigin::Real;
        let mut tx = runtime.pool().begin().await.unwrap();
        insert_evidence(&mut tx, &item).await.unwrap();
        tx.commit().await.unwrap();
        let day = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        runtime
            .record_demand_observation("entity:nvidia", "gpu", "positive", &item.id, day)
            .await
            .unwrap();
        runtime
            .record_demand_observation(
                "entity:nvidia",
                "gpu",
                "positive",
                &item.id,
                day + ChronoDuration::hours(1),
            )
            .await
            .unwrap();
        assert_eq!(
            runtime
                .demand_persistence("entity:nvidia", "gpu", "positive")
                .await
                .unwrap(),
            1
        );
        runtime
            .record_demand_observation(
                "entity:nvidia",
                "gpu",
                "positive",
                &item.id,
                day + ChronoDuration::days(1),
            )
            .await
            .unwrap();
        assert_eq!(
            runtime
                .demand_persistence("entity:nvidia", "gpu", "positive")
                .await
                .unwrap(),
            2
        );
    }

    #[test]
    fn diagnostics_contract_serializes_rates_and_resource_bounds() {
        let diagnostics = IntelligenceDiagnostics {
            history_capacity_per_series: 4_096,
            records_per_minute: 3,
            observations_per_minute: 21,
            ..Default::default()
        };
        let value = serde_json::to_value(diagnostics).unwrap();
        assert_eq!(value["historyCapacityPerSeries"], 4_096);
        assert_eq!(value["recordsPerMinute"], 3);
        assert_eq!(value["observationsPerMinute"], 21);
    }
}
