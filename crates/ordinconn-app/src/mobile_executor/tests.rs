use super::*;
use chrono::Utc;
use mobile_runtime::*;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
fn capture(value: &str) -> MobileCapture {
    let s = "executor-fixture-session";
    let snapshot = MobileUiSnapshot::from_elements(
        s,
        "com.android.settings",
        "Settings",
        1080,
        2400,
        vec![RawMobileElement {
            text: Some(value.into()),
            role: "text".into(),
            class_name: "android.widget.TextView".into(),
            content_description: None,
            bounds: MobileBounds {
                x: 10,
                y: 150,
                width: 900,
                height: 80,
            },
            clickable: true,
            scrollable: false,
            enabled: true,
            focused: false,
            selected: false,
            resource_id: Some("com.android.settings:id/title".into()),
            password: false,
        }],
    );
    let frame = MobileFrame::from_png(s, 1080, 2400, vec![1, 2, 3]);
    let observation = MobileObservation::from_snapshot(
        &snapshot,
        &frame,
        None,
        "generic-android",
        PrivacyClass::UserAllowed,
    );
    MobileCapture {
        session: MobileDeviceSession {
            session_id: s.into(),
            device_id: "emulator-5554".into(),
            platform: MobilePlatform::Android,
            device_type: MobileDeviceType::Emulator,
            os_version: "16".into(),
            screen_width: 1080,
            screen_height: 2400,
            connected_at: Utc::now(),
            current_app: Some("com.android.settings".into()),
            current_activity: Some("Settings".into()),
            status: MobileSessionStatus::Connected,
            last_observation_at: Some(Utc::now()),
        },
        snapshot,
        frame,
        observation,
    }
}
struct Harness {
    latest: Mutex<MobileCapture>,
    after: Mutex<MobileCapture>,
    actions: AtomicUsize,
    fail_action: bool,
    fail_observe: bool,
    no_after: bool,
    delay_ms: u64,
    cancel_on_action: Mutex<Option<tokio_util::sync::CancellationToken>>,
}
impl Harness {
    fn new(before: MobileCapture, after: MobileCapture) -> Arc<Self> {
        Arc::new(Self {
            latest: Mutex::new(before),
            after: Mutex::new(after),
            actions: AtomicUsize::new(0),
            fail_action: false,
            fail_observe: false,
            no_after: false,
            delay_ms: 0,
            cancel_on_action: Mutex::new(None),
        })
    }
}
#[async_trait::async_trait]
impl MobileExecutorRuntime for Harness {
    fn current_capture(&self) -> Option<MobileCapture> {
        Some(self.latest.lock().unwrap().clone())
    }
    async fn observe(&self) -> Result<MobileCapture, MobileGoalError> {
        if self.delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        }
        if self.fail_observe {
            return Err(MobileGoalError::new(MobileGoalErrorCode::ObserveFailed));
        }
        let mut c = self.latest.lock().unwrap().clone();
        c.snapshot.snapshot_id = format!("snapshot_{}", uuid::Uuid::now_v7());
        c.snapshot.captured_at = Utc::now();
        c.observation = MobileObservation::from_snapshot(
            &c.snapshot,
            &c.frame,
            None,
            "generic-android",
            PrivacyClass::UserAllowed,
        );
        *self.latest.lock().unwrap() = c.clone();
        Ok(c)
    }
    async fn action(
        &self,
        r: MobileActionRequest,
        _lease: Arc<DeviceExecutionLease>,
        progress: ExecutorProgressSender,
    ) -> Result<ExecutorAction, MobileGoalError> {
        self.actions.fetch_add(1, Ordering::SeqCst);
        if self.fail_action {
            return Err(MobileGoalError::new(MobileGoalErrorCode::ActionFailed));
        }
        for stage in [
            ExecutorActionStage::ActionCompleted,
            ExecutorActionStage::ObserveAfterStarted,
        ] {
            let (acknowledged, ack) = tokio::sync::oneshot::channel();
            progress
                .send(ExecutorProgress {
                    stage,
                    acknowledged,
                })
                .unwrap();
            let _ = ack.await;
        }
        let b = self.latest.lock().unwrap().clone();
        let mut a = self.after.lock().unwrap().clone();
        a.snapshot.snapshot_id = format!("snapshot_{}", uuid::Uuid::now_v7());
        a.snapshot.captured_at = Utc::now();
        a.observation = MobileObservation::from_snapshot(
            &a.snapshot,
            &a.frame,
            None,
            "generic-android",
            PrivacyClass::UserAllowed,
        );
        *self.latest.lock().unwrap() = a.clone();
        let receipt = MobileActionReceipt {
            action_id: r.action_id,
            session_id: r.session_id,
            snapshot_id: r.snapshot_id,
            target: r.target,
            decision: MobileActionDecision::Allowed,
            status: MobileActionStatus::Executed,
            requested_at: r.requested_at,
            completed_at: Utc::now(),
            pre_package: b.snapshot.package_name.clone(),
            pre_activity: b.snapshot.activity.clone(),
            pre_frame_hash: b.observation.frame_hash.clone(),
            pre_ui_tree_hash: b.observation.ui_tree_hash.clone(),
            post_package: Some(a.snapshot.package_name.clone()),
            post_snapshot_id: Some(a.snapshot.snapshot_id.clone()),
            post_activity: Some(a.snapshot.activity.clone()),
            post_frame_hash: Some(a.observation.frame_hash.clone()),
            post_ui_tree_hash: Some(a.observation.ui_tree_hash.clone()),
            verification: if self.no_after {
                None
            } else {
                Some(VerificationResult::Verified)
            },
            text_length: r.text.as_ref().map(|t| t.len()),
            text_sha256: r.text.as_ref().map(|t| t.sha256()),
            command_sent: true,
            input_value_verified: None,
        };
        if let Some(token) = self.cancel_on_action.lock().unwrap().as_ref() {
            token.cancel();
        }
        Ok(ExecutorAction {
            receipt,
            capture: if self.no_after { None } else { Some(a) },
        })
    }
}
async fn prepared(
    kind: MobileStepType,
    expected: ExpectedStepResult,
) -> (
    tempfile::TempDir,
    Arc<AppRuntime>,
    MobileGoal,
    MobilePlanStep,
    Arc<Harness>,
) {
    let d = tempfile::tempdir().unwrap();
    let rt = AppRuntime::initialize(&d.path().join("executor.sqlite3"))
        .await
        .unwrap();
    rt.save_mobile_settings(&crate::MobileRuntimeSettings {
        allowed_apps: vec!["com.android.settings".into()],
        ..Default::default()
    })
    .await
    .unwrap();
    let repo = rt.mobile_goal_repository();
    let g = repo
        .create_goal(
            "Scroll down the current Settings page once",
            Default::default(),
        )
        .await
        .unwrap();
    repo.update_goal_status(&g.id, MobileGoalStatus::Planning, None)
        .await
        .unwrap();
    let p = repo
        .create_plan(
            &g.id,
            1,
            &g.objective,
            vec![NewMobileStep {
                sequence: 1,
                step_type: kind,
                reason: "Safe Settings navigation".into(),
                risk: if matches!(
                    kind,
                    MobileStepType::Observe | MobileStepType::Wait | MobileStepType::Extract
                ) {
                    MobileStepRisk::ReadOnly
                } else {
                    MobileStepRisk::Reversible
                },
                target_ref: None,
                input_text: None,
                expected_result: Some(expected),
                wait_ms: if kind == MobileStepType::Wait {
                    Some(1)
                } else {
                    None
                },
                extraction_intent: if kind == MobileStepType::Extract {
                    Some("Visible Settings labels".into())
                } else {
                    None
                },
            }],
        )
        .await
        .unwrap();
    repo.activate_plan(&p.id).await.unwrap();
    let s = repo.list_steps(&p.id).await.unwrap().remove(0);
    let b = capture("Before page");
    let a = capture("After page");
    rt.record_mobile_capture(&b).await.unwrap();
    repo.attach_step_observations(&s.id, Some(b.observation.id.clone()), None)
        .await
        .unwrap();
    (d, rt, g, s, Harness::new(b, a))
}
#[tokio::test]
async fn one_swipe_persists_real_trace_result_and_running_goal_then_stops() {
    let (_d, rt, g, s, h) =
        prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
    let out = rt.execute_mobile_goal_step(&g.id, h.clone()).await.unwrap();
    assert!(out.verified);
    assert_eq!(out.status, MobileStepStatus::Verified);
    assert!(
        out.execution_id.is_some()
            && out.observation_before_id.is_some()
            && out.observation_after_id.is_some()
            && out.action_receipt_id.is_some()
    );
    let stored = rt
        .mobile_goal_repository()
        .get_step_result(&s.id)
        .await
        .unwrap()
        .unwrap();
    assert!(stored.verified);
    let goal = rt.mobile_goal_repository().get_goal(&g.id).await.unwrap();
    assert_eq!(goal.status, MobileGoalStatus::Running);
    assert!(goal.started_at.is_some());
    assert_eq!(h.actions.load(Ordering::SeqCst), 1);
    assert!(rt.execute_mobile_goal_step(&g.id, h.clone()).await.is_err());
    assert_eq!(h.actions.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn successful_receipt_with_wrong_expected_state_fails_the_step_and_goal() {
    let (_d, rt, g, s, h) = prepared(
        MobileStepType::Back,
        ExpectedStepResult::ActivityEquals {
            package: "com.android.settings".into(),
            activity: "ExpectedTarget".into(),
        },
    )
    .await;
    let out = rt.execute_mobile_goal_step(&g.id, h.clone()).await.unwrap();
    assert!(!out.verified);
    assert_eq!(out.error.unwrap().code, MobileGoalErrorCode::VerifyFailed);
    assert_eq!(
        rt.mobile_goal_repository()
            .get_step(&s.id)
            .await
            .unwrap()
            .status,
        MobileStepStatus::Failed
    );
    assert_eq!(
        rt.mobile_goal_repository()
            .get_goal(&g.id)
            .await
            .unwrap()
            .status,
        MobileGoalStatus::Failed
    );
}
#[tokio::test]
async fn observe_and_wait_are_bounded_steps_without_device_mutation() {
    for k in [MobileStepType::Observe, MobileStepType::Wait] {
        let (_d, rt, g, _s, h) = prepared(k, ExpectedStepResult::NoChangeExpected).await;
        let out = rt.execute_mobile_goal_step(&g.id, h.clone()).await.unwrap();
        assert!(out.verified);
        assert_eq!(h.actions.load(Ordering::SeqCst), 0);
    }
}
#[test]
fn leases_exclude_other_goals_release_on_drop_and_survive_poison() {
    let leases = DeviceExecutionLeases::default();
    let g = MobileGoalId::new();
    let first = leases.acquire("emulator-5554", Some(&g), "one").unwrap();
    assert!(
        leases
            .acquire("emulator-5554", Some(&MobileGoalId::new()), "two")
            .is_err()
    );
    assert!(
        leases
            .acquire("emulator-5556", Some(&g), "different-device")
            .is_ok()
    );
    drop(first);
    assert!(
        leases
            .acquire("emulator-5554", Some(&g), "after-release")
            .is_ok()
    );
    let panicked = std::panic::catch_unwind(|| {
        let _lease = leases
            .acquire("emulator-5554", Some(&g), "panic-owner")
            .unwrap();
        panic!("fixture executor panic");
    });
    assert!(panicked.is_err());
    assert!(
        leases
            .acquire("emulator-5554", Some(&g), "after-panic")
            .is_ok()
    );
    let _ = std::panic::catch_unwind(|| {
        let _owners = leases.owners.lock().unwrap();
        panic!("fixture lease mutex poison");
    });
    assert!(
        leases
            .acquire("emulator-5554", Some(&g), "after-poison")
            .is_ok()
    );
}
#[tokio::test]
async fn extraction_adds_a_source_backed_object_evidence_and_feed_projection() {
    let (_d, rt, g, s, h) =
        prepared(MobileStepType::Extract, ExpectedStepResult::NewDataObject).await;
    let initial = rt.mobile_workspace_data().await.unwrap().feed.len();
    let out = rt.execute_mobile_goal_step(&g.id, h.clone()).await.unwrap();
    assert!(out.verified);
    assert!(out.data_object_id.is_some());
    assert_eq!(out.evidence_ids.len(), 1);
    let feed = rt.mobile_workspace_data().await.unwrap().feed;
    let object = feed
        .iter()
        .find(|x| Some(&x.id) == out.data_object_id.as_ref())
        .unwrap();
    assert_eq!(object.data_type, "mobile_observation_object");
    assert!(object.mobile_observation_id.is_some());
    assert_eq!(object.evidence_ids, out.evidence_ids);
    assert!(feed.len() > initial);
    assert_eq!(
        rt.mobile_goal_repository()
            .get_step_evidence(&s.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(h.actions.load(Ordering::SeqCst), 0);
}

async fn rewrite_goal(rt: &AppRuntime, g: &MobileGoal, change: impl FnOnce(&mut MobileGoal)) {
    let mut g = rt.mobile_goal_repository().get_goal(&g.id).await.unwrap();
    change(&mut g);
    sqlx::query("UPDATE mobile_goals SET domain_json=? WHERE id=?")
        .bind(serde_json::to_string(&g).unwrap())
        .bind(g.id.as_str())
        .execute(rt.pool())
        .await
        .unwrap();
}
async fn rewrite_step(
    rt: &AppRuntime,
    s: &MobilePlanStep,
    change: impl FnOnce(&mut MobilePlanStep),
) {
    let mut s = rt.mobile_goal_repository().get_step(&s.id).await.unwrap();
    change(&mut s);
    sqlx::query("UPDATE mobile_plan_steps SET risk=?,sequence=?,domain_json=? WHERE id=?")
        .bind(
            format!("{:?}", s.risk)
                .to_uppercase()
                .replace("APPROVALREQUIRED", "APPROVAL_REQUIRED")
                .replace("READONLY", "READ_ONLY"),
        )
        .bind(s.sequence)
        .bind(serde_json::to_string(&s).unwrap())
        .bind(s.id.as_str())
        .execute(rt.pool())
        .await
        .unwrap();
}
#[tokio::test]
async fn missing_terminal_inactive_and_non_next_goals_fail_before_runtime_action() {
    let (_d, rt, g, s, h) =
        prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
    assert!(
        rt.execute_mobile_goal_step(&MobileGoalId::new(), h.clone())
            .await
            .is_err()
    );
    rt.mobile_goal_repository()
        .update_goal_status(&g.id, MobileGoalStatus::Stopped, None)
        .await
        .unwrap();
    assert!(rt.execute_mobile_goal_step(&g.id, h.clone()).await.is_err());
    assert_eq!(h.actions.load(Ordering::SeqCst), 0);
    let (_d, rt, g, s2, h) =
        prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
    sqlx::query("UPDATE mobile_plans SET status='FAILED',domain_json=json_set(domain_json,'$.status','FAILED') WHERE id=?").bind(s2.plan_id.as_str()).execute(rt.pool()).await.unwrap();
    assert!(rt.execute_mobile_goal_step(&g.id, h.clone()).await.is_err());
    assert_eq!(h.actions.load(Ordering::SeqCst), 0);
    let _ = s;
    let (_d, rt, g, _s, h) = prepared(
        MobileStepType::Observe,
        ExpectedStepResult::NoChangeExpected,
    )
    .await;
    let repo = rt.mobile_goal_repository();
    let plan = repo
        .create_plan(
            &g.id,
            2,
            &g.objective,
            (1..=2)
                .map(|sequence| NewMobileStep {
                    sequence,
                    step_type: MobileStepType::Observe,
                    reason: "Ordered observation step".into(),
                    risk: MobileStepRisk::ReadOnly,
                    target_ref: None,
                    input_text: None,
                    expected_result: Some(ExpectedStepResult::NoChangeExpected),
                    wait_ms: None,
                    extraction_intent: None,
                })
                .collect(),
        )
        .await
        .unwrap();
    repo.activate_plan(&plan.id).await.unwrap();
    let ordered = repo.list_steps(&plan.id).await.unwrap();
    let capture = h.current_capture().unwrap();
    for step in &ordered {
        repo.attach_step_observations(&step.id, Some(capture.observation.id.clone()), None)
            .await
            .unwrap();
    }
    let mut forged = repo.executor_snapshot(&g.id).await.unwrap();
    forged.step = ordered[1].clone();
    assert!(
        repo.executor_claim(&forged, &capture, "non-next-claim")
            .await
            .is_err()
    );
    let first = rt.execute_mobile_goal_step(&g.id, h.clone()).await.unwrap();
    assert_eq!(first.step_id, ordered[0].id);
    assert_eq!(
        repo.get_step(&ordered[1].id).await.unwrap().status,
        MobileStepStatus::Pending
    );
    assert_eq!(h.actions.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn all_budget_limits_stop_before_an_action() {
    for code in [
        MobileGoalErrorCode::StepLimitReached,
        MobileGoalErrorCode::TimeLimitReached,
        MobileGoalErrorCode::Stalled,
    ] {
        let (_d, rt, g, s, h) =
            prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
        match code {
            MobileGoalErrorCode::StepLimitReached => {
                rewrite_step(&rt, &s, |s| s.sequence = 9).await
            }
            MobileGoalErrorCode::TimeLimitReached => {
                rewrite_goal(
                    &rt,
                    &rt.mobile_goal_repository().get_goal(&g.id).await.unwrap(),
                    |g| g.runtime_deadline = Some(Utc::now() - chrono::Duration::seconds(1)),
                )
                .await
            }
            _ => {
                rewrite_goal(
                    &rt,
                    &rt.mobile_goal_repository().get_goal(&g.id).await.unwrap(),
                    |g| g.identical_observation_count = 3,
                )
                .await
            }
        };
        let e = rt
            .execute_mobile_goal_step(&g.id, h.clone())
            .await
            .unwrap_err();
        assert!(matches!(e,AppError::MobileGoal(e) if e.code==code));
        assert_eq!(h.actions.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn approval_and_forbidden_risks_never_call_mobile_runtime() {
    for risk in [MobileStepRisk::ApprovalRequired, MobileStepRisk::Forbidden] {
        let (_d, rt, g, s, h) =
            prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
        rewrite_step(&rt, &s, |s| s.risk = risk).await;
        let out = rt.execute_mobile_goal_step(&g.id, h.clone()).await;
        if risk == MobileStepRisk::ApprovalRequired {
            assert_eq!(out.unwrap().status, MobileStepStatus::WaitingApproval);
            assert!(
                rt.mobile_goal_repository()
                    .get_goal(&g.id)
                    .await
                    .unwrap()
                    .started_at
                    .is_none()
            );
        } else {
            assert!(
                matches!(out,Err(AppError::MobileGoal(e)) if e.code==MobileGoalErrorCode::PolicyBlocked)
            );
        }
        assert_eq!(h.actions.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn concurrent_duplicate_ipc_cannot_emit_two_actions() {
    let (_d, rt, g, _s, h) =
        prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
    let (a, b) = tokio::join!(
        rt.execute_mobile_goal_step(&g.id, h.clone()),
        rt.execute_mobile_goal_step(&g.id, h.clone())
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert_eq!(h.actions.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn owner_approval_is_exact_bound_consumed_once_and_cannot_replay() {
    let (_d, rt, g, s, h) = prepared(
        MobileStepType::Back,
        ExpectedStepResult::ActivityEquals {
            package: "com.android.settings".into(),
            activity: "Settings".into(),
        },
    )
    .await;
    // Install the explicit owner target in this already-planned fixture.
    sqlx::query("INSERT INTO mobile_goal_completion_targets(goal_id,target_json) VALUES (?,?)")
        .bind(g.id.as_str())
        .bind(
            serde_json::to_string(&MobileCompletionTarget::ActivityEquals {
                package: "com.android.settings".into(),
                activity: "Settings".into(),
            })
            .unwrap(),
        )
        .execute(rt.pool())
        .await
        .unwrap();
    rewrite_step(&rt, &s, |s| s.risk = MobileStepRisk::ApprovalRequired).await;
    let waiting = rt.execute_mobile_goal_step(&g.id, h.clone()).await.unwrap();
    assert_eq!(waiting.status, MobileStepStatus::WaitingApproval);
    assert_eq!(h.actions.load(Ordering::SeqCst), 0);
    let approval = rt
        .mobile_goal_repository()
        .get_step_approval(&s.id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        rt.approve_mobile_step(&g.id, &s.id, "wrong-request", h.clone())
            .await
            .is_err()
    );
    let executed = rt
        .approve_mobile_step(&g.id, &s.id, &approval.id, h.clone())
        .await
        .unwrap();
    assert!(
        executed.verified,
        "approval execution error: {:?}",
        executed.error
    );
    assert_eq!(h.actions.load(Ordering::SeqCst), 1);
    assert_eq!(
        rt.mobile_goal_repository()
            .get_goal(&g.id)
            .await
            .unwrap()
            .status,
        MobileGoalStatus::Completed
    );
    assert!(
        rt.approve_mobile_step(&g.id, &s.id, &approval.id, h.clone())
            .await
            .is_err()
    );
    assert!(rt.execute_mobile_goal_step(&g.id, h.clone()).await.is_err());
    assert_eq!(h.actions.load(Ordering::SeqCst), 1);
    let consumed = rt
        .mobile_goal_repository()
        .get_step_approval(&s.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(consumed.token_state, approval_engine::TokenState::Consumed);
}

#[tokio::test]
async fn mobile_approval_rejects_a_new_observation_and_restart() {
    for restart in [false, true] {
        let (_d, rt, g, s, h) =
            prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
        rewrite_step(&rt, &s, |s| s.risk = MobileStepRisk::ApprovalRequired).await;
        rt.execute_mobile_goal_step(&g.id, h.clone()).await.unwrap();
        let approval = rt
            .mobile_goal_repository()
            .get_step_approval(&s.id)
            .await
            .unwrap()
            .unwrap();
        if restart {
            rt.mobile_goal_repository()
                .recover_interrupted()
                .await
                .unwrap();
        } else {
            rt.record_mobile_capture(&h.observe().await.unwrap())
                .await
                .unwrap();
        }
        assert!(
            rt.approve_mobile_step(&g.id, &s.id, &approval.id, h.clone())
                .await
                .is_err()
        );
        assert_eq!(h.actions.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn claimed_step_recovery_fails_closed_and_never_replays() {
    let (d, rt, g, s, h) =
        prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
    let repo = rt.mobile_goal_repository();
    let snapshot = repo.executor_snapshot(&g.id).await.unwrap();
    repo.executor_claim(
        &snapshot,
        &h.current_capture().unwrap(),
        "crashed-execution",
    )
    .await
    .unwrap();
    rt.pool().close().await;
    let reopened = AppRuntime::initialize(&d.path().join("executor.sqlite3"))
        .await
        .unwrap();
    assert_eq!(
        reopened
            .mobile_goal_repository()
            .get_step(&s.id)
            .await
            .unwrap()
            .error_code,
        Some(MobileGoalErrorCode::InterruptedByRestart)
    );
    let status: String = sqlx::query_scalar(
        "SELECT status FROM mobile_executor_attempts WHERE id='crashed-execution'",
    )
    .fetch_one(reopened.pool())
    .await
    .unwrap();
    assert_eq!(status, "interrupted");
    assert!(
        reopened
            .execute_mobile_goal_step(&g.id, h.clone())
            .await
            .is_err()
    );
    assert_eq!(h.actions.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn stale_planned_context_is_not_rebound_to_an_unrelated_screen() {
    let (_d, rt, g, _s, h) =
        prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
    h.latest.lock().unwrap().snapshot.elements[0].text = Some("Other page".into());
    let out = rt.execute_mobile_goal_step(&g.id, h.clone()).await.unwrap();
    assert_eq!(out.error.unwrap().code, MobileGoalErrorCode::InvalidPlan);
    assert_eq!(h.actions.load(Ordering::SeqCst), 0);
    assert!(
        rt.device_execution_leases()
            .acquire("emulator-5554", None, "released")
            .is_ok()
    );
}
#[tokio::test]
async fn final_audit_failure_rolls_back_result_without_replaying_the_external_action() {
    let (_d, rt, g, s, h) =
        prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
    sqlx::query("CREATE TRIGGER reject_executor_result BEFORE INSERT ON runtime_events WHEN NEW.event_type='mobile.executor_step_completed' BEGIN SELECT RAISE(ABORT,'fixture audit failure'); END").execute(rt.pool()).await.unwrap();
    assert!(rt.execute_mobile_goal_step(&g.id, h.clone()).await.is_err());
    assert_eq!(
        rt.mobile_goal_repository()
            .get_step(&s.id)
            .await
            .unwrap()
            .status,
        MobileStepStatus::Executing
    );
    assert!(
        rt.mobile_goal_repository()
            .get_step_result(&s.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(h.actions.load(Ordering::SeqCst), 1);
    assert!(rt.execute_mobile_goal_step(&g.id, h.clone()).await.is_err());
    assert_eq!(h.actions.load(Ordering::SeqCst), 1);
    assert!(
        rt.device_execution_leases()
            .acquire("emulator-5554", None, "released")
            .is_ok()
    );
}
#[tokio::test]
async fn completion_requires_user_bound_typed_support_and_owned_verified_observations() {
    let (_d, rt, g, s, h) =
        prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
    let proposal = mobile_runtime::planner::MobileCompletionProposal {
        reason: "A bounded navigation goal was reached".into(),
        supporting_observation_ids: vec![h.current_capture().unwrap().observation.id],
    };
    assert!(
        rt.verify_mobile_goal_completion(&g.id, &proposal)
            .await
            .is_err()
    );
    rt.mobile_goal_repository()
        .set_mobile_completion_criterion(&g.id, &s.id)
        .await
        .unwrap();
    assert!(
        rt.verify_mobile_goal_completion(&g.id, &proposal)
            .await
            .is_err()
    );
    let out = rt.execute_mobile_goal_step(&g.id, h.clone()).await.unwrap();
    let proposal = mobile_runtime::planner::MobileCompletionProposal {
        reason: "A bounded navigation goal was reached".into(),
        supporting_observation_ids: vec![
            out.observation_before_id.clone().unwrap(),
            out.observation_after_id.clone().unwrap(),
        ],
    };
    let mut fabricated = proposal.clone();
    fabricated
        .supporting_observation_ids
        .push("unowned-observation".into());
    assert!(
        rt.verify_mobile_goal_completion(&g.id, &fabricated)
            .await
            .is_err()
    );
    let completed = rt
        .verify_mobile_goal_completion(&g.id, &proposal)
        .await
        .unwrap();
    assert_eq!(completed.status, MobileGoalStatus::Completed);
}
#[tokio::test]
async fn executor_events_preserve_phase_order_and_do_not_contain_screen_text() {
    let (_d, rt, g, _s, h) =
        prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
    rt.execute_mobile_goal_step(&g.id, h).await.unwrap();
    let events = rt.audit_timeline(g.id.as_str()).await.unwrap();
    let names: Vec<_> = events
        .iter()
        .map(|e| e.event_type.as_str())
        .filter(|n| n.starts_with("mobile.executor_"))
        .collect();
    assert_eq!(
        names,
        vec![
            "mobile.executor_step_claimed",
            "mobile.executor_observe_before_started",
            "mobile.executor_observe_before_completed",
            "mobile.executor_action_started",
            "mobile.executor_action_completed",
            "mobile.executor_observe_after_started",
            "mobile.executor_observe_after_completed",
            "mobile.executor_verify_started",
            "mobile.executor_verify_succeeded",
            "mobile.executor_step_completed"
        ]
    );
    for event in events {
        assert!(!event.payload_json.contains("Before page"));
        assert!(!event.payload_json.contains("After page"));
    }
}

#[tokio::test]
async fn observe_action_and_missing_after_failures_release_the_device_without_verification() {
    for variant in 0..3 {
        let (_d, rt, g, s, mut h) =
            prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
        let harness = Arc::get_mut(&mut h).unwrap();
        match variant {
            0 => harness.fail_observe = true,
            1 => harness.fail_action = true,
            _ => harness.no_after = true,
        };
        let out = rt.execute_mobile_goal_step(&g.id, h.clone()).await.unwrap();
        assert!(!out.verified);
        assert_eq!(out.status, MobileStepStatus::Failed);
        assert_eq!(
            rt.mobile_goal_repository()
                .get_goal(&g.id)
                .await
                .unwrap()
                .status,
            MobileGoalStatus::Failed
        );
        assert!(
            rt.mobile_goal_repository()
                .get_step_result(&s.id)
                .await
                .unwrap()
                .unwrap()
                .error
                .is_some()
        );
        assert!(
            rt.device_execution_leases()
                .acquire("emulator-5554", None, "after-failure")
                .is_ok()
        );
        assert_eq!(
            h.actions.load(Ordering::SeqCst),
            if variant == 0 { 0 } else { 1 }
        );
        assert!(rt.execute_mobile_goal_step(&g.id, h.clone()).await.is_err());
    }
}
#[tokio::test]
async fn another_goal_device_owner_blocks_action_and_claim_audit_failure_rolls_back() {
    let (_d, rt, g, s, h) =
        prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
    let other = MobileGoalId::new();
    let lease = rt
        .device_execution_leases()
        .acquire("emulator-5554", Some(&other), "other-goal")
        .unwrap();
    assert!(rt.execute_mobile_goal_step(&g.id, h.clone()).await.is_err());
    assert_eq!(h.actions.load(Ordering::SeqCst), 0);
    assert_eq!(
        rt.mobile_goal_repository()
            .get_step(&s.id)
            .await
            .unwrap()
            .status,
        MobileStepStatus::Pending
    );
    drop(lease);
    sqlx::query("CREATE TRIGGER reject_claim_audit BEFORE INSERT ON runtime_events WHEN NEW.event_type='mobile.executor_step_claimed' BEGIN SELECT RAISE(ABORT,'fixture audit failure'); END").execute(rt.pool()).await.unwrap();
    assert!(rt.execute_mobile_goal_step(&g.id, h.clone()).await.is_err());
    assert_eq!(
        rt.mobile_goal_repository()
            .get_step(&s.id)
            .await
            .unwrap()
            .status,
        MobileStepStatus::Pending
    );
    assert!(
        rt.mobile_goal_repository()
            .get_goal(&g.id)
            .await
            .unwrap()
            .started_at
            .is_none()
    );
    assert_eq!(h.actions.load(Ordering::SeqCst), 0);
    assert!(
        rt.device_execution_leases()
            .acquire("emulator-5554", None, "released")
            .is_ok()
    );
}
#[tokio::test]
async fn cancellation_and_elapsed_observe_budget_prevent_action_and_bounded_wait() {
    for variant in 0..3 {
        let (_d, rt, g, _s, mut h) = prepared(
            if variant == 2 {
                MobileStepType::Wait
            } else {
                MobileStepType::ScrollDown
            },
            if variant == 2 {
                ExpectedStepResult::NoChangeExpected
            } else {
                ExpectedStepResult::UiChanged
            },
        )
        .await;
        if variant == 0 {
            rt.planner_cancellation.cancel();
            assert!(rt.execute_mobile_goal_step(&g.id, h.clone()).await.is_err());
        } else {
            Arc::get_mut(&mut h).unwrap().delay_ms = 20;
            rewrite_goal(&rt, &g, |g| g.step_budget.max_runtime_ms = 1).await;
            let out = rt.execute_mobile_goal_step(&g.id, h.clone()).await.unwrap();
            assert_eq!(
                out.error.unwrap().code,
                MobileGoalErrorCode::TimeLimitReached
            );
        }
        assert_eq!(h.actions.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn extraction_audit_failure_rolls_back_object_evidence_and_metrics() {
    let (_d, rt, g, s, h) =
        prepared(MobileStepType::Extract, ExpectedStepResult::NewDataObject).await;
    let count = rt
        .mobile_workspace_data()
        .await
        .unwrap()
        .feed
        .iter()
        .filter(|o| o.data_type == "mobile_observation_object")
        .count();
    sqlx::query("CREATE TRIGGER reject_extract_event BEFORE INSERT ON runtime_events WHEN NEW.event_type='mobile.data_updated' BEGIN SELECT RAISE(ABORT,'fixture audit failure'); END").execute(rt.pool()).await.unwrap();
    assert!(rt.execute_mobile_goal_step(&g.id, h.clone()).await.is_err());
    assert_eq!(
        rt.mobile_workspace_data()
            .await
            .unwrap()
            .feed
            .iter()
            .filter(|o| o.data_type == "mobile_observation_object")
            .count(),
        count
    );
    assert!(
        rt.mobile_goal_repository()
            .get_step_result(&s.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        rt.mobile_goal_repository()
            .get_step_evidence(&s.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(h.actions.load(Ordering::SeqCst), 0);
}

// Local HTTP fixtures exercise the production Gateway/Planner/Executor wiring,
// without claiming a live provider or real-device autonomous acceptance.
async fn runner_fixture() -> (tempfile::TempDir, Arc<AppRuntime>, MobileGoal, Arc<Harness>) {
    let dir = tempfile::tempdir().unwrap();
    let app = AppRuntime::initialize(&dir.path().join("runner.sqlite3"))
        .await
        .unwrap();
    app.save_mobile_settings(&crate::MobileRuntimeSettings {
        allowed_apps: vec!["com.android.settings".into()],
        ..Default::default()
    })
    .await
    .unwrap();
    let goal = app
        .mobile_goal_repository()
        .create_goal("Reach the public final Settings page", Default::default())
        .await
        .unwrap();
    let h = Harness::new(capture("Before page"), capture("After page"));
    (dir, app, goal, h)
}
async fn runner_provider(
    app: &Arc<AppRuntime>,
    h: &Arc<Harness>,
    mode: &'static str,
) -> (tokio::task::JoinHandle<()>, Arc<tokio::sync::Notify>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    app.upsert_model_provider(crate::ModelProviderConfig {
        id: "phase5-fixture".into(),
        name: "Local HTTP integration fixture".into(),
        provider_type: "openai_compatible_chat".into(),
        base_url: url,
        credential_ref: None,
        default_model: "fixture-model".into(),
        temperature: 0.0,
        context_window: 8192,
        enabled: true,
        capabilities: serde_json::to_value(model_gateway::ProviderCapabilities {
            chat_completions: true,
            ..Default::default()
        })
        .unwrap(),
    })
    .await
    .unwrap();
    let started = Arc::new(tokio::sync::Notify::new());
    let signal = started.clone();
    let h = h.clone();
    let app = app.clone();
    let server = tokio::spawn(async move {
        for index in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = vec![];
            let mut buffer = [0; 4096];
            let split = loop {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(i) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                    break i + 4;
                }
            };
            let header = String::from_utf8_lossy(&bytes[..split]);
            let length: usize = header
                .lines()
                .find_map(|line| {
                    line.split_once(':')
                        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                        .map(|(_, value)| value.trim().parse().unwrap())
                })
                .unwrap();
            while bytes.len() < split + length {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let body: serde_json::Value =
                serde_json::from_slice(&bytes[split..split + length]).unwrap();
            let context: serde_json::Value =
                serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
            assert_eq!(body["model"], "fixture-model");
            assert!(context["currentObservation"]["observationId"].is_string());
            signal.notify_one();
            if mode == "stop-model" {
                std::future::pending::<()>().await;
            }
            if mode == "stop-action" {
                *h.cancel_on_action.lock().unwrap() = Some(app.mobile_cancellation_token());
            }
            let action = if index == 0 {
                serde_json::json!({"type":"scroll_down","reason":"Navigate visible public labels","expected_result":{"kind":"UI_CHANGED"}})
            } else {
                let mut final_capture = capture("Final page");
                final_capture.snapshot.activity = "FinalPage".into();
                final_capture.session.current_activity = Some("FinalPage".into());
                *h.after.lock().unwrap() = final_capture;
                serde_json::json!({"type":"tap_element","reason":"Open the public target page","target_element_ref":context["currentObservation"]["elements"][0]["elementRef"],"expected_result":{"kind":"ACTIVITY_EQUALS","package":"com.android.settings","activity":"FinalPage"}})
            };
            let decision=serde_json::json!({"decision":"next_action","action":action,"completion":null,"failure":null}).to_string();
            let response=serde_json::json!({"choices":[{"message":{"content":decision},"finish_reason":"stop"}]}).to_string();
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",response.len(),response).as_bytes()).await.unwrap();
        }
    });
    (server, started)
}
#[tokio::test]
async fn production_runner_completes_two_gateway_decisions_with_owner_bound_truth() {
    let (_d, app, g, h) = runner_fixture().await;
    app.mobile_goal_repository()
        .set_completion_target(
            &g.id,
            &MobileCompletionTarget::ActivityEquals {
                package: "com.android.settings".into(),
                activity: "FinalPage".into(),
            },
        )
        .await
        .unwrap();
    let (server, _) = runner_provider(&app, &h, "complete").await;
    let completed = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        app.run_mobile_goal(&g.id, h.clone(), |_| Ok(None)),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(completed.status, MobileGoalStatus::Completed);
    assert_eq!(h.actions.load(Ordering::SeqCst), 2);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_model_calls WHERE goal_id=?")
        .bind(g.id.as_str())
        .fetch_one(app.pool())
        .await
        .unwrap();
    assert_eq!(count, 2);
    let plans = app
        .mobile_goal_repository()
        .list_plans_for_goal(&g.id)
        .await
        .unwrap();
    assert_eq!(plans.len(), 2);
    for p in plans {
        let step = app
            .mobile_goal_repository()
            .list_steps(&p.id)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(step.status, MobileStepStatus::Verified);
        assert!(
            app.mobile_goal_repository()
                .get_step_result(&step.id)
                .await
                .unwrap()
                .unwrap()
                .action_receipt_id
                .is_some()
        );
    }
    server.await.unwrap();
}
#[tokio::test]
async fn stop_cancels_model_wait_and_settles_inflight_action_without_next_decision() {
    for mode in ["stop-model", "stop-action"] {
        let (_d, app, g, h) = runner_fixture().await;
        let (server, started) = runner_provider(&app, &h, mode).await;
        let runtime = app.clone();
        let goal = g.id.clone();
        let host = h.clone();
        let runner =
            tokio::spawn(async move { runtime.run_mobile_goal(&goal, host, |_| Ok(None)).await });
        started.notified().await;
        if mode == "stop-model" {
            tokio::time::timeout(std::time::Duration::from_secs(2), app.stop_mobile_goals())
                .await
                .unwrap()
                .unwrap();
        }
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), runner)
            .await
            .unwrap()
            .unwrap();
        assert!(
            matches!(result,Err(AppError::MobileGoal(e)) if e.code==MobileGoalErrorCode::UserStopped)
        );
        assert_eq!(
            app.mobile_goal_repository()
                .get_goal(&g.id)
                .await
                .unwrap()
                .status,
            MobileGoalStatus::Stopped
        );
        assert_eq!(
            h.actions.load(Ordering::SeqCst),
            if mode == "stop-model" { 0 } else { 1 }
        );
        let calls: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM mobile_model_calls WHERE goal_id=?")
                .bind(g.id.as_str())
                .fetch_one(app.pool())
                .await
                .unwrap();
        assert_eq!(calls, 1);
        if mode == "stop-action" {
            let step = app
                .mobile_goal_repository()
                .get_goal_plan(&g.id)
                .await
                .unwrap()
                .unwrap()
                .steps
                .remove(0);
            assert!(
                app.mobile_goal_repository()
                    .get_step_result(&step.id)
                    .await
                    .unwrap()
                    .unwrap()
                    .verified
            );
        }
        server.abort();
    }
}
#[tokio::test]
async fn zero_provider_runner_never_observes_or_requests_credentials() {
    let (_d, app, g, h) = runner_fixture().await;
    let result = app
        .run_mobile_goal(&g.id, h.clone(), |_| panic!("no credential access"))
        .await;
    assert!(
        matches!(result,Err(AppError::MobileGoal(e)) if e.code==MobileGoalErrorCode::ModelNotConfigured)
    );
    assert_eq!(
        app.mobile_goal_repository()
            .get_goal(&g.id)
            .await
            .unwrap()
            .status,
        MobileGoalStatus::Pending
    );
    assert_eq!(h.actions.load(Ordering::SeqCst), 0);
    assert!(
        app.mobile_workspace_data()
            .await
            .unwrap()
            .observations
            .is_empty()
    );
}

#[tokio::test]
async fn runner_call_action_and_elapsed_budgets_stop_before_an_extra_side_effect() {
    for limit in ["calls", "actions", "elapsed"] {
        let (_d, app, g, mut h) = runner_fixture().await;
        let mut goal = g.clone();
        match limit {
            "calls" => goal.step_budget.max_model_calls = 1,
            "actions" => goal.step_budget.max_execution_actions = 1,
            _ => {
                goal.step_budget.max_runtime_ms = 1;
                Arc::get_mut(&mut h).unwrap().delay_ms = 30;
            }
        }
        rewrite_goal(&app, &g, |g| g.step_budget = goal.step_budget).await;
        let (server, _) = runner_provider(&app, &h, "complete").await;
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            app.run_mobile_goal(&g.id, h.clone(), |_| Ok(None)),
        )
        .await
        .unwrap();
        assert!(
            matches!(result,Err(AppError::MobileGoal(e)) if e.code==if limit=="elapsed" {MobileGoalErrorCode::TimeLimitReached} else {MobileGoalErrorCode::StepLimitReached})
        );
        assert_eq!(
            h.actions.load(Ordering::SeqCst),
            if limit == "elapsed" { 0 } else { 1 }
        );
        assert_eq!(
            app.mobile_goal_repository()
                .get_goal(&g.id)
                .await
                .unwrap()
                .status,
            MobileGoalStatus::Failed
        );
        server.abort();
    }
}

#[tokio::test]
async fn a_new_observation_skips_an_unclaimed_stale_step_and_allows_fresh_planning() {
    let (_d, app, g, s, h) =
        prepared(MobileStepType::ScrollDown, ExpectedStepResult::UiChanged).await;
    assert!(
        !app.mobile_goal_repository()
            .skip_stale_pending_step(&g.id, &s.id, false)
            .await
            .unwrap()
    );
    let latest = h.observe().await.unwrap();
    app.record_mobile_capture(&latest).await.unwrap();
    assert!(
        app.mobile_goal_repository()
            .skip_stale_pending_step(&g.id, &s.id, false)
            .await
            .unwrap()
    );
    assert_eq!(
        app.mobile_goal_repository()
            .get_step(&s.id)
            .await
            .unwrap()
            .status,
        MobileStepStatus::Skipped
    );
    assert!(
        app.execute_mobile_goal_step(&g.id, h.clone())
            .await
            .is_err()
    );
    assert_eq!(h.actions.load(Ordering::SeqCst), 0);
    let context = app
        .mobile_goal_repository()
        .planner_context(&g.id, Some(&latest.observation.id))
        .await
        .unwrap();
    assert_eq!(context.steps_used, 1);
    assert_eq!(context.remaining_steps, 7);
    assert_eq!(
        context.current_observation.unwrap().observation_id,
        latest.observation.id
    );
}

#[tokio::test]
async fn stop_during_an_approved_final_action_settles_receipt_without_completing_or_resuming_goal()
{
    let (_d, rt, g, s, h) = prepared(
        MobileStepType::Back,
        ExpectedStepResult::ActivityEquals {
            package: "com.android.settings".into(),
            activity: "Settings".into(),
        },
    )
    .await;
    sqlx::query("INSERT INTO mobile_goal_completion_targets(goal_id,target_json) VALUES (?,?)")
        .bind(g.id.as_str())
        .bind(
            serde_json::to_string(&MobileCompletionTarget::ActivityEquals {
                package: "com.android.settings".into(),
                activity: "Settings".into(),
            })
            .unwrap(),
        )
        .execute(rt.pool())
        .await
        .unwrap();
    rewrite_step(&rt, &s, |s| s.risk = MobileStepRisk::ApprovalRequired).await;
    assert_eq!(
        rt.execute_mobile_goal_step(&g.id, h.clone())
            .await
            .unwrap()
            .status,
        MobileStepStatus::WaitingApproval
    );
    let approval = rt
        .mobile_goal_repository()
        .get_step_approval(&s.id)
        .await
        .unwrap()
        .unwrap();
    *h.cancel_on_action.lock().unwrap() = Some(rt.mobile_cancellation_token());
    let executed = rt
        .approve_mobile_step(&g.id, &s.id, &approval.id, h.clone())
        .await
        .unwrap();
    assert!(
        executed.verified,
        "the emitted action must still settle its receipt and verification"
    );
    assert_eq!(
        rt.mobile_goal_repository()
            .get_goal(&g.id)
            .await
            .unwrap()
            .status,
        MobileGoalStatus::Stopped
    );
    assert_eq!(h.actions.load(Ordering::SeqCst), 1);
    assert!(
        rt.approve_mobile_step(&g.id, &s.id, &approval.id, h.clone())
            .await
            .is_err()
    );
}
