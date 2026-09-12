use ordinconn_app::AppRuntime;
use sqlx::Row;

#[tokio::test]
async fn migrations_create_required_tables_and_sqlite_safety_pragmas() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("ordinconn.sqlite3");
    let runtime = AppRuntime::initialize(&database).await.unwrap();

    let tables: Vec<String> = sqlx::query("SELECT name FROM sqlite_master WHERE type = 'table'")
        .fetch_all(runtime.pool())
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("name"))
        .collect();
    for required in [
        "agent_threads",
        "agent_turns",
        "agent_items",
        "model_providers",
        "connectors",
        "evidence",
        "signals",
        "agent_reports",
        "watchlists",
        "trade_proposals",
        "approval_requests",
        "execution_records",
        "automations",
        "settings",
        "runtime_events",
    ] {
        assert!(tables.contains(&required.to_owned()), "missing {required}");
    }
    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(runtime.pool())
        .await
        .unwrap();
    let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(runtime.pool())
        .await
        .unwrap();
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
    assert_eq!(foreign_keys, 1);
    let provider_columns: Vec<String> = sqlx::query("PRAGMA table_info(model_providers)")
        .fetch_all(runtime.pool())
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("name"))
        .collect();
    assert!(provider_columns.contains(&"temperature".to_owned()));
    assert!(provider_columns.contains(&"context_window".to_owned()));
}

#[tokio::test]
async fn startup_recovers_unsafe_active_turns_as_interrupted() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("recovery.sqlite3");
    let runtime = AppRuntime::initialize(&database).await.unwrap();
    sqlx::query("INSERT INTO agent_threads (id, title, created_at, updated_at) VALUES ('thread-1', 'Recovery', datetime('now'), datetime('now'))")
        .execute(runtime.pool()).await.unwrap();
    sqlx::query("INSERT INTO agent_turns (id, thread_id, status, created_at) VALUES ('turn-1', 'thread-1', 'running', datetime('now'))")
        .execute(runtime.pool()).await.unwrap();

    let recovered = runtime.recover_interrupted_tasks().await.unwrap();
    assert_eq!(recovered, 1);
    let status: String = sqlx::query_scalar("SELECT status FROM agent_turns WHERE id = 'turn-1'")
        .fetch_one(runtime.pool())
        .await
        .unwrap();
    let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM runtime_events WHERE event_type = 'runtime.task_interrupted' AND turn_id = 'turn-1'")
        .fetch_one(runtime.pool()).await.unwrap();
    assert_eq!(status, "interrupted");
    assert_eq!(event_count, 1);
}

#[tokio::test]
async fn runtime_events_reject_update_and_delete() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("append-only.sqlite3");
    let runtime = AppRuntime::initialize(&database).await.unwrap();
    sqlx::query("INSERT INTO runtime_events (id, event_type, aggregate_type, aggregate_id, payload_json, created_at, sequence) VALUES ('event-1', 'test.created', 'test', 'test-1', '{}', datetime('now'), 1)")
        .execute(runtime.pool()).await.unwrap();
    assert!(
        sqlx::query("UPDATE runtime_events SET event_type = 'changed' WHERE id = 'event-1'")
            .execute(runtime.pool())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM runtime_events WHERE id = 'event-1'")
            .execute(runtime.pool())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn orderly_shutdown_interrupts_persisted_active_turns_before_closing_database() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("shutdown.sqlite3");
    let runtime = AppRuntime::initialize(&database).await.unwrap();
    sqlx::query("INSERT INTO agent_threads (id, title, created_at, updated_at) VALUES ('thread-shutdown', 'Shutdown', datetime('now'), datetime('now'))")
        .execute(runtime.pool()).await.unwrap();
    sqlx::query("INSERT INTO agent_turns (id, thread_id, status, created_at) VALUES ('turn-shutdown', 'thread-shutdown', 'waiting_tool', datetime('now'))")
        .execute(runtime.pool()).await.unwrap();

    runtime.shutdown().await.unwrap();

    let pool = ordinconn_app::db::open_database(&database).await.unwrap();
    let status: String =
        sqlx::query_scalar("SELECT status FROM agent_turns WHERE id = 'turn-shutdown'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM runtime_events WHERE event_type = 'runtime.task_interrupted' AND turn_id = 'turn-shutdown'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(status, "interrupted");
    assert_eq!(event_count, 1);
}
