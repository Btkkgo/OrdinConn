use super::*;
use crate::*;
fn capture(texts: &[&str]) -> MobileCapture {
    let snapshot = MobileUiSnapshot::from_elements(
        "s",
        "com.android.settings",
        ".Settings",
        100,
        200,
        texts
            .iter()
            .enumerate()
            .map(|(i, text)| RawMobileElement {
                text: Some((*text).into()),
                role: "textview".into(),
                class_name: "android.widget.TextView".into(),
                content_description: None,
                resource_id: Some(format!("id/item{i}")),
                bounds: MobileBounds {
                    x: 0,
                    y: i as u32 * 20,
                    width: 100,
                    height: 20,
                },
                clickable: false,
                scrollable: false,
                enabled: true,
                focused: false,
                selected: false,
                password: false,
            })
            .collect(),
    );
    let frame = MobileFrame::from_png("s", 100, 200, vec![]);
    let observation = crate::MobileObservation::from_snapshot(
        &snapshot,
        &frame,
        None,
        "generic-android",
        PrivacyClass::UserAllowed,
    );
    MobileCapture {
        session: MobileDeviceSession {
            session_id: "s".into(),
            device_id: "emulator-fixture".into(),
            platform: MobilePlatform::Android,
            device_type: MobileDeviceType::Emulator,
            os_version: "15".into(),
            screen_width: 100,
            screen_height: 200,
            connected_at: Utc::now(),
            current_app: Some(snapshot.package_name.clone()),
            current_activity: Some(snapshot.activity.clone()),
            status: MobileSessionStatus::Connected,
            last_observation_at: Some(Utc::now()),
        },
        snapshot,
        frame,
        observation,
    }
}
#[test]
fn normalizes_whitespace_without_changing_content() {
    assert_eq!(normalize_text("  BTC \n  42 \t "), "BTC 42");
}
#[test]
fn observation_keeps_source_bounds_and_previous_identity() {
    let o = observation_from_capture(&capture(&[" BTC "]), Some("obs_before".into()));
    assert_eq!(o.source, "android_ui_tree");
    assert_eq!(o.element_count, 1);
    assert_eq!(o.elements[0].text.as_deref(), Some("BTC"));
    assert_eq!(o.elements[0].bounds.height, 20);
    assert_eq!(o.previous_observation_id.as_deref(), Some("obs_before"));
}
#[test]
fn no_change_ignores_observation_ids_and_timestamps() {
    let a = observation_from_capture(&capture(&["BTC"]), None);
    let b = observation_from_capture(&capture(&["BTC"]), None);
    assert!(!diff_observations(&a, &b).changed);
    assert_eq!(a.ui_tree_hash, b.ui_tree_hash);
}
#[test]
fn added_removed_text_app_activity_and_selection_changes() {
    let a = observation_from_capture(&capture(&["BTC", "ETH"]), None);
    let mut b = observation_from_capture(&capture(&["BTC", "SOL", "NVDA"]), None);
    let d = diff_observations(&a, &b);
    assert_eq!(d.added_elements.len(), 1);
    assert!(d.change_types.contains(&"TEXT_CHANGED".into()));
    assert_eq!(diff_observations(&b, &a).removed_elements.len(), 1);
    b.package_name = "com.example.public".into();
    b.activity_name = ".Detail".into();
    b.elements[0].selected = true;
    let d = diff_observations(&a, &b);
    for kind in ["APP_CHANGED", "ACTIVITY_CHANGED", "SELECTION_CHANGED"] {
        assert!(d.change_types.contains(&kind.into()));
    }
}
#[test]
fn extracts_numeric_list_and_preserves_provenance() {
    let mut o = observation_from_capture(
        &capture(&["BTC", "67,842", "2026-10-02 11:42", "Connected"]),
        None,
    );
    o.elements[0].role = "list_item".into();
    let data = extract_mobile_data(&o);
    for kind in [
        "visible_text",
        "numeric_value",
        "list_item",
        "timestamp_like_text",
        "status",
    ] {
        assert!(data.iter().any(|d| d.data_type == kind), "{kind}");
    }
    let objects = build_data_objects(&o, &data);
    assert!(objects.iter().any(|d| d.object_type == "metric"));
    assert!(
        objects
            .iter()
            .all(|d| d.provenance.observation_id == o.id && !d.provenance.element_ids.is_empty())
    );
    let other = observation_from_capture(
        &capture(&["BTC", "67,842", "2026-10-02 11:42", "Connected"]),
        None,
    );
    let first = build_data_objects(&other, &extract_mobile_data(&other));
    assert!(first.iter().any(|d| {
        objects
            .iter()
            .any(|x| x.deduplication_key == d.deduplication_key)
    }));
}
#[test]
fn empty_page_extracts_nothing() {
    let o = observation_from_capture(&capture(&[]), None);
    assert!(extract_mobile_data(&o).is_empty());
    assert!(build_data_objects(&o, &[]).is_empty());
}
#[test]
fn sensitive_values_never_enter_projection_or_extraction() {
    let mut c = capture(&["safe title", "123456"]);
    c.snapshot.elements[1].resource_id = Some("id/pin".into());
    let o = observation_from_capture(&c, None);
    let json = serde_json::to_string(&o).unwrap();
    assert!(!json.contains("123456"));
    assert!(
        o.redactions
            .iter()
            .any(|x| x.contains("REDACTED_SENSITIVE_ELEMENT"))
    );
    assert!(!extract_mobile_data(&o).iter().any(|d| d.value == "123456"));
}

