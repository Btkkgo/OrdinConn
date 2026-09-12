use agent_runtime::PageContext;
use ordinconn_app::AppRuntime;

#[tokio::test]
async fn mock_pipeline_persists_eighteen_evidence_backed_signals() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&directory.path().join("signals.sqlite3"))
        .await
        .unwrap();
    runtime.seed_demo_data().await.unwrap();
    let snapshot = runtime.snapshot().await.unwrap();
    assert_eq!(snapshot.signals.len(), 18);
    assert_eq!(
        snapshot
            .signals
            .iter()
            .filter(|signal| signal.market == "traditional")
            .count(),
        9
    );
    assert_eq!(
        snapshot
            .signals
            .iter()
            .filter(|signal| signal.market == "crypto")
            .count(),
        9
    );
    assert!(
        snapshot
            .signals
            .iter()
            .all(|signal| !signal.evidence.is_empty())
    );
    let candidate_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM signal_candidates")
        .fetch_one(runtime.pool())
        .await
        .unwrap();
    let evidence_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM evidence")
        .fetch_one(runtime.pool())
        .await
        .unwrap();
    assert_eq!(candidate_count, 18);
    assert_eq!(evidence_count, 18);
}

#[tokio::test]
async fn btc_signal_completes_report_approval_and_paper_execution() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&directory.path().join("closed-loop.sqlite3"))
        .await
        .unwrap();
    runtime.seed_demo_data().await.unwrap();
    let snapshot = runtime.snapshot().await.unwrap();
    let signal = snapshot
        .signals
        .iter()
        .find(|signal| signal.asset == "BTC" && signal.category == "exchange")
        .unwrap();

    let report = runtime.create_report(&signal.id).await.unwrap();
    let proposal = runtime.create_trade_proposal(&signal.id).await.unwrap();
    let approval = runtime.request_approval(&proposal.id).await.unwrap();
    let execution = runtime
        .approve_and_execute_paper(&approval.id)
        .await
        .unwrap();

    assert_eq!(report.signal_id, signal.id);
    assert_eq!(execution.status, "completed");
    assert_eq!(execution.adapter_id, "paper");
    let events = runtime.audit_timeline(&proposal.id).await.unwrap();
    for required in [
        "proposal.created",
        "approval.requested",
        "approval.approved",
        "approval.token_issued",
        "approval.token_consumed",
        "execution.created",
        "execution.completed",
    ] {
        assert!(
            events.iter().any(|event| event.event_type == required),
            "missing {required}"
        );
    }
    assert!(
        events
            .iter()
            .all(|event| !event.payload_json.contains("token"))
    );
}

#[tokio::test]
async fn same_approval_cannot_execute_twice() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&directory.path().join("single-use.sqlite3"))
        .await
        .unwrap();
    runtime.seed_demo_data().await.unwrap();
    let signal_id = runtime
        .snapshot()
        .await
        .unwrap()
        .signals
        .into_iter()
        .find(|signal| signal.asset == "BTC" && signal.category == "exchange")
        .unwrap()
        .id;
    let proposal = runtime.create_trade_proposal(&signal_id).await.unwrap();
    let approval = runtime.request_approval(&proposal.id).await.unwrap();
    assert!(
        runtime
            .approve_and_execute_paper(&approval.id)
            .await
            .is_ok()
    );
    assert!(
        runtime
            .approve_and_execute_paper(&approval.id)
            .await
            .is_err()
    );
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM execution_records WHERE approval_request_id = ?")
            .bind(&approval.id)
            .fetch_one(runtime.pool())
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn agent_turn_returns_immediately_and_completes_over_event_bus() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&directory.path().join("agent.sqlite3"))
        .await
        .unwrap();
    runtime.seed_demo_data().await.unwrap();
    let signal = runtime
        .snapshot()
        .await
        .unwrap()
        .signals
        .into_iter()
        .find(|signal| signal.asset == "BTC" && signal.category == "exchange")
        .unwrap();
    let mut events = runtime.subscribe();
    let started = runtime
        .start_agent_turn(
            None,
            "Why did this signal appear?",
            PageContext {
                page: "signals".into(),
                market: Some("crypto".into()),
                asset: Some("BTC".into()),
                signal_id: Some(signal.id.clone()),
                evidence_ids: signal.evidence.iter().map(|item| item.id.clone()).collect(),
            },
        )
        .await
        .unwrap();
    assert!(started.thread_id.starts_with("thread_"));
    assert!(started.turn_id.starts_with("turn_"));
    let completed = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let event = events.recv().await.unwrap();
            if event.event_type == "agent.message_completed" {
                break event;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(completed.turn_id.as_deref(), Some(started.turn_id.as_str()));
    let items = runtime.thread_items(&started.thread_id).await.unwrap();
    assert_eq!(items.len(), 2);
    assert!(items[1].content.starts_with("Mock Model:"));
}
