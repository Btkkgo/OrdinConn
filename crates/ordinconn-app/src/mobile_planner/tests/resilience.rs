use super::*;

#[derive(Default)]
pub(super) struct RecordingBackoff(Mutex<Vec<Duration>>);
#[async_trait]
impl retry::Backoff for RecordingBackoff {
    fn delay(&self, error: &ModelError, attempt: u32) -> Option<Duration> {
        retry::delay(error, attempt, 0)
    }
    async fn wait(&self, delay: Duration) {
        self.0.lock().unwrap().push(delay);
    }
}
fn rejected(status: u16, retry_after_ms: Option<u64>) -> ModelError {
    ModelError::HttpRejected {
        status,
        code: status.to_string(),
        message: "[REDACTED]".into(),
        retry_after_ms,
    }
}

#[test]
fn bounded_jitter_retry_after_and_status_eligibility_are_deterministic() {
    for status in [408, 429, 500, 502, 503, 504] {
        for (attempt, base) in [(0, 1000), (1, 2000), (2, 4000)] {
            assert_eq!(
                retry::delay(&rejected(status, None), attempt, 0),
                Some(Duration::from_millis(base))
            );
            assert_eq!(
                retry::delay(&rejected(status, None), attempt, 250),
                Some(Duration::from_millis(base + 250))
            );
            assert_eq!(
                retry::delay(&rejected(status, None), attempt, u64::MAX),
                Some(Duration::from_millis(base + 250))
            );
        }
        assert_eq!(retry::delay(&rejected(status, None), 3, 0), None);
        assert_eq!(retry::delay(&rejected(status, None), u32::MAX, 0), None);
    }
    for status in [400, 401, 402, 403, 404, 405, 422, 501] {
        assert_eq!(retry::delay(&rejected(status, Some(2000)), 0, 0), None);
    }
    assert_eq!(
        retry::delay(&rejected(429, Some(3000)), 0, 100),
        Some(Duration::from_millis(3100))
    );
    assert_eq!(
        retry::delay(&rejected(503, Some(u64::MAX)), 0, 250),
        Some(Duration::from_secs(10))
    );
    assert_eq!(
        retry::delay(&ModelError::MalformedResponse("fixture".into()), 0, 0),
        None
    );
}

async fn sequence(status: u16, failures: usize, expected_ok: bool) {
    let (_d, r, g, p) = setup().await;
    let context = r
        .mobile_goal_repository()
        .planner_context(&g.id, None)
        .await
        .unwrap();
    let m = UnavailableModel {
        inner: model(OBSERVE),
        failures,
        status,
    };
    let backoff = RecordingBackoff::default();
    let result = p
        .run_with_backoff(
            &context,
            &provider(),
            &m,
            None,
            Duration::from_secs(30),
            &backoff,
            None,
        )
        .await;
    assert_eq!(result.is_ok(), expected_ok);
    let count = if expected_ok { failures + 1 } else { 4 };
    assert_eq!(m.inner.calls.load(Ordering::SeqCst), count);
    assert_http_planner_state(&r, &g, count as i64, if expected_ok { 1 } else { 0 }).await;
    assert_eq!(
        *backoff.0.lock().unwrap(),
        [1000, 2000, 4000]
            .into_iter()
            .take(count - 1)
            .map(Duration::from_millis)
            .collect::<Vec<_>>()
    );
    let attempts:Vec<String>=sqlx::query_scalar("SELECT payload_json FROM runtime_events WHERE event_type='mobile.provider_attempt' ORDER BY sequence").fetch_all(r.pool()).await.unwrap();
    assert_eq!(attempts.len(), failures.min(4));
    for value in attempts {
        let j: serde_json::Value = serde_json::from_str(&value).unwrap();
        assert_eq!(
            j["providerFailure"],
            if status == 429 {
                "RATE_LIMITED"
            } else {
                "UNAVAILABLE"
            }
        );
    }
}
#[tokio::test]
async fn unavailable_then_success() {
    sequence(503, 1, true).await;
}
#[tokio::test]
async fn unavailable_twice_then_success() {
    sequence(503, 2, true).await;
}
#[tokio::test]
async fn unavailable_three_times_then_success() {
    sequence(503, 3, true).await;
}
#[tokio::test]
async fn unavailable_exhausted() {
    sequence(503, 4, false).await;
}
#[tokio::test]
async fn rate_limited_then_success() {
    sequence(429, 1, true).await;
}
#[tokio::test]
async fn rate_limited_twice_then_success() {
    sequence(429, 2, true).await;
}
#[tokio::test]
async fn rate_limited_exhausted() {
    sequence(429, 4, false).await;
}

struct ParkedBackoff(std::sync::Arc<tokio::sync::Notify>);
#[async_trait]
impl retry::Backoff for ParkedBackoff {
    fn delay(&self, e: &ModelError, a: u32) -> Option<Duration> {
        retry::delay(e, a, 0)
    }
    async fn wait(&self, _: Duration) {
        self.0.notify_one();
        std::future::pending::<()>().await;
    }
}
async fn interrupted_backoff(cancel: bool) {
    let (_d, r, g, p) = setup().await;
    let c = r
        .mobile_goal_repository()
        .planner_context(&g.id, None)
        .await
        .unwrap();
    let token = p.cancellation.clone();
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let parked = ParkedBackoff(entered.clone());
    let task = tokio::spawn(async move {
        let m = UnavailableModel {
            inner: model(OBSERVE),
            failures: 10,
            status: 503,
        };
        let result = p
            .run_with_backoff(
                &c,
                &provider(),
                &m,
                None,
                Duration::from_secs(30),
                &parked,
                None,
            )
            .await;
        (result, m.inner.calls.load(Ordering::SeqCst))
    });
    entered.notified().await;
    if cancel {
        token.cancel();
    } else {
        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(31)).await;
        tokio::time::resume();
    }
    let (result, calls) = task.await.unwrap();
    assert!(
        matches!(result,Err(AppError::MobileGoal(e)) if e.code == if cancel {MobileGoalErrorCode::UserStopped}else{MobileGoalErrorCode::ModelError})
    );
    assert_eq!(calls, 1);
    assert_http_planner_state(&r, &g, 1, 0).await;
    if !cancel {
        let payload:String=sqlx::query_scalar("SELECT payload_json FROM runtime_events WHERE event_type='mobile.planner_failed' LIMIT 1").fetch_one(r.pool()).await.unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&payload).unwrap()["providerFailure"],
            "TIMEOUT"
        );
    }
}
#[tokio::test]
async fn stop_during_backoff_prevents_subsequent_attempt() {
    interrupted_backoff(true).await;
}
#[tokio::test]
async fn shared_deadline_stops_backoff_without_new_attempt() {
    interrupted_backoff(false).await;
}
