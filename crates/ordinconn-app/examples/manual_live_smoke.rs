use collector_runtime::{Collector, CollectorKind, WebSocketCollector, builtin_sources};
use ordinconn_app::AppRuntime;
use serde_json::json;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("ORDINCONN_LIVE_SMOKE").as_deref() != Ok("1") {
        return Err("set ORDINCONN_LIVE_SMOKE=1 to acknowledge real public-network access".into());
    }
    let directory = tempfile::tempdir()?;
    let runtime = AppRuntime::initialize(&directory.path().join("live-smoke.sqlite3")).await?;
    let rest_and_feeds = runtime.run_core_intelligence_once().await?;
    let websocket = builtin_sources()
        .into_iter()
        .find(|source| source.collector_kind == CollectorKind::WebSocket)
        .map(WebSocketCollector::new)
        .transpose()?;
    let websocket_result = if let Some(collector) = websocket {
        match tokio::time::timeout(Duration::from_secs(15), collector.subscribe()).await {
            Ok(Ok(records)) => json!({"status":"available","records":records.len()}),
            Ok(Err(error)) => json!({"status":"unavailable","error":error.to_string()}),
            Err(_) => json!({"status":"unavailable","error":"timeout"}),
        }
    } else {
        json!({"status":"unavailable","error":"not configured"})
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "runType": "manual_live_smoke",
            "usesPublicNetwork": true,
            "restAndFeeds": rest_and_feeds,
            "websocket": websocket_result
        }))?
    );
    Ok(())
}
