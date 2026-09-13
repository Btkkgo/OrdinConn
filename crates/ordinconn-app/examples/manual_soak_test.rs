use ordinconn_app::AppRuntime;
use serde_json::json;
use sqlx::Row;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("ORDINCONN_SOAK_TEST").as_deref() != Ok("1") {
        return Err("set ORDINCONN_SOAK_TEST=1 to acknowledge real public-network access".into());
    }
    let seconds = std::env::var("ORDINCONN_SOAK_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1_800)
        .clamp(30, 3_600);
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("soak.sqlite3");
    let runtime = AppRuntime::initialize(&database).await?;
    let initial_bytes = std::fs::metadata(&database)
        .map(|value| value.len())
        .unwrap_or_default();
    runtime.start_continuous_intelligence().await?;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(seconds) {
        tokio::time::sleep(
            Duration::from_secs(10)
                .min(Duration::from_secs(seconds).saturating_sub(started.elapsed())),
        )
        .await;
    }
    runtime.pause_continuous_intelligence().await?;
    let diagnostics = runtime.intelligence_diagnostics().await?;
    let duplicates: i64=sqlx::query("SELECT COALESCE(SUM(c-1),0) duplicates FROM (SELECT COUNT(*) c FROM raw_records GROUP BY source_id,content_hash HAVING c>1)")
        .fetch_one(runtime.pool()).await?.get("duplicates");
    let readiness=sqlx::query("SELECT readiness,COUNT(*) count FROM strategy_runs GROUP BY readiness ORDER BY readiness")
        .fetch_all(runtime.pool()).await?.into_iter().map(|row|json!({"readiness":row.get::<String,_>("readiness"),"count":row.get::<i64,_>("count")})).collect::<Vec<_>>();
    let scheduler = runtime.load_scheduler_state().await?;
    let max_scheduler_start_drift_ms = scheduler
        .iter()
        .filter_map(|state| {
            Some(
                (state.last_started_at? - state.last_scheduled_at?)
                    .num_milliseconds()
                    .max(0),
            )
        })
        .max()
        .unwrap_or_default();
    let triggered=sqlx::query("SELECT strategy_id,instrument_id,baseline_window,reason_codes_json,trigger_metrics_json FROM strategy_runs WHERE status='triggered' ORDER BY created_at")
        .fetch_all(runtime.pool()).await?.into_iter().map(|row|json!({"strategyId":row.get::<String,_>("strategy_id"),
            "instrumentId":row.get::<Option<String>,_>("instrument_id"),"baselineWindow":row.get::<Option<String>,_>("baseline_window"),
            "reasonCodes":serde_json::from_str::<serde_json::Value>(row.get("reason_codes_json")).unwrap_or_default(),
            "triggerMetrics":serde_json::from_str::<serde_json::Value>(row.get("trigger_metrics_json")).unwrap_or_default()})).collect::<Vec<_>>();
    let real_signals=sqlx::query("SELECT id,strategy_id,asset,published_at FROM signals WHERE data_origin='REAL' ORDER BY published_at")
        .fetch_all(runtime.pool()).await?.into_iter().map(|row|json!({"id":row.get::<String,_>("id"),"strategyId":row.get::<Option<String>,_>("strategy_id"),
            "instrumentId":row.get::<String,_>("asset"),"publishedAt":row.get::<Option<String>,_>("published_at")})).collect::<Vec<_>>();
    runtime.shutdown().await?;
    let final_bytes = std::fs::metadata(&database)
        .map(|value| value.len())
        .unwrap_or_default();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "runType":"manual_soak_test","usesPublicNetwork":true,"configuredSeconds":seconds,
            "elapsedSeconds":started.elapsed().as_secs(),"diagnostics":diagnostics,
            "duplicateRawRecords":duplicates,"readiness":readiness,"schedulerState":scheduler,
            "maxSchedulerStartDriftMs":max_scheduler_start_drift_ms,"triggeredStrategyRuns":triggered,"realPublishedSignals":real_signals,
            "databaseBytes":{"initial":initial_bytes,"final":final_bytes,"growth":final_bytes.saturating_sub(initial_bytes)},
            "boundedHistory":{"samples":diagnostics.history_samples,"series":diagnostics.history_series,
                "capacityPerSeries":diagnostics.history_capacity_per_series,
                "utilization":if diagnostics.history_series==0{0.0}else{diagnostics.history_samples as f64/(diagnostics.history_series*diagnostics.history_capacity_per_series) as f64}}
        }))?
    );
    Ok(())
}
