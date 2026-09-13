use crate::{AppError, AppRuntime, events::append_event, services::insert_evidence};
use chrono::{Duration, Utc};
use collector_runtime::{
    BinanceFuturesClient, Collector, CollectorKind, FUTURES_SYMBOLS, FuturesObservation,
    HttpCollector, NormalizedObservation, ObservationKind, RawRecord, builtin_sources,
    normalize_binance_24h, normalize_mempool, observation_from_feed_entry, parse_feed,
};
use evidence_core::{
    ClusterRelation, DataOrigin, Evidence, EvidenceClusterMember, EvidenceFingerprint,
    EvidenceRelation, FactualLevel, SourceType, classify_cluster_relation,
};
use market_core::{AssetRef, Market, new_id};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use signal_core::{
    EvidenceLink, SignalCandidate, SignalCategory, SignalDirection, SignalPublisher,
    StrategyProvenance,
};
use sqlx::{Row, Sqlite};
use strategy_core::{
    DerivativesStrategyEngine, HistoryWindow, MetricSample, Readiness, StrategyConfig,
    StrategyContext, StrategyEngine, StrategyResult, derivatives_strategy_definitions,
};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreIntelligenceRunSummary {
    pub sources_attempted: usize,
    pub sources_succeeded: usize,
    pub raw_records: usize,
    pub observations: usize,
    pub evidence: usize,
    pub candidates: usize,
    pub published_signals: usize,
    pub rejected_candidates: usize,
    pub schema_drift: usize,
    pub errors: Vec<String>,
}

