use mobile_runtime::execution::*;
#[test]
fn goal_state_machine_rejects_terminal_revival() {
    assert!(MobileGoalStatus::Pending.can_transition_to(MobileGoalStatus::Planning));
    assert!(MobileGoalStatus::Running.can_transition_to(MobileGoalStatus::WaitingApproval));
    assert!(MobileGoalStatus::WaitingApproval.can_transition_to(MobileGoalStatus::Running));
    for terminal in [
        MobileGoalStatus::Completed,
        MobileGoalStatus::Failed,
        MobileGoalStatus::Stopped,
    ] {
        for next in [
            MobileGoalStatus::Pending,
            MobileGoalStatus::Planning,
            MobileGoalStatus::Running,
        ] {
            assert!(!terminal.can_transition_to(next));
        }
    }
    assert!(!MobileGoalStatus::Pending.can_transition_to(MobileGoalStatus::Completed));
}
#[test]
fn step_state_machine_rejects_unverified_completion_and_reexecution() {
    assert!(MobileStepStatus::Pending.can_transition_to(MobileStepStatus::Executing));
    assert!(!MobileStepStatus::Pending.can_transition_to(MobileStepStatus::Verified));
    assert!(MobileStepStatus::Executing.can_transition_to(MobileStepStatus::Verified));
    assert!(!MobileStepStatus::Verified.can_transition_to(MobileStepStatus::Executing));
}
#[test]
fn typed_ids_reject_wrong_prefix_and_invalid_uuid() {
    let id = MobileGoalId::new();
    assert_eq!(
        serde_json::from_str::<MobileGoalId>(&serde_json::to_string(&id).unwrap()).unwrap(),
        id
    );
    assert!(serde_json::from_str::<MobilePlanId>(&serde_json::to_string(&id).unwrap()).is_err());
    assert!(serde_json::from_str::<MobileGoalId>("\"mobile_goal_bad\"").is_err());
}
#[test]
fn errors_have_machine_readable_codes_separate_from_messages() {
    let error = MobileGoalError::new(MobileGoalErrorCode::ModelNotConfigured);
    let value = serde_json::to_value(error).unwrap();
    assert_eq!(value["code"], "MODEL_NOT_CONFIGURED");
    assert!(value["message"].is_string());
    assert!(serde_json::from_str::<MobileGoalErrorCode>("\"invented\"").is_err());
}
#[test]
fn approval_hash_is_deterministic_and_binds_every_subject_field() {
    let subject = MobileApprovalSubject {
        goal_id: MobileGoalId::new(),
        plan_id: MobilePlanId::new(),
        step_id: MobilePlanStepId::new(),
        action_type: MobileStepType::TapElement,
        target_hash: Some("sha256:target".into()),
        observation_id: Some("observation-1".into()),
    };
    let hash = subject.canonical_hash().unwrap();
    let decoded: MobileApprovalSubject =
        serde_json::from_str(&serde_json::to_string(&subject).unwrap()).unwrap();
    assert_eq!(hash, decoded.canonical_hash().unwrap());
    let mut changes = vec![];
    let mut changed = subject.clone();
    changed.goal_id = MobileGoalId::new();
    changes.push(changed);
    let mut changed = subject.clone();
    changed.plan_id = MobilePlanId::new();
    changes.push(changed);
    let mut changed = subject.clone();
    changed.step_id = MobilePlanStepId::new();
    changes.push(changed);
    let mut changed = subject.clone();
    changed.action_type = MobileStepType::InputText;
    changes.push(changed);
    let mut changed = subject.clone();
    changed.target_hash = None;
    changes.push(changed);
    let mut changed = subject;
    changed.observation_id = Some("observation-2".into());
    changes.push(changed);
    for changed in changes {
        assert_ne!(hash, changed.canonical_hash().unwrap());
    }
}
#[test]
fn semantic_steps_reject_shell_and_coordinate_payloads() {
    assert!(serde_json::from_str::<NewMobileStep>(r#"{"sequence":1,"stepType":"TAP_ELEMENT","reason":"read","risk":"REVERSIBLE","targetRef":"@e1","command":"adb shell"}"#).is_err());
    assert!(
        serde_json::from_str::<ExpectedStepResult>(
            r#"{"kind":"ARBITRARY_SCRIPT","script":"anything"}"#
        )
        .is_err()
    );
}
