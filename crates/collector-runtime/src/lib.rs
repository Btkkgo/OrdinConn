use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use evidence_core::ReliabilityTier;
use feed_rs::parser;
use futures_util::StreamExt;
use market_core::{AssetRef, Market, new_id};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    time::Duration as StdDuration,
};
use thiserror::Error;
use tokio::sync::{Mutex, watch};
use tokio_tungstenite::connect_async;
use url::Url;

pub mod scheduler;
pub use scheduler::*;
pub mod binance_futures;
pub use binance_futures::*;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectorKind {
    Rest,
    WebSocket,
    Rss,
    Html,
    Browser,
    Computer,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceClass {
    Exchange,
    Regulator,
    CentralBank,
    Company,
    Blockchain,
    Industry,
    Search,
    Social,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthRequirement {
    None,
    UserConfiguredApiKey,
    Login,
    Paywall,
    Captcha,
    Cookie,
    PrivateAccess,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollPolicy {
    pub interval_seconds: u64,
    pub timeout_seconds: u64,
    pub max_retries: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitPolicy {
    pub requests: u32,
    pub per_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicy {
    pub raw_days: u32,
    pub observation_days: u32,
    pub audit_days: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDefinition {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub collector_kind: CollectorKind,
    pub classification: SourceClass,
    pub capabilities: Vec<String>,
    pub reliability_tier: ReliabilityTier,
    pub poll: PollPolicy,
    pub rate_limit: RateLimitPolicy,
    pub auth: AuthRequirement,
    pub retention: RetentionPolicy,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Unknown,
    Healthy,
    Degraded,
    Unavailable,
    SchemaDrift,
    Stopped,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorHealth {
    pub status: HealthStatus,
    pub consecutive_failures: u32,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub schema_version: Option<String>,
}

impl Default for CollectorHealth {
    fn default() -> Self {
        Self {
            status: HealthStatus::Unknown,
            consecutive_failures: 0,
            last_success_at: None,
            last_error: None,
            schema_version: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error("source is outside the public-data policy: {0}")]
    Policy(String),
    #[error("invalid source definition: {0}")]
    InvalidSource(String),
    #[error("request failed: {0}")]
    Request(String),
    #[error("unsupported collector operation")]
    Unsupported,
    #[error("schema drift: {0}")]
    SchemaDrift(String),
    #[error("parse failed: {0}")]
    Parse(String),
}

impl SourceDefinition {
    pub fn validate_public(&self) -> Result<(), CollectorError> {
        let url =
            Url::parse(&self.endpoint).map_err(|e| CollectorError::InvalidSource(e.to_string()))?;
        if !matches!(url.scheme(), "http" | "https" | "ws" | "wss") {
            return Err(CollectorError::Policy(
                "only public HTTP(S)/WS(S) endpoints are allowed".into(),
            ));
        }
        if matches!(
            self.auth,
            AuthRequirement::Login
                | AuthRequirement::Paywall
                | AuthRequirement::Captcha
                | AuthRequirement::Cookie
                | AuthRequirement::PrivateAccess
        ) {
            return Err(CollectorError::Policy(
                "login, paywall, CAPTCHA, cookie, and private access are prohibited".into(),
            ));
        }
        if self.poll.interval_seconds == 0
            || self.rate_limit.requests == 0
            || self.rate_limit.per_seconds == 0
        {
            return Err(CollectorError::InvalidSource(
                "poll and rate limits must be positive".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawRecord {
    pub id: String,
    pub source_id: String,
    pub external_id: Option<String>,
    pub canonical_url: Option<String>,
    pub retrieved_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub content_type: String,
    pub content_hash: String,
    pub title: Option<String>,
    pub raw_payload_ref: String,
    pub payload: String,
}

impl RawRecord {
    pub fn new(
        source_id: impl Into<String>,
        payload: impl Into<String>,
        content_type: impl Into<String>,
        url: Option<&str>,
    ) -> Result<Self, CollectorError> {
        let payload = payload.into();
        let hash = hex_sha256(payload.as_bytes());
        Ok(Self {
            id: new_id("raw"),
            source_id: source_id.into(),
            external_id: None,
            canonical_url: url.map(canonicalize_url).transpose()?,
            retrieved_at: Utc::now(),
            published_at: None,
            content_type: content_type.into(),
            content_hash: hash.clone(),
            title: None,
            raw_payload_ref: format!("sha256:{hash}"),
            payload,
        })
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn canonicalize_url(value: &str) -> Result<String, CollectorError> {
    let mut url = Url::parse(value).map_err(|e| CollectorError::Parse(e.to_string()))?;
    url.set_fragment(None);
    if url.path().len() > 1 && url.path().ends_with('/') {
        let trimmed = url.path().trim_end_matches('/').to_string();
        url.set_path(&trimmed);
    }
    let filtered: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| !k.starts_with("utm_") && k != "ref")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    url.set_query(None);
    if !filtered.is_empty() {
        url.query_pairs_mut().extend_pairs(filtered);
    }
    Ok(url.to_string().trim_end_matches('/').to_string())
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    PriceVolume,
    Volatility,
    Breadth,
    Macro,
    Event,
    Demand,
    Onchain,
    Funding,
    OpenInterest,
    OrderBook,
    Spread,
    Narrative,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedObservation {
    pub id: String,
    pub raw_record_id: String,
    pub source_id: String,
    pub kind: ObservationKind,
    pub asset: AssetRef,
    pub entity_id: Option<String>,
    pub observed_at: DateTime<Utc>,
    pub schema_version: String,
    pub metrics: HashMap<String, f64>,
    pub facts: Vec<String>,
    pub completeness: f64,
    pub stale: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DuplicateReason {
    ExternalId,
    CanonicalUrl,
    ContentHash,
    TitleTime,
}

#[derive(Default)]
pub struct Deduplicator {
    records: Vec<RawRecord>,
}

impl Deduplicator {
    pub fn insert(&mut self, record: RawRecord) -> Result<(), DuplicateReason> {
        for existing in &self.records {
            if record.source_id == existing.source_id
                && record.external_id.is_some()
                && record.external_id == existing.external_id
            {
                return Err(DuplicateReason::ExternalId);
            }
            if record.canonical_url.is_some() && record.canonical_url == existing.canonical_url {
                return Err(DuplicateReason::CanonicalUrl);
            }
            if record.content_hash == existing.content_hash {
                return Err(DuplicateReason::ContentHash);
            }
            if title_time_match(&record, existing) {
                return Err(DuplicateReason::TitleTime);
            }
        }
        self.records.push(record);
        Ok(())
    }
}

fn title_time_match(a: &RawRecord, b: &RawRecord) -> bool {
    let (Some(a_title), Some(b_title), Some(a_time), Some(b_time)) =
        (&a.title, &b.title, a.published_at, b.published_at)
    else {
        return false;
    };
    let a_words: HashSet<_> = a_title
        .to_lowercase()
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let b_words: HashSet<_> = b_title
        .to_lowercase()
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let union = a_words.union(&b_words).count().max(1);
    let similarity = a_words.intersection(&b_words).count() as f64 / union as f64;
    similarity >= 0.8 && (a_time - b_time).num_hours().abs() <= 24
}

#[derive(Default)]
pub struct SourceRegistry {
    sources: HashMap<String, SourceDefinition>,
}

impl SourceRegistry {
    pub fn register(&mut self, source: SourceDefinition) -> Result<(), CollectorError> {
        source.validate_public()?;
        if self.sources.contains_key(&source.id) {
            return Err(CollectorError::InvalidSource("duplicate source id".into()));
        }
        self.sources.insert(source.id.clone(), source);
        Ok(())
    }
    pub fn get(&self, id: &str) -> Option<&SourceDefinition> {
        self.sources.get(id)
    }
    pub fn all(&self) -> Vec<&SourceDefinition> {
        let mut items: Vec<_> = self.sources.values().collect();
        items.sort_by_key(|s| &s.id);
        items
    }
}

pub struct RateLimiter {
    policy: RateLimitPolicy,
    timestamps: Mutex<Vec<DateTime<Utc>>>,
}
impl RateLimiter {
    pub fn new(policy: RateLimitPolicy) -> Self {
        Self {
            policy,
            timestamps: Mutex::new(vec![]),
        }
    }
    pub async fn acquire(&self, now: DateTime<Utc>) -> Result<(), CollectorError> {
        let mut timestamps = self.timestamps.lock().await;
        let cutoff = now - Duration::seconds(self.policy.per_seconds as i64);
        timestamps.retain(|t| *t > cutoff);
        if timestamps.len() >= self.policy.requests as usize {
            return Err(CollectorError::Request(
                "rate limit budget exhausted".into(),
            ));
        }
        timestamps.push(now);
        Ok(())
    }
}

#[async_trait]
pub trait Collector: Send + Sync {
    fn source(&self) -> &SourceDefinition;
    async fn start(&self) -> Result<(), CollectorError>;
    async fn stop(&self);
    async fn fetch_once(&self) -> Result<Vec<RawRecord>, CollectorError>;
    async fn subscribe(&self) -> Result<Vec<RawRecord>, CollectorError> {
        Err(CollectorError::Unsupported)
    }
    async fn health(&self) -> CollectorHealth;
}

pub struct HttpCollector {
    source: SourceDefinition,
    client: reqwest::Client,
    health: Mutex<CollectorHealth>,
    stopped: watch::Sender<bool>,
    limiter: RateLimiter,
}

impl HttpCollector {
    pub fn new(source: SourceDefinition) -> Result<Self, CollectorError> {
        source.validate_public()?;
        let client = reqwest::Client::builder()
            .timeout(StdDuration::from_secs(source.poll.timeout_seconds))
            .redirect(reqwest::redirect::Policy::limited(3))
            .user_agent("OrdinConn/0.1 public-data collector")
            .build()
            .map_err(|e| CollectorError::Request(e.to_string()))?;
        let (stopped, _) = watch::channel(false);
        Ok(Self {
            limiter: RateLimiter::new(source.rate_limit.clone()),
            source,
            client,
            health: Mutex::new(CollectorHealth::default()),
            stopped,
        })
    }
    async fn mark_success(&self) {
        let mut h = self.health.lock().await;
        h.status = HealthStatus::Healthy;
        h.consecutive_failures = 0;
        h.last_success_at = Some(Utc::now());
        h.last_error = None;
    }
    async fn mark_failure(&self, message: String) {
        let mut h = self.health.lock().await;
        h.consecutive_failures += 1;
        h.status = if h.consecutive_failures > 2 {
            HealthStatus::Unavailable
        } else {
            HealthStatus::Degraded
        };
        h.last_error = Some(message);
    }
}

#[async_trait]
impl Collector for HttpCollector {
    fn source(&self) -> &SourceDefinition {
        &self.source
    }
    async fn start(&self) -> Result<(), CollectorError> {
        self.source.validate_public()
    }
    async fn stop(&self) {
        let _ = self.stopped.send(true);
        self.health.lock().await.status = HealthStatus::Stopped;
    }
    async fn fetch_once(&self) -> Result<Vec<RawRecord>, CollectorError> {
        self.limiter.acquire(Utc::now()).await?;
        let mut delay = 100_u64;
        for attempt in 0..=self.source.poll.max_retries {
            let result = self.client.get(&self.source.endpoint).send().await;
            match result {
                Ok(response) if response.status().is_success() => {
                    let final_url = response.url().as_str().to_string();
                    if final_url.contains("/login") || final_url.contains("/signin") {
                        return Err(CollectorError::Policy(
                            "redirected to authentication".into(),
                        ));
                    }
                    let content_type = response
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("application/octet-stream")
                        .to_string();
                    let body = response
                        .text()
                        .await
                        .map_err(|e| CollectorError::Request(e.to_string()))?;
                    self.mark_success().await;
                    return Ok(vec![RawRecord::new(
                        &self.source.id,
                        body,
                        content_type,
                        Some(&final_url),
                    )?]);
                }
                Ok(response)
                    if response.status().as_u16() == 429
                        && attempt < self.source.poll.max_retries =>
                {
                    let retry_after = response
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(0);
                    tokio::time::sleep(StdDuration::from_millis((retry_after * 1000).max(delay)))
                        .await;
                    delay = (delay * 2).min(2_000);
                }
                Ok(response) => {
                    let msg = format!("HTTP {}", response.status());
                    self.mark_failure(msg.clone()).await;
                    return Err(CollectorError::Request(msg));
                }
                Err(error) if attempt < self.source.poll.max_retries => {
                    tokio::time::sleep(StdDuration::from_millis(delay)).await;
                    delay = (delay * 2).min(2_000);
                    let _ = error;
                }
                Err(error) => {
                    let msg = error.to_string();
                    self.mark_failure(msg.clone()).await;
                    return Err(CollectorError::Request(msg));
                }
            }
        }
        Err(CollectorError::Request("retry budget exhausted".into()))
    }
    async fn health(&self) -> CollectorHealth {
        self.health.lock().await.clone()
    }
}

pub struct WebSocketCollector {
    source: SourceDefinition,
    health: Mutex<CollectorHealth>,
}
impl WebSocketCollector {
    pub fn new(source: SourceDefinition) -> Result<Self, CollectorError> {
        source.validate_public()?;
        Ok(Self {
            source,
            health: Mutex::new(CollectorHealth::default()),
        })
    }
}
#[async_trait]
impl Collector for WebSocketCollector {
    fn source(&self) -> &SourceDefinition {
        &self.source
    }
    async fn start(&self) -> Result<(), CollectorError> {
        self.source.validate_public()
    }
    async fn stop(&self) {
        self.health.lock().await.status = HealthStatus::Stopped;
    }
    async fn fetch_once(&self) -> Result<Vec<RawRecord>, CollectorError> {
        self.subscribe().await
    }
    async fn subscribe(&self) -> Result<Vec<RawRecord>, CollectorError> {
        let (mut stream, _) = connect_async(&self.source.endpoint)
            .await
            .map_err(|e| CollectorError::Request(e.to_string()))?;
        while let Some(message) = stream.next().await {
            let message = message.map_err(|e| CollectorError::Request(e.to_string()))?;
            if message.is_text() {
                self.health.lock().await.status = HealthStatus::Healthy;
                return Ok(vec![RawRecord::new(
                    &self.source.id,
                    message
                        .into_text()
                        .map_err(|e| CollectorError::Parse(e.to_string()))?
                        .to_string(),
                    "application/json",
                    Some(&self.source.endpoint),
                )?]);
            }
        }
        Err(CollectorError::Request(
            "stream closed before a data frame".into(),
        ))
    }
    async fn health(&self) -> CollectorHealth {
        self.health.lock().await.clone()
    }
}

pub trait BrowserCollector: Send + Sync {
    fn source(&self) -> &SourceDefinition;
}
pub trait ComputerCollector: Send + Sync {
    fn source(&self) -> &SourceDefinition;
}

pub struct FeedCollector {
    http: HttpCollector,
}

impl FeedCollector {
    pub fn new(source: SourceDefinition) -> Result<Self, CollectorError> {
        if source.collector_kind != CollectorKind::Rss {
            return Err(CollectorError::InvalidSource(
                "feed collector requires rss kind".into(),
            ));
        }
        Ok(Self {
            http: HttpCollector::new(source)?,
        })
    }
}

#[async_trait]
impl Collector for FeedCollector {
    fn source(&self) -> &SourceDefinition {
        self.http.source()
    }
    async fn start(&self) -> Result<(), CollectorError> {
        self.http.start().await
    }
    async fn stop(&self) {
        self.http.stop().await
    }
    async fn fetch_once(&self) -> Result<Vec<RawRecord>, CollectorError> {
        let documents = self.http.fetch_once().await?;
        let mut entries = Vec::new();
        for document in &documents {
            entries.extend(parse_feed(document)?);
        }
        Ok(entries)
    }
    async fn health(&self) -> CollectorHealth {
        self.http.health().await
    }
}

pub struct HtmlArticleCollector {
    http: HttpCollector,
}

impl HtmlArticleCollector {
    pub fn new(source: SourceDefinition) -> Result<Self, CollectorError> {
        if source.collector_kind != CollectorKind::Html {
            return Err(CollectorError::InvalidSource(
                "article collector requires html kind".into(),
            ));
        }
        Ok(Self {
            http: HttpCollector::new(source)?,
        })
    }
}

#[async_trait]
impl Collector for HtmlArticleCollector {
    fn source(&self) -> &SourceDefinition {
        self.http.source()
    }
    async fn start(&self) -> Result<(), CollectorError> {
        self.http.start().await
    }
    async fn stop(&self) {
        self.http.stop().await
    }
    async fn fetch_once(&self) -> Result<Vec<RawRecord>, CollectorError> {
        let mut records = self.http.fetch_once().await?;
        for record in &mut records {
            let article = extract_article(record)?;
            record.title = Some(article.title);
            record.payload = article.content;
        }
        Ok(records)
    }
    async fn health(&self) -> CollectorHealth {
        self.http.health().await
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebsiteWatch {
    pub id: String,
    pub source_id: String,
    pub url: String,
    pub selector: Option<String>,
    pub last_content_hash: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletWatch {
    pub id: String,
    pub chain: String,
    pub address_hash: String,
    pub label: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[async_trait]
pub trait SearchProvider: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, CollectorError>;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchTask {
    pub id: String,
    pub query: String,
    pub reason: String,
    pub discovered_urls: Vec<String>,
}

pub struct DiscoveryAgent<P: SearchProvider> {
    provider: P,
}
impl<P: SearchProvider> DiscoveryAgent<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
    pub async fn discover(
        &self,
        query: &str,
        reason: &str,
    ) -> Result<ResearchTask, CollectorError> {
        let results = self.provider.search(query).await?;
        let discovered_urls = results
            .into_iter()
            .filter_map(|item| {
                Url::parse(&item.url)
                    .ok()
                    .filter(|url| matches!(url.scheme(), "http" | "https"))
                    .map(|url| url.to_string())
            })
            .collect();
        Ok(ResearchTask {
            id: new_id("research_task"),
            query: query.into(),
            reason: reason.into(),
            discovered_urls,
        })
    }
}

pub fn parse_feed(record: &RawRecord) -> Result<Vec<RawRecord>, CollectorError> {
    if let Ok(feed) = parser::parse(record.payload.as_bytes()) {
        return feed
            .entries
            .into_iter()
            .map(|entry| {
                let payload = entry
                    .content
                    .and_then(|c| c.body)
                    .or_else(|| entry.summary.map(|s| s.content))
                    .unwrap_or_default();
                let mut item = RawRecord::new(
                    &record.source_id,
                    payload,
                    "text/html",
                    entry.links.first().map(|l| l.href.as_str()),
                )?;
                item.external_id = Some(entry.id);
                item.title = entry.title.map(|t| t.content);
                item.published_at = entry.published.or(entry.updated);
                Ok(item)
            })
            .collect();
    }
    let value: Value = serde_json::from_str(&record.payload)
        .map_err(|error| CollectorError::Parse(format!("unsupported feed: {error}")))?;
    let items = value
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| CollectorError::SchemaDrift("JSON Feed is missing items".into()))?;
    items
        .iter()
        .map(|entry| {
            let id = required_str(entry, "id")?;
            let payload = entry
                .get("content_html")
                .or_else(|| entry.get("content_text"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let url = entry.get("url").and_then(Value::as_str);
            let mut item = RawRecord::new(&record.source_id, payload, "text/html", url)?;
            item.external_id = Some(id.into());
            item.title = entry
                .get("title")
                .and_then(Value::as_str)
                .map(str::to_owned);
            item.published_at = entry
                .get("date_published")
                .and_then(Value::as_str)
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&Utc));
            Ok(item)
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Article {
    pub title: String,
    pub content: String,
    pub canonical_url: Option<String>,
}

pub fn extract_article(record: &RawRecord) -> Result<Article, CollectorError> {
    let document = Html::parse_document(&record.payload);
    let title_selector =
        Selector::parse("title").map_err(|e| CollectorError::Parse(e.to_string()))?;
    let article_selector =
        Selector::parse("article, main").map_err(|e| CollectorError::Parse(e.to_string()))?;
    let title = document
        .select(&title_selector)
        .next()
        .map(|n| n.text().collect::<Vec<_>>().join(" ").trim().to_string())
        .unwrap_or_default();
    let content = document
        .select(&article_selector)
        .next()
        .map(|n| {
            n.text()
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    if title.is_empty() || content.is_empty() {
        return Err(CollectorError::SchemaDrift(
            "expected title and article/main content".into(),
        ));
    }
    Ok(Article {
        title,
        content,
        canonical_url: record.canonical_url.clone(),
    })
}

pub fn normalize_binance_24h(
    record: &RawRecord,
) -> Result<Vec<NormalizedObservation>, CollectorError> {
    let values: Vec<Value> = serde_json::from_str(&record.payload)
        .or_else(|_| serde_json::from_str::<Value>(&record.payload).map(|v| vec![v]))
        .map_err(|e| CollectorError::Parse(e.to_string()))?;
    values
        .into_iter()
        .map(|value| {
            let symbol = required_str(&value, "symbol")?;
            let price = required_number(&value, "lastPrice")?;
            let volume = required_number(&value, "volume")?;
            let quote_volume = required_number(&value, "quoteVolume")?;
            let canonical = canonical_spot_instrument(symbol)?;
            Ok(NormalizedObservation {
                id: new_id("observation"),
                raw_record_id: record.id.clone(),
                source_id: record.source_id.clone(),
                kind: ObservationKind::PriceVolume,
                asset: AssetRef::new(Market::Crypto, canonical),
                entity_id: None,
                observed_at: record.retrieved_at,
                schema_version: "binance.24hr.v1".into(),
                metrics: HashMap::from([
                    ("last_price".into(), price),
                    ("base_volume_24h".into(), volume),
                    ("quote_volume_24h".into(), quote_volume),
                ]),
                facts: vec![format!(
                    "{symbol} 24h ticker observed from Binance public market data"
                )],
                completeness: 1.0,
                stale: false,
            })
        })
        .collect()
}

pub fn normalize_mempool(record: &RawRecord) -> Result<NormalizedObservation, CollectorError> {
    let value: Value =
        serde_json::from_str(&record.payload).map_err(|e| CollectorError::Parse(e.to_string()))?;
    let count = required_number(&value, "count")?;
    let vsize = required_number(&value, "vsize")?;
    let total_fee = required_number(&value, "total_fee")?;
    Ok(NormalizedObservation {
        id: new_id("observation"),
        raw_record_id: record.id.clone(),
        source_id: record.source_id.clone(),
        kind: ObservationKind::Onchain,
        asset: AssetRef::new(Market::Crypto, "BTC"),
        entity_id: None,
        observed_at: record.retrieved_at,
        schema_version: "mempool.summary.v1".into(),
        metrics: HashMap::from([
            ("transaction_count".into(), count),
            ("virtual_size".into(), vsize),
            ("total_fee_sats".into(), total_fee),
        ]),
        facts: vec!["Bitcoin public mempool summary observed".into()],
        completeness: 1.0,
        stale: false,
    })
}

pub fn observation_from_feed_entry(
    record: &RawRecord,
    kind: ObservationKind,
    asset: AssetRef,
    entity_id: Option<String>,
) -> NormalizedObservation {
    NormalizedObservation {
        id: new_id("observation"),
        raw_record_id: record.id.clone(),
        source_id: record.source_id.clone(),
        kind,
        asset,
        entity_id,
        observed_at: record.published_at.unwrap_or(record.retrieved_at),
        schema_version: "feed.entry.v1".into(),
        metrics: HashMap::new(),
        facts: vec![
            record.title.clone().unwrap_or_default(),
            record.payload.clone(),
        ]
        .into_iter()
        .filter(|v| !v.is_empty())
        .collect(),
        completeness: if record.title.is_some() && !record.payload.is_empty() {
            1.0
        } else {
            0.5
        },
        stale: false,
    }
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, CollectorError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| CollectorError::SchemaDrift(format!("missing string field {key}")))
}
fn required_number(value: &Value, key: &str) -> Result<f64, CollectorError> {
    let v = value
        .get(key)
        .ok_or_else(|| CollectorError::SchemaDrift(format!("missing numeric field {key}")))?;
    v.as_f64()
        .or_else(|| v.as_str()?.parse().ok())
        .ok_or_else(|| CollectorError::SchemaDrift(format!("invalid numeric field {key}")))
}

pub fn canonical_crypto_pair(symbol: &str) -> Result<String, CollectorError> {
    let upper = symbol.to_ascii_uppercase().replace(['-', '/', '_'], "");
    for quote in ["USDT", "USDC", "USD", "BTC", "ETH"] {
        if upper.ends_with(quote) && upper.len() > quote.len() {
            return Ok(format!("{}-{quote}", &upper[..upper.len() - quote.len()]));
        }
    }
    Err(CollectorError::SchemaDrift(format!(
        "unknown crypto instrument {symbol}"
    )))
}

pub fn schema_fingerprint(value: &Value) -> String {
    fn shape(value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut keys: Vec<_> = map.keys().collect();
                keys.sort();
                Value::Object(
                    keys.into_iter()
                        .map(|k| (k.clone(), shape(&map[k])))
                        .collect::<Map<_, _>>(),
                )
            }
            Value::Array(items) => Value::Array(items.first().map(shape).into_iter().collect()),
            Value::Null => Value::String("null".into()),
            Value::Bool(_) => Value::String("bool".into()),
            Value::Number(_) => Value::String("number".into()),
            Value::String(_) => Value::String("string".into()),
        }
    }
    hex_sha256(
        serde_json::to_string(&shape(value))
            .unwrap_or_default()
            .as_bytes(),
    )
}

pub fn builtin_sources() -> Vec<SourceDefinition> {
    let poll = |interval| PollPolicy {
        interval_seconds: interval,
        timeout_seconds: 15,
        max_retries: 2,
    };
    let rate = RateLimitPolicy {
        requests: 5,
        per_seconds: 1,
    };
    let retention = RetentionPolicy {
        raw_days: 7,
        observation_days: 365,
        audit_days: None,
    };
    vec![
        SourceDefinition { id: "binance-spot-24h".into(), name: "Binance Public Spot 24h".into(), endpoint: "https://data-api.binance.vision/api/v3/ticker/24hr?symbols=%5B%22BTCUSDT%22,%22ETHUSDT%22,%22SOLUSDT%22%5D".into(), collector_kind: CollectorKind::Rest, classification: SourceClass::Exchange, capabilities: vec!["price".into(), "volume".into()], reliability_tier: ReliabilityTier::Tier1Official, poll: poll(60), rate_limit: rate.clone(), auth: AuthRequirement::None, retention: retention.clone(), enabled: true },
        SourceDefinition { id: "binance-usdm-futures".into(), name: "Binance USD-M Futures Public Market Data".into(), endpoint: "https://fapi.binance.com".into(), collector_kind: CollectorKind::Rest, classification: SourceClass::Exchange, capabilities: vec!["mark_price".into(), "funding_rate".into(), "open_interest".into(), "agg_trade".into(), "best_bid_ask".into(), "limited_depth".into(), "ticker_24h".into(), "public_only".into()], reliability_tier: ReliabilityTier::Tier1Official, poll: poll(60), rate_limit: rate.clone(), auth: AuthRequirement::None, retention: retention.clone(), enabled: true },
        SourceDefinition { id: "binance-btc-trade-stream".into(), name: "Binance Public BTC Trade Stream".into(), endpoint: "wss://data-stream.binance.vision/ws/btcusdt@trade".into(), collector_kind: CollectorKind::WebSocket, classification: SourceClass::Exchange, capabilities: vec!["streaming".into(), "trades".into()], reliability_tier: ReliabilityTier::Tier1Official, poll: poll(1), rate_limit: rate.clone(), auth: AuthRequirement::None, retention: retention.clone(), enabled: true },
        SourceDefinition { id: "federal-reserve-press".into(), name: "Federal Reserve Press Releases".into(), endpoint: "https://www.federalreserve.gov/feeds/press_all.xml".into(), collector_kind: CollectorKind::Rss, classification: SourceClass::CentralBank, capabilities: vec!["official_events".into()], reliability_tier: ReliabilityTier::Tier1Official, poll: poll(900), rate_limit: rate.clone(), auth: AuthRequirement::None, retention: retention.clone(), enabled: true },
        SourceDefinition { id: "nvidia-newsroom".into(), name: "NVIDIA Newsroom".into(), endpoint: "https://nvidianews.nvidia.com/releases.xml".into(), collector_kind: CollectorKind::Rss, classification: SourceClass::Company, capabilities: vec!["company_events".into(), "demand_observations".into()], reliability_tier: ReliabilityTier::Tier1Official, poll: poll(1800), rate_limit: rate.clone(), auth: AuthRequirement::None, retention: retention.clone(), enabled: true },
        SourceDefinition { id: "mempool-space-summary".into(), name: "mempool.space Public Mempool".into(), endpoint: "https://mempool.space/api/mempool".into(), collector_kind: CollectorKind::Rest, classification: SourceClass::Blockchain, capabilities: vec!["btc_mempool".into(), "onchain".into()], reliability_tier: ReliabilityTier::Tier2Primary, poll: poll(60), rate_limit: rate, auth: AuthRequirement::None, retention, enabled: true },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::SinkExt;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    use tokio_tungstenite::{accept_async, tungstenite::Message};

    fn source(auth: AuthRequirement) -> SourceDefinition {
        SourceDefinition {
            id: "s".into(),
            name: "S".into(),
            endpoint: "https://example.com/feed".into(),
            collector_kind: CollectorKind::Rss,
            classification: SourceClass::Company,
            capabilities: vec![],
            reliability_tier: ReliabilityTier::Tier1Official,
            poll: PollPolicy {
                interval_seconds: 60,
                timeout_seconds: 5,
                max_retries: 1,
            },
            rate_limit: RateLimitPolicy {
                requests: 1,
                per_seconds: 1,
            },
            auth,
            retention: RetentionPolicy {
                raw_days: 1,
                observation_days: 30,
                audit_days: None,
            },
            enabled: true,
        }
    }

    #[test]
    fn private_access_sources_fail_closed() {
        assert!(matches!(
            source(AuthRequirement::Paywall).validate_public(),
            Err(CollectorError::Policy(_))
        ));
    }
    #[test]
    fn registry_rejects_duplicate_identity() {
        let mut registry = SourceRegistry::default();
        registry.register(source(AuthRequirement::None)).unwrap();
        assert!(registry.register(source(AuthRequirement::None)).is_err());
    }
    #[test]
    fn canonical_url_removes_tracking() {
        assert_eq!(
            canonicalize_url("https://example.com/a/?utm_source=x&b=2#top").unwrap(),
            "https://example.com/a?b=2"
        );
    }
    #[test]
    fn dedup_uses_external_id_url_hash_and_title_time() {
        let mut first = RawRecord::new("s", "one", "text/plain", Some("https://e.test/a")).unwrap();
        first.external_id = Some("42".into());
        first.title = Some("Federal Reserve issues policy statement".into());
        first.published_at = Some(Utc::now());
        let mut d = Deduplicator::default();
        d.insert(first.clone()).unwrap();
        let mut by_id = RawRecord::new("s", "two", "text/plain", Some("https://e.test/b")).unwrap();
        by_id.external_id = Some("42".into());
        assert_eq!(d.insert(by_id), Err(DuplicateReason::ExternalId));
        let by_url =
            RawRecord::new("other", "three", "text/plain", Some("https://e.test/a")).unwrap();
        assert_eq!(d.insert(by_url), Err(DuplicateReason::CanonicalUrl));
        let by_hash =
            RawRecord::new("other", "one", "text/plain", Some("https://e.test/c")).unwrap();
        assert_eq!(d.insert(by_hash), Err(DuplicateReason::ContentHash));
        let mut by_title =
            RawRecord::new("other", "four", "text/plain", Some("https://e.test/d")).unwrap();
        by_title.title = first.title.clone();
        by_title.published_at = first.published_at;
        assert_eq!(d.insert(by_title), Err(DuplicateReason::TitleTime));
    }
    #[tokio::test]
    async fn rate_limit_is_enforced_without_network() {
        let limiter = RateLimiter::new(RateLimitPolicy {
            requests: 1,
            per_seconds: 10,
        });
        let now = Utc::now();
        limiter.acquire(now).await.unwrap();
        assert!(limiter.acquire(now).await.is_err());
    }
    #[test]
    fn parses_rss_fixture() {
        let xml = r#"<rss version="2.0"><channel><title>Official</title><item><guid>x1</guid><title>Policy update</title><link>https://example.com/a</link><description>Facts only.</description><pubDate>Fri, 12 Sep 2025 10:00:00 GMT</pubDate></item></channel></rss>"#;
        let raw = RawRecord::new(
            "fed",
            xml,
            "application/rss+xml",
            Some("https://example.com/feed"),
        )
        .unwrap();
        let items = parse_feed(&raw).unwrap();
        assert_eq!(items[0].external_id.as_deref(), Some("x1"));
        assert_eq!(items[0].title.as_deref(), Some("Policy update"));
    }

    #[test]
    fn parses_json_feed_fixture() {
        let json = r#"{"version":"https://jsonfeed.org/version/1.1","title":"Official","items":[{"id":"json-1","url":"https://example.com/json-1","title":"Company update","content_text":"Observed facts.","date_published":"2026-09-12T10:00:00Z"}]}"#;
        let raw = RawRecord::new(
            "company",
            json,
            "application/feed+json",
            Some("https://example.com/feed.json"),
        )
        .unwrap();
        let items = parse_feed(&raw).unwrap();
        assert_eq!(items[0].external_id.as_deref(), Some("json-1"));
        assert_eq!(items[0].payload, "Observed facts.");
    }
    #[test]
    fn extracts_main_article_and_detects_drift() {
        let good = RawRecord::new(
            "nvidia",
            "<html><title>Demand</title><main>Shipment demand increased.</main></html>",
            "text/html",
            Some("https://example.com/a"),
        )
        .unwrap();
        assert!(
            extract_article(&good)
                .unwrap()
                .content
                .contains("increased")
        );
        let bad = RawRecord::new(
            "nvidia",
            "<html><div>layout changed</div></html>",
            "text/html",
            None,
        )
        .unwrap();
        assert!(matches!(
            extract_article(&bad),
            Err(CollectorError::SchemaDrift(_))
        ));
    }
    #[test]
    fn normalizes_binance_and_fails_closed_on_schema_drift() {
        let raw = RawRecord::new(
            "binance",
            r#"[{"symbol":"BTCUSDT","lastPrice":"60000","volume":"100","quoteVolume":"6000000"}]"#,
            "application/json",
            None,
        )
        .unwrap();
        let obs = normalize_binance_24h(&raw).unwrap();
        assert_eq!(obs[0].asset.symbol, "BTC-USDT-SPOT");
        let drift = RawRecord::new(
            "binance",
            r#"{"symbol":"BTCUSDT"}"#,
            "application/json",
            None,
        )
        .unwrap();
        assert!(matches!(
            normalize_binance_24h(&drift),
            Err(CollectorError::SchemaDrift(_))
        ));
    }
    #[test]
    fn normalizes_public_mempool_summary() {
        let raw = RawRecord::new(
            "mempool",
            r#"{"count":120000,"vsize":80000000,"total_fee":123456}"#,
            "application/json",
            None,
        )
        .unwrap();
        let obs = normalize_mempool(&raw).unwrap();
        assert_eq!(obs.metrics["transaction_count"], 120000.0);
    }
    #[test]
    fn schema_fingerprint_ignores_values_but_not_shape() {
        let a = serde_json::json!({"x": 1, "nested":{"ok":true}});
        let b = serde_json::json!({"x": 2, "nested":{"ok":false}});
        let c = serde_json::json!({"y": 2});
        assert_eq!(schema_fingerprint(&a), schema_fingerprint(&b));
        assert_ne!(schema_fingerprint(&a), schema_fingerprint(&c));
    }
    #[test]
    fn builtin_sources_are_public_and_explicit() {
        let sources = builtin_sources();
        assert_eq!(sources.len(), 6);
        assert!(
            sources
                .iter()
                .all(|s| s.validate_public().is_ok() && !s.capabilities.is_empty())
        );
    }

    async fn local_http(body: &'static str, status: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 2048];
            let _ = socket.read(&mut request).await.unwrap();
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        format!("http://{address}/data")
    }

    #[tokio::test]
    async fn rest_collector_uses_local_http_fixture_and_updates_health() {
        let mut definition = source(AuthRequirement::None);
        definition.collector_kind = CollectorKind::Rest;
        definition.endpoint = local_http(r#"{"ok":true}"#, "200 OK").await;
        let collector = HttpCollector::new(definition).unwrap();
        let records = collector.fetch_once().await.unwrap();
        assert_eq!(records[0].payload, r#"{"ok":true}"#);
        assert_eq!(collector.health().await.status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn websocket_collector_reads_local_data_frame() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut websocket = accept_async(socket).await.unwrap();
            websocket
                .send(Message::text(r#"{"trade":"fixture"}"#))
                .await
                .unwrap();
        });
        let mut definition = source(AuthRequirement::None);
        definition.collector_kind = CollectorKind::WebSocket;
        definition.endpoint = format!("ws://{address}/stream");
        let collector = WebSocketCollector::new(definition).unwrap();
        let records = collector.subscribe().await.unwrap();
        assert!(records[0].payload.contains("fixture"));
    }

    struct FixtureSearch;
    #[async_trait]
    impl SearchProvider for FixtureSearch {
        async fn search(&self, _query: &str) -> Result<Vec<SearchResult>, CollectorError> {
            Ok(vec![
                SearchResult {
                    title: "Public filing".into(),
                    url: "https://example.com/filing".into(),
                    snippet: "fact".into(),
                },
                SearchResult {
                    title: "Unsafe".into(),
                    url: "file:///private".into(),
                    snippet: "ignored".into(),
                },
            ])
        }
    }

    #[tokio::test]
    async fn discovery_creates_research_task_without_creating_evidence() {
        let task = DiscoveryAgent::new(FixtureSearch)
            .discover("AI demand", "source discovery")
            .await
            .unwrap();
        assert_eq!(task.discovered_urls, vec!["https://example.com/filing"]);
    }
}
