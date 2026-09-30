use super::*;
use crate::{MobileRuntimeSettings, ModelProviderConfig};
use mobile_runtime::{execution::*, *};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn fixture() -> (tempfile::TempDir, Arc<AppRuntime>, MobileGoalId) {
    let dir = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&dir.path().join("source.sqlite3"))
        .await
        .unwrap();
    runtime
        .upsert_model_provider(ModelProviderConfig {
            id: "fixture-saved-provider".into(),
            name: "Google Gemini".into(),
            provider_type: "openai_compatible_chat".into(),
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai/".into(),
            credential_ref: Some("keyring:fixture-saved-provider".into()),
            default_model: "gemini-3.6-flash".into(),
            temperature: 0.2,
            context_window: 32768,
            enabled: true,
            capabilities: serde_json::to_value(ProviderCapabilities {
                chat_completions: true,
                ..Default::default()
            })
            .unwrap(),
        })
        .await
        .unwrap();
    runtime
        .save_mobile_settings(&MobileRuntimeSettings {
            allowed_apps: vec!["com.android.settings".into()],
            ..Default::default()
        })
        .await
        .unwrap();
    let snapshot = MobileUiSnapshot::from_elements(
        "fixture-session",
        "com.android.settings",
        "com.android.settings.Settings$InternetActivity",
        1080,
        2400,
        vec![],
    );
    let frame = MobileFrame::from_png("fixture-session", 1080, 2400, vec![1, 2, 3]);
    let observation = MobileObservation::from_snapshot(
        &snapshot,
        &frame,
        None,
        "fixture-android",
        PrivacyClass::UserAllowed,
    );
    let observation_id = observation.id.clone();
    let now = chrono::Utc::now();
    runtime
        .record_mobile_capture(&MobileCapture {
            session: MobileDeviceSession {
                session_id: "fixture-session".into(),
                device_id: "fixture-device".into(),
                platform: MobilePlatform::Android,
                device_type: MobileDeviceType::Emulator,
                os_version: "fixture".into(),
                screen_width: 1080,
                screen_height: 2400,
                connected_at: now,
                current_app: Some("com.android.settings".into()),
                current_activity: Some("com.android.settings.Settings$InternetActivity".into()),
                status: MobileSessionStatus::Connected,
                last_observation_at: Some(now),
            },
            snapshot,
            frame,
            observation,
        })
        .await
        .unwrap();
    let repo = runtime.mobile_goal_repository();
    let goal = repo
        .create_goal(
            "Return to Android Settings homepage",
            MobileGoalBudget::default(),
        )
        .await
        .unwrap();
    repo.set_completion_target(
        &goal.id,
        &MobileCompletionTarget::PageEquals {
            package: "com.android.settings".into(),
            activity: "com.android.settings.homepage.SettingsHomepageActivity".into(),
            visible_text: vec![
                "Search Settings".into(),
                "Network & internet".into(),
                "Connected devices".into(),
            ],
        },
    )
    .await
    .unwrap();
    repo.begin_autonomous_goal(&goal.id).await.unwrap();
    repo.update_goal_status(
        &goal.id,
        MobileGoalStatus::Failed,
        Some(MobileGoalError::new(MobileGoalErrorCode::ModelError)),
    )
    .await
    .unwrap();
    sqlx::query("INSERT INTO mobile_model_calls(id,goal_id,observation_id,created_at) VALUES ('fixture-call',?,?,?)")
        .bind(goal.id.as_str()).bind(observation_id).bind(now.to_rfc3339()).execute(runtime.pool()).await.unwrap();
    (dir, runtime, goal.id)
}

#[tokio::test]
async fn prepare_preserves_saved_source_and_creates_five_equal_isolated_inputs() {
    let (_dir, runtime, goal) = fixture().await;
    let session = runtime
        .prepare_internal_planner_acceptance(&goal)
        .await
        .unwrap();
    assert_eq!(session.isolated.len(), 5);
    assert!(!session.used);
    assert_eq!(session.readiness().provider_count, 1);
    assert_eq!(session.readiness().android_actions_executed, 0);
    runtime.verify_acceptance_source(&session).await.unwrap();
    for replay in &session.isolated {
        assert_eq!(action_count(replay.pool()).await.unwrap(), 0);
        let calls: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_model_calls")
            .fetch_one(replay.pool())
            .await
            .unwrap();
        assert_eq!(calls, 0);
    }
    assert_eq!(
        runtime
            .mobile_goal_repository()
            .get_goal(&goal)
            .await
            .unwrap()
            .status,
        MobileGoalStatus::Failed
    );
}

