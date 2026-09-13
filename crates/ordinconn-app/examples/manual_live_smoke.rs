use collector_runtime::BinanceFuturesWebSocket;
use ordinconn_app::AppRuntime;
use serde_json::json;
use sqlx::Row;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("ORDINCONN_LIVE_SMOKE").as_deref() != Ok("1") {
        return Err("set ORDINCONN_LIVE_SMOKE=1 to acknowledge real public-network access".into());
    }
    let directory = tempfile::tempdir()?;
    let runtime = AppRuntime::initialize(&directory.path().join("live-smoke.sqlite3")).await?;
    let rest_and_feeds = runtime.run_core_intelligence_once().await?;
    let stream = BinanceFuturesWebSocket::public(Duration::from_secs(15))?;
    let mut stream_events = stream.subscribe();
    stream.start().await?;
    let websocket_result = match tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let collector_runtime::FuturesStreamEvent::Data(value) = stream_events.recv().await?
            {
                break Ok::<_, tokio::sync::broadcast::error::RecvError>(value);
            }
        }
    })
    .await
    {
        Ok(Ok(value)) => {
            json!({"status":"available","sample":value.chars().take(300).collect::<String>()})
        }
        Ok(Err(error)) => json!({"status":"unavailable","error":error.to_string()}),
        Err(_) => json!({"status":"unavailable","error":"timeout"}),
    };
    stream.shutdown().await;
    let futures_rows = sqlx::query("SELECT instrument,observation_type,observed_at,domain_json FROM market_observations WHERE source_id='binance-usdm-futures' ORDER BY observed_at DESC LIMIT 21")
        .fetch_all(runtime.pool()).await?;
    let futures_observations: Vec<_> = futures_rows.into_iter().map(|row| json!({
        "instrumentId":row.get::<String,_>("instrument"), "type":row.get::<String,_>("observation_type"),
        "exchangeEventTime":row.get::<String,_>("observed_at"), "observation":serde_json::from_str::<serde_json::Value>(row.get("domain_json")).unwrap_or_default()
    })).collect();
    let strategy_runs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM strategy_runs")
        .fetch_one(runtime.pool())
        .await?;
    let strategy_readiness = sqlx::query(
        "SELECT readiness,COUNT(*) count FROM strategy_runs GROUP BY readiness ORDER BY readiness",
    )
    .fetch_all(runtime.pool())
    .await?
    .into_iter()
    .map(|row| json!({"readiness":row.get::<String,_>("readiness"),"count":row.get::<i64,_>("count")}))
    .collect::<Vec<_>>();
    let candidates: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM signal_candidates WHERE data_origin='REAL'")
            .fetch_one(runtime.pool())
            .await?;
    let signals: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM signals WHERE data_origin='REAL'")
        .fetch_one(runtime.pool())
        .await?;
    let clusters: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM evidence_clusters")
        .fetch_one(runtime.pool())
        .await?;
    let demand_observations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM demand_timelines")
        .fetch_one(runtime.pool())
        .await?;
    let demand_buckets: i64 =
        sqlx::query_scalar("SELECT COUNT(DISTINCT time_bucket) FROM demand_timelines")
            .fetch_one(runtime.pool())
            .await?;
    let btc_network_buckets: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM market_metric_buckets WHERE instrument_id='BTC-NETWORK'",
    )
    .fetch_one(runtime.pool())
    .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "runType": "manual_live_smoke",
            "usesPublicNetwork": true,
            "restAndFeeds": rest_and_feeds,
            "futuresWebsocket": websocket_result,
            "futuresObservations": futures_observations,
            "actualStrategyRuns":strategy_runs,
            "strategyReadiness":strategy_readiness,
            "actualRealCandidates":candidates,
            "actualRealPublishedSignals":signals
            ,"evidenceClusters":clusters,"demandTimeline":{"observations":demand_observations,"distinctTimeBuckets":demand_buckets},
            "btcNetworkBaselineBuckets":btc_network_buckets
        }))?
    );
    runtime.shutdown().await?;
    Ok(())
}