struct Driver {
    observations: std::collections::VecDeque<Result<super::MobileObservation, InteractionError>>,
    calls: Vec<&'static str>,
    elapsed: u64,
    cancel: bool,
    cancel_after: Option<usize>,
    fail_action: bool,
    checkpoints: Vec<MobileActionResult>,
}
impl ManualInteractionDriver for Driver {
    fn observe(&mut self) -> Result<super::MobileObservation, InteractionError> {
        self.calls.push("observe");
        self.observations
            .pop_front()
            .unwrap_or(Err(InteractionError::Timeout))
    }
    fn act(&mut self) -> Result<(), InteractionError> {
        self.calls.push("act");
        if self.fail_action {
            Err(InteractionError::ActionFailed)
        } else {
            Ok(())
        }
    }
    fn cancelled(&self) -> bool {
        self.cancel || self.cancel_after.is_some_and(|n| self.calls.len() >= n)
    }
    fn elapsed_ms(&self) -> u64 {
        self.elapsed
    }
    fn poll_interval(&mut self) {
        self.elapsed += 200;
    }
    fn checkpoint(&mut self, r: &MobileActionResult) -> Result<(), InteractionError> {
        self.checkpoints.push(r.clone());
        Ok(())
    }
}
fn result() -> MobileActionResult {
    MobileActionResult {
        action_id: "action_fixture".into(),
        action_type: MobileActionType::Tap,
        started_at: Utc::now(),
        completed_at: None,
        status: "running".into(),
        device_id: "emulator-fixture".into(),
        before_observation_id: None,
        after_observation_id: None,
        error_code: None,
        error_message: None,
        triggered_by: "owner".into(),
    }
}
fn driver() -> Driver {
    let a = observation_from_capture(&capture(&["Before"]), None);
    let b = observation_from_capture(&capture(&["After"]), Some(a.id.clone()));
    let mut c = b.clone();
    c.id = "obs_stable".into();
    Driver {
        observations: vec![Ok(a), Ok(b), Ok(c)].into(),
        calls: vec![],
        elapsed: 0,
        cancel: false,
        cancel_after: None,
        fail_action: false,
        checkpoints: vec![],
    }
}
#[test]
fn action_observes_before_after_and_waits_for_stability() {
    let mut d = driver();
    let r = run_manual_interaction(&mut d, result());
    assert_eq!(d.calls, vec!["observe", "act", "observe", "observe"]);
    assert_eq!(r.status, "completed");
    assert!(r.before_observation_id.is_some());
    assert_eq!(r.after_observation_id.as_deref(), Some("obs_stable"));
    assert!(
        d.checkpoints
            .iter()
            .any(|r| r.status == "running" && r.before_observation_id.is_some())
    );
}
#[test]
fn cancelled_action_never_sends_input() {
    let mut d = driver();
    d.cancel = true;
    let r = run_manual_interaction(&mut d, result());
    assert_eq!(r.status, "cancelled");
    assert!(d.calls.is_empty());
    assert!(r.after_observation_id.is_none());
}
#[test]
fn failed_action_never_fabricates_after_state() {
    let mut d = driver();
    d.fail_action = true;
    let r = run_manual_interaction(&mut d, result());
    assert_eq!(r.status, "failed");
    assert_eq!(d.calls, vec!["observe", "act"]);
    assert!(r.after_observation_id.is_none());
}
#[test]
fn disconnected_device_preserves_failure() {
    let mut d = driver();
    d.observations = vec![Err(InteractionError::DeviceDisconnected)].into();
    let r = run_manual_interaction(&mut d, result());
    assert_eq!(r.error_code.as_deref(), Some("DEVICE_DISCONNECTED"));
    assert!(r.before_observation_id.is_none());
    assert!(r.after_observation_id.is_none());
}
#[test]
fn unstable_page_times_out_without_inventing_observation() {
    let mut d = driver();
    d.observations = vec![
        d.observations.pop_front().unwrap(),
        Err(InteractionError::Timeout),
    ]
    .into();
    let r = run_manual_interaction(&mut d, result());
    assert_eq!(r.status, "failed");
    assert_eq!(r.error_code.as_deref(), Some("UI_STABILITY_TIMEOUT"));
    assert!(r.after_observation_id.is_none());
}

