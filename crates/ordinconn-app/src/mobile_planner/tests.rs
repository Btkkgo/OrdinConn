use super::*;
use async_trait::async_trait;
use model_gateway::ModelError;
use std::sync::atomic::{AtomicUsize, Ordering};
struct TestModel {
    raw: String,
    calls: AtomicUsize,
    delay: Duration,
    capabilities: ProviderCapabilities,
}
#[async_trait]
impl ModelProviderAdapter for TestModel {
    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }
    async fn complete(
        &self,
        r: &UnifiedModelRequest,
        key: Option<&str>,
    ) -> Result<Vec<ModelEvent>, ModelError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(r.tools.is_empty());
        assert!(r.structured_output.is_some());
        assert!(key.is_none());
        tokio::time::sleep(self.delay).await;
        Ok(vec![
            ModelEvent::MessageDelta {
                text: self.raw.clone(),
            },
            ModelEvent::Completed,
        ])
    }
    async fn stream(
        &self,
        _: &UnifiedModelRequest,
        _: Option<&str>,
        _: tokio::sync::mpsc::Sender<ModelEvent>,
    ) -> Result<(), ModelError> {
        panic!("planner must not stream")
    }
}
async fn setup() -> (
    tempfile::TempDir,
    std::sync::Arc<AppRuntime>,
    MobileGoal,
    MobilePlanner,
) {
    let dir = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&dir.path().join("planner.sqlite3"))
        .await
        .unwrap();
    let goal = runtime
        .mobile_goal_repository()
        .create_goal("Read public information", MobileGoalBudget::default())
        .await
        .unwrap();
    let planner = MobilePlanner::new(
        runtime.pool().clone(),
        runtime.event_bus.clone(),
        CancellationToken::new(),
    );
    (dir, runtime, goal, planner)
}
fn provider() -> ResolvedProvider {
    ResolvedProvider {
        id: "test-provider".into(),
        model: "fixture-model".into(),
        base_url: "http://127.0.0.1".into(),
        credential_required: false,
        capabilities: ProviderCapabilities {
            chat_completions: true,
            ..Default::default()
        },
    }
}
fn model(raw: &str) -> TestModel {
    TestModel {
        raw: raw.into(),
        calls: AtomicUsize::new(0),
        delay: Duration::ZERO,
        capabilities: ProviderCapabilities {
            chat_completions: true,
            ..Default::default()
        },
    }
}
const OBSERVE: &str = r#"{"decision":"next_action","action":{"type":"observe","reason":"Read public UI","expected_result":{"kind":"UI_CHANGED"}},"completion":null,"failure":null}"#;
#[tokio::test]
async fn newer_observation_invalidates_inflight_planner_decision() {
    let (_dir, r, g, p) = setup().await;
    let old = observation_fixture(&r, false).await;
    let context = r
        .mobile_goal_repository()
        .planner_context(&g.id, Some(&old))
        .await
        .unwrap();
    let latest = observation_fixture(&r, false).await;
    assert_ne!(old, latest);
    let result = p
        .run(
            &context,
            &provider(),
            &model(OBSERVE),
            None,
            Duration::from_secs(1),
        )
        .await;
    assert!(
        result.is_err(),
        "a decision bound to an older observation must not persist"
    );
    assert!(
        r.mobile_goal_repository()
            .get_goal_plan(&g.id)
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn invalid_model_attempts_spend_the_persisted_goal_call_budget() {
    let (_d, r, g, p) = setup().await;
    let mut goal = r.mobile_goal_repository().get_goal(&g.id).await.unwrap();
    goal.step_budget.max_model_calls = 1;
    sqlx::query("UPDATE mobile_goals SET domain_json=? WHERE id=?")
        .bind(serde_json::to_string(&goal).unwrap())
        .bind(g.id.as_str())
        .execute(r.pool())
        .await
        .unwrap();
    let context = r
        .mobile_goal_repository()
        .planner_context(&g.id, None)
        .await
        .unwrap();
    let m = model("malformed JSON");
    let result = p
        .run(&context, &provider(), &m, None, Duration::from_secs(1))
        .await;
    assert!(
        matches!(result,Err(AppError::MobileGoal(e)) if e.code==MobileGoalErrorCode::StepLimitReached)
    );
    assert_eq!(m.calls.load(Ordering::SeqCst), 1);
    let calls: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_model_calls WHERE goal_id=?")
        .bind(g.id.as_str())
        .fetch_one(r.pool())
        .await
        .unwrap();
    assert_eq!(calls, 1);
    assert!(
        p.run(&context, &provider(), &m, None, Duration::from_secs(1))
            .await
            .is_err()
    );
    assert_eq!(m.calls.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn zero_provider_fails_closed_without_plans_or_steps_or_state_damage() {
    let (_dir, r, g, p) = setup().await;
    let e = p
        .plan_mobile_goal(&g.id, None, |_| panic!("no secret access"))
        .await
        .unwrap_err();
    assert!(matches!(
        e,
        AppError::MobileGoal(MobileGoalError {
            code: MobileGoalErrorCode::ModelNotConfigured,
            ..
        })
    ));
    assert!(
        r.mobile_goal_repository()
            .get_goal_plan(&g.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        r.mobile_goal_repository()
            .get_goal(&g.id)
            .await
            .unwrap()
            .status,
        MobileGoalStatus::Pending
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM mobile_plan_steps")
            .fetch_one(r.pool())
            .await
            .unwrap(),
        0
    );
}
#[tokio::test]
async fn valid_decision_persists_one_pending_step_and_metadata_without_execution() {
    let (_dir, r, g, p) = setup().await;
    let c = r
        .mobile_goal_repository()
        .planner_context(&g.id, None)
        .await
        .unwrap();
    let m = model(OBSERVE);
    let out = p
        .run(&c, &provider(), &m, None, Duration::from_secs(1))
        .await
        .unwrap();
    assert!(out.waiting_executor);
    let plan = r
        .mobile_goal_repository()
        .get_goal_plan(&g.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(plan.plan.revision, 1);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].sequence, 1);
    assert_eq!(plan.steps[0].status, MobileStepStatus::Pending);
    assert_eq!(plan.steps[0].risk, MobileStepRisk::ReadOnly);
    assert!(plan.steps[0].action_id.is_none());
    let goal = r.mobile_goal_repository().get_goal(&g.id).await.unwrap();
    assert_eq!(goal.status, MobileGoalStatus::Planning);
    assert!(goal.started_at.is_none());
    let payloads: Vec<(String, String)> =
        sqlx::query_as("SELECT event_type,payload_json FROM runtime_events WHERE aggregate_id=?")
            .bind(g.id.as_str())
            .fetch_all(r.pool())
            .await
            .unwrap();
    assert!(payloads.iter().any(|(e, _)| e == "mobile.planner_started"));
    let (_, payload) = payloads
        .iter()
        .find(|(e, _)| e == "mobile.planner_succeeded")
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(payload).unwrap();
    assert_eq!(v["plannerVersion"], "MOBILE_PLANNER_V1");
    assert_eq!(v["schemaVersion"], "mobile_next_action_v1");
    assert!(v.get("stepId").is_some());
    assert!(v.get("objective").is_none());
    assert!(
        r.mobile_goal_repository()
            .planner_context(&g.id, None)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn invalid_output_retries_twice_timeout_is_bounded_and_failures_do_not_stick_planning() {
    let (_dir, r, g, p) = setup().await;
    let c = r
        .mobile_goal_repository()
        .planner_context(&g.id, None)
        .await
        .unwrap();
    let m = model("{");
    assert!(
        p.run(&c, &provider(), &m, None, Duration::from_secs(1))
            .await
            .is_err()
    );
    assert_eq!(m.calls.load(Ordering::SeqCst), 2);
    let mut m = model(OBSERVE);
    m.delay = Duration::from_secs(1);
    let e = p
        .run(&c, &provider(), &m, None, Duration::from_millis(5))
        .await
        .unwrap_err();
    assert!(matches!(
        e,
        AppError::MobileGoal(MobileGoalError {
            code: MobileGoalErrorCode::ModelError,
            ..
        })
    ));
    assert_eq!(
        r.mobile_goal_repository()
            .get_goal(&g.id)
            .await
            .unwrap()
            .status,
        MobileGoalStatus::Pending
    );
    assert!(
        r.mobile_goal_repository()
            .get_goal_plan(&g.id)
            .await
            .unwrap()
            .is_none()
    );
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM runtime_events WHERE event_type='mobile.planner_failed'",
    )
    .fetch_one(r.pool())
    .await
    .unwrap();
    assert_eq!(n, 2);
}

async fn configured(
    r: &AppRuntime,
    id: &str,
    enabled: bool,
    base_url: &str,
    caps: ProviderCapabilities,
) {
    r.upsert_model_provider(crate::ModelProviderConfig {
        id: id.into(),
        name: id.into(),
        provider_type: "openai_compatible_chat".into(),
        base_url: base_url.into(),
        credential_ref: None,
        default_model: "fixture-model".into(),
        temperature: 0.0,
        context_window: 8192,
        enabled,
        capabilities: serde_json::to_value(caps).unwrap(),
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn provider_selection_rejects_disabled_ambiguous_and_empty_model_configs() {
    let (_dir, r, _, _) = setup().await;
    configured(&r, "p1", false, "http://127.0.0.1", provider().capabilities).await;
    assert!(matches!(
        resolve_provider(r.pool()).await,
        Err(AppError::MobileGoal(MobileGoalError {
            code: MobileGoalErrorCode::ModelNotSelected,
            ..
        }))
    ));
    configured(&r, "p1", true, "http://127.0.0.1", provider().capabilities).await;
    assert_eq!(resolve_provider(r.pool()).await.unwrap().id, "p1");
    configured(&r, "p2", true, "http://127.0.0.1", provider().capabilities).await;
    assert!(resolve_provider(r.pool()).await.is_err());
    configured(&r, "p2", false, "http://127.0.0.1", provider().capabilities).await;
    sqlx::query("UPDATE model_providers SET default_model='' WHERE id='p1'")
        .execute(r.pool())
        .await
        .unwrap();
    assert!(resolve_provider(r.pool()).await.is_err());
}
#[tokio::test]
async fn native_and_json_only_real_gateway_production_service_pass_local_validation() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for native in [true, false] {
        let (_dir, r, g, _) = setup().await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = vec![];
            let mut buffer = [0; 4096];
            loop {
                let n = socket.read(&mut buffer).await.unwrap();
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&bytes[..pos]);
                    let len = head
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= pos + 4 + len {
                        break;
                    }
                }
            }
            let body = json!({"choices":[{"message":{"content":OBSERVE},"finish_reason":"stop"}]})
                .to_string();
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            String::from_utf8(bytes).unwrap()
        });
        configured(
            &r,
            "p1",
            true,
            &url,
            ProviderCapabilities {
                chat_completions: true,
                structured_output: native,
                json_mode: !native,
                ..Default::default()
            },
        )
        .await;
        let outcome = r
            .plan_mobile_goal(&g.id, None, |_| {
                panic!("anonymous fixture must not read keyring")
            })
            .await
            .unwrap();
        assert!(outcome.waiting_executor);
        let wire = server.await.unwrap();
        let request: serde_json::Value =
            serde_json::from_str(wire.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(
            request["response_format"]["type"],
            if native { "json_schema" } else { "json_object" }
        );
        assert!(request.get("tools").is_none());
        assert_eq!(request["model"], "fixture-model");
        let context: serde_json::Value =
            serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
        assert_eq!(context["observationMissing"], true);
        assert_eq!(context["remainingSteps"], 8);
    }
}
async fn verified_fixture(r: &AppRuntime, g: &MobileGoal, p: &MobilePlanner) -> MobilePlanStep {
    let repo = r.mobile_goal_repository();
    let context = repo.planner_context(&g.id, None).await.unwrap();
    let out = p
        .run(
            &context,
            &provider(),
            &model(OBSERVE),
            None,
            Duration::from_secs(1),
        )
        .await
        .unwrap();
    if repo.get_goal(&g.id).await.unwrap().status == MobileGoalStatus::Planning {
        repo.update_goal_status(&g.id, MobileGoalStatus::Running, None)
            .await
            .unwrap();
    }
    let id = out.step_id.unwrap();
    repo.update_step_status(&id, MobileStepStatus::Executing, None)
        .await
        .unwrap();
    repo.record_step_result(MobileStepResult {
        step_id: id.clone(),
        outcome: MobileStepOutcome::Verified,
        verified: true,
        observation_before_id: None,
        observation_after_id: None,
        action_receipt_id: None,
        evidence_ids: vec![],
        error: None,
        created_at: chrono::Utc::now(),
    })
    .await
    .unwrap();
    repo.get_step(&id).await.unwrap()
}
#[tokio::test]
async fn verified_fixture_history_yields_step_two_and_preserves_revision_one() {
    let (_dir, r, g, p) = setup().await;
    let first = verified_fixture(&r, &g, &p).await;
    let repo = r.mobile_goal_repository();
    let c = repo.planner_context(&g.id, None).await.unwrap();
    assert_eq!(c.steps_used, 1);
    assert_eq!(c.recent_steps[0].step_id, first.id);
    assert_eq!(c.recent_steps[0].status, MobileStepStatus::Verified);
    p.run(
        &c,
        &provider(),
        &model(OBSERVE),
        None,
        Duration::from_secs(1),
    )
    .await
    .unwrap();
    let plan = repo.get_goal_plan(&g.id).await.unwrap().unwrap();
    assert_eq!(plan.plan.revision, 2);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].sequence, 2);
    assert_eq!(
        repo.get_step(&first.id).await.unwrap().status,
        MobileStepStatus::Verified
    );
}
#[tokio::test]
async fn exhausted_budget_prevents_gateway_or_credential_access() {
    let (_dir, r, g, p) = setup().await;
    verified_fixture(&r, &g, &p).await;
    let mut goal = r.mobile_goal_repository().get_goal(&g.id).await.unwrap();
    goal.step_budget.max_steps = 1;
    sqlx::query("UPDATE mobile_goals SET domain_json=? WHERE id=?")
        .bind(serde_json::to_string(&goal).unwrap())
        .bind(g.id.as_str())
        .execute(r.pool())
        .await
        .unwrap();
    configured(
        &r,
        "p1",
        true,
        "http://127.0.0.1:1",
        provider().capabilities,
    )
    .await;
    let e = r
        .plan_mobile_goal(&g.id, None, |_| {
            panic!("budget precheck must precede key access")
        })
        .await
        .unwrap_err();
    assert!(matches!(
        e,
        AppError::MobileGoal(MobileGoalError {
            code: MobileGoalErrorCode::StepLimitReached,
            ..
        })
    ));
}
#[tokio::test]
async fn pending_step_and_terminal_goal_prevent_network_requests() {
    let (_dir, r, g, p) = setup().await;
    let c = r
        .mobile_goal_repository()
        .planner_context(&g.id, None)
        .await
        .unwrap();
    p.run(
        &c,
        &provider(),
        &model(OBSERVE),
        None,
        Duration::from_secs(1),
    )
    .await
    .unwrap();
    configured(
        &r,
        "p1",
        true,
        "http://127.0.0.1:1",
        provider().capabilities,
    )
    .await;
    assert!(matches!(
        r.plan_mobile_goal(&g.id, None, |_| panic!()).await,
        Err(AppError::MobileGoal(MobileGoalError {
            code: MobileGoalErrorCode::InvalidStateTransition,
            ..
        }))
    ));
    r.mobile_goal_repository()
        .update_goal_status(&g.id, MobileGoalStatus::Stopped, None)
        .await
        .unwrap();
    assert!(r.plan_mobile_goal(&g.id, None, |_| panic!()).await.is_err());
}
#[tokio::test]
async fn planner_audit_failure_rolls_back_entire_plan_and_step_commit() {
    let (_dir, r, g, p) = setup().await;
    let c = r
        .mobile_goal_repository()
        .planner_context(&g.id, None)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER reject_planner_success BEFORE INSERT ON runtime_events WHEN NEW.event_type='mobile.planner_succeeded' BEGIN SELECT RAISE(ABORT,'fixture failure'); END").execute(r.pool()).await.unwrap();
    assert!(
        p.run(
            &c,
            &provider(),
            &model(OBSERVE),
            None,
            Duration::from_secs(1)
        )
        .await
        .is_err()
    );
    assert!(
        r.mobile_goal_repository()
            .get_goal_plan(&g.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        r.mobile_goal_repository()
            .get_goal(&g.id)
            .await
            .unwrap()
            .status,
        MobileGoalStatus::Pending
    );
    for table in [
        "mobile_plans",
        "mobile_plan_steps",
        "mobile_planner_decisions",
    ] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table}"))
                .fetch_one(r.pool())
                .await
                .unwrap(),
            0
        );
    }
}
#[tokio::test]
async fn concurrent_stale_decisions_can_persist_only_one_pending_step() {
    let (_dir, r, g, _p) = setup().await;
    let c = r
        .mobile_goal_repository()
        .planner_context(&g.id, None)
        .await
        .unwrap();
    let decision = decode_decision(OBSERVE, &c).unwrap();
    let repo = r.mobile_goal_repository();
    let (a, b) = tokio::join!(
        repo.persist_planner_decision(&c, decision.clone(), "p", "m", 0),
        repo.persist_planner_decision(&c, decision, "p", "m", 0)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM mobile_plan_steps WHERE status='PENDING'"
        )
        .fetch_one(r.pool())
        .await
        .unwrap(),
        1
    );
}
#[tokio::test]
async fn cancellation_drops_inflight_model_request_without_persistence() {
    let (_dir, r, g, p) = setup().await;
    let c = r
        .mobile_goal_repository()
        .planner_context(&g.id, None)
        .await
        .unwrap();
    let mut m = model(OBSERVE);
    m.delay = Duration::from_secs(10);
    let cancel = p.cancellation.clone();
    let task = async move {
        tokio::time::sleep(Duration::from_millis(5)).await;
        cancel.cancel();
    };
    let resolved = provider();
    let (out, ()) = tokio::join!(p.run(&c, &resolved, &m, None, Duration::from_secs(1)), task);
    assert!(matches!(
        out,
        Err(AppError::MobileGoal(MobileGoalError {
            code: MobileGoalErrorCode::UserStopped,
            ..
        }))
    ));
    assert!(
        r.mobile_goal_repository()
            .get_goal_plan(&g.id)
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn oversize_model_output_never_retries_or_persists() {
    let (_dir, r, g, p) = setup().await;
    let c = r
        .mobile_goal_repository()
        .planner_context(&g.id, None)
        .await
        .unwrap();
    let m = model(&"x".repeat(16_385));
    assert!(
        p.run(&c, &provider(), &m, None, Duration::from_secs(1))
            .await
            .is_err()
    );
    assert_eq!(m.calls.load(Ordering::SeqCst), 1);
    assert!(
        r.mobile_goal_repository()
            .get_goal_plan(&g.id)
            .await
            .unwrap()
            .is_none()
    );
}

async fn observation_fixture(r: &AppRuntime, sensitive: bool) -> String {
    use mobile_runtime::*;
    let now = chrono::Utc::now();
    let session_id = "fixture-session";
    let mut elements = vec![];
    for index in 0..100 {
        elements.push(RawMobileElement {
            text: Some(
                if index == 0 {
                    "fixture editable text"
                } else {
                    "Public headline"
                }
                .into(),
            ),
            role: "text".into(),
            class_name: if index == 0 {
                "android.widget.EditText"
            } else {
                "android.widget.TextView"
            }
            .into(),
            content_description: None,
            bounds: MobileBounds {
                x: 1,
                y: 1,
                width: 10,
                height: 10,
            },
            clickable: true,
            scrollable: false,
            enabled: true,
            focused: true,
            selected: false,
            resource_id: None,
            password: matches!((sensitive, index), (true, 0)),
        });
    }
    let snapshot = MobileUiSnapshot::from_elements(
        session_id,
        "com.example.news",
        "Main",
        1080,
        2400,
        elements,
    );
    let frame = MobileFrame::from_png(session_id, 1080, 2400, vec![1, 2, 3]);
    let observation = MobileObservation::from_snapshot(
        &snapshot,
        &frame,
        None,
        "generic-android",
        PrivacyClass::UserAllowed,
    );
    let id = observation.id.clone();
    r.record_mobile_capture(&MobileCapture {
        session: MobileDeviceSession {
            session_id: session_id.into(),
            device_id: "fixture-device".into(),
            platform: MobilePlatform::Android,
            device_type: MobileDeviceType::Emulator,
            os_version: "fixture".into(),
            screen_width: 1080,
            screen_height: 2400,
            connected_at: now,
            current_app: Some("com.example.news".into()),
            current_activity: Some("Main".into()),
            status: MobileSessionStatus::Connected,
            last_observation_at: Some(now),
        },
        snapshot,
        frame,
        observation,
    })
    .await
    .unwrap();
    r.save_mobile_settings(&crate::MobileRuntimeSettings {
        allowed_apps: vec!["com.example.news".into()],
        ..Default::default()
    })
    .await
    .unwrap();
    id
}
#[tokio::test]
async fn context_is_bounded_and_retains_lineage_without_sensitive_editable_content() {
    let (_dir, r, g, _) = setup().await;
    let id = observation_fixture(&r, false).await;
    let c = r
        .mobile_goal_repository()
        .planner_context(&g.id, Some(&id))
        .await
        .unwrap();
    let o = c.current_observation.as_ref().unwrap();
    assert_eq!(o.elements.len(), 80);
    assert!(o.elements[0].text.is_none());
    assert_eq!(o.observation_id, id);
    assert_eq!(o.device_session_id, "fixture-session");
    assert!(
        !serde_json::to_string(&c)
            .unwrap()
            .contains("fixture editable text")
    );
    assert!(serde_json::to_vec(&c).unwrap().len() <= 32_768);
}
#[tokio::test]
async fn sensitive_snapshot_never_sends_text_or_creates_input_step() {
    let (_dir, r, g, p) = setup().await;
    let id = observation_fixture(&r, true).await;
    let c = r
        .mobile_goal_repository()
        .planner_context(&g.id, Some(&id))
        .await
        .unwrap();
    let o = c.current_observation.as_ref().unwrap();
    assert!(o.sensitive_screen);
    assert!(o.elements.iter().all(|e| e.text.is_none()));
    let raw = r#"{"decision":"next_action","action":{"type":"input_text","reason":"Enter public query","target_element_ref":"@e1","input_text":"public","expected_result":{"kind":"UI_CHANGED"}},"completion":null,"failure":null}"#;
    let m = model(raw);
    let e = p
        .run(&c, &provider(), &m, None, Duration::from_secs(1))
        .await
        .unwrap_err();
    assert!(matches!(
        e,
        AppError::MobileGoal(MobileGoalError {
            code: MobileGoalErrorCode::PolicyBlocked,
            ..
        })
    ));
    assert_eq!(m.calls.load(Ordering::SeqCst), 1);
    assert!(
        r.mobile_goal_repository()
            .get_goal_plan(&g.id)
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn completion_and_cannot_proceed_persist_proposals_without_completing_goal() {
    let (_dir, r, g, p) = setup().await;
    let id = observation_fixture(&r, false).await;
    let c = r
        .mobile_goal_repository()
        .planner_context(&g.id, Some(&id))
        .await
        .unwrap();
    let raw=json!({"decision":"complete","action":null,"completion":{"reason":"Visible criteria satisfied","supporting_observation_ids":[id]},"failure":null}).to_string();
    let out = p
        .run(&c, &provider(), &model(&raw), None, Duration::from_secs(1))
        .await
        .unwrap();
    assert!(matches!(
        out.decision,
        MobilePlannerDecisionDto::CompletionProposal
    ));
    assert!(out.step_id.is_none());
    assert!(!out.waiting_executor);
    assert_eq!(
        r.mobile_goal_repository()
            .get_goal(&g.id)
            .await
            .unwrap()
            .status,
        MobileGoalStatus::Pending
    );
    let raw = r#"{"decision":"cannot_proceed","action":null,"completion":null,"failure":{"code":"NEEDS_NEW_OBSERVATION","reason":"A fresh screen is needed"}}"#;
    p.run(&c, &provider(), &model(raw), None, Duration::from_secs(1))
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM mobile_planner_decisions")
            .fetch_one(r.pool())
            .await
            .unwrap(),
        2
    );
    assert!(
        r.mobile_goal_repository()
            .get_goal_plan(&g.id)
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn wait_duration_and_extraction_intent_survive_step_persistence() {
    for action in [
        json!({"type":"wait","reason":"Await public UI","wait_ms":5000,"expected_result":{"kind":"NO_CHANGE_EXPECTED"}}),
        json!({"type":"extract","reason":"Inspect public facts","extraction_intent":"Read visible facts","expected_result":{"kind":"NEW_DATA_OBJECT"}}),
    ] {
        let (_dir, r, g, p) = setup().await;
        let id = observation_fixture(&r, false).await;
        let c = r
            .mobile_goal_repository()
            .planner_context(&g.id, Some(&id))
            .await
            .unwrap();
        let raw =
            json!({"decision":"next_action","action":action,"completion":null,"failure":null})
                .to_string();
        let out = p
            .run(&c, &provider(), &model(&raw), None, Duration::from_secs(1))
            .await
            .unwrap();
        let step = r
            .mobile_goal_repository()
            .get_step(&out.step_id.unwrap())
            .await
            .unwrap();
        assert_eq!(step.risk, MobileStepRisk::ReadOnly);
        assert_eq!(step.observation_before_id, Some(id));
        if step.step_type == MobileStepType::Wait {
            assert_eq!(step.wait_ms, Some(5000))
        } else {
            assert_eq!(
                step.extraction_intent.as_deref(),
                Some("Read visible facts")
            )
        };
        assert!(step.action_id.is_none());
    }
}
#[tokio::test]
async fn history_limits_five_recent_steps_and_counts_all_used_steps() {
    let (_dir, r, g, p) = setup().await;
    for _ in 0..6 {
        verified_fixture(&r, &g, &p).await;
    }
    let c = r
        .mobile_goal_repository()
        .planner_context(&g.id, None)
        .await
        .unwrap();
    assert_eq!(c.steps_used, 6);
    assert_eq!(c.remaining_steps, 2);
    assert_eq!(c.recent_steps.len(), 5);
    assert_eq!(c.recent_steps[0].sequence, 2);
    assert_eq!(c.recent_steps[4].sequence, 6);
}
#[tokio::test]
async fn shutdown_revokes_future_planner_requests_before_key_access() {
    let (_dir, r, g, _) = setup().await;
    r.shutdown().await.unwrap();
    let e = r
        .plan_mobile_goal(&g.id, None, |_| panic!("no key access after shutdown"))
        .await
        .unwrap_err();
    assert!(matches!(
        e,
        AppError::MobileGoal(MobileGoalError {
            code: MobileGoalErrorCode::UserStopped,
            ..
        })
    ));
}

#[tokio::test]
async fn sensitive_observation_class_blocks_context_even_when_snapshot_labels_are_public() {
    let (_dir, r, g, _) = setup().await;
    let id = observation_fixture(&r, false).await;
    sqlx::query("UPDATE mobile_observations SET domain_json=json_set(domain_json,'$.privacyClass','sensitive') WHERE id=?").bind(&id).execute(r.pool()).await.unwrap();
    let result = r
        .mobile_goal_repository()
        .planner_context(&g.id, Some(&id))
        .await;
    assert!(matches!(
        result,
        Err(AppError::MobileGoal(MobileGoalError {
            code: MobileGoalErrorCode::PolicyBlocked,
            ..
        }))
    ));
}

#[tokio::test]
async fn planner_keeps_full_safe_goal_objective_instead_of_truncating_completion_criteria() {
    let (_dir, r, _, _) = setup().await;
    let objective = "观察公开页面信息".repeat(70);
    let goal = r
        .mobile_goal_repository()
        .create_goal(&objective, MobileGoalBudget::default())
        .await
        .unwrap();
    let context = r
        .mobile_goal_repository()
        .planner_context(&goal.id, None)
        .await
        .unwrap();
    assert_eq!(context.objective, objective);
}