#[tokio::test]
async fn changed_saved_metadata_fails_closed_before_credential_resolution() {
    let (_dir, runtime, goal) = fixture().await;
    let mut session = runtime
        .prepare_internal_planner_acceptance(&goal)
        .await
        .unwrap();
    sqlx::query("UPDATE model_providers SET temperature=0.7")
        .execute(runtime.pool())
        .await
        .unwrap();
    let credential_calls = AtomicUsize::new(0);
    let result = runtime
        .run_internal_planner_acceptance(&mut session, |_| {
            credential_calls.fetch_add(1, Ordering::SeqCst);
            Ok(None)
        })
        .await;
    assert!(result.is_err());
    assert_eq!(credential_calls.load(Ordering::SeqCst), 0);
    assert!(!session.used);
    assert_eq!(action_count(runtime.pool()).await.unwrap(), 0);
}

#[tokio::test]
async fn internal_harness_uses_real_service_retry_budget_and_never_executes() {
    let (_dir, runtime, goal) = fixture().await;
    let mut session = runtime
        .prepare_internal_planner_acceptance(&goal)
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    // Unit fixture overrides only private disposable replay databases, never the saved source.
    for replay in &session.isolated {
        sqlx::query("UPDATE model_providers SET base_url=?")
            .bind(&base)
            .execute(replay.pool())
            .await
            .unwrap();
    }
    let server = tokio::spawn(async move {
        for call in 0..6 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let read = socket.read(&mut buffer).await.unwrap();
                assert_ne!(read, 0);
                bytes.extend_from_slice(&buffer[..read]);
                if let Some(start) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&bytes[..start]);
                    let length = head
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|size| size.trim().parse::<usize>().ok())
                        })
                        .unwrap();
                    if bytes.len() >= start + 4 + length {
                        let request: serde_json::Value =
                            serde_json::from_slice(&bytes[start + 4..start + 4 + length]).unwrap();
                        assert_eq!(request["model"], "gemini-3.6-flash");
                        assert!(request.get("response_format").is_none());
                        assert!(request.get("tools").is_none());
                        assert_eq!(request["messages"].as_array().unwrap().len(), 2);
                        break;
                    }
                }
            }
            let (status, body) = if call == 0 {
                (503, json!({"error":{"code":503,"status":"UNAVAILABLE","message":"The model is overloaded. Please try again later."}}).to_string())
            } else {
                let decision = r#"{"decision":"next_action","action":{"type":"observe","reason":"Read public UI","expected_result":{"kind":"UI_CHANGED"}},"completion":null,"failure":null}"#;
                (200, json!({"choices":[{"finish_reason":"stop","message":{"content":decision,"extra_content":{"google":{"thought_signature":"fixture-private-signature"}}}}]}).to_string())
            };
            socket.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
    });
    let credential_calls = AtomicUsize::new(0);
    let report = runtime
        .run_internal_planner_acceptance_with_pacing(
            &mut session,
            |id| {
                assert_eq!(id, "fixture-saved-provider");
                credential_calls.fetch_add(1, Ordering::SeqCst);
                Ok(Some("fixture-credential".into()))
            },
            Duration::ZERO,
            Duration::ZERO,
        )
        .await
        .unwrap();
    server.await.unwrap();
    assert_eq!(report.passed, 5);
    assert_eq!(report.http_attempts, 6);
    assert_eq!(report.http_503_retries, 1);
    assert_eq!(credential_calls.load(Ordering::SeqCst), 5);
    assert_eq!(report.logical_calls[0].budgeted_model_calls, 2);
    assert!(report.logical_calls.iter().all(|call| matches!(
        call.decision,
        Some(mobile_runtime::planner::MobilePlannerDecisionDto::NextAction)
    )));
    assert_eq!(report.android_actions_executed, 0);
    assert!(report.production_data_unchanged);
    let encoded = serde_json::to_string(&report).unwrap();
    assert!(!encoded.contains("fixture-credential"));
    assert!(!encoded.contains("fixture-private-signature"));
    assert!(!encoded.contains("Read public UI"));
    assert!(
        runtime
            .run_internal_planner_acceptance(&mut session, |_| panic!("session reused"))
            .await
            .is_err()
    );
}
