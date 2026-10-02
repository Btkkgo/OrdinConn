//! Manual acquisition models layered on the existing Mobile Runtime.
use crate::{MobileBounds, MobileCapture};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileUIElement {
    pub id: String,
    pub role: String,
    pub class_name: String,
    pub text: Option<String>,
    pub content_description: Option<String>,
    pub resource_id: Option<String>,
    pub clickable: bool,
    pub scrollable: bool,
    pub editable: bool,
    pub enabled: bool,
    pub selected: bool,
    pub bounds: MobileBounds,
    pub redacted: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileObservation {
    pub id: String,
    pub device_id: String,
    pub captured_at: DateTime<Utc>,
    pub package_name: String,
    pub activity_name: String,
    pub screen_width: u32,
    pub screen_height: u32,
    pub ui_tree_hash: String,
    pub element_count: usize,
    pub elements: Vec<MobileUIElement>,
    pub source: String,
    pub previous_observation_id: Option<String>,
    pub redactions: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationDiff {
    pub before_observation_id: String,
    pub after_observation_id: String,
    pub changed: bool,
    pub change_types: Vec<String>,
    pub added_elements: Vec<String>,
    pub removed_elements: Vec<String>,
    pub changed_elements: Vec<String>,
    pub summary: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedMobileData {
    pub id: String,
    pub observation_id: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub value: String,
    pub element_id: String,
    pub confidence: f64,
    pub extraction_method: String,
    pub captured_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileProvenance {
    pub observation_id: String,
    pub extraction_method: String,
    pub element_ids: Vec<String>,
    pub activity_name: String,
    pub extracted_data_ids: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileDataObject {
    pub id: String,
    pub source_type: String,
    pub device_id: String,
    pub package_name: String,
    pub observation_id: String,
    pub object_type: String,
    pub title: Option<String>,
    pub content: String,
    pub values: BTreeMap<String, serde_json::Value>,
    pub captured_at: DateTime<Utc>,
    pub provenance: MobileProvenance,
    pub deduplication_key: String,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MobileActionType {
    Observe,
    Tap,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    Back,
    Home,
    InputText,
    OpenApp,
    Stop,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileActionResult {
    pub action_id: String,
    pub action_type: MobileActionType,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: String,
    pub device_id: String,
    pub before_observation_id: Option<String>,
    pub after_observation_id: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub triggered_by: String,
}
pub fn normalize_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn hash(value: &impl Serialize) -> String {
    use sha2::{Digest, Sha256};
    format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("serializable model"))
    )
}
/// Conservative local filter. Values are never included in redaction diagnostics.
pub fn prohibited_manual_target(value: &str) -> bool {
    let v = value.to_lowercase();
    sensitive_label(&v)
        || [
            "erase",
            "factory reset",
            "delete",
            "remove data",
            "reset all",
            "reset device",
            "sign in",
            "log in",
            "login",
            "send message",
            "publish",
            "post content",
            "confirm purchase",
            "buy",
            "pay",
            "checkout",
            "install apk",
            "uninstall",
            "发送",
            "发布",
            "删除",
            "清除数据",
            "恢复出厂",
            "登录",
            "购买",
            "支付",
            "安装",
        ]
        .iter()
        .any(|term| v.contains(term))
}
pub fn prohibited_manual_input(value: &str) -> bool {
    sensitive_label(value)
        || !crate::is_safe_mobile_input(value)
        || ((4..=8).contains(&value.len()) && value.bytes().all(|c| c.is_ascii_digit()))
        || (value.len() == 64 && value.bytes().all(|c| c.is_ascii_hexdigit()))
        || matches!(value.split_whitespace().count(), 12 | 24)
}
pub fn sensitive_label(value: &str) -> bool {
    let value = value.to_lowercase();
    [
        "password",
        "passcode",
        "otp",
        "verification code",
        "private key",
        "seed phrase",
        "mnemonic",
        "payment credential",
        "card number",
        "cvv",
        "security code",
        "one-time",
        "验证码",
        "密码",
        "助记词",
        "私钥",
        "银行卡",
    ]
    .iter()
    .any(|term| value.contains(term))
        || value
            .split(|c: char| !c.is_alphanumeric())
            .any(|token| token == "pin" || token == "2fa")
}
pub fn observation_from_capture(
    capture: &MobileCapture,
    previous: Option<String>,
) -> MobileObservation {
    let mut redactions = Vec::new();
    let mut occurrences = BTreeMap::<String, usize>::new();
    let elements = capture
        .snapshot
        .elements
        .iter()
        .map(|element| {
            let label = format!(
                "{} {} {}",
                element.text.as_deref().unwrap_or_default(),
                element.content_description.as_deref().unwrap_or_default(),
                element.resource_id.as_deref().unwrap_or_default()
            );
            let redacted = element.text.as_deref() == Some("[REDACTED]")
                || sensitive_label(&label)
                || capture
                    .snapshot
                    .redactions
                    .iter()
                    .any(|r| r.starts_with(&format!("element:{}:", element.element_ref)));
            let identity = hash(&(
                element.resource_id.as_deref(),
                &element.class_name,
                &element.bounds,
            ));
            let occurrence = occurrences.entry(identity.clone()).or_default();
            *occurrence += 1;
            let id = format!(
                "el_{}_{}",
                identity.trim_start_matches("sha256:"),
                occurrence
            );
            if redacted {
                redactions.push(format!("REDACTED_SENSITIVE_ELEMENT:{id}"));
            }
            MobileUIElement {
                id,
                role: element.role.clone(),
                class_name: element.class_name.clone(),
                text: if redacted {
                    None
                } else {
                    element
                        .text
                        .as_deref()
                        .map(normalize_text)
                        .filter(|s| !s.is_empty())
                },
                content_description: if redacted {
                    None
                } else {
                    element
                        .content_description
                        .as_deref()
                        .map(normalize_text)
                        .filter(|s| !s.is_empty())
                },
                resource_id: if redacted {
                    None
                } else {
                    element.resource_id.clone()
                },
                clickable: element.clickable,
                scrollable: element.scrollable,
                editable: element.class_name.ends_with("EditText"),
                enabled: element.enabled,
                selected: element.selected,
                bounds: element.bounds.clone(),
                redacted,
            }
        })
        .collect::<Vec<_>>();
    MobileObservation {
        id: capture.observation.id.clone(),
        device_id: capture.session.device_id.clone(),
        captured_at: capture.snapshot.captured_at,
        package_name: capture.snapshot.package_name.clone(),
        activity_name: capture.snapshot.activity.clone(),
        screen_width: capture.snapshot.screen_width,
        screen_height: capture.snapshot.screen_height,
        ui_tree_hash: hash(&elements),
        element_count: elements.len(),
        elements,
        source: "android_ui_tree".into(),
        previous_observation_id: previous,
        redactions,
    }
}
pub fn diff_observations(before: &MobileObservation, after: &MobileObservation) -> ObservationDiff {
    let a = before
        .elements
        .iter()
        .map(|e| (&e.id, e))
        .collect::<BTreeMap<_, _>>();
    let b = after
        .elements
        .iter()
        .map(|e| (&e.id, e))
        .collect::<BTreeMap<_, _>>();
    let mut kinds = std::collections::BTreeSet::new();
    if before.package_name != after.package_name {
        kinds.insert("APP_CHANGED");
    }
    if before.activity_name != after.activity_name {
        kinds.insert("ACTIVITY_CHANGED");
    }
    let added = after
        .elements
        .iter()
        .filter(|e| !a.contains_key(&e.id))
        .map(|e| e.id.clone())
        .collect::<Vec<_>>();
    let removed = before
        .elements
        .iter()
        .filter(|e| !b.contains_key(&e.id))
        .map(|e| e.id.clone())
        .collect::<Vec<_>>();
    if !added.is_empty() {
        kinds.insert("ELEMENT_ADDED");
    }
    if !removed.is_empty() {
        kinds.insert("ELEMENT_REMOVED");
    }
    let mut changed = Vec::new();
    for e in &after.elements {
        if let Some(old) = a.get(&e.id) {
            if old.text != e.text || old.content_description != e.content_description {
                kinds.insert("TEXT_CHANGED");
            }
            if old.selected != e.selected {
                kinds.insert("SELECTION_CHANGED");
            }
            if hash(old) != hash(e) {
                changed.push(e.id.clone());
                kinds.insert("ELEMENT_STATE_CHANGED");
            }
        }
    }
    if (!added.is_empty() || !removed.is_empty() || kinds.contains("TEXT_CHANGED"))
        && before
            .elements
            .iter()
            .chain(&after.elements)
            .any(|e| e.scrollable)
    {
        kinds.insert("SCROLL_CONTENT_CHANGED");
    }
    let meaningful = !kinds.is_empty();
    if !meaningful {
        kinds.insert("NO_MEANINGFUL_CHANGE");
    }
    ObservationDiff {
        before_observation_id: before.id.clone(),
        after_observation_id: after.id.clone(),
        changed: meaningful,
        change_types: kinds.into_iter().map(str::to_owned).collect(),
        summary: format!(
            "added={} removed={} changed={}",
            added.len(),
            removed.len(),
            changed.len()
        ),
        added_elements: added,
        removed_elements: removed,
        changed_elements: changed,
    }
}
fn numeric(value: &str) -> Option<f64> {
    value
        .replace([',', '%', '$', '€', '¥'], "")
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
}
pub fn extract_mobile_data(observation: &MobileObservation) -> Vec<ExtractedMobileData> {
    let mut data = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let first = observation
        .elements
        .iter()
        .find(|e| !e.redacted && !e.editable && e.text.is_some())
        .map(|e| &e.id);
    let has_list = observation.elements.iter().any(|e| {
        e.scrollable || e.class_name.contains("RecyclerView") || e.class_name.contains("ListView")
    });
    for e in &observation.elements {
        if e.redacted || e.editable {
            continue;
        }
        for value in [e.text.as_deref(), e.content_description.as_deref()]
            .into_iter()
            .flatten()
        {
            let value = normalize_text(value);
            if value.is_empty() || sensitive_label(&value) || value == "[REDACTED]" {
                continue;
            }
            let mut kinds = vec![("visible_text", "ui_tree", 1.0)];
            if first == Some(&e.id) {
                kinds.push(("title", "rule", 0.6));
            }
            if e.role.contains("list") || (has_list && !e.clickable) {
                kinds.push(("list_item", "rule", 0.65));
            }
            if e.class_name.ends_with("Button") || e.role == "button" {
                kinds.push(("button", "ui_tree", 1.0));
            }
            if value.starts_with("https://")
                || value.starts_with("http://")
                || (e.clickable && e.role.contains("link"))
            {
                kinds.push(("link_like_element", "rule", 0.75));
            }
            if numeric(&value).is_some() {
                kinds.push(("numeric_value", "rule", 0.85));
            }
            if value.chars().filter(char::is_ascii_digit).count() >= 4
                && (value.contains(':') || value.contains('-') || value.contains('/'))
            {
                kinds.push(("timestamp_like_text", "rule", 0.65));
            }
            if [
                "connected",
                "disconnected",
                "enabled",
                "disabled",
                "online",
                "offline",
                "loading",
                "completed",
                "active",
                "pending",
                "已连接",
                "已启用",
                "已完成",
            ]
            .iter()
            .any(|v| value.to_lowercase() == *v)
            {
                kinds.push(("status", "rule", 0.8));
            }
            if e.selected {
                kinds.push(("selected_item", "ui_tree", 1.0));
            }
            for (kind, method, confidence) in kinds {
                if !seen.insert((e.id.clone(), kind, value.clone())) {
                    continue;
                }
                data.push(ExtractedMobileData {
                    id: format!("extract_{}", uuid::Uuid::now_v7()),
                    observation_id: observation.id.clone(),
                    data_type: kind.into(),
                    value: value.clone(),
                    element_id: e.id.clone(),
                    confidence,
                    extraction_method: method.into(),
                    captured_at: observation.captured_at,
                });
            }
        }
    }
    data
}
pub fn build_data_objects(
    observation: &MobileObservation,
    data: &[ExtractedMobileData],
) -> Vec<MobileDataObject> {
    let mut objects = Vec::new();
    for e in &observation.elements {
        let items = data
            .iter()
            .filter(|d| d.observation_id == observation.id && d.element_id == e.id)
            .collect::<Vec<_>>();
        if items.is_empty() || e.redacted {
            continue;
        }
        let has = |kind: &str| items.iter().any(|d| d.data_type == kind);
        let object_type = if has("numeric_value") {
            "metric"
        } else if has("status") || has("selected_item") {
            "status"
        } else if has("list_item") {
            "list"
        } else if has("link_like_element") {
            "content"
        } else {
            "text"
        };
        let content = items
            .iter()
            .filter(|d| d.data_type == "visible_text")
            .map(|d| d.value.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(" · ");
        let method = if object_type == "text" {
            "ui_tree"
        } else {
            "rule"
        };
        let mut values = BTreeMap::new();
        for d in &items {
            values.insert(
                d.data_type.clone(),
                serde_json::Value::String(d.value.clone()),
            );
        }
        if let Some(n) = items
            .iter()
            .find(|d| d.data_type == "numeric_value")
            .and_then(|d| numeric(&d.value))
        {
            values.insert("number".into(), serde_json::json!(n));
        }
        // Omit transient observation identity and bounds: repeated/moved content must not flood the stream.
        let key = hash(&(
            &observation.device_id,
            &observation.package_name,
            &observation.activity_name,
            &e.resource_id,
            &e.class_name,
            object_type,
            &content,
        ));
        objects.push(MobileDataObject {
            id: format!("data_{}", uuid::Uuid::now_v7()),
            source_type: "android".into(),
            device_id: observation.device_id.clone(),
            package_name: observation.package_name.clone(),
            observation_id: observation.id.clone(),
            object_type: object_type.into(),
            title: has("title").then(|| content.clone()),
            content,
            values,
            captured_at: observation.captured_at,
            provenance: MobileProvenance {
                observation_id: observation.id.clone(),
                extraction_method: method.into(),
                element_ids: vec![e.id.clone()],
                activity_name: observation.activity_name.clone(),
                extracted_data_ids: items.iter().map(|d| d.id.clone()).collect(),
            },
            deduplication_key: key,
        });
    }
    objects
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionError {
    Cancelled,
    Timeout,
    DeviceDisconnected,
    ActionFailed,
    TargetChanged,
    PolicyBlocked,
    PersistenceFailed,
}
impl InteractionError {
    pub fn code(self) -> &'static str {
        match self {
            Self::Cancelled => "CANCELLED",
            Self::Timeout => "UI_STABILITY_TIMEOUT",
            Self::DeviceDisconnected => "DEVICE_DISCONNECTED",
            Self::ActionFailed => "ACTION_FAILED",
            Self::TargetChanged => "TARGET_CHANGED",
            Self::PolicyBlocked => "POLICY_BLOCKED",
            Self::PersistenceFailed => "PERSISTENCE_FAILED",
        }
    }
}
/// The desktop adapter supplies the existing ADB host; fixtures supply deterministic trees.
pub trait ManualInteractionDriver {
    fn observe(&mut self) -> Result<MobileObservation, InteractionError>;
    fn act(&mut self) -> Result<(), InteractionError>;
    fn cancelled(&self) -> bool;
    fn elapsed_ms(&self) -> u64;
    fn poll_interval(&mut self);
    fn checkpoint(&mut self, result: &MobileActionResult) -> Result<(), InteractionError>;
}
pub fn run_manual_interaction(
    driver: &mut impl ManualInteractionDriver,
    mut result: MobileActionResult,
) -> MobileActionResult {
    let run = (|| -> Result<(), InteractionError> {
        if driver.cancelled() {
            return Err(InteractionError::Cancelled);
        }
        let before = driver.observe()?;
        if result.device_id.is_empty() {
            result.device_id = before.device_id.clone();
        }
        if result.device_id != before.device_id {
            return Err(InteractionError::DeviceDisconnected);
        }
        result.before_observation_id = Some(before.id.clone());
        driver.checkpoint(&result)?;
        if result.action_type == MobileActionType::Observe {
            result.after_observation_id = Some(before.id);
            return Ok(());
        }
        if driver.cancelled() {
            return Err(InteractionError::Cancelled);
        }
        driver.act()?;
        let start = driver.elapsed_ms();
        let mut prior: Option<MobileObservation> = None;
        loop {
            if driver.cancelled() {
                return Err(InteractionError::Cancelled);
            }
            if driver.elapsed_ms().saturating_sub(start) >= 5_000 {
                return Err(InteractionError::Timeout);
            }
            let current = driver.observe()?;
            if current.device_id != result.device_id {
                return Err(InteractionError::DeviceDisconnected);
            }
            result.after_observation_id = Some(current.id.clone());
            if driver.cancelled() {
                return Err(InteractionError::Cancelled);
            }
            if prior.as_ref().is_some_and(|p| {
                p.package_name == current.package_name
                    && p.activity_name == current.activity_name
                    && p.element_count == current.element_count
                    && p.ui_tree_hash == current.ui_tree_hash
            }) {
                break;
            }
            prior = Some(current);
            driver.poll_interval();
        }
        Ok(())
    })();
    result.completed_at = Some(Utc::now());
    match run {
        Ok(()) => result.status = "completed".into(),
        Err(e) => {
            result.status = if e == InteractionError::Cancelled {
                "cancelled"
            } else {
                "failed"
            }
            .into();
            result.error_code = Some(e.code().into());
            result.error_message = Some(e.code().into());
        }
    }
    if driver.checkpoint(&result).is_err() {
        result.status = "failed".into();
        result.error_code = Some("PERSISTENCE_FAILED".into());
        result.error_message = Some("PERSISTENCE_FAILED".into());
    }
    result
}
#[cfg(test)]
mod tests;
