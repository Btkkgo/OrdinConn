use crate::{CollectorError, canonicalize_url};
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use market_core::new_id;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::{
    sync::{Mutex, broadcast, watch},
    task::JoinHandle,
};
use tokio_tungstenite::tungstenite::Message;
use url::Url;

pub const FUTURES_SYMBOLS: &[&str] = &["BTCUSDT", "ETHUSDT", "SOLUSDT"];

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuturesWebSocketDiagnostics {
    pub connections: u64,
    pub subscriptions: u64,
    pub messages: u64,
    pub reconnects: u64,
    pub stale_disconnects: u64,
    pub last_message_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "detail")]
pub enum FuturesStreamEvent {
    Connected,
    Subscribed,
    Data(String),
    Stale,
    Reconnecting,
    Stopped,
    Error(String),
}

pub struct BinanceFuturesWebSocket {
    endpoint: String,
    streams: Vec<String>,
    stale_after: std::time::Duration,
    shutdown: watch::Sender<bool>,
    events: broadcast::Sender<FuturesStreamEvent>,
    diagnostics: Arc<Mutex<FuturesWebSocketDiagnostics>>,
    handle: Mutex<Option<JoinHandle<()>>>,
    running: AtomicBool,
}

impl BinanceFuturesWebSocket {
    pub fn public(stale_after: std::time::Duration) -> Result<Self, CollectorError> {
        Self::new(
            "wss://fstream.binance.com/ws",
            default_futures_streams(),
            stale_after,
        )
    }

    pub fn new(
        endpoint: &str,
        streams: Vec<String>,
        stale_after: std::time::Duration,
    ) -> Result<Self, CollectorError> {
        let url = Url::parse(endpoint)
            .map_err(|error| CollectorError::InvalidSource(error.to_string()))?;
        if !matches!(url.scheme(), "ws" | "wss") || streams.is_empty() {
            return Err(CollectorError::Policy(
                "futures stream requires WS(S) and subscriptions".into(),
            ));
        }
        let (shutdown, _) = watch::channel(false);
        let (events, _) = broadcast::channel(256);
        Ok(Self {
            endpoint: endpoint.into(),
            streams,
            stale_after: stale_after.max(std::time::Duration::from_millis(50)),
            shutdown,
            events,
            diagnostics: Arc::new(Mutex::new(Default::default())),
            handle: Mutex::new(None),
            running: AtomicBool::new(false),
        })
    }

