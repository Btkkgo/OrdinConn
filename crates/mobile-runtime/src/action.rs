use crate::{
    GenericAndroidSkill, MobileDeviceSession, MobileDeviceType, MobileObservation,
    MobileSessionStatus, MobileUiSnapshot, VerificationResult,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Clone, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SensitiveText(String);

impl SensitiveText {
    pub fn new(value: String) -> Self {
        Self(value)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.chars().count()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn sha256(&self) -> String {
        format!("sha256:{:x}", Sha256::digest(self.0.as_bytes()))
    }
}

impl fmt::Debug for SensitiveText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MobileActionTarget {
    Tap { element_ref: String },
    Swipe { direction: SwipeDirection },
    Type { element_ref: String },
    Back,
    Home,
    OpenApp { package_name: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileActionRequest {
    pub action_id: String,
    pub session_id: String,
    pub snapshot_id: String,
    pub expected_package: String,
    pub requested_at: DateTime<Utc>,
    pub target: MobileActionTarget,
    #[serde(default, skip_serializing)]
    pub text: Option<SensitiveText>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MobileActionDenyReason {
    InactiveSession,
    PhysicalDevice,
    BudgetExceeded,
    PackageNotAllowed,
    ForegroundChanged,
    StaleSnapshot,
    SensitiveScreen,
    TargetMissing,
    SensitiveTarget,
    TargetDisabled,
    TargetNotClickable,
    TargetNotEditable,
    InvalidBounds,
    UnsafeText,
    InvalidAction,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "outcome", content = "reason", rename_all = "snake_case")]
pub enum MobileActionDecision {
    Allowed,
    Denied(MobileActionDenyReason),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MobileActionStatus {
    Blocked,
    Executed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileActionReceipt {
    pub action_id: String,
    pub session_id: String,
    pub snapshot_id: String,
    pub target: MobileActionTarget,
    pub decision: MobileActionDecision,
    pub status: MobileActionStatus,
    pub requested_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub pre_package: String,
    pub pre_activity: String,
    pub pre_frame_hash: String,
    pub pre_ui_tree_hash: String,
    pub post_package: Option<String>,
    pub post_activity: Option<String>,
    pub post_frame_hash: Option<String>,
    pub post_ui_tree_hash: Option<String>,
    pub verification: Option<VerificationResult>,
    pub text_length: Option<usize>,
    pub text_sha256: Option<String>,
}

/// Independent policy evaluator. The host must recheck the live foreground before input.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_action(
    request: &MobileActionRequest,
    active_session: &MobileDeviceSession,
    snapshot: &MobileUiSnapshot,
    allowed_apps: &[String],
    action_count: u8,
    foreground_package: &str,
    foreground_activity: &str,
    now: DateTime<Utc>,
) -> MobileActionDecision {
    use MobileActionDenyReason as D;
    let deny = |reason| MobileActionDecision::Denied(reason);
    if active_session.status != MobileSessionStatus::Connected
        || request.session_id != active_session.session_id
        || snapshot.session_id != active_session.session_id
    {
        return deny(D::InactiveSession);
    }
    if active_session.device_type != MobileDeviceType::Emulator
        || !active_session.device_id.starts_with("emulator-")
    {
        return deny(D::PhysicalDevice);
    }
    if action_count >= 20 {
        return deny(D::BudgetExceeded);
    }
    if !allowed_apps
        .iter()
        .any(|package| package == &snapshot.package_name)
        || !allowed_apps
            .iter()
            .any(|package| package == &request.expected_package)
    {
        return deny(D::PackageNotAllowed);
    }
    if request.expected_package != snapshot.package_name
        || foreground_package != snapshot.package_name
        || foreground_activity != snapshot.activity
    {
        return deny(D::ForegroundChanged);
    }
    if request.snapshot_id != snapshot.snapshot_id
        || now
            .signed_duration_since(snapshot.captured_at)
            .num_milliseconds()
            .abs()
            > 10_000
        || request
            .requested_at
            .signed_duration_since(now)
            .num_milliseconds()
            .abs()
            > 10_000
    {
        return deny(D::StaleSnapshot);
    }
    if snapshot.sensitive_state.is_some()
        && !matches!(
            request.target,
            MobileActionTarget::Back | MobileActionTarget::Home
        )
    {
        return deny(D::SensitiveScreen);
    }
    match &request.target {
        MobileActionTarget::Tap { element_ref } | MobileActionTarget::Type { element_ref } => {
            let Some(element) = snapshot
                .elements
                .iter()
                .find(|element| &element.element_ref == element_ref)
            else {
                return deny(D::TargetMissing);
            };
            if snapshot
                .redactions
                .iter()
                .any(|redaction| redaction.starts_with(&format!("element:{element_ref}:")))
                || element.text.as_deref() == Some("[REDACTED]")
            {
                return deny(D::SensitiveTarget);
            }
            if !element.enabled {
                return deny(D::TargetDisabled);
            }
            if element.bounds.width == 0
                || element.bounds.height == 0
                || element
                    .bounds
                    .x
                    .checked_add(element.bounds.width)
                    .is_none_or(|edge| edge > snapshot.screen_width)
                || element
                    .bounds
                    .y
                    .checked_add(element.bounds.height)
                    .is_none_or(|edge| edge > snapshot.screen_height)
            {
                return deny(D::InvalidBounds);
            }
            if matches!(request.target, MobileActionTarget::Tap { .. }) && !element.clickable {
                return deny(D::TargetNotClickable);
            }
            if matches!(request.target, MobileActionTarget::Type { .. }) {
                if !element.class_name.ends_with("EditText") {
                    return deny(D::TargetNotEditable);
                }
                if !request
                    .text
                    .as_ref()
                    .is_some_and(|text| safe_text(text.as_str()))
                {
                    return deny(D::UnsafeText);
                }
            } else if request.text.is_some() {
                return deny(D::InvalidAction);
            }
        }
        MobileActionTarget::Swipe { .. } | MobileActionTarget::Back | MobileActionTarget::Home => {
            if request.text.is_some() {
                return deny(D::InvalidAction);
            }
            if snapshot.screen_width < 100 || snapshot.screen_height < 100 {
                return deny(D::InvalidBounds);
            }
        }
        MobileActionTarget::OpenApp { package_name } => {
            if request.text.is_some() {
                return deny(D::InvalidAction);
            }
            if !allowed_apps.iter().any(|allowed| allowed == package_name) {
                return deny(D::PackageNotAllowed);
            }
            if !safe_package(package_name) {
                return deny(D::InvalidAction);
            }
        }
    }
    MobileActionDecision::Allowed
}

fn safe_text(value: &str) -> bool {
    let length = value.chars().count();
    length > 0
        && length <= 256
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == ' ')
}

fn safe_package(value: &str) -> bool {
    !value.is_empty()
        && value.contains('.')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_'))
}

pub fn verify_action(before: &MobileObservation, after: &MobileObservation) -> VerificationResult {
    if before.device_session_id != after.device_session_id {
        return VerificationResult::UnexpectedState;
    }
    if before.package_name == after.package_name
        && before.activity == after.activity
        && before.frame_hash == after.frame_hash
        && before.ui_tree_hash == after.ui_tree_hash
    {
        VerificationResult::NoChange
    } else {
        VerificationResult::Verified
    }
}

impl GenericAndroidSkill {
    pub fn v2_navigation() -> Self {
        Self {
            skill_id: "generic-android".into(),
            version: "2.0.0".into(),
            allowed_actions: vec![
                "observe",
                "inspect_element",
                "tap",
                "swipe",
                "type",
                "back",
                "home",
                "open_app",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            blocked_actions: vec!["search".into()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        MobileBounds, MobileDeviceType, MobileFrame, MobilePlatform, MobileSessionStatus,
        PrivacyClass, RawMobileElement,
    };

    fn fixture() -> (MobileDeviceSession, MobileUiSnapshot) {
        let snapshot = MobileUiSnapshot::from_elements(
            "session-1",
            "com.android.settings",
            ".Settings",
            1080,
            2400,
            vec![
                RawMobileElement {
                    text: Some("Network".into()),
                    role: "button".into(),
                    class_name: "android.widget.Button".into(),
                    content_description: None,
                    bounds: MobileBounds {
                        x: 100,
                        y: 200,
                        width: 300,
                        height: 80,
                    },
                    clickable: true,
                    scrollable: false,
                    enabled: true,
                    focused: false,
                    selected: false,
                    resource_id: Some("com.android.settings:id/network".into()),
                    password: false,
                },
                RawMobileElement {
                    text: Some("Search".into()),
                    role: "edittext".into(),
                    class_name: "android.widget.EditText".into(),
                    content_description: None,
                    bounds: MobileBounds {
                        x: 100,
                        y: 300,
                        width: 500,
                        height: 80,
                    },
                    clickable: true,
                    scrollable: false,
                    enabled: true,
                    focused: true,
                    selected: false,
                    resource_id: Some("com.android.settings:id/search".into()),
                    password: false,
                },
            ],
        );
        let session = MobileDeviceSession {
            session_id: "session-1".into(),
            device_id: "emulator-5554".into(),
            platform: MobilePlatform::Android,
            device_type: MobileDeviceType::Emulator,
            os_version: "16".into(),
            screen_width: 1080,
            screen_height: 2400,
            connected_at: snapshot.captured_at,
            current_app: Some(snapshot.package_name.clone()),
            current_activity: Some(snapshot.activity.clone()),
            status: MobileSessionStatus::Connected,
            last_observation_at: Some(snapshot.captured_at),
        };
        (session, snapshot)
    }

    fn request(snapshot: &MobileUiSnapshot, target: MobileActionTarget) -> MobileActionRequest {
        MobileActionRequest {
            action_id: "action-1".into(),
            session_id: snapshot.session_id.clone(),
            snapshot_id: snapshot.snapshot_id.clone(),
            expected_package: snapshot.package_name.clone(),
            requested_at: snapshot.captured_at,
            target,
            text: None,
        }
    }

    #[test]
    fn safe_targets_allow_but_policy_preconditions_fail_closed_in_order() {
        let (mut session, snapshot) = fixture();
        let allowed = vec!["com.android.settings".to_owned()];
        let policy = |session: &MobileDeviceSession,
                      action_count: u8,
                      foreground: &str,
                      req: &MobileActionRequest| {
            evaluate_action(
                req,
                session,
                &snapshot,
                &allowed,
                action_count,
                foreground,
                &snapshot.activity,
                snapshot.captured_at,
            )
        };
        let tap = request(
            &snapshot,
            MobileActionTarget::Tap {
                element_ref: "@e1".into(),
            },
        );
        assert_eq!(
            policy(&session, 0, "com.android.settings", &tap),
            MobileActionDecision::Allowed
        );
        assert_eq!(
            policy(&session, 20, "com.android.settings", &tap),
            MobileActionDecision::Denied(MobileActionDenyReason::BudgetExceeded)
        );
        assert_eq!(
            policy(&session, 0, "com.example", &tap),
            MobileActionDecision::Denied(MobileActionDenyReason::ForegroundChanged)
        );
        session.device_type = MobileDeviceType::Physical;
        assert_eq!(
            policy(&session, 0, "com.android.settings", &tap),
            MobileActionDecision::Denied(MobileActionDenyReason::PhysicalDevice)
        );
        session.device_type = MobileDeviceType::Emulator;
        session.status = MobileSessionStatus::Ended;
        assert_eq!(
            policy(&session, 0, "com.android.settings", &tap),
            MobileActionDecision::Denied(MobileActionDenyReason::InactiveSession)
        );
    }

    #[test]
    fn stale_sensitive_and_invalid_targets_are_denied() {
        let (session, mut snapshot) = fixture();
        let allowed = vec!["com.android.settings".to_owned()];
        let decide = |req: &MobileActionRequest, snapshot: &MobileUiSnapshot| {
            evaluate_action(
                req,
                &session,
                snapshot,
                &allowed,
                0,
                "com.android.settings",
                &snapshot.activity,
                snapshot.captured_at,
            )
        };
        let mut tap = request(
            &snapshot,
            MobileActionTarget::Tap {
                element_ref: "@missing".into(),
            },
        );
        assert_eq!(
            decide(&tap, &snapshot),
            MobileActionDecision::Denied(MobileActionDenyReason::TargetMissing)
        );
        tap.snapshot_id = "old-snapshot".into();
        assert_eq!(
            decide(&tap, &snapshot),
            MobileActionDecision::Denied(MobileActionDenyReason::StaleSnapshot)
        );
        tap.snapshot_id = snapshot.snapshot_id.clone();
        snapshot.sensitive_state = Some(VerificationResult::FinancialActionBlocked);
        assert_eq!(
            decide(&tap, &snapshot),
            MobileActionDecision::Denied(MobileActionDenyReason::SensitiveScreen)
        );
        assert_eq!(
            decide(&request(&snapshot, MobileActionTarget::Back), &snapshot),
            MobileActionDecision::Allowed
        );
    }

    #[test]
    fn type_is_bounded_redacted_and_never_serializes_plaintext() {
        let (session, snapshot) = fixture();
        let allowed = vec!["com.android.settings".to_owned()];
        let mut req = request(
            &snapshot,
            MobileActionTarget::Type {
                element_ref: "@e2".into(),
            },
        );
        req.text = Some(SensitiveText::new("wifi".into()));
        assert_eq!(
            evaluate_action(
                &req,
                &session,
                &snapshot,
                &allowed,
                0,
                "com.android.settings",
                &snapshot.activity,
                snapshot.captured_at
            ),
            MobileActionDecision::Allowed
        );
        assert!(!format!("{req:?}").contains("wifi"));
        assert!(!serde_json::to_string(&req).unwrap().contains("wifi"));
        assert_eq!(req.text.as_ref().unwrap().len(), 4);
        req.text = Some(SensitiveText::new("abc;rm".into()));
        assert_eq!(
            evaluate_action(
                &req,
                &session,
                &snapshot,
                &allowed,
                0,
                "com.android.settings",
                &snapshot.activity,
                snapshot.captured_at
            ),
            MobileActionDecision::Denied(MobileActionDenyReason::UnsafeText)
        );
        req.text = Some(SensitiveText::new("a".repeat(257)));
        assert_eq!(
            evaluate_action(
                &req,
                &session,
                &snapshot,
                &allowed,
                0,
                "com.android.settings",
                &snapshot.activity,
                snapshot.captured_at
            ),
            MobileActionDecision::Denied(MobileActionDenyReason::UnsafeText)
        );
    }

    #[test]
    fn verification_requires_change_and_skill_v1_stays_observe_only() {
        let (_, snapshot) = fixture();
        let pre = MobileObservation::from_snapshot(
            &snapshot,
            &MobileFrame::from_png("session-1", 1080, 2400, vec![1]),
            None,
            "settings",
            PrivacyClass::UserAllowed,
        );
        let mut post = pre.clone();
        assert_eq!(verify_action(&pre, &post), VerificationResult::NoChange);
        post.frame_hash = "sha256:changed".into();
        assert_eq!(verify_action(&pre, &post), VerificationResult::Verified);
        assert!(
            !GenericAndroidSkill::v1()
                .allowed_actions
                .contains(&"tap".to_owned())
        );
        assert!(
            GenericAndroidSkill::v2_navigation()
                .allowed_actions
                .contains(&"tap".to_owned())
        );
    }

    #[test]
    fn all_six_targets_are_bounded_and_invalid_targets_deny() {
        let (session, mut snapshot) = fixture();
        let allowed = vec!["com.android.settings".to_owned()];
        let decide = |req: &MobileActionRequest, snap: &MobileUiSnapshot| {
            evaluate_action(
                req,
                &session,
                snap,
                &allowed,
                0,
                "com.android.settings",
                &snap.activity,
                snap.captured_at,
            )
        };
        for target in [
            MobileActionTarget::Tap {
                element_ref: "@e1".into(),
            },
            MobileActionTarget::Swipe {
                direction: SwipeDirection::Up,
            },
            MobileActionTarget::Back,
            MobileActionTarget::Home,
            MobileActionTarget::OpenApp {
                package_name: "com.android.settings".into(),
            },
        ] {
            assert_eq!(
                decide(&request(&snapshot, target), &snapshot),
                MobileActionDecision::Allowed
            );
        }
        assert_eq!(
            decide(
                &request(
                    &snapshot,
                    MobileActionTarget::OpenApp {
                        package_name: "com.example.other".into()
                    }
                ),
                &snapshot
            ),
            MobileActionDecision::Denied(MobileActionDenyReason::PackageNotAllowed)
        );
        let mut req = request(
            &snapshot,
            MobileActionTarget::Type {
                element_ref: "@e2".into(),
            },
        );
        req.text = Some(SensitiveText::new("wifi".into()));
        assert_eq!(decide(&req, &snapshot), MobileActionDecision::Allowed);
        req.target = MobileActionTarget::Type {
            element_ref: "@e1".into(),
        };
        assert_eq!(
            decide(&req, &snapshot),
            MobileActionDecision::Denied(MobileActionDenyReason::TargetNotEditable)
        );
        req.target = MobileActionTarget::Type {
            element_ref: "@e2".into(),
        };
        snapshot.elements[1].enabled = false;
        assert_eq!(
            decide(&req, &snapshot),
            MobileActionDecision::Denied(MobileActionDenyReason::TargetDisabled)
        );
        snapshot.elements[1].enabled = true;
        snapshot.redactions.push("element:@e2:sensitive".into());
        assert_eq!(
            decide(&req, &snapshot),
            MobileActionDecision::Denied(MobileActionDenyReason::SensitiveTarget)
        );
    }

    #[test]
    fn stale_snapshot_wrong_package_and_unicode_input_deny() {
        let (session, snapshot) = fixture();
        let allowed = vec!["com.android.settings".to_owned()];
        let decide = |req: &MobileActionRequest, now| {
            evaluate_action(
                req,
                &session,
                &snapshot,
                &allowed,
                0,
                "com.android.settings",
                &snapshot.activity,
                now,
            )
        };
        let mut req = request(
            &snapshot,
            MobileActionTarget::Tap {
                element_ref: "@e1".into(),
            },
        );
        req.expected_package = "com.example.wrong".into();
        assert_eq!(
            decide(&req, snapshot.captured_at),
            MobileActionDecision::Denied(MobileActionDenyReason::PackageNotAllowed)
        );
        req.expected_package = snapshot.package_name.clone();
        assert_eq!(
            decide(&req, snapshot.captured_at + chrono::Duration::seconds(11)),
            MobileActionDecision::Denied(MobileActionDenyReason::StaleSnapshot)
        );
        req.target = MobileActionTarget::Type {
            element_ref: "@e2".into(),
        };
        req.text = Some(SensitiveText::new("无线".into()));
        assert_eq!(
            decide(&req, snapshot.captured_at),
            MobileActionDecision::Denied(MobileActionDenyReason::UnsafeText)
        );
    }
}
