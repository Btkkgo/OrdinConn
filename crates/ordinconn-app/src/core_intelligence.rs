use crate::{AppError, AppRuntime, events::append_event, services::insert_evidence};
use chrono::{Duration, Utc};
use collector_runtime::{
    Collector, CollectorKind, HttpCollector, NormalizedObservation, ObservationKind, RawRecord,
    builtin_sources, normalize_binance_24h, normalize_mempool, observation_from_feed_entry,
    parse_feed,
};
use evidence_core::{Evidence, EvidenceRelation, FactualLevel, SourceType};
use market_core::{AssetRef, Market, new_id};
use serde::{Deserialize, Serialize};
use serde_json::json;
use signal_core::{
    CandidateStatus, EvidenceLink, SignalCandidate, SignalCategory, SignalDirection,
    SignalPublisher, StrategyProvenance,
};
use sqlx::Sqlite;
use strategy_core::{StrategyContext, StrategyEngine, StrategyResult};

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
        tx.commit().await?;
        Ok(())
    }

    pub async fn run_core_intelligence_once(&self) -> Result<CoreIntelligenceRunSummary, AppError> {
        let mut summary = CoreIntelligenceRunSummary::default();
        for source in builtin_sources()
            .into_iter()
            .filter(|source| source.enabled && source.collector_kind != CollectorKind::WebSocket)
        {
            summary.sources_attempted += 1;
            let collector = match HttpCollector::new(source.clone()) {
                Ok(value) => value,
                Err(error) => {
                    summary.errors.push(format!("{}: {error}", source.id));
                    continue;
                }
            };
            let run_id = new_id("collector_run");
            sqlx::query("INSERT INTO collector_runs (id,source_id,status,started_at) VALUES (?,?,'running',?)")
                .bind(&run_id).bind(&source.id).bind(Utc::now().to_rfc3339())
                .execute(self.pool()).await?;
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
                            sqlx::query("UPDATE sources SET health_status='healthy',updated_at=? WHERE id=?")
                                .bind(Utc::now().to_rfc3339()).bind(&source.id)
                                .execute(self.pool()).await?;
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
                            sqlx::query(
                                "UPDATE sources SET health_status=?,updated_at=? WHERE id=?",
                            )
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
        }
        Ok(summary)
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
            evidence.metadata = json!({"observationId": observation.id, "schemaVersion": observation.schema_version});
            insert_evidence(&mut tx, &evidence).await?;
            summary.evidence += 1;

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
            let mut candidate = SignalCandidate::new(
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
            });
            if !result.triggered {
                candidate.status = CandidateStatus::InsufficientEvidence;
                summary.rejected_candidates += 1;
            }
            summary.candidates += 1;
            sqlx::query("INSERT INTO signal_candidates (id,market,category,asset,title,summary,status,created_at,strategy_id,strategy_version,parameter_snapshot_json,reason_codes_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)")
                .bind(&candidate.id).bind(candidate.asset.market.as_str()).bind(candidate.category.as_str())
                .bind(&candidate.asset.symbol).bind(&candidate.title).bind(&candidate.summary)
                .bind(enum_json(&candidate.status)).bind(Utc::now().to_rfc3339())
                .bind(&result.strategy_id).bind(&result.strategy_version)
                .bind(serde_json::to_string(&result.parameter_snapshot)?)
                .bind(serde_json::to_string(&result.reason_codes)?)
                .execute(&mut *tx).await?;
            if result.triggered
                && let Ok(signal) = SignalPublisher.publish(candidate)
            {
                sqlx::query("INSERT INTO signals (id,candidate_id,market,category,asset,title,summary,direction,confidence,urgency,time_horizon,evidence_quality,agent_id,model_id,status,created_at,updated_at,domain_json,strategy_id,strategy_version) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
                    .bind(&signal.id).bind(&signal.candidate_id).bind(signal.market())
                    .bind(signal.category.as_str()).bind(&signal.asset.symbol).bind(&signal.title)
                    .bind(&signal.summary).bind(enum_json(&signal.direction)).bind(signal.confidence)
                    .bind(signal.urgency).bind(&signal.time_horizon).bind(signal.evidence_quality)
                    .bind(&signal.agent_id).bind(&signal.model_id).bind(enum_json(&signal.status))
                    .bind(signal.created_at.to_rfc3339()).bind(signal.updated_at.to_rfc3339())
                    .bind(serde_json::to_string(&signal)?).bind(&result.strategy_id)
                    .bind(&result.strategy_version).execute(&mut *tx).await?;
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
    }
}

async fn persist_strategy_run(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    result: &StrategyResult,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO strategy_runs (id,strategy_id,strategy_version,status,direction,score,reason_codes_json,observation_ids_json,parameter_snapshot_json,rejected_reason,created_at) VALUES (?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&result.id).bind(&result.strategy_id).bind(&result.strategy_version)
        .bind(if result.triggered { "triggered" } else if result.rejected_reason.is_some() { "rejected" } else { "not_triggered" })
        .bind(enum_json(&result.direction)).bind(result.score)
        .bind(serde_json::to_string(&result.reason_codes)?)
        .bind(serde_json::to_string(&result.observation_ids)?)
        .bind(serde_json::to_string(&result.parameter_snapshot)?)
        .bind(&result.rejected_reason).bind(Utc::now().to_rfc3339())
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
    }

    #[tokio::test]
    async fn missing_rolling_history_persists_rejected_candidate_not_signal() {
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

        assert_eq!(summary.candidates, 1);
        assert_eq!(summary.rejected_candidates, 1);
        assert_eq!(summary.published_signals, 0);
        let status: String = sqlx::query_scalar("SELECT status FROM signal_candidates")
            .fetch_one(runtime.pool())
            .await
            .unwrap();
        assert_eq!(status, "insufficient_evidence");
    }
}
