use mobile_runtime::planner::MAX_PLANNER_ATTEMPTS;
use model_gateway::ModelError;
use std::time::Duration;

pub const MAX_RETRY_DELAY_MS: u64 = 10_000;
pub const MAX_JITTER_MS: u64 = 250;

/// Zero-based failed attempt; only explicitly identified transient HTTP errors retry.
pub fn delay(error: &ModelError, attempt: u32, jitter_ms: u64) -> Option<Duration> {
    let ModelError::HttpRejected {
        status: 408 | 429 | 500 | 502 | 503 | 504,
        retry_after_ms,
        ..
    } = error
    else {
        return None;
    };
    if attempt >= MAX_PLANNER_ATTEMPTS - 1 {
        return None;
    }
    let base = 1000u64 << attempt;
    let millis = base
        .max(retry_after_ms.unwrap_or(0))
        .saturating_add(jitter_ms.min(MAX_JITTER_MS))
        .min(MAX_RETRY_DELAY_MS);
    Some(Duration::from_millis(millis))
}

#[async_trait::async_trait]
pub(super) trait Backoff: Send + Sync {
    fn delay(&self, error: &ModelError, attempt: u32) -> Option<Duration>;
    async fn wait(&self, delay: Duration);
}

pub(super) struct ExponentialBackoff;
#[async_trait::async_trait]
impl Backoff for ExponentialBackoff {
    fn delay(&self, error: &ModelError, attempt: u32) -> Option<Duration> {
        // UUID v7's random low bits provide independent, bounded local jitter.
        delay(
            error,
            attempt,
            (uuid::Uuid::now_v7().as_u128() as u64) % (MAX_JITTER_MS + 1),
        )
    }
    async fn wait(&self, delay: Duration) {
        tokio::time::sleep(delay).await;
    }
}