impl AppRuntime {
    pub async fn initialize_core_intelligence_catalogs(&self) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let mut tx = self.pool().begin().await?;
        for source in builtin_sources() {
            sqlx::query("INSERT INTO sources (id,name,endpoint,collector_type,classification,capabilities_json,reliability_tier,poll_policy_json,rate_limit_json,auth_requirement,retention_policy_json,enabled,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,endpoint=excluded.endpoint,capabilities_json=excluded.capabilities_json,reliability_tier=excluded.reliability_tier,poll_policy_json=excluded.poll_policy_json,rate_limit_json=excluded.rate_limit_json,retention_policy_json=excluded.retention_policy_json,enabled=excluded.enabled,updated_at=excluded.updated_at")
                .bind(&source.id).bind(&source.name).bind(&source.endpoint)
                .bind(enum_json(&source.collector_kind)).bind(enum_json(&source.classification))
                .bind(serde_json::to_string(&source.capabilities)?)
                .bind(enum_json(&source.reliability_tier)).bind(serde_json::to_string(&source.poll)?)
                .bind(serde_json::to_string(&source.rate_limit)?).bind(enum_json(&source.auth))
                .bind(serde_json::to_string(&source.retention)?).bind(source.enabled)
                .bind(&now).bind(&now).execute(&mut *tx).await?;
        }
        for definition in StrategyEngine::default().definitions() {
            sqlx::query("INSERT OR IGNORE INTO strategy_definitions (id,version,market,category,required_observations_json,rolling_windows_json,parameters_json,created_at) VALUES (?,?,?,?,?,?,?,?)")
                .bind(&definition.id).bind(&definition.version).bind(definition.market.as_str())
                .bind(definition.category.as_str())
                .bind(serde_json::to_string(&definition.required_observation_kinds)?)
                .bind(serde_json::to_string(&definition.rolling_windows)?)
                .bind(serde_json::to_string(&definition.parameters)?).bind(&now)
                .execute(&mut *tx).await?;
        }
        for definition in derivatives_strategy_definitions(&StrategyConfig::default()) {
            sqlx::query("INSERT OR IGNORE INTO strategy_definitions (id,version,market,category,required_observations_json,rolling_windows_json,parameters_json,created_at) VALUES (?,?,?,?,?,?,?,?)")
                .bind(&definition.id).bind(&definition.version).bind(definition.market.as_str())
                .bind(definition.category.as_str())
                .bind(serde_json::to_string(&definition.required_observation_kinds)?)
                .bind(serde_json::to_string(&definition.rolling_windows)?)
                .bind(serde_json::to_string(&definition.parameters)?).bind(&now)
                .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn run_core_intelligence_once(&self) -> Result<CoreIntelligenceRunSummary, AppError> {
        let mut summary = CoreIntelligenceRunSummary::default();
        for source in builtin_sources()
            .into_iter()
            .filter(|source| source.enabled && source.collector_kind != CollectorKind::WebSocket)
        {
            let next = self.run_source_once(&source.id).await?;
            summary.sources_attempted += next.sources_attempted;
            summary.sources_succeeded += next.sources_succeeded;
            summary.raw_records += next.raw_records;
            summary.observations += next.observations;
            summary.evidence += next.evidence;
            summary.candidates += next.candidates;
            summary.published_signals += next.published_signals;
            summary.rejected_candidates += next.rejected_candidates;
            summary.schema_drift += next.schema_drift;
            summary.errors.extend(next.errors);
        }
        Ok(summary)
    }

    pub(crate) async fn run_source_once(
        &self,
        source_id: &str,
    ) -> Result<CoreIntelligenceRunSummary, AppError> {
        let mut summary = CoreIntelligenceRunSummary {
            sources_attempted: 1,
            ..Default::default()
        };
        let source = builtin_sources()
            .into_iter()
            .find(|source| source.id == source_id)
            .ok_or(AppError::NotFound("source"))?;
        if source.id == "binance-usdm-futures" {
            return self.collect_futures_once().await;
        }
        if source.collector_kind == CollectorKind::WebSocket || !source.enabled {
            return Ok(summary);
        }
        let collector = match HttpCollector::new(source.clone()) {
            Ok(value) => value,
            Err(error) => {
                summary.errors.push(format!("{}: {error}", source.id));
                return Ok(summary);
            }
        };
        let run_id = new_id("collector_run");
        sqlx::query(
            "INSERT INTO collector_runs (id,source_id,status,started_at) VALUES (?,?,'running',?)",
        )
        .bind(&run_id)
        .bind(&source.id)
        .bind(Utc::now().to_rfc3339())
        .execute(self.pool())
        .await?;
        match collector.fetch_once().await {
            Ok(records) => {
                let count = records.len();
                match self
                    .process_source_records(&source.id, records, &mut summary)
                    .await
                {
                    Ok(()) => {
                        summary.sources_succeeded += 1;
                        sqlx::query("UPDATE collector_runs SET status='completed',records_collected=?,completed_at=? WHERE id=?")
                                .bind(count as i64).bind(Utc::now().to_rfc3339()).bind(&run_id)
                                .execute(self.pool()).await?;
                        sqlx::query(
                            "UPDATE sources SET health_status='healthy',updated_at=? WHERE id=?",
                        )
                        .bind(Utc::now().to_rfc3339())
                        .bind(&source.id)
                        .execute(self.pool())
                        .await?;
                    }
                    Err(error) => {
                        let is_schema_drift = error.to_string().contains("schema");
                        summary.schema_drift += usize::from(is_schema_drift);
                        summary.errors.push(format!("{}: {error}", source.id));
                        let mut tx = self.pool().begin().await?;
                        sqlx::query("UPDATE collector_runs SET status='failed',error_code=?,completed_at=? WHERE id=?")
                                .bind(if is_schema_drift { "schema_drift" } else { "normalization_failed" })
                                .bind(Utc::now().to_rfc3339()).bind(&run_id)
                                .execute(&mut *tx).await?;
                        sqlx::query("UPDATE sources SET health_status=?,updated_at=? WHERE id=?")
                            .bind(if is_schema_drift {
                                "schema_drift"
                            } else {
                                "degraded"
                            })
                            .bind(Utc::now().to_rfc3339())
                            .bind(&source.id)
                            .execute(&mut *tx)
                            .await?;
                        if is_schema_drift {
                            append_event(
                                &mut tx,
                                "collector.schema_drift",
                                "source",
                                &source.id,
                                None,
                                None,
                                Some(&source.id),
                                &json!({"collectorRunId":run_id}),
                            )
                            .await?;
                        }
                        tx.commit().await?;
                    }
                }
            }
            Err(error) => {
                summary.errors.push(format!("{}: {error}", source.id));
                sqlx::query("UPDATE collector_runs SET status='failed',error_code='collection_failed',completed_at=? WHERE id=?")
                        .bind(Utc::now().to_rfc3339()).bind(&run_id)
                        .execute(self.pool()).await?;
                sqlx::query(
                    "UPDATE sources SET health_status='unavailable',updated_at=? WHERE id=?",
                )
                .bind(Utc::now().to_rfc3339())
                .bind(&source.id)
                .execute(self.pool())
                .await?;
            }
        }
        Ok(summary)
    }

    async fn collect_futures_once(&self) -> Result<CoreIntelligenceRunSummary, AppError> {
        let mut summary = CoreIntelligenceRunSummary {
            sources_attempted: 1,
            ..Default::default()
        };
        let client = BinanceFuturesClient::new("https://fapi.binance.com")?;
        let mut all = Vec::new();
        for symbol in FUTURES_SYMBOLS {
            match client.fetch_symbol(symbol).await {
                Ok(mut observations) => all.append(&mut observations),
                Err(error) => summary.errors.push(format!("{symbol}: {error}")),
            }
        }
        if all.is_empty() {
            return Ok(summary);
        }
        summary.sources_succeeded = 1;
        summary.raw_records = FUTURES_SYMBOLS.len();
        summary.observations = all.len();
        self.persist_futures_observations(&all, &mut summary)
            .await?;
        Ok(summary)
    }

    pub(crate) async fn ingest_futures_stream_message(
        &self,
        payload: &str,
    ) -> Result<usize, AppError> {
        let observations =
            collector_runtime::normalize_futures_stream_message(payload, Utc::now())?;
        let mut history = self.history.lock().await;
        for observation in &observations {
            let volume = observation
                .metrics
                .get("trade_volume")
                .copied()
                .unwrap_or_default();
            for (metric, value) in &observation.metrics {
                history.ingest(
                    &observation.instrument_id,
                    metric,
                    MetricSample {
                        event_time: observation.exchange_event_time,
                        received_at: observation.received_at,
                        value: *value,
                        volume,
                    },
                );
            }
        }
        Ok(observations.len())
    }

    async fn persist_futures_observations(
        &self,
        observations: &[FuturesObservation],
        summary: &mut CoreIntelligenceRunSummary,
    ) -> Result<(), AppError> {
        for symbol in FUTURES_SYMBOLS {
            let selected: Vec<_> = observations
                .iter()
                .filter(|item| item.external_symbol == *symbol)
                .collect();
            if selected.is_empty() {
                continue;
            }
            let record = RawRecord::new(
                "binance-usdm-futures",
                serde_json::to_string(&selected)?,
                "application/json",
                None,
            )?;
            let mut tx = self.pool().begin().await?;
            sqlx::query("INSERT INTO raw_records (id,source_id,external_id,canonical_url,content_type,content_hash,title,published_at,retrieved_at,raw_payload_ref,payload_excerpt,expires_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)")
                .bind(&record.id).bind("binance-usdm-futures").bind(&record.external_id).bind(&record.canonical_url)
                .bind(&record.content_type).bind(&record.content_hash).bind(format!("{symbol} public USD-M snapshot"))
                .bind(selected.iter().map(|item| item.exchange_event_time).max().map(|value| value.to_rfc3339()))
                .bind(record.retrieved_at.to_rfc3339()).bind(&record.raw_payload_ref)
                .bind(record.payload.chars().take(65_536).collect::<String>())
                .bind((Utc::now() + Duration::days(7)).to_rfc3339()).execute(&mut *tx).await?;
            for observation in &selected {
                sqlx::query("INSERT INTO market_observations (id,raw_record_id,source_id,observation_type,market,instrument,observed_at,schema_version,completeness,stale,domain_json,expires_at) VALUES (?,?,?,?,?,?,?,?,?,0,?,?)")
                    .bind(&observation.id).bind(&record.id).bind(&observation.source_id)
                    .bind(enum_json(&observation.kind)).bind("crypto")
                    .bind(&observation.instrument_id)
                    .bind(observation.exchange_event_time.to_rfc3339()).bind(&observation.schema_version)
                    .bind(1.0_f64).bind(serde_json::to_string(observation)?)
                    .bind((Utc::now()+Duration::days(365)).to_rfc3339()).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }

        let mut buckets = Vec::new();
        {
            let mut history = self.history.lock().await;
            for observation in observations {
                let volume = observation
                    .metrics
                    .get("trade_volume")
                    .or_else(|| observation.metrics.get("volume_24h"))
                    .copied()
                    .unwrap_or_default();
                for (metric, value) in &observation.metrics {
                    let outcome = history.ingest(
                        &observation.instrument_id,
                        metric,
                        MetricSample {
                            event_time: observation.exchange_event_time,
                            received_at: observation.received_at,
                            value: *value,
                            volume,
                        },
                    );
                    if matches!(outcome, strategy_core::IngestResult::DiscardedOutOfOrder) {
                        sqlx::query("INSERT INTO intelligence_diagnostics (singleton_id,out_of_order_discards,updated_at) VALUES (1,1,?) ON CONFLICT(singleton_id) DO UPDATE SET out_of_order_discards=out_of_order_discards+1,updated_at=excluded.updated_at")
                            .bind(Utc::now().to_rfc3339()).execute(self.pool()).await?;
                    }
                    for window in [
                        HistoryWindow::OneMinute,
                        HistoryWindow::FiveMinutes,
                        HistoryWindow::FifteenMinutes,
                        HistoryWindow::OneHour,
                        HistoryWindow::FourHours,
                        HistoryWindow::TwentyFourHours,
                    ] {
                        let seconds = observation
                            .exchange_event_time
                            .timestamp()
                            .div_euclid(window.seconds())
                            * window.seconds();
                        if let Some(bucket_time) = chrono::DateTime::from_timestamp(seconds, 0)
                            && let Some(bucket) = history.bucket(
                                &observation.instrument_id,
                                metric,
                                window,
                                bucket_time,
                            )
                        {
                            buckets.push(bucket);
                        }
                    }
                }
            }
        }
        for bucket in &buckets {
            self.persist_metric_bucket(bucket).await?;
        }

        for symbol in FUTURES_SYMBOLS {
            let instrument = collector_runtime::canonical_futures_instrument(symbol)?;
            let now = Utc::now();
            let mut history = self.history.lock().await;
            let five_minute_seconds = now
                .timestamp()
                .div_euclid(HistoryWindow::FiveMinutes.seconds())
                * HistoryWindow::FiveMinutes.seconds();
            if let Some(bucket_time) = chrono::DateTime::from_timestamp(five_minute_seconds, 0)
                && let Some(volume_bucket) = history.bucket(
                    &instrument,
                    "trade_volume",
                    HistoryWindow::FiveMinutes,
                    bucket_time,
                )
            {
                history.ingest(
                    &instrument,
                    "trade_volume_5m",
                    MetricSample {
                        event_time: bucket_time,
                        received_at: now,
                        value: volume_bucket.volume_sum,
                        volume: volume_bucket.volume_sum,
                    },
                );
            }
            let baseline = |metric: &str, window: HistoryWindow, minimum: usize| {
                history.baseline(
                    &instrument,
                    metric,
                    window,
                    now,
                    minimum,
                    Duration::minutes(5),
                )
            };
            let funding = baseline("funding_rate", HistoryWindow::TwentyFourHours, 8);
            let oi = baseline("open_interest", HistoryWindow::FifteenMinutes, 3);
            let price = baseline("mark_price", HistoryWindow::FifteenMinutes, 3);
            let volume = baseline("trade_volume_5m", HistoryWindow::TwentyFourHours, 6);
            let imbalance = baseline("book_imbalance", HistoryWindow::FiveMinutes, 3);
            let spread = baseline("spread_bps", HistoryWindow::FiveMinutes, 3);
            drop(history);
            let engine = DerivativesStrategyEngine::default();
            let results = vec![
                engine.funding(&funding),
                engine.open_interest(&oi),
                engine.price_oi_divergence(&price, &oi),
                engine.volume(&volume),
                engine.orderbook_imbalance(&imbalance),
                engine.spread(&spread),
            ];
            for result in results {
                self.persist_futures_strategy_result(&result, observations, summary)
                    .await?;
            }
        }
        Ok(())
    }

    async fn persist_futures_strategy_result(
        &self,
        result: &StrategyResult,
        observations: &[FuturesObservation],
        summary: &mut CoreIntelligenceRunSummary,
    ) -> Result<(), AppError> {
        let mut tx = self.pool().begin().await?;
        persist_strategy_run(&mut tx, result).await?;
        if result.triggered && result.readiness == Readiness::Ready {
            append_event(&mut tx, "strategy.triggered", "strategy_run", &result.id, None, None, Some(&result.id),
                &json!({"strategyId":result.strategy_id,"instrumentId":result.instrument_id,"baselineWindow":result.baseline_window,"triggerMetrics":result.trigger_metrics})).await?;
        }
        tx.commit().await?;
        if !result.triggered || result.readiness != Readiness::Ready {
            return Ok(());
        }
        let instrument = result
            .instrument_id
            .as_deref()
            .ok_or(AppError::InvalidData)?;
        let recently_published: i64=sqlx::query_scalar("SELECT COUNT(*) FROM signals WHERE data_origin='REAL' AND strategy_id=? AND asset=? AND julianday(published_at)>=julianday('now','-5 minutes')")
            .bind(&result.strategy_id).bind(instrument).fetch_one(self.pool()).await?;
        if recently_published > 0 {
            return Ok(());
        }
        let latest = observations
            .iter()
            .filter(|item| item.instrument_id == instrument)
            .max_by_key(|item| item.exchange_event_time)
            .ok_or(AppError::InvalidData)?;
        let mut evidence = Evidence::new(
            "binance-usdm-futures",
            SourceType::ExchangeApi,
            AssetRef::new(Market::Crypto, instrument),
            format!("{instrument} derivatives baseline anomaly"),
            format!(
                "Observed public Binance USD-M metrics triggered {}.",
                result.strategy_id
            ),
            FactualLevel::Observed,
            0.95,
            1.0,
            latest.exchange_event_time,
        );
        evidence.data_origin = DataOrigin::Real;
        evidence.metadata = json!({"strategyRunId":result.id,"observationId":latest.id,"metrics":result.trigger_metrics});
        let link = EvidenceLink {
            evidence: evidence.clone(),
            relation: EvidenceRelation::Primary,
        };
        let mut candidate = result
            .to_candidate(
                AssetRef::new(Market::Crypto, instrument),
                SignalCategory::Exchange,
                format!("{instrument} derivatives anomaly"),
                "A ready rolling baseline crossed its configured threshold.",
                vec![link],
            )
            .ok_or(AppError::InvalidData)?
            .with_data_origin(DataOrigin::Real);
        if let Some(provenance) = candidate.strategy.as_mut() {
            provenance.source_ids = vec!["binance-usdm-futures".into()];
            provenance.observation_ids = vec![latest.id.clone()];
        }
        let signal = SignalPublisher
            .publish(candidate.clone())
            .map_err(|_| AppError::InvalidData)?;
        let mut tx = self.pool().begin().await?;
        insert_evidence(&mut tx, &evidence).await?;
        upsert_evidence_cluster_for_record(&mut tx, &evidence, None, None).await?;
        sqlx::query("INSERT INTO signal_candidates (id,market,category,asset,title,summary,status,created_at,strategy_id,strategy_version,parameter_snapshot_json,reason_codes_json,data_origin) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&candidate.id).bind("crypto").bind(candidate.category.as_str()).bind(&candidate.asset.symbol)
            .bind(&candidate.title).bind(&candidate.summary).bind("published").bind(Utc::now().to_rfc3339())
            .bind(&result.strategy_id).bind(&result.strategy_version).bind(serde_json::to_string(&result.parameter_snapshot)?)
            .bind(serde_json::to_string(&result.reason_codes)?).bind("REAL").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO signals (id,candidate_id,market,category,asset,title,summary,direction,confidence,urgency,time_horizon,evidence_quality,agent_id,model_id,status,created_at,updated_at,domain_json,strategy_id,strategy_version,data_origin,published_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&signal.id).bind(&signal.candidate_id).bind("crypto").bind(signal.category.as_str()).bind(&signal.asset.symbol)
            .bind(&signal.title).bind(&signal.summary).bind(enum_json(&signal.direction)).bind(signal.confidence).bind(signal.urgency)
            .bind(&signal.time_horizon).bind(signal.evidence_quality).bind(&signal.agent_id).bind(&signal.model_id).bind(enum_json(&signal.status))
            .bind(signal.created_at.to_rfc3339()).bind(signal.updated_at.to_rfc3339()).bind(serde_json::to_string(&signal)?)
            .bind(&result.strategy_id).bind(&result.strategy_version).bind("REAL").bind(signal.published_at.map(|value| value.to_rfc3339()))
            .execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO signal_evidence (signal_id,evidence_id,relation) VALUES (?,?,'primary')",
        )
        .bind(&signal.id)
        .bind(&evidence.id)
        .execute(&mut *tx)
        .await?;
        append_event(&mut tx, "signal.created", "signal", &signal.id, None, None, Some(&signal.id),
            &json!({"sourceId":"binance-usdm-futures","strategyRunId":result.id,"dataOrigin":"real"})).await?;
        tx.commit().await?;
        summary.evidence += 1;
        summary.candidates += 1;
        summary.published_signals += 1;
        Ok(())
    }

    async fn process_source_records(
        &self,
        source_id: &str,
        records: Vec<RawRecord>,
        summary: &mut CoreIntelligenceRunSummary,
    ) -> Result<(), AppError> {
        let expanded: Vec<RawRecord> =
            if matches!(source_id, "federal-reserve-press" | "nvidia-newsroom") {
                let mut entries = Vec::new();
                for record in &records {
                    entries.extend(parse_feed(record)?);
                }
                entries.into_iter().take(12).collect()
            } else {
                records
            };
        for record in expanded {
            let observations = match source_id {
                "binance-spot-24h" => normalize_binance_24h(&record)?,
                "mempool-space-summary" => {
                    vec![normalize_mempool(&record)?]
                }
                "federal-reserve-press" => vec![observation_from_feed_entry(
                    &record,
                    ObservationKind::Event,
                    AssetRef::new(Market::Traditional, "USD"),
                    Some("entity:federal-reserve".into()),
                )],
                "nvidia-newsroom" => vec![observation_from_feed_entry(
                    &record,
                    ObservationKind::Demand,
                    AssetRef::new(Market::Traditional, "NVDA"),
                    Some("entity:nvidia".into()),
                )],
                _ => vec![],
            };
            self.persist_pipeline_record(source_id, record, observations, summary)
                .await?;
        }
        Ok(())
    }

    async fn persist_pipeline_record(
        &self,
        source_id: &str,
        record: RawRecord,
        observations: Vec<NormalizedObservation>,
        summary: &mut CoreIntelligenceRunSummary,
    ) -> Result<(), AppError> {
        let source_type = match source_id {
            "binance-spot-24h" => SourceType::ExchangeApi,
            "mempool-space-summary" => SourceType::Blockchain,
            "federal-reserve-press" => SourceType::OfficialNews,
            "nvidia-newsroom" => SourceType::CompanyDisclosure,
            _ => SourceType::OfficialApi,
        };
        let mut tx = self.pool().begin().await?;
        let inserted = sqlx::query("INSERT OR IGNORE INTO raw_records (id,source_id,external_id,canonical_url,content_type,content_hash,title,published_at,retrieved_at,raw_payload_ref,payload_excerpt,expires_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&record.id).bind(source_id).bind(&record.external_id).bind(&record.canonical_url)
            .bind(&record.content_type).bind(&record.content_hash).bind(&record.title)
            .bind(record.published_at.map(|value| value.to_rfc3339()))
            .bind(record.retrieved_at.to_rfc3339()).bind(&record.raw_payload_ref)
            .bind(record.payload.chars().take(65_536).collect::<String>())
            .bind((Utc::now() + Duration::days(7)).to_rfc3339())
            .execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            tx.rollback().await?;
            return Ok(());
        }
        summary.raw_records += 1;
        for observation in observations {
            sqlx::query("INSERT INTO market_observations (id,raw_record_id,source_id,observation_type,market,instrument,entity_id,observed_at,schema_version,completeness,stale,domain_json,expires_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
                .bind(&observation.id).bind(&observation.raw_record_id).bind(source_id)
                .bind(enum_json(&observation.kind)).bind(observation.asset.market.as_str())
                .bind(&observation.asset.symbol).bind(&observation.entity_id)
                .bind(observation.observed_at.to_rfc3339()).bind(&observation.schema_version)
                .bind(observation.completeness).bind(observation.stale)
                .bind(serde_json::to_string(&observation)?)
                .bind((Utc::now() + Duration::days(365)).to_rfc3339())
                .execute(&mut *tx).await?;
            summary.observations += 1;

            let mut aggregate_buckets = Vec::new();
            {
                let mut history = self.history.lock().await;
                let instrument = if source_id == "binance-spot-24h" {
                    observation.asset.symbol.clone()
                } else if source_id == "mempool-space-summary" {
                    "BTC-NETWORK".to_string()
                } else {
                    observation.asset.symbol.clone()
                };
                for (metric, value) in &observation.metrics {
                    history.ingest(
                        &instrument,
                        metric,
                        MetricSample {
                            event_time: observation.observed_at,
                            received_at: record.retrieved_at,
                            value: *value,
                            volume: 0.0,
                        },
                    );
                    for window in [HistoryWindow::OneHour, HistoryWindow::TwentyFourHours] {
                        let seconds = observation
                            .observed_at
                            .timestamp()
                            .div_euclid(window.seconds())
                            * window.seconds();
                        if let Some(bucket_time) = chrono::DateTime::from_timestamp(seconds, 0)
                            && let Some(bucket) =
                                history.bucket(&instrument, metric, window, bucket_time)
                        {
                            aggregate_buckets.push(bucket);
                        }
                    }
                }
            }
            for bucket in aggregate_buckets {
                sqlx::query("INSERT INTO market_metric_buckets (instrument_id,metric,window,bucket_time,open,high,low,close,sum,sample_count,mean,stddev,volume_sum,first_event_at,last_event_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(instrument_id,metric,window,bucket_time) DO UPDATE SET high=excluded.high,low=excluded.low,close=excluded.close,sum=excluded.sum,sample_count=excluded.sample_count,mean=excluded.mean,stddev=excluded.stddev,last_event_at=excluded.last_event_at,updated_at=excluded.updated_at")
                    .bind(&bucket.instrument_id).bind(&bucket.metric).bind(bucket.window.as_str()).bind(bucket.bucket_time.to_rfc3339())
                    .bind(bucket.open).bind(bucket.high).bind(bucket.low).bind(bucket.close).bind(bucket.sum).bind(bucket.count as i64)
                    .bind(bucket.mean).bind(bucket.stddev).bind(bucket.volume_sum).bind(bucket.first_event_at.to_rfc3339())
                    .bind(bucket.last_event_at.to_rfc3339()).bind(Utc::now().to_rfc3339()).execute(&mut *tx).await?;
            }

            let mut evidence = Evidence::new(
                source_id,
                source_type,
                observation.asset.clone(),
                record
                    .title
                    .clone()
                    .unwrap_or_else(|| format!("{} observation", observation.asset.symbol)),
                observation.facts.join(" "),
                if matches!(
                    source_type,
                    SourceType::OfficialNews | SourceType::CompanyDisclosure
                ) {
                    FactualLevel::Official
                } else {
                    FactualLevel::Observed
                },
                if source_id == "mempool-space-summary" {
                    0.85
                } else {
                    0.95
                },
                observation.completeness,
                observation.observed_at,
            );
            evidence.raw_reference = Some(record.raw_payload_ref.clone());
            evidence.metadata = json!({"observationId": observation.id, "schemaVersion": observation.schema_version,
                "canonicalUrl":record.canonical_url,"contentHash":record.content_hash});
            evidence.data_origin = DataOrigin::Real;
            insert_evidence(&mut tx, &evidence).await?;
            upsert_evidence_cluster_for_record(
                &mut tx,
                &evidence,
                record.canonical_url.as_deref(),
                Some(&record.content_hash),
            )
            .await?;
            summary.evidence += 1;
            if source_id == "nvidia-newsroom" {
                sqlx::query("INSERT OR IGNORE INTO demand_timelines (id,entity_id,topic,direction,time_bucket,evidence_id,observed_at,created_at) VALUES (?,?,?,?,?,?,?,?)")
                    .bind(new_id("demand_observation")).bind("entity:nvidia").bind("ai_infrastructure")
                    .bind("neutral").bind(observation.observed_at.format("%Y-%m-%d").to_string())
                    .bind(&evidence.id).bind(observation.observed_at.to_rfc3339()).bind(Utc::now().to_rfc3339())
                    .execute(&mut *tx).await?;
            }

            let (strategy_id, category) = match source_id {
                "binance-spot-24h" => ("crypto.c.volume_expansion", SignalCategory::Exchange),
                "mempool-space-summary" => ("crypto.a.onchain_pressure", SignalCategory::Onchain),
                "federal-reserve-press" => ("traditional.b.official_event", SignalCategory::Event),
                "nvidia-newsroom" => ("traditional.c.demand_score", SignalCategory::Demand),
                _ => ("traditional.b.official_event", SignalCategory::Event),
            };
            let result = StrategyEngine::default()
                .evaluate(
                    strategy_id,
                    &StrategyContext {
                        now: Utc::now(),
                        observations: vec![observation.clone()],
                    },
                )
                .unwrap_or_else(|error| {
                    rejected_strategy(strategy_id, &observation.id, error.to_string())
                });
            persist_strategy_run(&mut tx, &result).await?;
            if !result.triggered || result.readiness != strategy_core::Readiness::Ready {
                summary.rejected_candidates += 1;
                append_event(&mut tx, "strategy.not_ready_or_not_triggered", "strategy_run", &result.id, None, None,
                    Some(&result.id), &json!({"strategyId":result.strategy_id,"readiness":enum_json(&result.readiness),"reasonCodes":result.reason_codes})).await?;
                continue;
            }
            let candidate = SignalCandidate::new(
                observation.asset.clone(),
                category,
                format!("{} candidate", observation.asset.symbol),
                format!("Candidate derived from {source_id} through strategy {strategy_id}."),
                SignalDirection::Watch,
                vec![EvidenceLink {
                    evidence: evidence.clone(),
                    relation: EvidenceRelation::Primary,
                }],
            )
            .with_strategy(StrategyProvenance {
                strategy_id: result.strategy_id.clone(),
                strategy_version: result.strategy_version.clone(),
                parameter_snapshot: result.parameter_snapshot.clone(),
                reason_codes: result.reason_codes.clone(),
                observation_ids: result.observation_ids.clone(),
                baseline_window: result.baseline_window.clone(),
                trigger_metrics: result.trigger_metrics.clone(),
                source_ids: vec![source_id.into()],
                evidence_ids: vec![evidence.id.clone()],
                input_snapshot: result.input_snapshot.clone(),
            })
            .with_data_origin(DataOrigin::Real);
            summary.candidates += 1;
            sqlx::query("INSERT INTO signal_candidates (id,market,category,asset,title,summary,status,created_at,strategy_id,strategy_version,parameter_snapshot_json,reason_codes_json,data_origin) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
                .bind(&candidate.id).bind(candidate.asset.market.as_str()).bind(candidate.category.as_str())
                .bind(&candidate.asset.symbol).bind(&candidate.title).bind(&candidate.summary)
                .bind(enum_json(&candidate.status)).bind(Utc::now().to_rfc3339())
                .bind(&result.strategy_id).bind(&result.strategy_version)
                .bind(serde_json::to_string(&result.parameter_snapshot)?)
                .bind(serde_json::to_string(&result.reason_codes)?)
                .bind("REAL")
                .execute(&mut *tx).await?;
            if let Ok(signal) = SignalPublisher.publish(candidate) {
                sqlx::query("INSERT INTO signals (id,candidate_id,market,category,asset,title,summary,direction,confidence,urgency,time_horizon,evidence_quality,agent_id,model_id,status,created_at,updated_at,domain_json,strategy_id,strategy_version,data_origin,published_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
                    .bind(&signal.id).bind(&signal.candidate_id).bind(signal.market())
                    .bind(signal.category.as_str()).bind(&signal.asset.symbol).bind(&signal.title)
                    .bind(&signal.summary).bind(enum_json(&signal.direction)).bind(signal.confidence)
                    .bind(signal.urgency).bind(&signal.time_horizon).bind(signal.evidence_quality)
                    .bind(&signal.agent_id).bind(&signal.model_id).bind(enum_json(&signal.status))
                    .bind(signal.created_at.to_rfc3339()).bind(signal.updated_at.to_rfc3339())
                    .bind(serde_json::to_string(&signal)?).bind(&result.strategy_id)
                    .bind(&result.strategy_version).bind("REAL")
                    .bind(signal.published_at.map(|value|value.to_rfc3339())).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO signal_evidence (signal_id,evidence_id,relation) VALUES (?,?,'primary')")
                    .bind(&signal.id).bind(&evidence.id).execute(&mut *tx).await?;
                append_event(&mut tx, "signal.created", "signal", &signal.id, None, None,
                    Some(&signal.id), &json!({"sourceId":source_id,"strategyId":result.strategy_id,"strategyVersion":result.strategy_version})).await?;
                summary.published_signals += 1;
            }
        }
        tx.commit().await?;
        Ok(())
    }
}

async fn upsert_evidence_cluster_for_record(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    evidence: &Evidence,
    canonical_url: Option<&str>,
    content_hash: Option<&str>,
) -> Result<(), AppError> {
    let normalized_title = evidence
        .title
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    let topic_key = format!(
        "{}:{}",
        evidence.asset.symbol,
        normalized_title.chars().take(120).collect::<String>()
    );
    let existing=sqlx::query("SELECT id,original_evidence_id FROM evidence_clusters WHERE topic_key=? ORDER BY created_at LIMIT 1")
        .bind(&topic_key).fetch_optional(&mut **tx).await?;
    let now = Utc::now().to_rfc3339();
    let (cluster_id, relation) = if let Some(row) = existing {
        let cluster_id: String = row.get("id");
        let original_id: Option<String> = row.get("original_evidence_id");
        let relation = if let Some(original_id) = original_id {
            let original_json: String =
                sqlx::query_scalar("SELECT domain_json FROM evidence WHERE id=?")
                    .bind(original_id)
                    .fetch_one(&mut **tx)
                    .await?;
            let original: Evidence = serde_json::from_str(&original_json)?;
            classify_cluster_relation(
                &EvidenceFingerprint {
                    source_id: original.source,
                    canonical_url: original
                        .metadata
                        .get("canonicalUrl")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                    original_url: original
                        .metadata
                        .get("originalUrl")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                    content: original.content,
                },
                &EvidenceFingerprint {
                    source_id: evidence.source.clone(),
                    canonical_url: canonical_url.map(str::to_owned),
                    original_url: None,
                    content: evidence.content.clone(),
                },
                false,
            )
        } else {
            ClusterRelation::Original
        };
        (cluster_id, relation)
    } else {
        let cluster_id = new_id("evidence_cluster");
        sqlx::query("INSERT INTO evidence_clusters (id,topic_key,original_evidence_id,members_json,independent_confirmations,contradiction_count,created_at,updated_at) VALUES (?,?,?,'[]',0,0,?,?)")
            .bind(&cluster_id).bind(&topic_key).bind(&evidence.id).bind(&now).bind(&now).execute(&mut **tx).await?;
        (cluster_id, ClusterRelation::Original)
    };
    sqlx::query("INSERT OR IGNORE INTO evidence_cluster_members (cluster_id,evidence_id,source_id,canonical_url,content_hash,relation,created_at) VALUES (?,?,?,?,?,?,?)")
        .bind(&cluster_id).bind(&evidence.id).bind(&evidence.source).bind(canonical_url).bind(content_hash)
        .bind(match relation{ClusterRelation::Original=>"original",ClusterRelation::Syndication=>"syndication",
            ClusterRelation::Independent=>"independent",ClusterRelation::Contradicting=>"contradicting"}).bind(&now).execute(&mut **tx).await?;
    let rows=sqlx::query("SELECT evidence_id,source_id,relation FROM evidence_cluster_members WHERE cluster_id=? ORDER BY created_at,evidence_id")
        .bind(&cluster_id).fetch_all(&mut **tx).await?;
    let members = rows
        .into_iter()
        .map(|row| EvidenceClusterMember {
            evidence_id: row.get("evidence_id"),
            source_id: row.get("source_id"),
            relation: match row.get::<String, _>("relation").as_str() {
                "original" => ClusterRelation::Original,
                "syndication" => ClusterRelation::Syndication,
                "independent" => ClusterRelation::Independent,
                _ => ClusterRelation::Contradicting,
            },
        })
        .collect::<Vec<_>>();
    let independent = members
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
        .saturating_sub(1);
    let contradictions = members
        .iter()
        .filter(|member| member.relation == ClusterRelation::Contradicting)
        .count();
    sqlx::query("UPDATE evidence_clusters SET members_json=?,independent_confirmations=?,contradiction_count=?,updated_at=? WHERE id=?")
        .bind(serde_json::to_string(&members)?).bind(independent as i64).bind(contradictions as i64).bind(&now).bind(&cluster_id)
        .execute(&mut **tx).await?;
    Ok(())
}

fn rejected_strategy(strategy_id: &str, observation_id: &str, reason: String) -> StrategyResult {
    StrategyResult {
        id: new_id("strategy_run"),
        strategy_id: strategy_id.into(),
        strategy_version: "1.0.0".into(),
        triggered: false,
        direction: SignalDirection::Watch,
        score: 0.0,
        reason_codes: vec!["insufficient_history_or_metric".into()],
        observation_ids: vec![observation_id.into()],
        parameter_snapshot: json!({}),
        rejected_reason: Some(reason),
        readiness: strategy_core::Readiness::MissingInput,
        instrument_id: None,
        baseline_window: None,
        input_snapshot: Value::Null,
        trigger_metrics: Default::default(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
    }
}

async fn persist_strategy_run(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    result: &StrategyResult,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO strategy_runs (id,strategy_id,strategy_version,status,direction,score,reason_codes_json,observation_ids_json,parameter_snapshot_json,rejected_reason,created_at,readiness,instrument_id,baseline_window,input_snapshot_json,trigger_metrics_json,started_at,completed_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&result.id).bind(&result.strategy_id).bind(&result.strategy_version)
        .bind(if result.triggered { "triggered" } else if result.rejected_reason.is_some() { "rejected" } else { "not_triggered" })
        .bind(enum_json(&result.direction)).bind(result.score)
        .bind(serde_json::to_string(&result.reason_codes)?)
        .bind(serde_json::to_string(&result.observation_ids)?)
        .bind(serde_json::to_string(&result.parameter_snapshot)?)
        .bind(&result.rejected_reason).bind(Utc::now().to_rfc3339())
        .bind(enum_json(&result.readiness)).bind(&result.instrument_id).bind(&result.baseline_window)
        .bind(serde_json::to_string(&result.input_snapshot)?).bind(serde_json::to_string(&result.trigger_metrics)?)
        .bind(result.started_at.map(|value| value.to_rfc3339())).bind(result.completed_at.map(|value| value.to_rfc3339()))
        .execute(&mut **tx).await?;
    Ok(())
}

fn enum_json<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fixture_feed_persists_complete_evidence_gated_pipeline() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = AppRuntime::initialize(&directory.path().join("pipeline.sqlite3"))
            .await
            .unwrap();
        let xml = r#"<rss version="2.0"><channel><title>Federal Reserve</title><item><guid>official-1</guid><title>Federal Reserve issues rate decision</title><link>https://example.com/official-1</link><description>Official rate decision facts.</description></item></channel></rss>"#;
        let raw = RawRecord::new(
            "federal-reserve-press",
            xml,
            "application/rss+xml",
            Some("https://example.com/feed"),
        )
        .unwrap();
        let mut summary = CoreIntelligenceRunSummary::default();
        runtime
            .process_source_records("federal-reserve-press", vec![raw], &mut summary)
            .await
            .unwrap();

        assert_eq!(summary.raw_records, 1);
        assert_eq!(summary.observations, 1);
        assert_eq!(summary.evidence, 1);
        assert_eq!(summary.candidates, 1);
        assert_eq!(summary.published_signals, 1);
        let audit_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM runtime_events WHERE event_type='signal.created'",
        )
        .fetch_one(runtime.pool())
        .await
        .unwrap();
        assert_eq!(audit_count, 1);
        let cluster_members: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM evidence_cluster_members")
                .fetch_one(runtime.pool())
                .await
                .unwrap();
        assert_eq!(cluster_members, 1);
    }

    #[tokio::test]
    async fn missing_rolling_history_persists_strategy_run_without_candidate_or_signal() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = AppRuntime::initialize(&directory.path().join("reject.sqlite3"))
            .await
            .unwrap();
        let raw = RawRecord::new(
            "binance-spot-24h",
            r#"[{"symbol":"BTCUSDT","lastPrice":"60000","volume":"100","quoteVolume":"6000000"}]"#,
            "application/json",
            Some("https://example.com/ticker"),
        )
        .unwrap();
        let mut summary = CoreIntelligenceRunSummary::default();
        runtime
            .process_source_records("binance-spot-24h", vec![raw], &mut summary)
            .await
            .unwrap();

        assert_eq!(summary.candidates, 0);
        assert_eq!(summary.rejected_candidates, 1);
        assert_eq!(summary.published_signals, 0);
        let candidates: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM signal_candidates")
            .fetch_one(runtime.pool())
            .await
            .unwrap();
        assert_eq!(candidates, 0);
    }
}
