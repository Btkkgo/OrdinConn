use chrono::Utc;
use mobile_runtime::{
    EvidenceStatus, ExtractionMethod, MobileBounds, MobileCapture, MobileDeviceSession,
    MobileDeviceType, MobileFrame, MobileObservation, MobilePlatform, MobileSessionStatus,
    MobileUiSnapshot, PrivacyClass, RawMobileElement,
};
use ordinconn_app::{AppRuntime, MobileResearchBudget, MobileRuntimeSettings};
use sqlx::Row;

fn fixture_capture() -> MobileCapture {
    let session_id = "mobile-session-1".to_owned();
    let snapshot = MobileUiSnapshot::from_elements(
        &session_id,
        "com.example.news",
        "com.example.news/.MainActivity",
        1080,
        2400,
        vec![RawMobileElement {
            text: Some("BTC ETF inflows rose; secret-screen-copy".into()),
            role: "text".into(),
            class_name: "android.widget.TextView".into(),
            content_description: None,
            bounds: MobileBounds {
                x: 10,
                y: 20,
                width: 900,
                height: 80,
            },
            clickable: false,
            scrollable: false,
            enabled: true,
            focused: false,
            selected: false,
            resource_id: Some("com.example.news:id/headline".into()),
            password: false,
        }],
    );
    let frame = MobileFrame::from_png(&session_id, 1080, 2400, vec![1, 2, 3, 4]);
    let observation = MobileObservation::from_snapshot(
        &snapshot,
        &frame,
        Some("research-task-1".into()),
        "generic-android",
        PrivacyClass::UserAllowed,
    );
    MobileCapture {
        session: MobileDeviceSession {
            session_id,
            device_id: "emulator-5554".into(),
            platform: MobilePlatform::Android,
            device_type: MobileDeviceType::Emulator,
            os_version: "15".into(),
            screen_width: 1080,
            screen_height: 2400,
            connected_at: Utc::now(),
            current_app: Some("com.example.news".into()),
            current_activity: Some("com.example.news/.MainActivity".into()),
            status: MobileSessionStatus::Connected,
            last_observation_at: Some(Utc::now()),
        },
        snapshot,
        frame,
        observation,
    }
}

#[tokio::test]
async fn mobile_capture_projects_to_feed_without_becoming_evidence_or_signal() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&directory.path().join("mobile.sqlite3"))
        .await
        .unwrap();
    let capture = fixture_capture();
    let observation_id = capture.observation.id.clone();

    runtime.record_mobile_capture(&capture).await.unwrap();
    runtime
        .set_warehouse_entry(&observation_id, true, true, vec!["macro".into()])
        .await
        .unwrap();

    let workspace = runtime.mobile_workspace_data().await.unwrap();
    let item = workspace
        .feed
        .iter()
        .find(|item| item.id == observation_id)
        .unwrap();
    assert_eq!(item.evidence_status, "observation_only");
    assert_eq!(item.source_method, "MOBILE");
    assert!(
        item.visible_facts
            .iter()
            .any(|fact| fact.contains("BTC ETF"))
    );
    assert!(
        workspace
            .warehouse
            .iter()
            .any(|entry| entry.item_id == observation_id && entry.favorite && entry.saved)
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM evidence")
            .fetch_one(runtime.pool())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM signal_candidates")
            .fetch_one(runtime.pool())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        capture.observation.evidence_status,
        EvidenceStatus::ObservationOnly
    );
    assert_eq!(
        capture.observation.extraction_method,
        ExtractionMethod::Accessibility
    );
}

#[tokio::test]
async fn mobile_settings_budget_and_strategy_state_are_persisted() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&directory.path().join("settings.sqlite3"))
        .await
        .unwrap();
    let settings = MobileRuntimeSettings {
        android_sdk: Some("/opt/android".into()),
        allowed_apps: vec!["com.example.news".into()],
        screenshot_retention: "memory_only".into(),
        research_budget: MobileResearchBudget {
            max_duration_seconds: 120,
            max_steps: 20,
            max_scrolls: 5,
            max_pages: 10,
            max_observations: 20,
            max_model_calls: 5,
        },
        text_scale: 100,
    };
    runtime.save_mobile_settings(&settings).await.unwrap();
    let task = runtime
        .create_mobile_research_task(
            "Track BTC ETF headlines",
            vec!["com.example.news".into()],
            MobileResearchBudget {
                max_duration_seconds: 120,
                max_steps: 20,
                max_scrolls: 5,
                max_pages: 10,
                max_observations: 20,
                max_model_calls: 5,
            },
        )
        .await
        .unwrap();
    let strategy_id: String =
        sqlx::query_scalar("SELECT id FROM strategy_definitions ORDER BY id LIMIT 1")
            .fetch_one(runtime.pool())
            .await
            .unwrap();
    runtime
        .set_strategy_enabled(&strategy_id, false)
        .await
        .unwrap();

    let workspace = runtime.mobile_workspace_data().await.unwrap();
    assert_eq!(workspace.settings.allowed_apps, settings.allowed_apps);
    assert!(
        workspace
            .strategies
            .iter()
            .any(|strategy| strategy.id == strategy_id && !strategy.enabled)
    );
    let row = sqlx::query(
        "SELECT mobile_packages_json,mobile_budget_json FROM research_tasks WHERE id=?",
    )
    .bind(&task.id)
    .fetch_one(runtime.pool())
    .await
    .unwrap();
    assert!(
        row.get::<String, _>("mobile_packages_json")
            .contains("com.example.news")
    );
    assert!(
        row.get::<String, _>("mobile_budget_json")
            .contains("maxDurationSeconds")
    );
}

#[tokio::test]
async fn mobile_audit_payloads_contain_metadata_not_screen_content() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&directory.path().join("audit.sqlite3"))
        .await
        .unwrap();
    let capture = fixture_capture();
    runtime.record_mobile_capture(&capture).await.unwrap();

    let payloads: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM runtime_events WHERE event_type LIKE 'mobile.%' ORDER BY sequence",
    )
    .fetch_all(runtime.pool())
    .await
    .unwrap();
    assert_eq!(payloads.len(), 3);
    assert!(
        payloads
            .iter()
            .all(|payload| !payload.contains("secret-screen-copy"))
    );
    assert!(payloads.iter().any(|payload| payload.contains("frameHash")));
    assert!(
        payloads
            .iter()
            .any(|payload| payload.contains("elementCount"))
    );
}
