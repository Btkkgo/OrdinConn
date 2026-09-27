use chrono::Utc;
use mobile_runtime::{execution::*, executor::*, *};
fn capture(text: &str) -> MobileCapture {
    let snapshot = MobileUiSnapshot::from_elements(
        "session",
        "com.android.settings",
        "Settings",
        1080,
        2400,
        vec![RawMobileElement {
            text: Some(text.into()),
            role: "textbox".into(),
            class_name: "android.widget.EditText".into(),
            content_description: None,
            bounds: MobileBounds {
                x: 10,
                y: 200,
                width: 400,
                height: 60,
            },
            clickable: true,
            scrollable: false,
            enabled: true,
            focused: true,
            selected: false,
            resource_id: Some("com.android.settings:id/search".into()),
            password: false,
        }],
    );
    let frame = MobileFrame::from_png("session", 1080, 2400, vec![1, 2, 3]);
    let observation = MobileObservation::from_snapshot(
        &snapshot,
        &frame,
        None,
        "settings",
        PrivacyClass::UserAllowed,
    );
    MobileCapture {
        session: MobileDeviceSession {
            session_id: "session".into(),
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
#[test]
fn text_verification_checks_the_value_not_a_changed_page() {
    let b = capture("before");
    let a = capture("wrong");
    let e = ExpectedStepResult::TextEquals {
        element_ref: "@e1".into(),
        value: "OrdinConn M3 Test".into(),
    };
    assert_eq!(
        MobileVerificationEngine::verify(&e, &b, &a, None, false),
        MobileVerificationOutcome::Failed
    );
    let a = capture("OrdinConn M3 Test");
    assert_eq!(
        MobileVerificationEngine::verify(&e, &b, &a, None, false),
        MobileVerificationOutcome::Verified
    );
}
#[test]
fn verification_uses_exact_activity_semantic_targets_and_structured_changes() {
    let b = capture("before");
    let mut a = b.clone();
    a.frame.frame_hash = "different-png".into();
    assert_eq!(
        MobileVerificationEngine::verify(&ExpectedStepResult::UiChanged, &b, &a, None, false),
        MobileVerificationOutcome::Failed
    );
    a.snapshot.activity = "Different".into();
    a.observation.activity = "Different".into();
    assert_eq!(
        MobileVerificationEngine::verify(&ExpectedStepResult::ActivityChanged, &b, &a, None, false),
        MobileVerificationOutcome::Verified
    );
    assert_eq!(
        MobileVerificationEngine::verify(
            &ExpectedStepResult::ActivityEquals {
                package: "com.android.settings".into(),
                activity: "Wrong".into()
            },
            &b,
            &a,
            None,
            false
        ),
        MobileVerificationOutcome::Failed
    );
    assert_eq!(
        MobileVerificationEngine::verify(
            &ExpectedStepResult::ActivityEquals {
                package: "com.android.settings".into(),
                activity: "Different".into()
            },
            &b,
            &a,
            None,
            false
        ),
        MobileVerificationOutcome::Verified
    );
    let mut a = b.clone();
    a.snapshot.elements[0].resource_id = Some("different-control".into());
    assert_eq!(
        MobileVerificationEngine::verify(
            &ExpectedStepResult::ElementVisible {
                element_ref: "@e1".into()
            },
            &b,
            &a,
            None,
            false
        ),
        MobileVerificationOutcome::Failed
    );
    assert_eq!(
        MobileVerificationEngine::verify(
            &ExpectedStepResult::ElementVisible {
                element_ref: "@e1".into()
            },
            &b,
            &b,
            None,
            false
        ),
        MobileVerificationOutcome::Verified
    );
    assert_eq!(
        MobileVerificationEngine::verify(&ExpectedStepResult::NewDataObject, &b, &b, None, false),
        MobileVerificationOutcome::Failed
    );
    assert_eq!(
        MobileVerificationEngine::verify(&ExpectedStepResult::NewDataObject, &b, &b, None, true),
        MobileVerificationOutcome::Verified
    );
    assert_eq!(
        MobileVerificationEngine::verify(
            &ExpectedStepResult::NoChangeExpected,
            &b,
            &b,
            None,
            false
        ),
        MobileVerificationOutcome::Verified
    );
}
#[test]
fn different_devices_sessions_sensitive_and_unverified_observations_cannot_verify() {
    let b = capture("before");
    for variant in 0..4 {
        let mut a = b.clone();
        match variant {
            0 => a.session.device_id = "emulator-5556".into(),
            1 => a.snapshot.session_id = "other".into(),
            2 => a.observation.privacy_class = PrivacyClass::Sensitive,
            _ => a.observation.verification_status = VerificationResult::Interrupted,
        };
        assert_eq!(
            MobileVerificationEngine::verify(
                &ExpectedStepResult::NoChangeExpected,
                &b,
                &a,
                None,
                false
            ),
            MobileVerificationOutcome::Inconclusive
        );
    }
}
#[test]
fn system_status_clock_changes_are_not_business_ui_changes() {
    let mut b = capture("value");
    let mut e = b.snapshot.elements[0].clone();
    e.resource_id = Some("com.android.systemui:id/clock".into());
    e.class_name = "android.widget.TextView".into();
    e.element_ref = "@e2".into();
    e.text = Some("12:00".into());
    b.snapshot.elements.push(e);
    let mut a = b.clone();
    a.snapshot.elements[1].text = Some("12:01".into());
    assert!(same_app_ui(&b.snapshot, &a.snapshot));
    assert_eq!(
        MobileVerificationEngine::verify(&ExpectedStepResult::UiChanged, &b, &a, None, false),
        MobileVerificationOutcome::Failed
    );
}
fn step(b: &MobileCapture, kind: MobileStepType) -> MobilePlanStep {
    MobilePlanStep {
        id: MobilePlanStepId::new(),
        plan_id: MobilePlanId::new(),
        sequence: 1,
        step_type: kind,
        status: MobileStepStatus::Pending,
        reason: "Safe navigation".into(),
        risk: MobileStepRisk::Reversible,
        target_ref: None,
        input_text: None,
        expected_result: Some(ExpectedStepResult::UiChanged),
        wait_ms: None,
        extraction_intent: None,
        created_at: Utc::now(),
        started_at: None,
        finished_at: None,
        observation_before_id: Some(b.observation.id.clone()),
        observation_after_id: None,
        action_id: None,
        evidence_id: None,
        error_code: None,
        error_message: None,
    }
}
#[test]
fn semantic_resolver_maps_scroll_and_element_actions_without_saved_coordinates() {
    let b = capture("before");
    let allow = vec!["com.android.settings".into()];
    for (kind, direction) in [
        (MobileStepType::ScrollDown, SwipeDirection::Up),
        (MobileStepType::ScrollUp, SwipeDirection::Down),
    ] {
        let s = step(&b, kind);
        let r = resolve_semantic_step(&s, &b, Some(&b.snapshot), "execution", &allow)
            .unwrap()
            .request
            .unwrap();
        assert_eq!(r.target, MobileActionTarget::Swipe { direction });
    }
    let mut s = step(&b, MobileStepType::TapElement);
    s.target_ref = Some("@e1".into());
    assert_eq!(
        resolve_semantic_step(&s, &b, Some(&b.snapshot), "execution", &allow)
            .unwrap()
            .request
            .unwrap()
            .target,
        MobileActionTarget::Tap {
            element_ref: "@e1".into()
        }
    );
    s.step_type = MobileStepType::InputText;
    s.input_text = Some("OrdinConn M3 Test".into());
    s.expected_result = Some(ExpectedStepResult::TextEquals {
        element_ref: "@e1".into(),
        value: "OrdinConn M3 Test".into(),
    });
    assert!(resolve_semantic_step(&s, &b, Some(&b.snapshot), "execution", &allow).is_ok());
}
#[test]
fn resolver_blocks_stale_targets_sensitive_text_fake_risk_weak_back_and_unbounded_wait() {
    let b = capture("before");
    let allow = vec!["com.android.settings".into()];
    let mut s = step(&b, MobileStepType::TapElement);
    s.target_ref = Some("@invented".into());
    assert_eq!(
        resolve_semantic_step(&s, &b, Some(&b.snapshot), "execution", &allow)
            .unwrap_err()
            .code,
        MobileGoalErrorCode::TargetNotFound
    );
    s.target_ref = Some("@e1".into());
    s.risk = MobileStepRisk::ReadOnly;
    assert_eq!(
        resolve_semantic_step(&s, &b, Some(&b.snapshot), "execution", &allow)
            .unwrap_err()
            .code,
        MobileGoalErrorCode::PolicyBlocked
    );
    s = step(&b, MobileStepType::Back);
    assert!(resolve_semantic_step(&s, &b, Some(&b.snapshot), "execution", &allow).is_err());
    s = step(&b, MobileStepType::Wait);
    s.risk = MobileStepRisk::ReadOnly;
    s.wait_ms = Some(5001);
    s.expected_result = Some(ExpectedStepResult::NoChangeExpected);
    assert!(resolve_semantic_step(&s, &b, Some(&b.snapshot), "execution", &allow).is_err());
    s = step(&b, MobileStepType::InputText);
    s.target_ref = Some("@e1".into());
    s.input_text = Some("password".into());
    s.expected_result = Some(ExpectedStepResult::TextEquals {
        element_ref: "@e1".into(),
        value: "password".into(),
    });
    assert_eq!(
        resolve_semantic_step(&s, &b, Some(&b.snapshot), "execution", &allow)
            .unwrap_err()
            .code,
        MobileGoalErrorCode::PolicyBlocked
    );
    let mut a = b.clone();
    a.snapshot.elements[0].text = Some("different".into());
    s = step(&b, MobileStepType::ScrollDown);
    assert!(resolve_semantic_step(&s, &a, Some(&b.snapshot), "execution", &allow).is_err());
}
#[test]
fn editable_values_preserve_business_whitespace_and_case() {
    let b = capture("OrdinConn M3 Test ");
    assert_eq!(
        b.snapshot.elements[0].text.as_deref(),
        Some("OrdinConn M3 Test ")
    );
    let a = capture("OrdinConn M3 Test");
    assert_eq!(
        MobileVerificationEngine::verify(
            &ExpectedStepResult::TextEquals {
                element_ref: "@e1".into(),
                value: "OrdinConn M3 Test ".into()
            },
            &b,
            &a,
            None,
            false
        ),
        MobileVerificationOutcome::Failed
    );
}

fn input_receipt(b: &MobileCapture, a: &MobileCapture) -> MobileActionReceipt {
    MobileActionReceipt {
        action_id: "action".into(),
        session_id: b.session.session_id.clone(),
        snapshot_id: b.snapshot.snapshot_id.clone(),
        target: MobileActionTarget::Type {
            element_ref: "@e1".into(),
        },
        decision: MobileActionDecision::Allowed,
        status: MobileActionStatus::Executed,
        requested_at: Utc::now(),
        completed_at: Utc::now(),
        pre_package: b.snapshot.package_name.clone(),
        pre_activity: b.snapshot.activity.clone(),
        pre_frame_hash: b.observation.frame_hash.clone(),
        pre_ui_tree_hash: b.observation.ui_tree_hash.clone(),
        post_package: Some(a.snapshot.package_name.clone()),
        post_activity: Some(a.snapshot.activity.clone()),
        post_frame_hash: Some(a.observation.frame_hash.clone()),
        post_ui_tree_hash: Some(a.observation.ui_tree_hash.clone()),
        post_snapshot_id: Some(a.snapshot.snapshot_id.clone()),
        verification: Some(VerificationResult::Verified),
        text_length: Some(18),
        text_sha256: Some(SensitiveText::new("OrdinConn M3 Test".into()).sha256()),
        command_sent: true,
        input_value_verified: Some(true),
    }
}
#[test]
fn redacted_input_verifies_only_a_bound_exact_value_attestation() {
    let b = capture("before");
    let a = capture("[REDACTED]");
    let e = ExpectedStepResult::TextEquals {
        element_ref: "@e1".into(),
        value: "OrdinConn M3 Test".into(),
    };
    let mut r = input_receipt(&b, &a);
    r.text_length = Some("OrdinConn M3 Test".chars().count());
    assert_eq!(
        MobileVerificationEngine::verify(&e, &b, &a, Some(&r), false),
        MobileVerificationOutcome::Verified
    );
    for variant in 0..5 {
        let mut bad = r.clone();
        match variant {
            0 => bad.input_value_verified = Some(false),
            1 => bad.text_sha256 = Some("sha256:wrong".into()),
            2 => {
                bad.target = MobileActionTarget::Type {
                    element_ref: "@e2".into(),
                }
            }
            3 => bad.text_length = Some(15),
            _ => bad.verification = Some(VerificationResult::UnexpectedState),
        };
        assert_ne!(
            MobileVerificationEngine::verify(&e, &b, &a, Some(&bad), false),
            MobileVerificationOutcome::Verified
        );
    }
    let mut bad = r.clone();
    bad.post_snapshot_id = Some("unrelated".into());
    assert_eq!(
        MobileVerificationEngine::verify(&e, &b, &a, Some(&bad), false),
        MobileVerificationOutcome::Inconclusive
    );
}
#[test]
fn extraneous_semantic_fields_and_ambiguous_targets_fail_closed() {
    let b = capture("before");
    let allow = vec!["com.android.settings".into()];
    let mut s = step(&b, MobileStepType::ScrollDown);
    s.input_text = Some("unexpected".into());
    assert!(resolve_semantic_step(&s, &b, Some(&b.snapshot), "exec", &allow).is_err());
    s.input_text = None;
    s.wait_ms = Some(1);
    assert!(resolve_semantic_step(&s, &b, Some(&b.snapshot), "exec", &allow).is_err());
    let mut b = b;
    b.snapshot.elements.push(b.snapshot.elements[0].clone());
    let mut s = step(&b, MobileStepType::TapElement);
    s.target_ref = Some("@e1".into());
    assert!(resolve_semantic_step(&s, &b, Some(&b.snapshot), "exec", &allow).is_err());
}

#[test]
fn qualified_system_resource_ids_are_metadata_but_sensitive_ids_stay_blocked() {
    let mut b = capture("Search settings");
    b.snapshot.elements[0].resource_id =
        Some("com.google.android.settings.intelligence:id/open_search_view_edit_text".into());
    let mut s = step(&b, MobileStepType::InputText);
    s.target_ref = Some("@e1".into());
    s.input_text = Some("ORDINCONN M3 INPUT abc123".into());
    s.expected_result = Some(ExpectedStepResult::TextEquals {
        element_ref: "@e1".into(),
        value: s.input_text.clone().unwrap(),
    });
    let allowed = vec!["com.android.settings".into()];
    assert!(resolve_semantic_step(&s, &b, Some(&b.snapshot), "execution", &allowed).is_ok());
    for resource in [
        "com.android.settings:id/password",
        "com.android.settings:id/wallet_sign",
        "com.android.settings:id/private_key",
        "com.android.settings:id/send_message",
    ] {
        b.snapshot.elements[0].resource_id = Some(resource.into());
        assert_eq!(
            resolve_semantic_step(&s, &b, Some(&b.snapshot), "execution", &allowed)
                .unwrap_err()
                .code,
            MobileGoalErrorCode::PolicyBlocked
        );
    }
    assert!(
        mobile_runtime::planner::prompt_android_identifier("com.android.settings/id/shell command")
            .is_none()
    );
}
