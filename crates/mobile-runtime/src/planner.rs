//! Untrusted single-decision contract. No host, model adapter, or execution access.
use crate::execution::*;
use serde::{Deserialize, Serialize};

pub const PLANNER_VERSION: &str = "MOBILE_PLANNER_V1";
pub const SCHEMA_VERSION: &str = "mobile_next_action_v1";
pub const MAX_PLANNER_OUTPUT_BYTES: usize = 16_384;
pub const MAX_RECENT_STEPS: usize = 5;
pub const MAX_ELEMENTS: usize = 80;
pub const MAX_CONTEXT_BYTES: usize = 32_768;
pub const MAX_WAIT_MS: u32 = 5_000;
pub const MAX_PLANNER_ATTEMPTS: u32 = 4;
pub const PLANNER_TIMEOUT_MS: u64 = 30_000;
pub const SYSTEM_PROMPT: &str = include_str!("mobile_planner_v1.txt");

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobilePlannerElement {
    pub element_ref: String,
    pub resource_id: Option<String>,
    pub text: Option<String>,
    pub clickable: bool,
    pub editable: bool,
    pub enabled: bool,
    pub sensitive: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobilePlannerObservation {
    pub observation_id: String,
    pub snapshot_id: String,
    pub device_session_id: String,
    pub captured_at: String,
    pub package: String,
    pub activity: String,
    pub screen_width: u32,
    pub screen_height: u32,
    pub sensitive_screen: bool,
    pub elements: Vec<MobilePlannerElement>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobilePlannerHistory {
    pub step_id: MobilePlanStepId,
    pub sequence: u32,
    pub step_type: MobileStepType,
    pub status: MobileStepStatus,
    pub error_code: Option<MobileGoalErrorCode>,
    pub observation_before_id: Option<String>,
    pub observation_after_id: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobilePlannerContext {
    pub goal_id: MobileGoalId,
    pub objective: String,
    pub completion_target: Option<MobileCompletionTarget>,
    pub goal_status: MobileGoalStatus,
    pub plan_revision: Option<u32>,
    pub step_budget: u32,
    pub steps_used: u32,
    pub remaining_steps: u32,
    pub observation_missing: bool,
    pub current_observation: Option<MobilePlannerObservation>,
    pub recent_steps: Vec<MobilePlannerHistory>,
    pub recent_failures: Vec<MobileGoalErrorCode>,
    pub allowed_action_types: Vec<String>,
    pub forbidden_capabilities: Vec<String>,
    pub allowed_apps: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum MobileNextAction {
    Observe {
        reason: String,
        expected_result: ExpectedStepResult,
    },
    ScrollDown {
        reason: String,
        expected_result: ExpectedStepResult,
    },
    ScrollUp {
        reason: String,
        expected_result: ExpectedStepResult,
    },
    TapElement {
        reason: String,
        target_element_ref: String,
        expected_result: ExpectedStepResult,
    },
    InputText {
        reason: String,
        target_element_ref: String,
        input_text: String,
        expected_result: ExpectedStepResult,
    },
    Back {
        reason: String,
        expected_result: ExpectedStepResult,
    },
    Wait {
        reason: String,
        wait_ms: u32,
        expected_result: ExpectedStepResult,
    },
    Extract {
        reason: String,
        extraction_intent: String,
        expected_result: ExpectedStepResult,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MobileCompletionProposal {
    pub reason: String,
    pub supporting_observation_ids: Vec<String>,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MobilePlannerFailureCode {
    NeedsNewObservation,
    TargetNotFound,
    InsufficientContext,
    GoalUnsupported,
    SafetyBlocked,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MobilePlannerFailure {
    pub code: MobilePlannerFailureCode,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum MobilePlannerDecision {
    NextAction {
        action: MobileNextAction,
    },
    Complete {
        completion: MobileCompletionProposal,
    },
    CannotProceed {
        failure: MobilePlannerFailure,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobilePlannerOutcome {
    pub decision: MobilePlannerDecisionDto,
    pub goal_id: MobileGoalId,
    pub plan_id: Option<MobilePlanId>,
    pub step_id: Option<MobilePlanStepId>,
    pub waiting_executor: bool,
    pub completion: Option<MobileCompletionProposal>,
    pub failure: Option<MobilePlannerFailure>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MobilePlannerDecisionDto {
    NextAction,
    CompletionProposal,
    CannotProceed,
}

fn fail(code: MobileGoalErrorCode) -> MobileGoalError {
    MobileGoalError::new(code)
}
fn bounded_text(text: &str) -> bool {
    !text.trim().is_empty() && text.len() <= 512 && !text.chars().any(char::is_control)
}
/// Conservative reuse of runtime privacy policy, plus communication/irreversible targets.
pub fn unsafe_text(text: &str) -> bool {
    crate::is_sensitive_mobile_text(text)
        || {
            let lower = text.to_lowercase();
            [
                "adb",
                "shell",
                "javascript",
                "/bin/",
                "sudo",
                "curl ",
                "powershell",
                "secret",
                "seed",
                "mnemonic",
                "wallet",
                "otp",
                "delete account",
                "submit",
                "publish",
                "send message",
                "send",
                "post",
                "confirm",
                "payment",
                "password",
                "私钥",
                "助记词",
                "发布",
                "发送",
                "注销",
                "删除账户",
                "提交",
                "确认",
            ]
            .iter()
            .any(|term| lower.contains(term))
        }
        || text
            .split_whitespace()
            .any(|word| word.is_ascii() && word.len() > 32)
        || text
            .split_whitespace()
            .any(|word| word.len() >= 4 && word.bytes().all(|b| b.is_ascii_digit()))
}
pub fn prompt_text(text: &str) -> Option<String> {
    if unsafe_text(text) || text.split_whitespace().count() >= 12 {
        None
    } else {
        Some(text.chars().filter(|c| !c.is_control()).take(256).collect())
    }
}
/// Android component/resource identifiers are typed metadata, not free-form secrets.
/// Validate their grammar and semantic words without treating a qualified ID as one long token.
pub fn prompt_android_identifier(value: &str) -> Option<String> {
    if value.is_empty()
        || value.len() > 256
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_./:$".contains(&b))
    {
        return None;
    }
    let words = value
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if unsafe_text(&words) {
        None
    } else {
        Some(value.into())
    }
}
/// Preserve complete bounded goals; only UI labels use the shorter text projection.
pub fn prompt_objective(text: &str) -> Option<String> {
    prompt_text(text)?;
    Some(
        text.chars()
            .filter(|c| !c.is_control())
            .take(1000)
            .collect(),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DecisionEnvelope {
    decision: String,
    #[serde(deserialize_with = "required_option")]
    action: Option<MobileNextAction>,
    #[serde(deserialize_with = "required_option")]
    completion: Option<MobileCompletionProposal>,
    #[serde(deserialize_with = "required_option")]
    failure: Option<MobilePlannerFailure>,
}
fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}
pub fn decode_decision(
    raw: &str,
    context: &MobilePlannerContext,
) -> Result<MobilePlannerDecision, MobileGoalError> {
    if raw.len() > MAX_PLANNER_OUTPUT_BYTES || raw.trim().is_empty() {
        return Err(fail(MobileGoalErrorCode::InvalidModelOutput));
    }
    let envelope: DecisionEnvelope =
        serde_json::from_str(raw).map_err(|_| fail(MobileGoalErrorCode::InvalidModelOutput))?;
    if context.remaining_steps == 0 {
        return Err(fail(MobileGoalErrorCode::StepLimitReached));
    }
    if context.goal_status.is_terminal() || context.goal_status == MobileGoalStatus::WaitingApproval
    {
        return Err(fail(MobileGoalErrorCode::InvalidStateTransition));
    }
    match (
        envelope.decision.as_str(),
        envelope.action,
        envelope.completion,
        envelope.failure,
    ) {
        ("next_action", Some(action), None, None) => {
            action.validated_step(context, 1)?;
            Ok(MobilePlannerDecision::NextAction { action })
        }
        ("complete", None, Some(completion), None) => {
            if !bounded_text(&completion.reason)
                || unsafe_text(&completion.reason)
                || completion.supporting_observation_ids.is_empty()
                || completion.supporting_observation_ids.len() > MAX_RECENT_STEPS + 1
                || completion.supporting_observation_ids.iter().any(|id| {
                    !context
                        .current_observation
                        .as_ref()
                        .is_some_and(|o| &o.observation_id == id)
                        && !context.recent_steps.iter().any(|s| {
                            s.status == MobileStepStatus::Verified
                                && s.observation_after_id.as_ref() == Some(id)
                        })
                })
            {
                return Err(fail(MobileGoalErrorCode::InvalidPlan));
            }
            Ok(MobilePlannerDecision::Complete { completion })
        }
        ("cannot_proceed", None, None, Some(failure)) => {
            if !bounded_text(&failure.reason) || unsafe_text(&failure.reason) {
                return Err(fail(MobileGoalErrorCode::InvalidPlan));
            }
            Ok(MobilePlannerDecision::CannotProceed { failure })
        }
        _ => Err(fail(MobileGoalErrorCode::InvalidModelOutput)),
    }
}
impl MobileNextAction {
    pub fn validated_step(
        &self,
        c: &MobilePlannerContext,
        sequence: u32,
    ) -> Result<NewMobileStep, MobileGoalError> {
        use MobileNextAction::*;
        let (kind, reason, expected, target, input, wait, intent) = match self {
            Observe {
                reason,
                expected_result,
            } => (
                MobileStepType::Observe,
                reason,
                expected_result,
                None,
                None,
                None,
                None,
            ),
            ScrollDown {
                reason,
                expected_result,
            } => (
                MobileStepType::ScrollDown,
                reason,
                expected_result,
                None,
                None,
                None,
                None,
            ),
            ScrollUp {
                reason,
                expected_result,
            } => (
                MobileStepType::ScrollUp,
                reason,
                expected_result,
                None,
                None,
                None,
                None,
            ),
            TapElement {
                reason,
                target_element_ref,
                expected_result,
            } => (
                MobileStepType::TapElement,
                reason,
                expected_result,
                Some(target_element_ref),
                None,
                None,
                None,
            ),
            InputText {
                reason,
                target_element_ref,
                input_text,
                expected_result,
            } => (
                MobileStepType::InputText,
                reason,
                expected_result,
                Some(target_element_ref),
                Some(input_text),
                None,
                None,
            ),
            Back {
                reason,
                expected_result,
            } => (
                MobileStepType::Back,
                reason,
                expected_result,
                None,
                None,
                None,
                None,
            ),
            Wait {
                reason,
                wait_ms,
                expected_result,
            } => (
                MobileStepType::Wait,
                reason,
                expected_result,
                None,
                None,
                Some(*wait_ms),
                None,
            ),
            Extract {
                reason,
                extraction_intent,
                expected_result,
            } => (
                MobileStepType::Extract,
                reason,
                expected_result,
                None,
                None,
                None,
                Some(extraction_intent),
            ),
        };
        if !bounded_text(reason) {
            return Err(fail(MobileGoalErrorCode::InvalidPlan));
        }
        if unsafe_text(reason)
            || input.is_some_and(|text| {
                !crate::is_safe_mobile_input(text)
                    || unsafe_text(text)
                    || text.split_whitespace().count() >= 12
            })
            || intent.is_some_and(|text| !bounded_text(text) || unsafe_text(text))
        {
            return Err(fail(MobileGoalErrorCode::PolicyBlocked));
        }
        if wait.is_some_and(|ms| !(1..=MAX_WAIT_MS).contains(&ms)) {
            return Err(fail(MobileGoalErrorCode::InvalidPlan));
        }
        if !matches!(kind, MobileStepType::Observe | MobileStepType::Wait) {
            let o = c
                .current_observation
                .as_ref()
                .ok_or_else(|| fail(MobileGoalErrorCode::InvalidPlan))?;
            if !c.allowed_apps.contains(&o.package)
                || o.sensitive_screen && kind != MobileStepType::Back
            {
                return Err(fail(MobileGoalErrorCode::PolicyBlocked));
            }
        }
        let element = |r: &str| -> Result<&MobilePlannerElement, MobileGoalError> {
            let e = c
                .current_observation
                .as_ref()
                .and_then(|o| o.elements.iter().find(|e| e.element_ref == r))
                .ok_or_else(|| fail(MobileGoalErrorCode::TargetNotFound))?;
            if e.sensitive || !e.enabled || e.text.as_deref().is_some_and(unsafe_text) {
                return Err(fail(MobileGoalErrorCode::PolicyBlocked));
            }
            Ok(e)
        };
        if let Some(target) = target {
            let e = element(target)?;
            if kind == MobileStepType::InputText && !e.editable
                || kind == MobileStepType::TapElement && !e.clickable
            {
                return Err(fail(MobileGoalErrorCode::InvalidPlan));
            }
        }
        let valid_expected = match expected {
            ExpectedStepResult::UiChanged => matches!(
                kind,
                MobileStepType::Observe
                    | MobileStepType::ScrollDown
                    | MobileStepType::ScrollUp
                    | MobileStepType::TapElement
                    | MobileStepType::InputText
                    | MobileStepType::Back
            ),
            ExpectedStepResult::ActivityChanged => {
                matches!(kind, MobileStepType::Back | MobileStepType::TapElement)
            }
            ExpectedStepResult::ElementVisible { element_ref } => {
                element(element_ref)?;
                matches!(
                    kind,
                    MobileStepType::ScrollDown
                        | MobileStepType::ScrollUp
                        | MobileStepType::TapElement
                        | MobileStepType::Back
                )
            }
            ExpectedStepResult::TextEquals { element_ref, value } => {
                let e = element(element_ref)?;
                e.editable
                    && kind == MobileStepType::InputText
                    && target == Some(element_ref)
                    && input == Some(value)
                    && crate::is_safe_mobile_input(value)
                    && !unsafe_text(value)
            }
            ExpectedStepResult::ActivityEquals { package, activity } => {
                matches!(kind, MobileStepType::Back | MobileStepType::TapElement)
                    && c.allowed_apps.contains(package)
                    && prompt_android_identifier(package).is_some()
                    && prompt_android_identifier(activity).is_some()
            }
            ExpectedStepResult::NewDataObject => kind == MobileStepType::Extract,
            ExpectedStepResult::NoChangeExpected => kind == MobileStepType::Wait,
        };
        if !valid_expected {
            return Err(fail(MobileGoalErrorCode::InvalidPlan));
        }
        let risk = if matches!(
            kind,
            MobileStepType::Observe | MobileStepType::Wait | MobileStepType::Extract
        ) {
            MobileStepRisk::ReadOnly
        } else {
            MobileStepRisk::Reversible
        };
        Ok(NewMobileStep {
            sequence,
            step_type: kind,
            reason: reason.clone(),
            risk,
            target_ref: target.cloned(),
            input_text: input.cloned(),
            expected_result: Some(expected.clone()),
            wait_ms: wait,
            extraction_intent: intent.cloned(),
        })
    }
}
/// Provider-native subset: root object, all properties required, no arbitrary fields.
pub fn decision_schema() -> serde_json::Value {
    use serde_json::json;
    fn object(properties: serde_json::Value) -> serde_json::Value {
        let required: Vec<String> = properties.as_object().unwrap().keys().cloned().collect();
        json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
    }
    let string = json!({"type":"string"});
    let expected = json!({"anyOf":[
        object(json!({"kind":{"type":"string","enum":["UI_CHANGED","ACTIVITY_CHANGED","NEW_DATA_OBJECT","NO_CHANGE_EXPECTED"]}})),
        object(json!({"kind":{"type":"string","enum":["ELEMENT_VISIBLE"]},"elementRef":string})),
        object(json!({"kind":{"type":"string","enum":["TEXT_EQUALS"]},"elementRef":string,"value":string})),
        object(json!({"kind":{"type":"string","enum":["ACTIVITY_EQUALS"]},"package":string,"activity":string}))
    ]});
    let mut actions = vec![];
    for kind in [
        "observe",
        "scroll_down",
        "scroll_up",
        "tap_element",
        "input_text",
        "back",
        "wait",
        "extract",
    ] {
        let mut props = json!({"type":{"type":"string","enum":[kind]},"reason":string,"expected_result":expected});
        if kind == "tap_element" || kind == "input_text" {
            props["target_element_ref"] = string.clone();
        }
        if kind == "input_text" {
            props["input_text"] = string.clone();
        }
        if kind == "wait" {
            props["wait_ms"] = json!({"type":"integer"});
        }
        if kind == "extract" {
            props["extraction_intent"] = string.clone();
        }
        actions.push(object(props));
    }
    actions.push(json!({"type":"null"}));
    object(
        json!({"decision":{"type":"string","enum":["next_action","complete","cannot_proceed"]},"action":{"anyOf":actions},
            "completion":{"anyOf":[object(json!({"reason":string,"supporting_observation_ids":{"type":"array","items":string}})),{"type":"null"}]},
            "failure":{"anyOf":[object(json!({"reason":string,"code":{"type":"string","enum":["NEEDS_NEW_OBSERVATION","TARGET_NOT_FOUND","INSUFFICIENT_CONTEXT","GOAL_UNSUPPORTED","SAFETY_BLOCKED"]}})),{"type":"null"}]}
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn context() -> MobilePlannerContext {
        MobilePlannerContext {
            completion_target: None,
            goal_id: MobileGoalId::new(),
            objective: "Read public information".into(),
            goal_status: MobileGoalStatus::Pending,
            plan_revision: None,
            step_budget: 8,
            steps_used: 0,
            remaining_steps: 8,
            observation_missing: false,
            current_observation: Some(MobilePlannerObservation {
                observation_id: "obs-1".into(),
                snapshot_id: "snapshot-1".into(),
                device_session_id: "session-1".into(),
                captured_at: "2026-09-27T00:00:00Z".into(),
                package: "com.android.settings".into(),
                activity: "Main".into(),
                screen_width: 1080,
                screen_height: 2400,
                sensitive_screen: false,
                elements: vec![MobilePlannerElement {
                    resource_id: None,
                    element_ref: "@e1".into(),
                    text: Some("Search".into()),
                    clickable: true,
                    editable: true,
                    enabled: true,
                    sensitive: false,
                }],
            }),
            recent_steps: vec![],
            recent_failures: vec![],
            allowed_action_types: vec![],
            forbidden_capabilities: vec![],
            allowed_apps: vec!["com.android.settings".into()],
        }
    }
    fn output(action: Value) -> String {
        json!({"decision":"next_action","action":action,"completion":null,"failure":null})
            .to_string()
    }
    fn action(t: &str, expected: &str) -> Value {
        json!({"type":t,"reason":"Inspect current public UI","expected_result":{"kind":expected}})
    }
    #[test]
    fn accepts_single_semantic_actions_and_locally_derives_risk() {
        for (t, e) in [
            ("observe", "UI_CHANGED"),
            ("scroll_down", "UI_CHANGED"),
            ("scroll_up", "UI_CHANGED"),
            ("back", "ACTIVITY_CHANGED"),
            ("wait", "NO_CHANGE_EXPECTED"),
            ("extract", "NEW_DATA_OBJECT"),
            ("tap_element", "UI_CHANGED"),
            ("input_text", "UI_CHANGED"),
        ] {
            let mut a = action(t, e);
            if t == "wait" {
                a["wait_ms"] = json!(5000)
            }
            if t == "extract" {
                a["extraction_intent"] = json!("Read visible public facts")
            }
            if t == "tap_element" || t == "input_text" {
                a["target_element_ref"] = json!("@e1")
            }
            if t == "input_text" {
                a["input_text"] = json!("public data")
            }
            let decision = decode_decision(&output(a), &context()).unwrap();
            let MobilePlannerDecision::NextAction { action } = decision else {
                panic!("one action required")
            };
            let step = action.validated_step(&context(), 1).unwrap();
            let expected_risk = if ["observe", "wait", "extract"].contains(&t) {
                MobileStepRisk::ReadOnly
            } else {
                MobileStepRisk::Reversible
            };
            assert_eq!(step.risk, expected_risk, "{t}");
        }
    }
    #[test]
    fn qualified_system_activity_uses_metadata_validation_without_allowing_sensitive_components() {
        let mut c = context();
        let package = "com.google.android.settings.intelligence";
        c.allowed_apps.push(package.into());
        let mut a = action("back", "ACTIVITY_CHANGED");
        a["expected_result"] = json!({"kind":"ACTIVITY_EQUALS","package":package,
            "activity":"com.google.android.settings.intelligence.modules.search.SearchActivity"});
        assert!(decode_decision(&output(a.clone()), &c).is_ok());
        for activity in [
            "com.example.password.Field",
            "com.example.wallet.Sign",
            "com.example.private_key.Export",
            "com.example.send_message.Compose",
        ] {
            a["expected_result"]["activity"] = json!(activity);
            assert!(
                decode_decision(&output(a.clone()), &c).is_err(),
                "{activity}"
            );
        }
    }
    #[test]
    fn rejects_malformed_empty_fenced_and_oversized_output() {
        for raw in ["", "{", "[]", "```json\n{}\n```", &" ".repeat(16_385)] {
            assert!(decode_decision(raw, &context()).is_err());
        }
    }
    #[test]
    fn rejects_unknown_multi_action_coordinates_commands_and_claimed_risk() {
        for extra in [
            "steps",
            "tap_x",
            "tap_y",
            "x1",
            "y1",
            "x2",
            "y2",
            "shell",
            "adb",
            "javascript",
            "security_override",
            "risk",
        ] {
            let mut a = action("observe", "UI_CHANGED");
            a[extra] = json!("read_only");
            assert!(decode_decision(&output(a), &context()).is_err(), "{extra}");
        }
        for t in ["stop", "complete", "shell", "adb", "tap 10 20", "unknown"] {
            assert!(decode_decision(&output(action(t, "UI_CHANGED")), &context()).is_err());
        }
        assert!(decode_decision(&json!({"decision":"next_action","steps":[action("observe","UI_CHANGED"),action("back","UI_CHANGED")]}).to_string(),&context()).is_err());
    }
    #[test]
    fn rejects_invented_sensitive_disabled_and_noneditable_targets() {
        let mut a = action("input_text", "UI_CHANGED");
        a["target_element_ref"] = json!("@invented");
        a["input_text"] = json!("public");
        assert!(decode_decision(&output(a.clone()), &context()).is_err());
        a["target_element_ref"] = json!("@e1");
        for flag in ["sensitive", "disabled", "noneditable"] {
            let mut c = context();
            let e = &mut c.current_observation.as_mut().unwrap().elements[0];
            match flag {
                "sensitive" => e.sensitive = true,
                "disabled" => e.enabled = false,
                _ => e.editable = false,
            };
            assert!(decode_decision(&output(a.clone()), &c).is_err());
        }
    }
    #[test]
    fn rejects_missing_reason_wrong_expectation_wait_bounds_and_extra_parameters() {
        let mut a = action("back", "NEW_DATA_OBJECT");
        assert!(decode_decision(&output(a.clone()), &context()).is_err());
        a = action("observe", "UI_CHANGED");
        a.as_object_mut().unwrap().remove("reason");
        assert!(decode_decision(&output(a), &context()).is_err());
        for ms in [0, 5001] {
            let mut a = action("wait", "NO_CHANGE_EXPECTED");
            a["wait_ms"] = json!(ms);
            assert!(decode_decision(&output(a), &context()).is_err());
        }
        let mut a = action("observe", "UI_CHANGED");
        a["input_text"] = json!("unnecessary");
        assert!(decode_decision(&output(a), &context()).is_err());
    }
    #[test]
    fn only_proposes_completion_with_current_observation_support() {
        let raw = |ids: Value| {
            json!({"decision":"complete","action":null,"completion":{"reason":"Observed criteria","supporting_observation_ids":ids},"failure":null}).to_string()
        };
        assert!(decode_decision(&raw(json!(["obs-1"])), &context()).is_ok());
        for ids in [json!([]), json!(["invented"])] {
            assert!(decode_decision(&raw(ids), &context()).is_err());
        }
        let raw=json!({"decision":"cannot_proceed","action":null,"completion":null,"failure":{"code":"INSUFFICIENT_CONTEXT","reason":"No public information"}}).to_string();
        assert!(decode_decision(&raw, &context()).is_ok());
    }
    #[test]
    fn missing_observation_allows_only_observe_or_wait_and_budget_blocks() {
        let mut c = context();
        c.current_observation = None;
        c.observation_missing = true;
        assert!(decode_decision(&output(action("observe", "UI_CHANGED")), &c).is_ok());
        for t in ["scroll_down", "scroll_up", "back", "extract"] {
            assert!(decode_decision(&output(action(t, "UI_CHANGED")), &c).is_err());
        }
        c.remaining_steps = 0;
        assert!(decode_decision(&output(action("observe", "UI_CHANGED")), &c).is_err());
    }
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn rejects_command_strings_secrets_and_sensitive_expected_values() {
        for text in [
            "adb shell input tap 1 2",
            "curl https example",
            "OTP 123456",
            "password",
            "private key",
            "seed phrase",
            "send message",
            "publish",
            "wallet signing",
            "delete account",
        ] {
            assert!(unsafe_text(text), "{text}");
            assert!(prompt_text(text).is_none());
        }
    }
    #[test]
    fn schema_native_objects_have_required_fixed_fields_and_no_root_union() {
        fn check(value: &serde_json::Value) {
            if value["type"] == "object" {
                assert_eq!(value["additionalProperties"], false);
                assert_eq!(
                    value["required"].as_array().unwrap().len(),
                    value["properties"].as_object().unwrap().len()
                );
            }
            match value {
                serde_json::Value::Object(o) => {
                    for v in o.values() {
                        check(v)
                    }
                }
                serde_json::Value::Array(a) => {
                    for v in a {
                        check(v)
                    }
                }
                _ => {}
            }
        }
        let schema = decision_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema.get("anyOf").is_none());
        check(&schema);
        assert_eq!(
            schema["properties"]["decision"]["enum"],
            json!(["next_action", "complete", "cannot_proceed"])
        );
    }
}

#[cfg(test)]
mod locale_privacy_tests {
    use super::*;
    #[test]
    fn preserves_ordinary_chinese_objectives_while_redacting_long_ascii_secret_tokens() {
        let text = "向下查看当前页面并提取新增的公开信息";
        assert_eq!(prompt_text(text).as_deref(), Some(text));
        assert!(prompt_text(&"a".repeat(64)).is_none());
    }
}
