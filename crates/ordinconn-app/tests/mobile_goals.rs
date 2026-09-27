use chrono::Utc;
use mobile_runtime::execution::*;
use mobile_runtime::{
    MobileBounds, MobileCapture, MobileDeviceSession, MobileDeviceType, MobileFrame,
    MobileObservation, MobilePlatform, MobileSessionStatus, MobileUiSnapshot, PrivacyClass,
    RawMobileElement,
};
use ordinconn_app::AppRuntime;

async fn runtime() -> (tempfile::TempDir, std::sync::Arc<AppRuntime>) {
    let dir = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&dir.path().join("goals.sqlite3"))
        .await
        .unwrap();
    (dir, runtime)
}
fn step(sequence: u32) -> NewMobileStep {
    NewMobileStep {
        wait_ms: None,
        extraction_intent: None,
        sequence,
        step_type: MobileStepType::Observe,
        reason: "Read current public screen".into(),
        risk: MobileStepRisk::ReadOnly,
        target_ref: None,
        input_text: None,
        expected_result: Some(ExpectedStepResult::NoChangeExpected),
    }
}
async fn active(runtime: &AppRuntime) -> (MobileGoal, MobilePlan, MobilePlanStep) {
    let repo = runtime.mobile_goal_repository();
    let goal = repo
        .create_goal("Read public page", MobileGoalBudget::default())
        .await
        .unwrap();
    repo.update_goal_status(&goal.id, MobileGoalStatus::Planning, None)
        .await
        .unwrap();
    let plan = repo
        .create_plan(&goal.id, 1, "Read public page", vec![step(1)])
        .await
        .unwrap();
    repo.activate_plan(&plan.id).await.unwrap();
    repo.update_goal_status(&goal.id, MobileGoalStatus::Running, None)
        .await
        .unwrap();
    let step = repo.get_next_pending_step(&plan.id).await.unwrap().unwrap();
    (goal, plan, step)
}