    pub async fn start(&self) -> Result<(), CollectorError> {
        if self.running.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        self.shutdown.send_replace(false);
        let endpoint = self.endpoint.clone();
        let streams = self.streams.clone();
        let stale_after = self.stale_after;
        let mut shutdown = self.shutdown.subscribe();
        let events = self.events.clone();
        let diagnostics = Arc::clone(&self.diagnostics);
        *self.handle.lock().await = Some(tokio::spawn(async move {
            let mut attempt = 0_u32;
            loop {
                if *shutdown.borrow() {
                    break;
                }
                match tokio_tungstenite::connect_async(&endpoint).await {
                    Ok((socket, _)) => {
                        attempt = 0;
                        diagnostics.lock().await.connections += 1;
                        let _ = events.send(FuturesStreamEvent::Connected);
                        let (mut sink, mut source) = socket.split();
                        let request =
                            serde_json::json!({"method":"SUBSCRIBE","params":streams,"id":1})
                                .to_string();
                        if sink.send(Message::Text(request.into())).await.is_err() {
                            continue;
                        }
                        diagnostics.lock().await.subscriptions += 1;
                        let _ = events.send(FuturesStreamEvent::Subscribed);
                        let reconnect = loop {
                            tokio::select! {
                                changed = shutdown.changed() => {
                                    if changed.is_err() || *shutdown.borrow() {
                                        let _ = sink.send(Message::Close(None)).await;
                                        break false;
                                    }
                                }
                                incoming = tokio::time::timeout(stale_after, source.next()) => {
                                    match incoming {
                                        Ok(Some(Ok(Message::Text(text)))) => {
                                            let acknowledged = serde_json::from_str::<Value>(&text)
                                                .ok()
                                                .is_some_and(|value| value.get("id").is_some() && value.get("result").is_some());
                                            if acknowledged { continue; }
                                            let mut state = diagnostics.lock().await;
                                            state.messages += 1; state.last_message_at=Some(Utc::now());
                                            drop(state);
                                            let _=events.send(FuturesStreamEvent::Data(text.to_string()));
                                        }
                                        Ok(Some(Ok(Message::Ping(payload)))) => { let _=sink.send(Message::Pong(payload)).await; }
                                        Ok(Some(Ok(Message::Close(_)))) | Ok(None) | Ok(Some(Err(_))) => break true,
                                        Err(_) => {
                                            diagnostics.lock().await.stale_disconnects += 1;
                                            let _=events.send(FuturesStreamEvent::Stale);
                                            let _=sink.send(Message::Close(None)).await;
                                            break true;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        };
                        if !reconnect {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = events.send(FuturesStreamEvent::Error(error.to_string()));
                    }
                }
                if *shutdown.borrow() {
                    break;
                }
                attempt = attempt.saturating_add(1);
                diagnostics.lock().await.reconnects += 1;
                let _ = events.send(FuturesStreamEvent::Reconnecting);
                let base = (1_u64 << attempt.min(5)).min(30);
                let jitter = streams.iter().map(String::len).sum::<usize>() as u64 % 251;
                tokio::select! {
                    _=tokio::time::sleep(std::time::Duration::from_millis(base*1_000+jitter))=>{},
                    _=shutdown.changed()=>{ if *shutdown.borrow(){break;} }
                }
            }
            let _ = events.send(FuturesStreamEvent::Stopped);
        }));
        Ok(())
    }

    pub async fn shutdown(&self) {
        self.shutdown.send_replace(true);
        if let Some(handle) = self.handle.lock().await.take() {
            let _ = handle.await;
        }
        self.running.store(false, Ordering::Release);
    }
    pub fn subscribe(&self) -> broadcast::Receiver<FuturesStreamEvent> {
        self.events.subscribe()
    }
    pub async fn diagnostics(&self) -> FuturesWebSocketDiagnostics {
        self.diagnostics.lock().await.clone()
    }
}

pub fn default_futures_streams() -> Vec<String> {
    FUTURES_SYMBOLS
        .iter()
        .flat_map(|symbol| {
            let symbol = symbol.to_ascii_lowercase();
            [
                format!("{symbol}@markPrice@1s"),
                format!("{symbol}@aggTrade"),
                format!("{symbol}@bookTicker"),
                format!("{symbol}@depth5@100ms"),
            ]
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketType {
    Spot,
    UsdmPerpetual,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FuturesObservationKind {
    MarkPrice,
    FundingRate,
    OpenInterest,
    Trade,
    OrderBook,
    Spread,
    Volume,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuturesObservation {
    pub id: String,
    pub source_id: String,
    pub instrument_id: String,
    pub market_type: MarketType,
    pub external_symbol: String,
    pub kind: FuturesObservationKind,
    pub exchange_event_time: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub schema_version: String,
    pub metrics: HashMap<String, f64>,
}

pub fn canonical_futures_instrument(symbol: &str) -> Result<String, CollectorError> {
    canonical_instrument(symbol, "PERP")
}
pub fn canonical_spot_instrument(symbol: &str) -> Result<String, CollectorError> {
    canonical_instrument(symbol, "SPOT")
}
fn canonical_instrument(symbol: &str, suffix: &str) -> Result<String, CollectorError> {
    let symbol = symbol.to_ascii_uppercase();
    if !FUTURES_SYMBOLS.contains(&symbol.as_str()) {
        return Err(CollectorError::SchemaDrift(format!(
            "unsupported Binance symbol {symbol}"
        )));
    }
    Ok(format!("{}-USDT-{suffix}", symbol.trim_end_matches("USDT")))
}

pub fn normalize_premium_index(
    payload: &str,
    received_at: DateTime<Utc>,
) -> Result<Vec<FuturesObservation>, CollectorError> {
    let value = object(payload)?;
    let symbol = string(&value, "symbol")?;
    let event_time = timestamp(&value, "time")?;
    Ok(vec![
        observation(
            symbol,
            FuturesObservationKind::MarkPrice,
            event_time,
            received_at,
            HashMap::from([
                ("mark_price".into(), number(&value, "markPrice")?),
                ("index_price".into(), number(&value, "indexPrice")?),
            ]),
        ),
        observation(
            symbol,
            FuturesObservationKind::FundingRate,
            event_time,
            received_at,
            HashMap::from([
                ("funding_rate".into(), number(&value, "lastFundingRate")?),
                (
                    "next_funding_time_ms".into(),
                    required_f64(&value, "nextFundingTime")?,
                ),
            ]),
        ),
    ])
}

pub fn normalize_open_interest(
    payload: &str,
    received_at: DateTime<Utc>,
) -> Result<FuturesObservation, CollectorError> {
    let value = object(payload)?;
    let symbol = string(&value, "symbol")?;
    let event_time = timestamp(&value, "time")?;
    Ok(observation(
        symbol,
        FuturesObservationKind::OpenInterest,
        event_time,
        received_at,
        HashMap::from([("open_interest".into(), number(&value, "openInterest")?)]),
    ))
}

pub fn normalize_book_ticker(
    payload: &str,
    received_at: DateTime<Utc>,
) -> Result<FuturesObservation, CollectorError> {
    let value = object(payload)?;
    let symbol = string(&value, "symbol").or_else(|_| string(&value, "s"))?;
    let event_time = timestamp_any(&value, &["time", "T", "E"])?;
    let bid = number_any(&value, &["bidPrice", "b"])?;
    let ask = number_any(&value, &["askPrice", "a"])?;
    if bid <= 0.0 || ask <= bid {
        return Err(CollectorError::SchemaDrift("invalid best bid/ask".into()));
    }
    let mid = (bid + ask) / 2.0;
    let spread = ask - bid;
    Ok(observation(
        symbol,
        FuturesObservationKind::Spread,
        event_time,
        received_at,
        HashMap::from([
            ("best_bid".into(), bid),
            ("best_ask".into(), ask),
            ("bid_quantity".into(), number_any(&value, &["bidQty", "B"])?),
            ("ask_quantity".into(), number_any(&value, &["askQty", "A"])?),
            ("spread".into(), spread),
            ("spread_bps".into(), spread / mid * 10_000.0),
        ]),
    ))
}

pub fn normalize_depth(
    symbol: &str,
    payload: &str,
    received_at: DateTime<Utc>,
) -> Result<FuturesObservation, CollectorError> {
    let value = object(payload)?;
    let event_time = timestamp_any(&value, &["T", "E"])?;
    let bids = levels(&value, "bids").or_else(|_| levels(&value, "b"))?;
    let asks = levels(&value, "asks").or_else(|_| levels(&value, "a"))?;
    if bids.is_empty() || asks.is_empty() {
        return Err(CollectorError::SchemaDrift("empty depth side".into()));
    }
    let bid_liquidity = bids
        .iter()
        .take(20)
        .map(|(price, quantity)| price * quantity)
        .sum::<f64>();
    let ask_liquidity = asks
        .iter()
        .take(20)
        .map(|(price, quantity)| price * quantity)
        .sum::<f64>();
    let total = bid_liquidity + ask_liquidity;
    if total <= 0.0 {
        return Err(CollectorError::SchemaDrift("zero depth liquidity".into()));
    }
    Ok(observation(
        symbol,
        FuturesObservationKind::OrderBook,
        event_time,
        received_at,
        HashMap::from([
            ("bid_liquidity".into(), bid_liquidity),
            ("ask_liquidity".into(), ask_liquidity),
            (
                "book_imbalance".into(),
                (bid_liquidity - ask_liquidity) / total,
            ),
            (
                "depth_levels".into(),
                (bids.len().min(20) + asks.len().min(20)) as f64,
            ),
        ]),
    ))
}

pub fn normalize_24h_ticker(
    payload: &str,
    received_at: DateTime<Utc>,
) -> Result<FuturesObservation, CollectorError> {
    let value = object(payload)?;
    let symbol = string(&value, "symbol")?;
    let event_time = timestamp(&value, "closeTime")?;
    Ok(observation(
        symbol,
        FuturesObservationKind::Volume,
        event_time,
        received_at,
        HashMap::from([
            ("last_price".into(), number(&value, "lastPrice")?),
            ("volume_24h".into(), number(&value, "volume")?),
            ("quote_volume_24h".into(), number(&value, "quoteVolume")?),
        ]),
    ))
}

pub fn normalize_agg_trades(
    symbol: &str,
    payload: &str,
    received_at: DateTime<Utc>,
) -> Result<FuturesObservation, CollectorError> {
    let values: Vec<Value> =
        serde_json::from_str(payload).map_err(|error| CollectorError::Parse(error.to_string()))?;
    if values.is_empty() {
        return Err(CollectorError::SchemaDrift("empty aggregate trades".into()));
    }
    let mut volume = 0.0;
    let mut notional = 0.0;
    let mut latest = 0_i64;
    for value in &values {
        let price = number(value, "p")?;
        let quantity = number(value, "q")?;
        volume += quantity;
        notional += price * quantity;
        latest = latest.max(integer(value, "T")?);
    }
    if volume <= 0.0 || !volume.is_finite() || !notional.is_finite() {
        return Err(CollectorError::SchemaDrift(
            "invalid aggregate trade totals".into(),
        ));
    }
    Ok(observation(
        symbol,
        FuturesObservationKind::Trade,
        from_millis(latest)?,
        received_at,
        HashMap::from([
            ("trade_volume".into(), volume),
            ("trade_notional".into(), notional),
            ("trade_vwap".into(), notional / volume),
            ("trade_count".into(), values.len() as f64),
        ]),
    ))
}

pub fn normalize_futures_stream_message(
    payload: &str,
    received_at: DateTime<Utc>,
) -> Result<Vec<FuturesObservation>, CollectorError> {
    let envelope = object(payload)?;
    let value = envelope.get("data").unwrap_or(&envelope);
    let event = string(value, "e")?;
    let symbol = string(value, "s")?;
    let event_time = timestamp_any(value, &["E", "T"])?;
    match event {
        "markPriceUpdate" => Ok(vec![
            observation(
                symbol,
                FuturesObservationKind::MarkPrice,
                event_time,
                received_at,
                HashMap::from([
                    ("mark_price".into(), number(value, "p")?),
                    ("index_price".into(), number(value, "i")?),
                ]),
            ),
            observation(
                symbol,
                FuturesObservationKind::FundingRate,
                event_time,
                received_at,
                HashMap::from([
                    ("funding_rate".into(), number(value, "r")?),
                    ("next_funding_time_ms".into(), required_f64(value, "T")?),
                ]),
            ),
        ]),
        "aggTrade" => {
            let price = number(value, "p")?;
            let quantity = number(value, "q")?;
            Ok(vec![observation(
                symbol,
                FuturesObservationKind::Trade,
                event_time,
                received_at,
                HashMap::from([
                    ("trade_volume".into(), quantity),
                    ("trade_notional".into(), price * quantity),
                    ("trade_vwap".into(), price),
                    ("trade_count".into(), 1.0),
                ]),
            )])
        }
        "bookTicker" => Ok(vec![normalize_book_ticker(
            &value.to_string(),
            received_at,
        )?]),
        "depthUpdate" => Ok(vec![normalize_depth(
            symbol,
            &value.to_string(),
            received_at,
        )?]),
        other => Err(CollectorError::SchemaDrift(format!(
            "unsupported futures stream event {other}"
        ))),
    }
}

fn observation(
    symbol: &str,
    kind: FuturesObservationKind,
    exchange_event_time: DateTime<Utc>,
    received_at: DateTime<Utc>,
    metrics: HashMap<String, f64>,
) -> FuturesObservation {
    FuturesObservation {
        id: new_id("futures_observation"),
        source_id: "binance-usdm-futures".into(),
        instrument_id: canonical_futures_instrument(symbol).unwrap_or_default(),
        market_type: MarketType::UsdmPerpetual,
        external_symbol: symbol.into(),
        kind,
        exchange_event_time,
        received_at,
        schema_version: "binance.usdm.v1".into(),
        metrics,
    }
}
fn object(payload: &str) -> Result<Value, CollectorError> {
    let value: Value =
        serde_json::from_str(payload).map_err(|error| CollectorError::Parse(error.to_string()))?;
    if !value.is_object() {
        return Err(CollectorError::SchemaDrift("expected JSON object".into()));
    }
    Ok(value)
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, CollectorError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| CollectorError::SchemaDrift(format!("missing string field {key}")))
}
fn integer(value: &Value, key: &str) -> Result<i64, CollectorError> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| CollectorError::SchemaDrift(format!("missing integer field {key}")))
}
fn required_f64(value: &Value, key: &str) -> Result<f64, CollectorError> {
    let parsed = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| CollectorError::SchemaDrift(format!("missing numeric field {key}")))?;
    finite(parsed, key)
}
fn number(value: &Value, key: &str) -> Result<f64, CollectorError> {
    let parsed = value
        .get(key)
        .and_then(|item| item.as_f64().or_else(|| item.as_str()?.parse().ok()))
        .ok_or_else(|| CollectorError::SchemaDrift(format!("invalid numeric field {key}")))?;
    finite(parsed, key)
}
fn finite(value: f64, key: &str) -> Result<f64, CollectorError> {
    value
        .is_finite()
        .then_some(value)
        .ok_or_else(|| CollectorError::SchemaDrift(format!("non-finite numeric field {key}")))
}
fn number_any(value: &Value, keys: &[&str]) -> Result<f64, CollectorError> {
    keys.iter()
        .find_map(|key| number(value, key).ok())
        .ok_or_else(|| {
            CollectorError::SchemaDrift(format!("missing numeric fields {}", keys.join(",")))
        })
}
fn timestamp(value: &Value, key: &str) -> Result<DateTime<Utc>, CollectorError> {
    from_millis(integer(value, key)?)
}
fn timestamp_any(value: &Value, keys: &[&str]) -> Result<DateTime<Utc>, CollectorError> {
    keys.iter()
        .find_map(|key| integer(value, key).ok())
        .map(from_millis)
        .transpose()?
        .ok_or_else(|| CollectorError::SchemaDrift("missing exchange event time".into()))
}
fn from_millis(value: i64) -> Result<DateTime<Utc>, CollectorError> {
    DateTime::from_timestamp_millis(value)
        .ok_or_else(|| CollectorError::SchemaDrift("invalid exchange event time".into()))
}
fn levels(value: &Value, key: &str) -> Result<Vec<(f64, f64)>, CollectorError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| CollectorError::SchemaDrift(format!("missing depth {key}")))?
        .iter()
        .map(|level| {
            let values = level
                .as_array()
                .ok_or_else(|| CollectorError::SchemaDrift("invalid depth level".into()))?;
            if values.len() < 2 {
                return Err(CollectorError::SchemaDrift("short depth level".into()));
            }
            Ok((
                values[0]
                    .as_str()
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| CollectorError::SchemaDrift("invalid depth price".into()))?,
                values[1]
                    .as_str()
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| CollectorError::SchemaDrift("invalid depth quantity".into()))?,
            ))
        })
        .collect()
}

pub struct BinanceFuturesClient {
    base_url: Url,
    client: reqwest::Client,
}
impl BinanceFuturesClient {
    pub fn new(base_url: &str) -> Result<Self, CollectorError> {
        let base_url = Url::parse(base_url)
            .map_err(|error| CollectorError::InvalidSource(error.to_string()))?;
        if !matches!(base_url.scheme(), "http" | "https") {
            return Err(CollectorError::Policy(
                "futures REST requires HTTP(S)".into(),
            ));
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("OrdinConn/0.1 public Binance USD-M collector")
            .build()
            .map_err(|error| CollectorError::Request(error.to_string()))?;
        Ok(Self { base_url, client })
    }
    pub async fn fetch_symbol(
        &self,
        symbol: &str,
    ) -> Result<Vec<FuturesObservation>, CollectorError> {
        canonical_futures_instrument(symbol)?;
        let received = Utc::now();
        let premium = self
            .get("/fapi/v1/premiumIndex", &[("symbol", symbol)])
            .await?;
        let oi = self
            .get("/fapi/v1/openInterest", &[("symbol", symbol)])
            .await?;
        let book = self
            .get("/fapi/v1/ticker/bookTicker", &[("symbol", symbol)])
            .await?;
        let depth = self
            .get("/fapi/v1/depth", &[("symbol", symbol), ("limit", "20")])
            .await?;
        let ticker = self
            .get("/fapi/v1/ticker/24hr", &[("symbol", symbol)])
            .await?;
        let trades = self
            .get(
                "/fapi/v1/aggTrades",
                &[("symbol", symbol), ("limit", "100")],
            )
            .await?;
        let mut result = normalize_premium_index(&premium, received)?;
        result.push(normalize_open_interest(&oi, received)?);
        result.push(normalize_book_ticker(&book, received)?);
        result.push(normalize_depth(symbol, &depth, received)?);
        result.push(normalize_24h_ticker(&ticker, received)?);
        result.push(normalize_agg_trades(symbol, &trades, received)?);
        Ok(result)
    }
    async fn get(&self, path: &str, query: &[(&str, &str)]) -> Result<String, CollectorError> {
        let url = self
            .base_url
            .join(path)
            .map_err(|error| CollectorError::InvalidSource(error.to_string()))?;
        let response = self
            .client
            .get(url)
            .query(query)
            .send()
            .await
            .map_err(|error| CollectorError::Request(error.to_string()))?;
        if !response.status().is_success() {
            return Err(CollectorError::Request(format!(
                "Binance USD-M HTTP {}",
                response.status()
            )));
        }
        response
            .text()
            .await
            .map_err(|error| CollectorError::Request(error.to_string()))
    }
    pub fn base_url(&self) -> String {
        canonicalize_url(self.base_url.as_str()).unwrap_or_else(|_| self.base_url.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;

    fn received() -> chrono::DateTime<Utc> {
        Utc.timestamp_millis_opt(1_700_000_000_500).unwrap()
    }

    #[test]
    fn canonical_ids_keep_spot_and_perpetual_separate_and_reject_other_symbols() {
        assert_eq!(
            canonical_futures_instrument("BTCUSDT").unwrap(),
            "BTC-USDT-PERP"
        );
        assert_eq!(
            canonical_spot_instrument("BTCUSDT").unwrap(),
            "BTC-USDT-SPOT"
        );
        assert_ne!(
            canonical_futures_instrument("BTCUSDT").unwrap(),
            canonical_spot_instrument("BTCUSDT").unwrap()
        );
        assert!(canonical_futures_instrument("BNBUSDT").is_err());
    }

    #[test]
    fn premium_index_yields_mark_price_and_funding_with_exchange_time() {
        let observations = normalize_premium_index(r#"{"symbol":"BTCUSDT","markPrice":"60001.5","indexPrice":"60000.0","lastFundingRate":"0.0001","nextFundingTime":1700006400000,"time":1700000000000}"#, received()).unwrap();
        assert_eq!(observations.len(), 2);
        assert_eq!(observations[0].kind, FuturesObservationKind::MarkPrice);
        assert_eq!(observations[0].metrics["mark_price"], 60001.5);
        assert_eq!(observations[1].kind, FuturesObservationKind::FundingRate);
        assert_eq!(observations[1].metrics["funding_rate"], 0.0001);
        assert_eq!(
            observations[0].exchange_event_time.timestamp_millis(),
            1_700_000_000_000
        );
    }

    #[test]
    fn open_interest_book_depth_ticker_and_trades_normalize_required_metrics() {
        let oi = normalize_open_interest(
            r#"{"symbol":"ETHUSDT","openInterest":"12345.67","time":1700000000000}"#,
            received(),
        )
        .unwrap();
        assert_eq!(oi.metrics["open_interest"], 12345.67);
        let spread = normalize_book_ticker(r#"{"symbol":"ETHUSDT","bidPrice":"3000.0","bidQty":"5","askPrice":"3000.6","askQty":"4","time":1700000000000}"#, received()).unwrap();
        assert!((spread.metrics["spread_bps"] - 1.9998).abs() < 0.001);
        let depth = normalize_depth("ETHUSDT", r#"{"lastUpdateId":1,"E":1700000000100,"T":1700000000000,"bids":[["3000","2"],["2999","3"]],"asks":[["3001","1"],["3002","1"]]}"#, received()).unwrap();
        assert!(depth.metrics["book_imbalance"] > 0.4);
        let ticker = normalize_24h_ticker(r#"{"symbol":"SOLUSDT","lastPrice":"150","volume":"100000","quoteVolume":"15000000","closeTime":1700000000000}"#, received()).unwrap();
        assert_eq!(ticker.metrics["volume_24h"], 100000.0);
        let trade = normalize_agg_trades("SOLUSDT", r#"[{"a":1,"p":"150","q":"2","T":1700000000000,"m":true},{"a":2,"p":"151","q":"3","T":1700000000200,"m":false}]"#, received()).unwrap();
        assert_eq!(trade.metrics["trade_volume"], 5.0);
        assert_eq!(trade.metrics["trade_count"], 2.0);
    }

    #[test]
    fn malformed_or_missing_exchange_time_fails_closed() {
        assert!(
            normalize_open_interest(r#"{"symbol":"BTCUSDT","openInterest":"100"}"#, received())
                .is_err()
        );
        assert!(
            normalize_depth(
                "BTCUSDT",
                r#"{"T":1700000000000,"bids":[],"asks":[]}"#,
                received()
            )
            .is_err()
        );
    }

    #[test]
    fn websocket_market_events_normalize_without_received_time_substitution() {
        let mark=normalize_futures_stream_message(r#"{"e":"markPriceUpdate","E":1700000000000,"s":"BTCUSDT","p":"60001","i":"60000","r":"0.0001","T":1700006400000}"#,received()).unwrap();
        assert_eq!(mark.len(), 2);
        assert_eq!(
            mark[0].exchange_event_time.timestamp_millis(),
            1_700_000_000_000
        );
        assert_ne!(mark[0].exchange_event_time, mark[0].received_at);
        let trade=normalize_futures_stream_message(r#"{"e":"aggTrade","E":1700000000200,"s":"SOLUSDT","p":"150","q":"2","T":1700000000100}"#,received()).unwrap();
        assert_eq!(trade[0].metrics["trade_notional"], 300.0);
    }

    #[tokio::test]
    async fn websocket_subscribes_forwards_data_and_awaits_shutdown() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            let subscription = socket.next().await.unwrap().unwrap();
            assert!(subscription.into_text().unwrap().contains("SUBSCRIBE"));
            socket
                .send(Message::Text(r#"{"e":"markPriceUpdate"}"#.into()))
                .await
                .unwrap();
            while let Some(Ok(message)) = socket.next().await {
                if message.is_close() {
                    break;
                }
            }
        });
        let stream = BinanceFuturesWebSocket::new(
            &format!("ws://{address}"),
            vec!["btcusdt@markPrice@1s".into()],
            std::time::Duration::from_secs(2),
        )
        .unwrap();
        let mut events = stream.subscribe();
        stream.start().await.unwrap();
        let mut saw_data = false;
        for _ in 0..4 {
            if matches!(
                tokio::time::timeout(std::time::Duration::from_secs(1), events.recv())
                    .await
                    .unwrap()
                    .unwrap(),
                FuturesStreamEvent::Data(_)
            ) {
                saw_data = true;
                break;
            }
        }
        assert!(saw_data);
        stream.shutdown().await;
        server.await.unwrap();
        let diagnostics = stream.diagnostics().await;
        assert_eq!(diagnostics.connections, 1);
        assert_eq!(diagnostics.subscriptions, 1);
        assert_eq!(diagnostics.messages, 1);
    }

    #[tokio::test]
    async fn stale_connection_reconnects_and_resubscribes() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for connection in 0..2 {
                let (stream, _) = listener.accept().await.unwrap();
                let mut socket = accept_async(stream).await.unwrap();
                let subscription = socket.next().await.unwrap().unwrap();
                assert!(subscription.into_text().unwrap().contains("SUBSCRIBE"));
                if connection == 1 {
                    socket
                        .send(Message::Text(r#"{"e":"aggTrade"}"#.into()))
                        .await
                        .unwrap();
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
        let stream = BinanceFuturesWebSocket::new(
            &format!("ws://{address}"),
            vec!["btcusdt@aggTrade".into()],
            std::time::Duration::from_millis(50),
        )
        .unwrap();
        stream.start().await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if stream.diagnostics().await.messages >= 1 {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();
        stream.shutdown().await;
        server.await.unwrap();
        let diagnostics = stream.diagnostics().await;
        assert!(diagnostics.connections >= 2);
        assert!(diagnostics.subscriptions >= 2);
        assert!(diagnostics.reconnects >= 1);
        assert!(diagnostics.stale_disconnects >= 1);
    }
}
