//! Semantic resolution and typed objective verification. No device commands or database.
use crate::{execution::*, *};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MobileVerificationOutcome {
    Verified,
    Failed,
    Inconclusive,
}
pub struct MobileVerificationEngine;
/// Compare application structure, excluding Android system chrome and PNG-only changes.
pub fn same_app_ui(a: &MobileUiSnapshot, b: &MobileUiSnapshot) -> bool {
    fn app_elements(s: &MobileUiSnapshot) -> Vec<serde_json::Value> {
        s.elements.iter().filter(|e| !e.resource_id.as_deref().is_some_and(|id|id.starts_with("com.android.systemui:"))).map(|e|serde_json::json!({"text":e.text,"role":e.role,"class":e.class_name,"description":e.content_description,"bounds":e.bounds,"clickable":e.clickable,"scrollable":e.scrollable,"enabled":e.enabled,"focused":e.focused,"selected":e.selected,"resourceId":e.resource_id})).collect()
    }
    a.package_name == b.package_name
        && a.activity == b.activity
        && a.screen_width == b.screen_width
        && a.screen_height == b.screen_height
        && app_elements(a) == app_elements(b)
}
/// A snapshot-local ref must be rebound by an unambiguous semantic identity, never an index.
pub fn matching_element<'a>(
    source: &MobileElement,
    after: &'a MobileUiSnapshot,
) -> Option<&'a MobileElement> {
    let mut found = after.elements.iter().filter(|e| {
        let label_matches = source.class_name.ends_with("EditText")
            || source
                .text
                .as_deref()
                .filter(|s| !s.is_empty() && *s != "[REDACTED]")
                .is_none_or(|text| e.text.as_deref() == Some(text));
        label_matches
            && e.class_name == source.class_name
            && match source.resource_id.as_deref().filter(|id| !id.is_empty()) {
                Some(id) => e.resource_id.as_deref() == Some(id),
                None => source
                    .content_description
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .is_some_and(|d| e.content_description.as_deref() == Some(d)),
            }
    });
    let first = found.next()?;
    if found.next().is_some() {
        None
    } else {
        Some(first)
    }
}
pub fn valid_capture(c: &MobileCapture) -> bool {
    c.session.status == MobileSessionStatus::Connected
        && c.session.device_type == MobileDeviceType::Emulator
        && c.session.device_id.starts_with("emulator-")
        && c.snapshot.session_id == c.session.session_id
        && c.frame.session_id == c.session.session_id
        && c.observation.device_session_id == c.session.session_id
        && c.observation.package_name == c.snapshot.package_name
        && c.observation.activity == c.snapshot.activity
        && c.snapshot.screen_width > 0
        && c.snapshot.screen_height > 0
        && c.observation.verification_status == VerificationResult::Verified
        && matches!(
            c.snapshot.package_name.as_str(),
            "com.android.settings" | "com.google.android.settings.intelligence"
        )
        && c.observation.privacy_class != PrivacyClass::Sensitive
        && c.snapshot.sensitive_state.is_none()
}
impl MobileVerificationEngine {
    pub fn verify(
        expected: &ExpectedStepResult,
        before: &MobileCapture,
        after: &MobileCapture,
        receipt: Option<&MobileActionReceipt>,
        new_data_object_persisted: bool,
    ) -> MobileVerificationOutcome {
        use MobileVerificationOutcome::*;
        if !valid_capture(before)
            || !valid_capture(after)
            || before.session.session_id != after.session.session_id
            || before.session.device_id != after.session.device_id
        {
            return Inconclusive;
        }
        if let Some(r) = receipt {
            if !r.command_sent
                || r.status != MobileActionStatus::Executed
                || r.decision != MobileActionDecision::Allowed
                || r.session_id != before.session.session_id
                || r.snapshot_id != before.snapshot.snapshot_id
                || r.post_snapshot_id.as_deref() != Some(after.snapshot.snapshot_id.as_str())
                || r.pre_ui_tree_hash != before.observation.ui_tree_hash
                || r.post_ui_tree_hash.as_deref() != Some(after.observation.ui_tree_hash.as_str())
            {
                return Inconclusive;
            }
        }
        let passed = match expected {
            ExpectedStepResult::UiChanged => !same_app_ui(&before.snapshot, &after.snapshot),
            ExpectedStepResult::NoChangeExpected => same_app_ui(&before.snapshot, &after.snapshot),
            ExpectedStepResult::ActivityChanged => {
                before.snapshot.package_name != after.snapshot.package_name
                    || before.snapshot.activity != after.snapshot.activity
            }
            ExpectedStepResult::ActivityEquals { package, activity } => {
                after.snapshot.package_name == *package && after.snapshot.activity == *activity
            }
            ExpectedStepResult::ElementVisible { element_ref } => before
                .snapshot
                .elements
                .iter()
                .find(|e| e.element_ref == *element_ref)
                .and_then(|e| matching_element(e, &after.snapshot))
                .is_some_and(|e| e.enabled && e.bounds.width > 0 && e.bounds.height > 0),
            ExpectedStepResult::TextEquals { element_ref, value } => {
                before.snapshot.package_name == after.snapshot.package_name
                    && before.snapshot.activity == after.snapshot.activity
                    && before
                        .snapshot
                        .elements
                        .iter()
                        .find(|e| e.element_ref == *element_ref)
                        .and_then(|e| matching_element(e, &after.snapshot))
                        .is_some_and(|e| {
                            let attested = receipt.is_some_and(|r|
                                matches!(&r.target, MobileActionTarget::Type { element_ref: target } if target == element_ref)
                                && r.input_value_verified == Some(true)
                                && r.verification == Some(VerificationResult::Verified)
                                && r.text_length == Some(value.chars().count())
                                && r.text_sha256.as_deref() == Some(SensitiveText::new(value.clone()).sha256().as_str()));
                            e.enabled && e.class_name.ends_with("EditText")
                                && match receipt {
                                    Some(r) if matches!(r.target,MobileActionTarget::Type{..}) => attested,
                                    _ => e.text.as_deref() == Some(value.as_str()),
                                }
                        })
            }
            ExpectedStepResult::NewDataObject => new_data_object_persisted,
        };
        if passed { Verified } else { Failed }
    }
}
#[derive(Debug)]
pub struct ResolvedSemanticAction {
    pub request: Option<MobileActionRequest>,
    pub expected_result: ExpectedStepResult,
}
pub fn resolve_semantic_step(
    step: &MobilePlanStep,
    before: &MobileCapture,
    planned: Option<&MobileUiSnapshot>,
    execution: &str,
    allowed: &[String],
) -> Result<ResolvedSemanticAction, MobileGoalError> {
    use MobileStepType::*;
    let fail = |code| MobileGoalError::new(code);
    if !valid_capture(before) {
        return Err(fail(MobileGoalErrorCode::PolicyBlocked));
    }
    if !allowed.contains(&before.snapshot.package_name)
        || !matches!(
            before.snapshot.package_name.as_str(),
            "com.android.settings" | "com.google.android.settings.intelligence"
        )
    {
        return Err(fail(MobileGoalErrorCode::PolicyBlocked));
    }
    let age = Utc::now()
        .signed_duration_since(before.snapshot.captured_at)
        .num_milliseconds();
    if !(0..=10_000).contains(&age) {
        return Err(fail(MobileGoalErrorCode::InvalidPlan));
    }
    if let Some(p) = planned {
        if p.session_id != before.session.session_id || !same_app_ui(p, &before.snapshot) {
            return Err(fail(MobileGoalErrorCode::InvalidPlan));
        }
    }
    let risk = if matches!(step.step_type, Observe | Wait | Extract) {
        MobileStepRisk::ReadOnly
    } else {
        MobileStepRisk::Reversible
    };
    if step.risk != risk {
        return Err(fail(MobileGoalErrorCode::PolicyBlocked));
    }
    if (step.target_ref.is_some() && !matches!(step.step_type, TapElement | InputText))
        || (step.input_text.is_some() && step.step_type != InputText)
        || (step.wait_ms.is_some() && step.step_type != Wait)
        || (step.extraction_intent.is_some() && step.step_type != Extract)
    {
        return Err(fail(MobileGoalErrorCode::InvalidStep));
    }
    let mut expected = step
        .expected_result
        .clone()
        .ok_or_else(|| fail(MobileGoalErrorCode::InvalidStep))?;
    let resolve_ref = |reference: &str| -> Result<String, MobileGoalError> {
        let old = planned
            .ok_or_else(|| fail(MobileGoalErrorCode::TargetNotFound))?
            .elements
            .iter()
            .find(|e| e.element_ref == reference)
            .ok_or_else(|| fail(MobileGoalErrorCode::TargetNotFound))?;
        let found = matching_element(old, &before.snapshot)
            .or_else(|| {
                // A non-labelled clickable container can be rebound only on an unchanged complete UI.
                // Compare its actual structure and bounds, never a planner-provided coordinate or index.
                let mut candidates = before.snapshot.elements.iter().filter(|e| {
                    e.class_name == old.class_name
                        && e.bounds == old.bounds
                        && e.resource_id == old.resource_id
                        && e.text == old.text
                        && e.content_description == old.content_description
                        && e.enabled == old.enabled
                        && e.clickable == old.clickable
                        && e.focused == old.focused
                });
                let first = candidates.next()?;
                if candidates.next().is_some() {
                    None
                } else {
                    Some(first)
                }
            })
            .ok_or_else(|| fail(MobileGoalErrorCode::TargetNotFound))?;
        Ok(found.element_ref.clone())
    };
    match &mut expected {
        ExpectedStepResult::ElementVisible { element_ref }
        | ExpectedStepResult::TextEquals { element_ref, .. } => {
            *element_ref = resolve_ref(element_ref)?
        }
        _ => {}
    }
    let target = match step.step_type {
        ScrollDown => Some(MobileActionTarget::Swipe {
            direction: SwipeDirection::Up,
        }),
        ScrollUp => Some(MobileActionTarget::Swipe {
            direction: SwipeDirection::Down,
        }),
        TapElement | InputText => Some({
            let reference = resolve_ref(
                step.target_ref
                    .as_deref()
                    .ok_or_else(|| fail(MobileGoalErrorCode::TargetNotFound))?,
            )?;
            let element = before
                .snapshot
                .elements
                .iter()
                .find(|e| e.element_ref == reference)
                .unwrap();
            if element
                .text
                .as_deref()
                .is_some_and(crate::planner::unsafe_text)
                || element
                    .content_description
                    .as_deref()
                    .is_some_and(crate::planner::unsafe_text)
                || element
                    .resource_id
                    .as_deref()
                    .is_some_and(|id| crate::planner::prompt_android_identifier(id).is_none())
                || before
                    .snapshot
                    .redactions
                    .iter()
                    .any(|r| r.starts_with(&format!("element:{reference}:")))
            {
                return Err(fail(MobileGoalErrorCode::PolicyBlocked));
            }
            if step.step_type == InputText {
                let text = step
                    .input_text
                    .as_deref()
                    .ok_or_else(|| fail(MobileGoalErrorCode::InvalidStep))?;
                if !is_safe_mobile_input(text) || crate::planner::unsafe_text(text) {
                    return Err(fail(MobileGoalErrorCode::PolicyBlocked));
                }
                if !matches!(&expected,ExpectedStepResult::TextEquals{element_ref,value} if element_ref==&reference && value==text)
                {
                    return Err(fail(MobileGoalErrorCode::InvalidStep));
                }
                MobileActionTarget::Type {
                    element_ref: reference,
                }
            } else {
                MobileActionTarget::Tap {
                    element_ref: reference,
                }
            }
        }),
        Back => {
            if !matches!(
                expected,
                ExpectedStepResult::ActivityChanged
                    | ExpectedStepResult::ActivityEquals { .. }
                    | ExpectedStepResult::ElementVisible { .. }
            ) {
                return Err(fail(MobileGoalErrorCode::InvalidStep));
            }
            Some(MobileActionTarget::Back)
        }
        Wait => {
            if !step
                .wait_ms
                .is_some_and(|ms| (1..=crate::planner::MAX_WAIT_MS).contains(&ms))
            {
                return Err(fail(MobileGoalErrorCode::InvalidStep));
            }
            None
        }
        Observe => {
            if !matches!(expected, ExpectedStepResult::NoChangeExpected) {
                return Err(fail(MobileGoalErrorCode::InvalidStep));
            }
            None
        }
        Extract => {
            if expected != ExpectedStepResult::NewDataObject
                || step
                    .extraction_intent
                    .as_deref()
                    .is_none_or(|s| s.trim().is_empty() || crate::planner::unsafe_text(s))
            {
                return Err(fail(MobileGoalErrorCode::InvalidStep));
            }
            None
        }
        _ => return Err(fail(MobileGoalErrorCode::PolicyBlocked)),
    };
    if matches!(step.step_type, ScrollDown | ScrollUp)
        && !matches!(
            expected,
            ExpectedStepResult::UiChanged | ExpectedStepResult::ElementVisible { .. }
        )
    {
        return Err(fail(MobileGoalErrorCode::InvalidStep));
    }
    let request = target.map(|target| MobileActionRequest {
        action_id: format!("mobile_action_{execution}"),
        session_id: before.session.session_id.clone(),
        snapshot_id: before.snapshot.snapshot_id.clone(),
        expected_package: before.snapshot.package_name.clone(),
        requested_at: Utc::now(),
        target,
        text: step.input_text.clone().map(SensitiveText::new),
    });
    if let Some(r) = &request {
        let policy = evaluate_action(
            r,
            &before.session,
            &before.snapshot,
            allowed,
            0,
            &before.snapshot.package_name,
            &before.snapshot.activity,
            Utc::now(),
        );
        if let MobileActionDecision::Denied(reason) = policy {
            return Err(fail(if reason == MobileActionDenyReason::TargetMissing {
                MobileGoalErrorCode::TargetNotFound
            } else {
                MobileGoalErrorCode::PolicyBlocked
            }));
        }
    }
    Ok(ResolvedSemanticAction {
        request,
        expected_result: expected,
    })
}
