use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{HashSet, VecDeque};
use uuid::Uuid;

pub mod action;
pub use action::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidDeviceInfo {
    pub id: String,
    pub status: String,
    pub model: Option<String>,
    pub avd_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidAvdInfo {
    pub name: String,
    pub status: String,
    pub device_profile: Option<String>,
    pub architecture: Option<String>,
    pub running: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidEnvironmentDiagnostics {
    pub sdk_status: String,
    pub adb_status: String,
    pub emulator_status: String,
    pub sdk_root: Option<String>,
    pub adb_path: Option<String>,
    pub emulator_path: Option<String>,
    pub sdkmanager_path: Option<String>,
    pub avdmanager_path: Option<String>,
    pub adb_version: Option<String>,
    pub available_avds: Vec<AndroidAvdInfo>,
    pub online_devices: Vec<AndroidDeviceInfo>,
}

impl Default for AndroidEnvironmentDiagnostics {
    fn default() -> Self {
        Self {
            sdk_status: "missing".into(),
            adb_status: "missing".into(),
            emulator_status: "missing".into(),
            sdk_root: None,
            adb_path: None,
            emulator_path: None,
            sdkmanager_path: None,
            avdmanager_path: None,
            adb_version: None,
            available_avds: Vec::new(),
            online_devices: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MobilePlatform {
    Android,
    Ios,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MobileDeviceType {
    Emulator,
    Physical,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MobileSessionStatus {
    Connected,
    Offline,
    Blocked,
    Ended,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationResult {
    Verified,
    NoChange,
    UnexpectedState,
    TargetMissing,
    PermissionRequired,
    LoginRequired,
    CaptchaBlocked,
    SensitiveFieldBlocked,
    FinancialActionBlocked,
    StaleObservation,
    AppCrashed,
    DeviceOffline,
    Interrupted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionMethod {
    Accessibility,
    Ocr,
    Vision,
    Hybrid,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyClass {
    Public,
    UserAllowed,
    Sensitive,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    ObservationOnly,
    Validated,
    Rejected,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileDeviceSession {
    pub session_id: String,
    pub device_id: String,
    pub platform: MobilePlatform,
    pub device_type: MobileDeviceType,
    pub os_version: String,
    pub screen_width: u32,
    pub screen_height: u32,
    pub connected_at: DateTime<Utc>,
    pub current_app: Option<String>,
    pub current_activity: Option<String>,
    pub status: MobileSessionStatus,
    pub last_observation_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawMobileElement {
    pub text: Option<String>,
    pub role: String,
    pub class_name: String,
    pub content_description: Option<String>,
    pub bounds: MobileBounds,
    pub clickable: bool,
    pub scrollable: bool,
    pub enabled: bool,
    pub focused: bool,
    pub selected: bool,
    pub resource_id: Option<String>,
    pub password: bool,
}

impl RawMobileElement {
    pub fn requires_redaction(&self) -> bool {
        let combined = format!(
            "{} {} {}",
            self.text.as_deref().unwrap_or_default(),
            self.content_description.as_deref().unwrap_or_default(),
            self.resource_id.as_deref().unwrap_or_default()
        )
        .to_ascii_lowercase();
        self.password
            || contains_any(&combined, SENSITIVE_TERMS)
            || contains_any(&combined, FINANCIAL_TERMS)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileElement {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub text: Option<String>,
    pub role: String,
    pub class_name: String,
    pub content_description: Option<String>,
    pub bounds: MobileBounds,
    pub clickable: bool,
    pub scrollable: bool,
    pub enabled: bool,
    pub focused: bool,
    pub selected: bool,
    pub resource_id: Option<String>,
    pub extraction_source: ExtractionMethod,
    pub confidence: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileUiSnapshot {
    pub snapshot_id: String,
    pub session_id: String,
    pub package_name: String,
    pub activity: String,
    pub screen_width: u32,
    pub screen_height: u32,
    pub captured_at: DateTime<Utc>,
    pub elements: Vec<MobileElement>,
    pub redactions: Vec<String>,
    pub sensitive_state: Option<VerificationResult>,
}

impl MobileUiSnapshot {
    pub fn from_elements(
        session_id: impl Into<String>,
        package_name: impl Into<String>,
        activity: impl Into<String>,
        screen_width: u32,
        screen_height: u32,
        raw_elements: Vec<RawMobileElement>,
    ) -> Self {
        let mut redactions = Vec::new();
        let mut sensitive_state = None;
        let elements = raw_elements
            .into_iter()
            .enumerate()
            .map(|(index, raw)| {
                let element_ref = format!("@e{}", index + 1);
                let combined = format!(
                    "{} {} {}",
                    raw.text.as_deref().unwrap_or_default(),
                    raw.content_description.as_deref().unwrap_or_default(),
                    raw.resource_id.as_deref().unwrap_or_default()
                )
                .to_ascii_lowercase();
                let financial = contains_any(&combined, FINANCIAL_TERMS);
                let sensitive = raw.password || contains_any(&combined, SENSITIVE_TERMS);
                if financial {
                    sensitive_state = Some(VerificationResult::FinancialActionBlocked);
                } else if sensitive && sensitive_state.is_none() {
                    sensitive_state = Some(VerificationResult::SensitiveFieldBlocked);
                }
                let (text, content_description) = if sensitive || financial {
                    redactions.push(format!("element:{element_ref}:sensitive"));
                    (Some("[REDACTED]".into()), None)
                } else {
                    (clean(raw.text), clean(raw.content_description))
                };
                MobileElement {
                    element_ref,
                    text,
                    role: raw.role,
                    class_name: raw.class_name,
                    content_description,
                    bounds: raw.bounds,
                    clickable: raw.clickable,
                    scrollable: raw.scrollable,
                    enabled: raw.enabled,
                    focused: raw.focused,
                    selected: raw.selected,
                    resource_id: raw.resource_id,
                    extraction_source: ExtractionMethod::Accessibility,
                    confidence: 1.0,
                }
            })
            .collect();
        Self {
            snapshot_id: format!("snapshot_{}", Uuid::now_v7()),
            session_id: session_id.into(),
            package_name: package_name.into(),
            activity: activity.into(),
            screen_width,
            screen_height,
            captured_at: Utc::now(),
            elements,
            redactions,
            sensitive_state,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileFrame {
    pub frame_id: String,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub width: u32,
    pub height: u32,
    pub orientation: String,
    pub frame_hash: String,
    pub data_url: String,
    #[serde(skip)]
    pub png_bytes: Vec<u8>,
}

impl MobileFrame {
    pub fn from_png(
        session_id: impl Into<String>,
        width: u32,
        height: u32,
        png_bytes: Vec<u8>,
    ) -> Self {
        let frame_hash = sha256_reference(&png_bytes);
        let data_url = format!("data:image/png;base64,{}", STANDARD.encode(&png_bytes));
        Self {
            frame_id: format!("frame_{}", Uuid::now_v7()),
            session_id: session_id.into(),
            timestamp: Utc::now(),
            width,
            height,
            orientation: if width > height {
                "landscape"
            } else {
                "portrait"
            }
            .into(),
            frame_hash,
            data_url,
            png_bytes,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileObservation {
    pub id: String,
    pub task_id: Option<String>,
    pub device_session_id: String,
    pub app_id: String,
    pub package_name: String,
    pub activity: String,
    pub screen_state: String,
    pub observed_at: DateTime<Utc>,
    pub frame_hash: String,
    pub ui_tree_hash: String,
    pub source_locator: String,
    pub visible_facts: Vec<String>,
    pub extracted_entities: Vec<String>,
    pub author: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub extraction_method: ExtractionMethod,
    pub extraction_confidence: f64,
    pub redactions: Vec<String>,
    pub privacy_class: PrivacyClass,
    pub verification_status: VerificationResult,
    pub evidence_status: EvidenceStatus,
    pub metadata: Value,
}

impl MobileObservation {
    pub fn from_snapshot(
        snapshot: &MobileUiSnapshot,
        frame: &MobileFrame,
        task_id: Option<String>,
        app_id: impl Into<String>,
        privacy_class: PrivacyClass,
    ) -> Self {
        let mut facts = Vec::new();
        let mut seen = HashSet::new();
        for element in &snapshot.elements {
            for value in [
                element.text.as_deref(),
                element.content_description.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                let value = value.trim();
                if !value.is_empty() && value != "[REDACTED]" && seen.insert(value.to_owned()) {
                    facts.push(value.to_owned());
                }
            }
        }
        let mut entities = Vec::new();
        let mut entity_seen = HashSet::new();
        for fact in &facts {
            for token in fact.split(|character: char| !character.is_ascii_alphanumeric()) {
                let upper = token.to_ascii_uppercase();
                if KNOWN_ENTITIES.contains(&upper.as_str()) && entity_seen.insert(upper.clone()) {
                    entities.push(upper);
                }
            }
        }
        let tree_json = serde_json::to_vec(&snapshot.elements).unwrap_or_default();
        let verification_status = snapshot
            .sensitive_state
            .unwrap_or(VerificationResult::Verified);
        Self {
            id: format!("mobile_observation_{}", Uuid::now_v7()),
            task_id,
            device_session_id: snapshot.session_id.clone(),
            app_id: app_id.into(),
            package_name: snapshot.package_name.clone(),
            activity: snapshot.activity.clone(),
            screen_state: if snapshot.elements.is_empty() {
                "empty"
            } else {
                "observed"
            }
            .into(),
            observed_at: snapshot.captured_at,
            frame_hash: frame.frame_hash.clone(),
            ui_tree_hash: sha256_reference(&tree_json),
            source_locator: format!(
                "android://{}/{}?snapshot={}",
                snapshot.package_name,
                snapshot.activity.trim_start_matches('/'),
                snapshot.snapshot_id
            ),
            visible_facts: if snapshot.sensitive_state.is_some() {
                Vec::new()
            } else {
                facts
            },
            extracted_entities: if snapshot.sensitive_state.is_some() {
                Vec::new()
            } else {
                entities
            },
            author: None,
            published_at: None,
            extraction_method: ExtractionMethod::Accessibility,
            extraction_confidence: if snapshot.elements.is_empty() {
                0.0
            } else {
                1.0
            },
            redactions: snapshot.redactions.clone(),
            privacy_class,
            verification_status,
            evidence_status: EvidenceStatus::ObservationOnly,
            metadata: json!({"snapshotId": snapshot.snapshot_id, "elementCount": snapshot.elements.len()}),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileCapture {
    pub session: MobileDeviceSession,
    pub snapshot: MobileUiSnapshot,
    pub frame: MobileFrame,
    pub observation: MobileObservation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericAndroidSkill {
    pub skill_id: String,
    pub version: String,
    pub allowed_actions: Vec<String>,
    pub blocked_actions: Vec<String>,
}

impl GenericAndroidSkill {
    pub fn v1() -> Self {
        Self {
            skill_id: "generic-android".into(),
            version: "1.0.0".into(),
            allowed_actions: vec!["observe".into(), "inspect_element".into()],
            blocked_actions: vec![
                "tap".into(),
                "swipe".into(),
                "type".into(),
                "back".into(),
                "home".into(),
                "open_app".into(),
                "search".into(),
            ],
        }
    }
}

pub struct ScreenFrameBuffer {
    capacity: usize,
    frames: VecDeque<MobileFrame>,
}

impl ScreenFrameBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            frames: VecDeque::new(),
        }
    }

    pub fn push(&mut self, frame: MobileFrame) {
        if self.frames.len() == self.capacity {
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }

    pub fn latest(&self) -> Option<&MobileFrame> {
        self.frames.back()
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

const SENSITIVE_TERMS: &[&str] = &[
    "password",
    "passcode",
    "otp",
    "one-time",
    "verification code",
    "seed phrase",
    "private key",
    "mnemonic",
    "banking",
    "payment",
];
const FINANCIAL_TERMS: &[&str] = &[
    "buy",
    "sell",
    "long",
    "short",
    "place order",
    "withdraw",
    "transfer",
    "deposit",
    "wallet sign",
    "sign transaction",
];
const KNOWN_ENTITIES: &[&str] = &["BTC", "ETH", "SOL", "NVDA", "NVIDIA", "FED"];

fn contains_any(value: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| value.contains(term))
}

fn clean(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

fn sha256_reference(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(text: &str, class_name: &str, password: bool) -> RawMobileElement {
        RawMobileElement {
            text: Some(text.into()),
            role: "text".into(),
            class_name: class_name.into(),
            content_description: None,
            bounds: MobileBounds {
                x: 10,
                y: 20,
                width: 100,
                height: 40,
            },
            clickable: true,
            scrollable: false,
            enabled: true,
            focused: false,
            selected: false,
            resource_id: Some("com.example:id/title".into()),
            password,
        }
    }

    #[test]
    fn snapshot_assigns_scoped_element_refs_and_redacts_sensitive_nodes() {
        let first = MobileUiSnapshot::from_elements(
            "session-1",
            "com.example",
            "com.example/.MainActivity",
            1080,
            2400,
            vec![
                raw("BTC ETF", "android.widget.TextView", false),
                raw("secret-value", "android.widget.EditText", true),
                raw("Place Order", "android.widget.Button", false),
            ],
        );
        let second = MobileUiSnapshot::from_elements(
            "session-1",
            "com.example",
            "com.example/.MainActivity",
            1080,
            2400,
            vec![raw("BTC ETF", "android.widget.TextView", false)],
        );

        assert_eq!(
            first
                .elements
                .iter()
                .map(|item| item.element_ref.as_str())
                .collect::<Vec<_>>(),
            vec!["@e1", "@e2", "@e3"]
        );
        assert_eq!(first.elements[1].text.as_deref(), Some("[REDACTED]"));
        assert_ne!(first.snapshot_id, second.snapshot_id);
        assert_eq!(second.elements[0].element_ref, "@e1");
        assert_eq!(
            first.sensitive_state,
            Some(VerificationResult::FinancialActionBlocked)
        );
    }

    #[test]
    fn raw_element_marks_financial_actions_for_redaction() {
        let element = raw("Transfer", "android.widget.Button", false);

        assert!(element.requires_redaction());
    }

    #[test]
    fn observation_deduplicates_facts_and_keeps_source_lineage() {
        let snapshot = MobileUiSnapshot::from_elements(
            "session-1",
            "com.example",
            "com.example/.MainActivity",
            1080,
            2400,
            vec![
                raw("BTC ETF", "android.widget.TextView", false),
                raw("BTC ETF", "android.widget.TextView", false),
                raw("SOL", "android.widget.TextView", false),
            ],
        );
        let frame = MobileFrame::from_png("session-1", 1080, 2400, vec![137, 80, 78, 71]);
        let observation = MobileObservation::from_snapshot(
            &snapshot,
            &frame,
            None,
            "Example",
            PrivacyClass::Public,
        );

        assert_eq!(observation.visible_facts, vec!["BTC ETF", "SOL"]);
        assert_eq!(observation.extracted_entities, vec!["BTC", "SOL"]);
        assert_eq!(observation.evidence_status, EvidenceStatus::ObservationOnly);
        assert!(
            observation
                .source_locator
                .starts_with("android://com.example/")
        );
        assert_eq!(observation.frame_hash, frame.frame_hash);
        assert!(observation.ui_tree_hash.starts_with("sha256:"));
    }

    #[test]
    fn screen_buffer_retains_only_the_latest_five_frames() {
        let mut buffer = ScreenFrameBuffer::new(5);
        for value in 0..6 {
            buffer.push(MobileFrame::from_png("session-1", 10, 20, vec![value]));
        }

        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.latest().unwrap().png_bytes, vec![5]);
    }

    #[test]
    fn generic_skill_is_observe_only() {
        let skill = GenericAndroidSkill::v1();
        assert_eq!(skill.allowed_actions, vec!["observe", "inspect_element"]);
        assert!(skill.blocked_actions.contains(&"tap".to_string()));
        assert!(skill.blocked_actions.contains(&"type".to_string()));
    }
}