#[test]
fn manual_actions_reject_destructive_account_and_publication_targets() {
    for label in [
        "Erase all data",
        "Factory reset",
        "Delete data",
        "Sign in",
        "Send message",
        "Publish",
        "Confirm purchase",
        "Install APK",
        "Enter verification code",
    ] {
        assert!(prohibited_manual_target(label), "{label}");
    }
    assert!(!prohibited_manual_target("Network settings"));
}
#[test]
fn manual_input_rejects_code_and_key_shaped_values() {
    for value in [
        "123456",
        "password value",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "one two three four five six seven eight nine ten eleven twelve",
    ] {
        assert!(prohibited_manual_input(value));
    }
    assert!(!prohibited_manual_input("wifi"));
}

#[test]
fn stop_during_last_observation_retains_evidence_and_cancels_action() {
    let mut d = driver();
    d.cancel_after = Some(4);
    let r = run_manual_interaction(&mut d, result());
    assert_eq!(r.status, "cancelled");
    assert_eq!(r.after_observation_id.as_deref(), Some("obs_stable"));
}
#[test]
fn extractor_reports_title_button_link_and_selected_sources() {
    let mut o = observation_from_capture(&capture(&["Overview", "https://example.invalid"]), None);
    o.elements[0].class_name = "android.widget.Button".into();
    o.elements[0].selected = true;
    let data = extract_mobile_data(&o);
    for kind in ["title", "button", "link_like_element", "selected_item"] {
        assert!(data.iter().any(|d| d.data_type == kind));
    }
    assert!(
        data.iter()
            .all(|d| d.extraction_method == "ui_tree" || d.extraction_method == "rule")
    );
}
#[test]
fn diff_reports_scroll_content_and_hierarchy_changes() {
    let mut a = observation_from_capture(&capture(&["before"]), None);
    a.elements[0].scrollable = true;
    let mut b = a.clone();
    b.id = "after".into();
    b.elements[0].text = Some("after".into());
    let d = diff_observations(&a, &b);
    assert!(d.change_types.contains(&"SCROLL_CONTENT_CHANGED".into()));
}
