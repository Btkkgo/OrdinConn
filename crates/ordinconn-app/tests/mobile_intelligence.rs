use chrono::Utc;
use mobile_runtime::{
    EvidenceStatus, ExtractionMethod, MobileActionDecision, MobileActionDenyReason,
    MobileActionReceipt, MobileActionRequest, MobileActionStatus, MobileActionTarget, MobileBounds,
    MobileCapture, MobileDeviceSession, MobileDeviceType, MobileFrame, MobileObservation,
    MobilePlatform, MobileSessionStatus, MobileUiSnapshot, PrivacyClass, RawMobileElement,
    SensitiveText, VerificationResult,
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
async fn mobile_action_intent_is_durable_before_input_and_excludes_text() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&directory.path().join("intent.sqlite3"))
        .await
        .unwrap();
    let capture = fixture_capture();
    let request = MobileActionRequest {
        action_id: "mobile-action-intent-1".into(),
        session_id: capture.session.session_id,
        snapshot_id: capture.snapshot.snapshot_id,
        expected_package: "com.example.news".into(),
        requested_at: Utc::now(),
        target: MobileActionTarget::Type {
            element_ref: "@e1".into(),
        },
        text: Some(SensitiveText::new("wifi".into())),
    };
    runtime.record_mobile_action_intent(&request).await.unwrap();
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM runtime_events WHERE event_type='mobile.action_intent'",
    )
    .fetch_one(runtime.pool())
    .await
    .unwrap();
    assert!(payload.contains("mobile-action-intent-1"));
    assert!(payload.contains("\"commandSent\":false"));
    assert!(!payload.contains("wifi"));
    let receipts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_action_receipts")
        .fetch_one(runtime.pool())
        .await
        .unwrap();
    assert_eq!(receipts, 0);
}

fn mobile_action_receipt(
    capture: &MobileCapture,
    status: MobileActionStatus,
) -> MobileActionReceipt {
    MobileActionReceipt {
        action_id: format!("action_{}", uuid::Uuid::now_v7()),
        session_id: capture.session.session_id.clone(),
        snapshot_id: capture.snapshot.snapshot_id.clone(),
        target: MobileActionTarget::Type {
            element_ref: "@e1".into(),
        },
        decision: if status == MobileActionStatus::Blocked {
            MobileActionDecision::Denied(MobileActionDenyReason::SensitiveTarget)
        } else {
            MobileActionDecision::Allowed
        },
        status,
        requested_at: Utc::now(),
        completed_at: Utc::now(),
        pre_package: capture.observation.package_name.clone(),
        pre_activity: capture.observation.activity.clone(),
        pre_frame_hash: capture.observation.frame_hash.clone(),
        pre_ui_tree_hash: capture.observation.ui_tree_hash.clone(),
        post_package: None,
        post_snapshot_id: None,
        post_activity: None,
        post_frame_hash: None,
        post_ui_tree_hash: None,
        verification: None,
        text_length: Some(4),
        text_sha256: Some("sha256:dummy".into()),
        command_sent: status == MobileActionStatus::Executed,
    }
}

#[tokio::test]
async fn mobile_action_receipts_and_events_persist_without_type_plaintext() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&directory.path().join("action.sqlite3"))
        .await
        .unwrap();
    let before = fixture_capture();
    runtime.record_mobile_capture(&before).await.unwrap();
    let blocked = mobile_action_receipt(&before, MobileActionStatus::Blocked);
    runtime.record_mobile_action(&blocked, None).await.unwrap();
    let after = fixture_capture();
    let mut executed = mobile_action_receipt(&before, MobileActionStatus::Executed);
    executed.post_package = Some(after.observation.package_name.clone());
    executed.post_snapshot_id = Some(after.snapshot.snapshot_id.clone());
    executed.post_activity = Some(after.observation.activity.clone());
    executed.post_frame_hash = Some(after.observation.frame_hash.clone());
    executed.post_ui_tree_hash = Some(after.observation.ui_tree_hash.clone());
    executed.verification = Some(VerificationResult::Verified);
    runtime
        .record_mobile_action(&executed, Some(&after))
        .await
        .unwrap();
    let workspace = runtime.mobile_workspace_data().await.unwrap();
    assert_eq!(
        workspace.latest_action_receipt.unwrap().action_id,
        executed.action_id
    );
    let event_types: Vec<String> = sqlx::query_scalar("SELECT event_type FROM runtime_events WHERE event_type LIKE 'mobile.action_%' ORDER BY sequence").fetch_all(runtime.pool()).await.unwrap();
    assert_eq!(
        event_types,
        vec![
            "mobile.action_requested",
            "mobile.action_blocked",
            "mobile.action_requested",
            "mobile.action_executed",
            "mobile.action_verified"
        ]
    );
    let stored: Vec<String> =
        sqlx::query_scalar("SELECT domain_json FROM mobile_action_receipts ORDER BY completed_at")
            .fetch_all(runtime.pool())
            .await
            .unwrap();
    assert_eq!(stored.len(), 2);
    let all_db_text: Vec<String> = sqlx::query_scalar("SELECT payload_json FROM runtime_events")
        .fetch_all(runtime.pool())
        .await
        .unwrap();
    assert!(!stored.join("").contains("wifi"));
    assert!(!all_db_text.join("").contains("wifi"));
}

#[tokio::test]
async fn mobile_action_duplicate_receipt_rolls_back_post_capture_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&directory.path().join("duplicate-action.sqlite3"))
        .await
        .unwrap();
    let before = fixture_capture();
    runtime.record_mobile_capture(&before).await.unwrap();
    let blocked = mobile_action_receipt(&before, MobileActionStatus::Blocked);
    runtime.record_mobile_action(&blocked, None).await.unwrap();
    let after = fixture_capture();
    let mut duplicate = mobile_action_receipt(&before, MobileActionStatus::Executed);
    duplicate.action_id = blocked.action_id.clone();
    duplicate.post_snapshot_id = Some(after.snapshot.snapshot_id.clone());
    duplicate.post_frame_hash = Some(after.observation.frame_hash.clone());
    duplicate.post_ui_tree_hash = Some(after.observation.ui_tree_hash.clone());
    assert!(
        runtime
            .record_mobile_action(&duplicate, Some(&after))
            .await
            .is_err()
    );
    let snapshots: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_ui_snapshots")
        .fetch_one(runtime.pool())
        .await
        .unwrap();
    assert_eq!(snapshots, 1);
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

#[tokio::test]
async fn mobile_session_shutdown_is_persisted_and_audited_once() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&directory.path().join("shutdown.sqlite3"))
        .await
        .unwrap();
    let capture = fixture_capture();
    let session_id = capture.session.session_id.clone();
    runtime.record_mobile_capture(&capture).await.unwrap();

    assert!(runtime.end_mobile_session(&session_id).await.unwrap());
    assert!(!runtime.end_mobile_session(&session_id).await.unwrap());

    let row = sqlx::query("SELECT status,domain_json FROM mobile_device_sessions WHERE id=?")
        .bind(&session_id)
        .fetch_one(runtime.pool())
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("status"), "ended");
    let stored: MobileDeviceSession =
        serde_json::from_str(&row.get::<String, _>("domain_json")).unwrap();
    assert_eq!(stored.status, MobileSessionStatus::Ended);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM runtime_events WHERE event_type='mobile.session_ended' AND aggregate_id=?",
        )
        .bind(&session_id)
        .fetch_one(runtime.pool())
        .await
        .unwrap(),
        1
    );
}