#[tokio::test]
async fn canonical_mobile_execution_tables_exist_in_the_existing_database() {
    let (_dir, runtime) = runtime().await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('mobile_goals','mobile_plans','mobile_plan_steps','mobile_step_results')").fetch_one(runtime.pool()).await.unwrap();
    assert_eq!(count, 4);
}
#[tokio::test]
async fn create_and_load_preserve_original_objective_and_defaults() {
    let (dir, runtime) = runtime().await;
    let goal = runtime
        .mobile_goal_repository()
        .create_goal(
            "  向下查看当前页面并提取新增信息  ",
            MobileGoalBudget::default(),
        )
        .await
        .unwrap();
    runtime.pool().close().await;
    let reopened = AppRuntime::initialize(&dir.path().join("goals.sqlite3"))
        .await
        .unwrap();
    let loaded = reopened
        .mobile_goal_repository()
        .get_goal(&goal.id)
        .await
        .unwrap();
    assert_eq!(loaded.objective, "  向下查看当前页面并提取新增信息  ");
    assert_eq!(loaded.status, MobileGoalStatus::Pending);
    assert_eq!(loaded.step_budget.max_steps, 8);
    assert_eq!(loaded.step_budget.max_runtime_ms, 120000);
    assert_eq!(loaded.step_budget.max_consecutive_failures, 2);
    assert_eq!(loaded.step_budget.max_identical_observations, 3);
    assert!(loaded.active_plan_id.is_none());
    assert!(
        reopened
            .mobile_goal_repository()
            .get_active_plan(&goal.id)
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn goal_ids_are_unique_and_listed_without_plans() {
    let (_dir, runtime) = runtime().await;
    let repo = runtime.mobile_goal_repository();
    let a = repo
        .create_goal("same", MobileGoalBudget::default())
        .await
        .unwrap();
    let b = repo
        .create_goal("same", MobileGoalBudget::default())
        .await
        .unwrap();
    assert_ne!(a.id, b.id);
    assert_eq!(repo.list_goals().await.unwrap().len(), 2);
    assert!(repo.list_plans_for_goal(&a.id).await.unwrap().is_empty());
}
#[tokio::test]
async fn goal_rejects_blank_objective_and_zero_budget() {
    let (_dir, runtime) = runtime().await;
    let repo = runtime.mobile_goal_repository();
    assert!(
        repo.create_goal("  ", MobileGoalBudget::default())
            .await
            .is_err()
    );
    let budget = MobileGoalBudget {
        max_steps: 0,
        ..Default::default()
    };
    assert!(repo.create_goal("valid", budget).await.is_err());
    assert!(repo.list_goals().await.unwrap().is_empty());
}
#[tokio::test]
async fn goal_state_transition_and_error_are_persisted() {
    let (_dir, runtime) = runtime().await;
    let repo = runtime.mobile_goal_repository();
    let goal = repo.create_goal("read", Default::default()).await.unwrap();
    repo.update_goal_status(&goal.id, MobileGoalStatus::Planning, None)
        .await
        .unwrap();
    let err = MobileGoalError::new(MobileGoalErrorCode::ModelNotConfigured);
    let failed = repo
        .update_goal_status(&goal.id, MobileGoalStatus::Failed, Some(err))
        .await
        .unwrap();
    assert_eq!(
        failed.last_error_code,
        Some(MobileGoalErrorCode::ModelNotConfigured)
    );
    assert!(failed.failed_at.is_some());
    assert_eq!(
        repo.get_goal(&goal.id).await.unwrap().status,
        MobileGoalStatus::Failed
    );
}
#[tokio::test]
async fn invalid_transition_and_terminal_resume_fail_without_events() {
    let (_dir, runtime) = runtime().await;
    let repo = runtime.mobile_goal_repository();
    let goal = repo.create_goal("read", Default::default()).await.unwrap();
    assert!(
        repo.update_goal_status(&goal.id, MobileGoalStatus::Completed, None)
            .await
            .is_err()
    );
    repo.update_goal_status(&goal.id, MobileGoalStatus::Stopped, None)
        .await
        .unwrap();
    let count = runtime
        .audit_timeline(goal.id.as_str())
        .await
        .unwrap()
        .len();
    assert!(
        repo.update_goal_status(&goal.id, MobileGoalStatus::Running, None)
            .await
            .is_err()
    );
    assert_eq!(
        runtime
            .audit_timeline(goal.id.as_str())
            .await
            .unwrap()
            .len(),
        count
    );
}
#[tokio::test]
async fn plan_revisions_supersede_without_overwriting_history() {
    let (_dir, runtime) = runtime().await;
    let (goal, old, _) = active(&runtime).await;
    let repo = runtime.mobile_goal_repository();
    let next = repo
        .create_plan(&goal.id, 2, "Revised objective", vec![step(1)])
        .await
        .unwrap();
    repo.activate_plan(&next.id).await.unwrap();
    let history = repo.list_plans_for_goal(&goal.id).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].id, old.id);
    assert_eq!(history[0].status, MobilePlanStatus::Superseded);
    assert!(history[0].superseded_at.is_some());
    assert_eq!(
        repo.get_active_plan(&goal.id).await.unwrap().unwrap().id,
        next.id
    );
    assert_eq!(
        repo.get_goal(&goal.id).await.unwrap().active_plan_id,
        Some(next.id)
    );
}
#[tokio::test]
async fn duplicate_plan_revision_rejected_and_no_orphan_steps() {
    let (_dir, runtime) = runtime().await;
    let (goal, _, _) = active(&runtime).await;
    let repo = runtime.mobile_goal_repository();
    assert!(
        repo.create_plan(&goal.id, 1, "duplicate", vec![step(1)])
            .await
            .is_err()
    );
    assert_eq!(repo.list_plans_for_goal(&goal.id).await.unwrap().len(), 1);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mobile_plan_steps")
        .fetch_one(runtime.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
}
#[tokio::test]
async fn duplicate_sequence_rolls_back_whole_plan_creation() {
    let (_dir, runtime) = runtime().await;
    let repo = runtime.mobile_goal_repository();
    let goal = repo.create_goal("read", Default::default()).await.unwrap();
    assert!(
        repo.create_plan(&goal.id, 1, "read", vec![step(1), step(1)])
            .await
            .is_err()
    );
    assert!(repo.list_plans_for_goal(&goal.id).await.unwrap().is_empty());
    assert_eq!(
        runtime
            .audit_timeline(goal.id.as_str())
            .await
            .unwrap()
            .len(),
        1
    );
}
#[tokio::test]
async fn create_steps_is_atomic_and_pending_sequence_is_ordered() {
    let (_dir, runtime) = runtime().await;
    let repo = runtime.mobile_goal_repository();
    let goal = repo.create_goal("read", Default::default()).await.unwrap();
    let plan = repo
        .create_plan(&goal.id, 1, "read", vec![step(2)])
        .await
        .unwrap();
    repo.create_steps(&plan.id, vec![step(1)]).await.unwrap();
    assert_eq!(
        repo.get_next_pending_step(&plan.id)
            .await
            .unwrap()
            .unwrap()
            .sequence,
        1
    );
    assert!(
        repo.create_steps(&plan.id, vec![step(3), step(2)])
            .await
            .is_err()
    );
    assert_eq!(repo.list_steps(&plan.id).await.unwrap().len(), 2);
}
#[tokio::test]
async fn typed_risk_expected_result_and_step_fields_round_trip() {
    let (_dir, runtime) = runtime().await;
    let repo = runtime.mobile_goal_repository();
    let goal = repo.create_goal("read", Default::default()).await.unwrap();
    let mut input = step(1);
    input.step_type = MobileStepType::TapElement;
    input.risk = MobileStepRisk::ApprovalRequired;
    input.target_ref = Some("@e1".into());
    input.expected_result = Some(ExpectedStepResult::ActivityEquals {
        package: "com.android.settings".into(),
        activity: ".Settings".into(),
    });
    let plan = repo
        .create_plan(&goal.id, 1, "read", vec![input])
        .await
        .unwrap();
    let loaded = repo.get_next_pending_step(&plan.id).await.unwrap().unwrap();
    assert_eq!(loaded.risk, MobileStepRisk::ApprovalRequired);
    assert_eq!(loaded.target_ref.as_deref(), Some("@e1"));
    assert!(matches!(
        loaded.expected_result,
        Some(ExpectedStepResult::ActivityEquals { .. })
    ));
}
#[tokio::test]
async fn step_approval_transitions_are_controlled() {
    let (_dir, runtime) = runtime().await;
    let (_, _, step) = active(&runtime).await;
    let repo = runtime.mobile_goal_repository();
    assert!(
        repo.update_step_status(&step.id, MobileStepStatus::Verified, None)
            .await
            .is_err()
    );
    repo.update_step_status(&step.id, MobileStepStatus::WaitingApproval, None)
        .await
        .unwrap();
    repo.update_step_status(&step.id, MobileStepStatus::Executing, None)
        .await
        .unwrap();
    let failed = repo
        .update_step_status(
            &step.id,
            MobileStepStatus::Failed,
            Some(MobileGoalError::new(MobileGoalErrorCode::ActionFailed)),
        )
        .await
        .unwrap();
    assert!(failed.finished_at.is_some());
    assert!(
        repo.update_step_status(&step.id, MobileStepStatus::Executing, None)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn database_rejects_cross_goal_active_plan_and_physical_deletion() {
    let (_dir, runtime) = runtime().await;
    let (goal, plan, step) = active(&runtime).await;
    let other = runtime
        .mobile_goal_repository()
        .create_goal("other", Default::default())
        .await
        .unwrap();
    assert!(sqlx::query("UPDATE mobile_goals SET active_plan_id=?,domain_json=json_set(domain_json,'$.activePlanId',?) WHERE id=?").bind(plan.id.as_str()).bind(plan.id.as_str()).bind(other.id.as_str()).execute(runtime.pool()).await.is_err());
    for (table, id) in [
        ("mobile_goals", goal.id.as_str()),
        ("mobile_plans", plan.id.as_str()),
        ("mobile_plan_steps", step.id.as_str()),
    ] {
        assert!(
            sqlx::query(&format!("DELETE FROM {table} WHERE id=?"))
                .bind(id)
                .execute(runtime.pool())
                .await
                .is_err()
        );
    }
}
#[tokio::test]
async fn goal_creation_rolls_back_when_audit_insert_fails() {
    let (_dir, runtime) = runtime().await;
    sqlx::query("CREATE TRIGGER fail_goal_event BEFORE INSERT ON runtime_events WHEN NEW.event_type='mobile.goal_created' BEGIN SELECT RAISE(ABORT,'test audit failure'); END").execute(runtime.pool()).await.unwrap();
    assert!(
        runtime
            .mobile_goal_repository()
            .create_goal("read", Default::default())
            .await
            .is_err()
    );
    assert!(
        runtime
            .mobile_goal_repository()
            .list_goals()
            .await
            .unwrap()
            .is_empty()
    );
}
#[tokio::test]
async fn plan_creation_rolls_back_when_audit_insert_fails() {
    let (_dir, runtime) = runtime().await;
    let repo = runtime.mobile_goal_repository();
    let goal = repo.create_goal("read", Default::default()).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_plan_event BEFORE INSERT ON runtime_events WHEN NEW.event_type='mobile.plan_created' BEGIN SELECT RAISE(ABORT,'test audit failure'); END").execute(runtime.pool()).await.unwrap();
    assert!(
        repo.create_plan(&goal.id, 1, "read", vec![step(1)])
            .await
            .is_err()
    );
    assert!(repo.list_plans_for_goal(&goal.id).await.unwrap().is_empty());
}
#[tokio::test]
async fn execution_cannot_start_two_steps_or_a_superseded_plan() {
    let (_dir, runtime) = runtime().await;
    let repo = runtime.mobile_goal_repository();
    let goal = repo.create_goal("read", Default::default()).await.unwrap();
    repo.update_goal_status(&goal.id, MobileGoalStatus::Planning, None)
        .await
        .unwrap();
    let plan = repo
        .create_plan(&goal.id, 1, "read", vec![step(1), step(2)])
        .await
        .unwrap();
    repo.activate_plan(&plan.id).await.unwrap();
    repo.update_goal_status(&goal.id, MobileGoalStatus::Running, None)
        .await
        .unwrap();
    let steps = repo.list_steps(&plan.id).await.unwrap();
    repo.update_step_status(&steps[0].id, MobileStepStatus::Executing, None)
        .await
        .unwrap();
    assert!(
        repo.update_step_status(&steps[1].id, MobileStepStatus::Executing, None)
            .await
            .is_err()
    );
    let next = repo
        .create_plan(&goal.id, 2, "read", vec![step(1)])
        .await
        .unwrap();
    assert!(
        repo.activate_plan(&next.id).await.is_err(),
        "cannot supersede an executing step"
    );
}
#[tokio::test]
async fn verified_step_result_and_runtime_event_are_atomic() {
    let (_dir, runtime) = runtime().await;
    let (goal, _, step) = active(&runtime).await;
    let repo = runtime.mobile_goal_repository();
    repo.update_step_status(&step.id, MobileStepStatus::Executing, None)
        .await
        .unwrap();
    let result = MobileStepResult {
        step_id: step.id.clone(),
        outcome: MobileStepOutcome::Verified,
        verified: true,
        observation_before_id: None,
        observation_after_id: None,
        action_receipt_id: None,
        evidence_ids: vec![],
        error: None,
        created_at: Utc::now(),
    };
    repo.record_step_result(result.clone()).await.unwrap();
    assert_eq!(
        repo.get_step(&step.id).await.unwrap().status,
        MobileStepStatus::Verified
    );
    assert_eq!(
        repo.get_step_result(&step.id)
            .await
            .unwrap()
            .unwrap()
            .outcome,
        MobileStepOutcome::Verified
    );
    assert!(repo.record_step_result(result).await.is_err());
    let events = runtime.audit_timeline(goal.id.as_str()).await.unwrap();
    let event = events
        .iter()
        .find(|e| e.event_type == "mobile.step_verified")
        .unwrap();
    let payload: serde_json::Value = serde_json::from_str(&event.payload_json).unwrap();
    assert_eq!(payload["goalId"], goal.id.as_str());
    assert_eq!(payload["stepId"], step.id.as_str());
    assert!(payload.get("objective").is_none());
}
#[tokio::test]
async fn restart_fails_closed_without_replaying_or_reviving_terminal_goals() {
    let (dir, runtime) = runtime().await;
    let (goal, plan, step) = active(&runtime).await;
    let repo = runtime.mobile_goal_repository();
    repo.update_step_status(&step.id, MobileStepStatus::Executing, None)
        .await
        .unwrap();
    let pending = repo
        .create_goal("pending", Default::default())
        .await
        .unwrap();
    runtime.pool().close().await;
    let reopened = AppRuntime::initialize(&dir.path().join("goals.sqlite3"))
        .await
        .unwrap();
    let repo = reopened.mobile_goal_repository();
    assert_eq!(
        repo.get_goal(&goal.id).await.unwrap().last_error_code,
        Some(MobileGoalErrorCode::InterruptedByRestart)
    );
    assert_eq!(
        repo.get_goal(&goal.id).await.unwrap().status,
        MobileGoalStatus::Failed
    );
    assert_eq!(
        repo.get_step(&step.id).await.unwrap().status,
        MobileStepStatus::Failed
    );
    assert_eq!(
        repo.list_plans_for_goal(&goal.id).await.unwrap()[0].status,
        MobilePlanStatus::Failed
    );
    assert_eq!(
        repo.get_goal(&pending.id).await.unwrap().status,
        MobileGoalStatus::Pending
    );
    assert_eq!(repo.recover_interrupted().await.unwrap(), 0);
    assert_eq!(plan.revision, 1);
}
#[tokio::test]
async fn links_reject_nonexistent_observation_action_and_evidence() {
    let (_dir, runtime) = runtime().await;
    let (_, _, step) = active(&runtime).await;
    let repo = runtime.mobile_goal_repository();
    assert!(
        repo.attach_step_observations(&step.id, Some("missing".into()), None)
            .await
            .is_err()
    );
    assert!(
        repo.attach_step_action(&step.id, "missing".into())
            .await
            .is_err()
    );
    assert!(
        repo.attach_step_evidence(&step.id, "missing".into(), "missing".into())
            .await
            .is_err()
    );
    assert!(repo.get_step(&step.id).await.unwrap().action_id.is_none());
}

fn fixture_capture() -> MobileCapture {
    let session_id = "mobile-session-1".to_owned();
    let snapshot = MobileUiSnapshot::from_elements(
        &session_id,
        "com.example.news",
        "com.example.news/.MainActivity",
        1080,
        2400,
        vec![RawMobileElement {
            text: Some("BTC ETF inflows rose; public screen fixture".into()),
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
async fn observations_action_receipt_evidence_and_result_have_queryable_provenance() {
    let (_dir, runtime) = runtime().await;
    runtime.seed_demo_data().await.unwrap();
    let (goal, _, s) = active(&runtime).await;
    let capture = fixture_capture();
    runtime.record_mobile_capture(&capture).await.unwrap();
    let repo = runtime.mobile_goal_repository();
    repo.attach_step_observations(
        &s.id,
        Some(capture.observation.id.clone()),
        Some(capture.observation.id.clone()),
    )
    .await
    .unwrap();
    let request = mobile_runtime::MobileActionRequest {
        action_id: "fixture-receipt".into(),
        session_id: capture.session.session_id.clone(),
        snapshot_id: capture.snapshot.snapshot_id.clone(),
        expected_package: capture.snapshot.package_name.clone(),
        requested_at: Utc::now(),
        target: mobile_runtime::MobileActionTarget::Back,
        text: None,
    };
    // Persistence-only pending intent. No input was sent and no device was contacted.
    runtime.record_mobile_action_intent(&request).await.unwrap();
    repo.attach_step_action(&s.id, request.action_id.clone())
        .await
        .unwrap();
    let evidence: String = sqlx::query_scalar("SELECT id FROM evidence LIMIT 1")
        .fetch_one(runtime.pool())
        .await
        .unwrap();
    repo.attach_step_evidence(&s.id, evidence.clone(), capture.observation.id.clone())
        .await
        .unwrap();
    let loaded = repo.get_step(&s.id).await.unwrap();
    assert_eq!(
        loaded.observation_before_id.as_deref(),
        Some(capture.observation.id.as_str())
    );
    assert_eq!(loaded.action_id.as_deref(), Some("fixture-receipt"));
    assert_eq!(loaded.evidence_id, Some(evidence.clone()));
    let links = repo.get_step_evidence(&s.id).await.unwrap();
    assert_eq!(links[0].observation_id, capture.observation.id);
    repo.update_step_status(&s.id, MobileStepStatus::Executing, None)
        .await
        .unwrap();
    repo.record_step_result(MobileStepResult {
        step_id: s.id.clone(),
        outcome: MobileStepOutcome::Failed,
        verified: false,
        observation_before_id: loaded.observation_before_id,
        observation_after_id: loaded.observation_after_id,
        action_receipt_id: loaded.action_id,
        evidence_ids: vec![evidence.clone()],
        error: Some(MobileGoalError::new(MobileGoalErrorCode::ActionFailed)),
        created_at: Utc::now(),
    })
    .await
    .unwrap();
    assert_eq!(
        repo.get_step_result(&s.id)
            .await
            .unwrap()
            .unwrap()
            .evidence_ids,
        vec![evidence]
    );
    let events = runtime.audit_timeline(goal.id.as_str()).await.unwrap();
    let event = events
        .iter()
        .find(|e| e.event_type == "mobile.step_evidence_attached")
        .unwrap();
    let payload: serde_json::Value = serde_json::from_str(&event.payload_json).unwrap();
    assert_eq!(payload["observationId"], capture.observation.id);
    assert_eq!(payload["stepId"], s.id.as_str());
}
#[tokio::test]
async fn result_failure_rolls_back_state_result_and_event_together() {
    let (_dir, runtime) = runtime().await;
    let (_, _, s) = active(&runtime).await;
    let repo = runtime.mobile_goal_repository();
    repo.update_step_status(&s.id, MobileStepStatus::Executing, None)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER fail_result_event BEFORE INSERT ON runtime_events WHEN NEW.event_type='mobile.step_verified' BEGIN SELECT RAISE(ABORT,'test audit failure'); END").execute(runtime.pool()).await.unwrap();
    let result = MobileStepResult {
        step_id: s.id.clone(),
        outcome: MobileStepOutcome::Verified,
        verified: true,
        observation_before_id: None,
        observation_after_id: None,
        action_receipt_id: None,
        evidence_ids: vec![],
        error: None,
        created_at: Utc::now(),
    };
    assert!(repo.record_step_result(result).await.is_err());
    assert_eq!(
        repo.get_step(&s.id).await.unwrap().status,
        MobileStepStatus::Executing
    );
    assert!(repo.get_step_result(&s.id).await.unwrap().is_none());
}
#[tokio::test]
async fn plan_activation_failure_rolls_back_old_plan_and_active_pointer() {
    let (_dir, runtime) = runtime().await;
    let (goal, old, _) = active(&runtime).await;
    let repo = runtime.mobile_goal_repository();
    let next = repo
        .create_plan(&goal.id, 2, "revision", vec![step(1)])
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER fail_activation BEFORE INSERT ON runtime_events WHEN NEW.event_type='mobile.plan_activated' BEGIN SELECT RAISE(ABORT,'test audit failure'); END").execute(runtime.pool()).await.unwrap();
    assert!(repo.activate_plan(&next.id).await.is_err());
    assert_eq!(
        repo.get_active_plan(&goal.id).await.unwrap().unwrap().id,
        old.id
    );
    assert_eq!(
        repo.list_plans_for_goal(&goal.id).await.unwrap()[0].status,
        MobilePlanStatus::Active
    );
}
#[tokio::test]
async fn forbidden_step_never_enters_executing_state() {
    let (_dir, runtime) = runtime().await;
    let repo = runtime.mobile_goal_repository();
    let goal = repo.create_goal("read", Default::default()).await.unwrap();
    repo.update_goal_status(&goal.id, MobileGoalStatus::Planning, None)
        .await
        .unwrap();
    let mut input = step(1);
    input.risk = MobileStepRisk::Forbidden;
    let plan = repo
        .create_plan(&goal.id, 1, "read", vec![input])
        .await
        .unwrap();
    repo.activate_plan(&plan.id).await.unwrap();
    repo.update_goal_status(&goal.id, MobileGoalStatus::Running, None)
        .await
        .unwrap();
    let s = repo.get_next_pending_step(&plan.id).await.unwrap().unwrap();
    assert!(
        repo.update_step_status(&s.id, MobileStepStatus::Executing, None)
            .await
            .is_err()
    );
    assert_eq!(
        repo.get_step(&s.id).await.unwrap().status,
        MobileStepStatus::Pending
    );
}
#[tokio::test]
async fn old_database_migration_preserves_events_observations_feed_research_and_settings() {
    let dir = tempfile::tempdir().unwrap();
    let old_migrations = dir.path().join("old-migrations");
    std::fs::create_dir(&old_migrations).unwrap();
    let current = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    for entry in std::fs::read_dir(current).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        // A pre-M3 database excludes every M3 migration and its dependencies.
        let version = name
            .to_string_lossy()
            .split('_')
            .next()
            .unwrap()
            .parse::<u32>()
            .unwrap();
        if version >= 8 {
            continue;
        }
        std::fs::copy(entry.path(), old_migrations.join(name)).unwrap();
    }
    let db = dir.path().join("legacy.sqlite3");
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&db)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(old_migrations.as_path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    let capture = fixture_capture();
    sqlx::query("INSERT INTO mobile_device_sessions(id,device_id,status,connected_at,domain_json) VALUES (?,?,'connected',?,?)").bind(&capture.session.session_id).bind(&capture.session.device_id).bind(capture.session.connected_at.to_rfc3339()).bind(serde_json::to_string(&capture.session).unwrap()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO mobile_ui_snapshots(id,session_id,package_name,activity,ui_tree_hash,element_count,captured_at,domain_json) VALUES (?,?,?,?,?,1,?,?)").bind(&capture.snapshot.snapshot_id).bind(&capture.session.session_id).bind(&capture.snapshot.package_name).bind(&capture.snapshot.activity).bind(&capture.observation.ui_tree_hash).bind(capture.snapshot.captured_at.to_rfc3339()).bind(serde_json::to_string(&capture.snapshot).unwrap()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO mobile_observations(id,session_id,snapshot_id,package_name,observed_at,frame_hash,ui_tree_hash,evidence_status,domain_json) VALUES (?,?,?,?,?,?,?,'observation_only',?)").bind(&capture.observation.id).bind(&capture.session.session_id).bind(&capture.snapshot.snapshot_id).bind(&capture.snapshot.package_name).bind(capture.observation.observed_at.to_rfc3339()).bind(&capture.observation.frame_hash).bind(&capture.observation.ui_tree_hash).bind(serde_json::to_string(&capture.observation).unwrap()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO warehouse_entries(id,item_id,favorite,saved,created_at,updated_at) VALUES ('saved-fixture',?,1,1,?,?)").bind(&capture.observation.id).bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO runtime_events(id,event_type,aggregate_type,aggregate_id,payload_json,created_at,sequence) VALUES ('legacy-event','mobile.observation','mobile_session','legacy','{}',?,1)").bind(Utc::now().to_rfc3339()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO research_tasks(id,query,reason,status,discovered_urls_json,created_at,mobile_budget_json) VALUES ('legacy-research','Original research','User-authorized mobile intelligence research','pending','[]',?,?)").bind(Utc::now().to_rfc3339()).bind(serde_json::to_string(&ordinconn_app::MobileResearchBudget::default()).unwrap()).execute(&pool).await.unwrap();
    sqlx::query("UPDATE settings SET value_json='\"zh-CN\"' WHERE key='language'")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let runtime = AppRuntime::initialize(&db).await.unwrap();
    let workspace = runtime.mobile_workspace_data().await.unwrap();
    assert_eq!(workspace.observations[0].id, capture.observation.id);
    assert_eq!(workspace.feed[0].id, capture.observation.id);
    assert!(workspace.feed[0].saved && workspace.feed[0].favorite);
    assert_eq!(workspace.research_tasks[0].query, "Original research");
    assert_eq!(
        runtime.audit_timeline("legacy").await.unwrap()[0].id,
        "legacy-event"
    );
    let locale: String = sqlx::query_scalar("SELECT value_json FROM settings WHERE key='language'")
        .fetch_one(runtime.pool())
        .await
        .unwrap();
    assert_eq!(locale, "\"zh-CN\"");
    assert!(
        runtime
            .mobile_goal_repository()
            .list_goals()
            .await
            .unwrap()
            .is_empty()
    );
    let violations = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(runtime.pool())
        .await
        .unwrap();
    assert!(violations.is_empty());
}

#[tokio::test]
async fn activating_an_older_draft_revision_cannot_roll_back_the_active_plan() {
    let (_dir, runtime) = runtime().await;
    let (goal, _, _) = active(&runtime).await;
    let repo = runtime.mobile_goal_repository();
    let second = repo
        .create_plan(&goal.id, 2, "second", vec![step(1)])
        .await
        .unwrap();
    let third = repo
        .create_plan(&goal.id, 3, "third", vec![step(1)])
        .await
        .unwrap();
    repo.activate_plan(&third.id).await.unwrap();
    assert!(repo.activate_plan(&second.id).await.is_err());
    assert_eq!(
        repo.get_active_plan(&goal.id).await.unwrap().unwrap().id,
        third.id
    );
}
#[tokio::test]
async fn pending_sequence_cannot_be_executed_out_of_order() {
    let (_dir, runtime) = runtime().await;
    let repo = runtime.mobile_goal_repository();
    let goal = repo.create_goal("read", Default::default()).await.unwrap();
    repo.update_goal_status(&goal.id, MobileGoalStatus::Planning, None)
        .await
        .unwrap();
    let plan = repo
        .create_plan(&goal.id, 1, "read", vec![step(1), step(2)])
        .await
        .unwrap();
    repo.activate_plan(&plan.id).await.unwrap();
    repo.update_goal_status(&goal.id, MobileGoalStatus::Running, None)
        .await
        .unwrap();
    let steps = repo.list_steps(&plan.id).await.unwrap();
    assert!(
        repo.update_step_status(&steps[1].id, MobileStepStatus::Executing, None)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn completing_goal_preserves_completed_plan_and_cannot_resume() {
    let (_dir, runtime) = runtime().await;
    let (goal, _, s) = active(&runtime).await;
    let repo = runtime.mobile_goal_repository();
    repo.update_step_status(&s.id, MobileStepStatus::Executing, None)
        .await
        .unwrap();
    repo.record_step_result(MobileStepResult {
        step_id: s.id,
        outcome: MobileStepOutcome::Verified,
        verified: true,
        observation_before_id: None,
        observation_after_id: None,
        action_receipt_id: None,
        evidence_ids: vec![],
        error: None,
        created_at: Utc::now(),
    })
    .await
    .unwrap();
    let done = repo
        .update_goal_status(&goal.id, MobileGoalStatus::Completed, None)
        .await
        .unwrap();
    assert!(done.completed_at.is_some());
    assert_eq!(
        repo.get_active_plan(&goal.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        MobilePlanStatus::Completed
    );
    assert!(
        repo.update_goal_status(&goal.id, MobileGoalStatus::Running, None)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn waiting_approval_goal_can_resume_or_stop_without_replaying_a_step() {
    let (_dir, runtime) = runtime().await;
    let (goal, _, s) = active(&runtime).await;
    let repo = runtime.mobile_goal_repository();
    repo.update_step_status(&s.id, MobileStepStatus::WaitingApproval, None)
        .await
        .unwrap();
    repo.update_goal_status(&goal.id, MobileGoalStatus::WaitingApproval, None)
        .await
        .unwrap();
    repo.update_goal_status(&goal.id, MobileGoalStatus::Running, None)
        .await
        .unwrap();
    assert_eq!(
        repo.get_step(&s.id).await.unwrap().status,
        MobileStepStatus::WaitingApproval
    );
    let stopped = repo
        .update_goal_status(
            &goal.id,
            MobileGoalStatus::Stopped,
            Some(MobileGoalError::new(MobileGoalErrorCode::UserStopped)),
        )
        .await
        .unwrap();
    assert!(stopped.stopped_at.is_some());
    assert_eq!(
        repo.get_step(&s.id).await.unwrap().status,
        MobileStepStatus::Stopped
    );
}
#[tokio::test]
async fn restart_also_interrupts_planning_and_waiting_approval() {
    let (dir, runtime) = runtime().await;
    let repo = runtime.mobile_goal_repository();
    let planning = repo
        .create_goal("planning", Default::default())
        .await
        .unwrap();
    repo.update_goal_status(&planning.id, MobileGoalStatus::Planning, None)
        .await
        .unwrap();
    let (waiting, _, s) = active(&runtime).await;
    repo.update_step_status(&s.id, MobileStepStatus::WaitingApproval, None)
        .await
        .unwrap();
    repo.update_goal_status(&waiting.id, MobileGoalStatus::WaitingApproval, None)
        .await
        .unwrap();
    runtime.pool().close().await;
    let reopened = AppRuntime::initialize(&dir.path().join("goals.sqlite3"))
        .await
        .unwrap();
    let repo = reopened.mobile_goal_repository();
    for id in [&planning.id, &waiting.id] {
        assert_eq!(
            repo.get_goal(id).await.unwrap().status,
            MobileGoalStatus::Failed
        );
        assert_eq!(
            repo.get_goal(id).await.unwrap().last_error_code,
            Some(MobileGoalErrorCode::InterruptedByRestart)
        );
    }
    assert_eq!(
        repo.get_step(&s.id).await.unwrap().status,
        MobileStepStatus::Failed
    );
}

#[tokio::test]
async fn pending_action_intent_cannot_be_reported_as_verified_execution() {
    let (_dir, runtime) = runtime().await;
    let (_, _, s) = active(&runtime).await;
    let repo = runtime.mobile_goal_repository();
    let capture = fixture_capture();
    runtime.record_mobile_capture(&capture).await.unwrap();
    repo.attach_step_observations(&s.id, Some(capture.observation.id.clone()), None)
        .await
        .unwrap();
    let request = mobile_runtime::MobileActionRequest {
        action_id: "unexecuted-intent".into(),
        session_id: capture.session.session_id,
        snapshot_id: capture.snapshot.snapshot_id,
        expected_package: capture.snapshot.package_name,
        requested_at: Utc::now(),
        target: mobile_runtime::MobileActionTarget::Back,
        text: None,
    };
    runtime.record_mobile_action_intent(&request).await.unwrap();
    repo.attach_step_action(&s.id, request.action_id.clone())
        .await
        .unwrap();
    repo.update_step_status(&s.id, MobileStepStatus::Executing, None)
        .await
        .unwrap();
    let result = MobileStepResult {
        step_id: s.id.clone(),
        outcome: MobileStepOutcome::Verified,
        verified: true,
        observation_before_id: Some(capture.observation.id),
        observation_after_id: None,
        action_receipt_id: Some(request.action_id),
        evidence_ids: vec![],
        error: None,
        created_at: Utc::now(),
    };
    assert!(repo.record_step_result(result).await.is_err());
    assert_eq!(
        repo.get_step(&s.id).await.unwrap().status,
        MobileStepStatus::Executing
    );
}

#[tokio::test]
async fn owner_input_completion_target_rejects_secret_like_text_before_persistence_or_model_context()
 {
    let dir = tempfile::tempdir().unwrap();
    let runtime = AppRuntime::initialize(&dir.path().join("owner-target.sqlite3"))
        .await
        .unwrap();
    let repo = runtime.mobile_goal_repository();
    let goal = repo
        .create_goal("Check system search input", Default::default())
        .await
        .unwrap();
    for value in [
        "a".repeat(64),
        "private key".into(),
        "seed phrase".into(),
        "send message".into(),
        "fixture ".repeat(12),
    ] {
        let target = MobileCompletionTarget::InputTextEquals {
            package: "com.google.android.settings.intelligence".into(),
            activity: "com.google.android.settings.intelligence.modules.search.SearchActivity"
                .into(),
            resource_id: "com.google.android.settings.intelligence:id/open_search_view_edit_text"
                .into(),
            value,
        };
        assert!(repo.set_completion_target(&goal.id, &target).await.is_err());
    }
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM mobile_goal_completion_targets WHERE goal_id=?")
            .bind(goal.id.as_str())
            .fetch_one(runtime.pool())
            .await
            .unwrap();
    assert_eq!(count, 0);
    assert_eq!(
        repo.get_goal(&goal.id).await.unwrap().status,
        MobileGoalStatus::Pending
    );
}
