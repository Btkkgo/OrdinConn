//! Persistent semantic contracts only. No planner, tool execution, or model fallback.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

macro_rules! typed_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);
        impl $name {
            pub fn new() -> Self {
                Self(format!("{}{}", $prefix, Uuid::now_v7()))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)?;
                let valid = value
                    .strip_prefix($prefix)
                    .and_then(|suffix| Uuid::parse_str(suffix).ok())
                    .is_some_and(|id| id.get_version_num() == 7);
                if !valid {
                    return Err(serde::de::Error::custom("Invalid typed mobile ID"));
                }
                Ok(Self(value))
            }
        }
    };
}
typed_id!(MobileGoalId, "mobile_goal_");
typed_id!(MobilePlanId, "mobile_plan_");
typed_id!(MobilePlanStepId, "mobile_step_");

macro_rules! contract_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name { $($variant),+ }
        impl $name { pub fn as_str(self) -> &'static str {
            match self { $(Self::$variant => contract_enum!(@name $variant)),+ }
        } }
    };
    (@name Pending) => { "PENDING" }; (@name Planning) => { "PLANNING" };
    (@name Running) => { "RUNNING" }; (@name WaitingApproval) => { "WAITING_APPROVAL" };
    (@name Completed) => { "COMPLETED" }; (@name Failed) => { "FAILED" }; (@name Stopped) => { "STOPPED" };
    (@name Draft) => { "DRAFT" }; (@name Active) => { "ACTIVE" }; (@name Superseded) => { "SUPERSEDED" };
    (@name Executing) => { "EXECUTING" }; (@name Verified) => { "VERIFIED" }; (@name Skipped) => { "SKIPPED" };
}
contract_enum!(MobileGoalStatus {
    Pending,
    Planning,
    Running,
    WaitingApproval,
    Completed,
    Failed,
    Stopped
});
contract_enum!(MobilePlanStatus {
    Draft,
    Active,
    Completed,
    Superseded,
    Failed
});
contract_enum!(MobileStepStatus {
    Pending,
    Executing,
    WaitingApproval,
    Verified,
    Failed,
    Skipped,
    Stopped
});
impl MobileGoalStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Stopped)
    }
    pub fn can_transition_to(self, next: Self) -> bool {
        use MobileGoalStatus::*;
        matches!(
            (self, next),
            (Pending, Planning | Failed | Stopped)
                | (Planning, Running | WaitingApproval | Failed | Stopped)
                | (Running, WaitingApproval | Completed | Failed | Stopped)
                | (WaitingApproval, Running | Failed | Stopped)
        )
    }
    pub fn event_type(self) -> &'static str {
        match self {
            Self::Pending => "mobile.goal_created",
            Self::Planning => "mobile.goal_planning",
            Self::Running => "mobile.goal_started",
            Self::WaitingApproval => "mobile.goal_waiting_approval",
            Self::Completed => "mobile.goal_completed",
            Self::Failed => "mobile.goal_failed",
            Self::Stopped => "mobile.goal_stopped",
        }
    }
}
impl MobileStepStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Verified | Self::Failed | Self::Skipped | Self::Stopped
        )
    }
    pub fn can_transition_to(self, next: Self) -> bool {
        use MobileStepStatus::*;
        matches!(
            (self, next),
            (Pending, Executing | WaitingApproval | Skipped | Stopped)
                | (WaitingApproval, Executing | Failed | Stopped)
                | (Executing, Verified | Failed | Stopped)
        )
    }
    pub fn event_type(self) -> &'static str {
        match self {
            Self::Pending => "mobile.step_created",
            Self::Executing => "mobile.step_started",
            Self::WaitingApproval => "mobile.step_waiting_approval",
            Self::Verified => "mobile.step_verified",
            Self::Failed => "mobile.step_failed",
            Self::Skipped => "mobile.step_skipped",
            Self::Stopped => "mobile.step_stopped",
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MobileStepType {
    Observe,
    ScrollDown,
    ScrollUp,
    TapElement,
    InputText,
    Back,
    Wait,
    Extract,
    Complete,
    Stop,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MobileStepRisk {
    ReadOnly,
    Reversible,
    ApprovalRequired,
    Forbidden,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "SCREAMING_SNAKE_CASE",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ExpectedStepResult {
    UiChanged,
    ElementVisible { element_ref: String },
    TextEquals { element_ref: String, value: String },
    ActivityChanged,
    ActivityEquals { package: String, activity: String },
    NewDataObject,
    NoChangeExpected,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MobileGoalErrorCode {
    DeviceDisconnected,
    ModelNotConfigured,
    ModelNotSelected,
    InvalidModelOutput,
    ModelError,
    ObserveFailed,
    TargetNotFound,
    ActionFailed,
    VerifyFailed,
    ApprovalRequired,
    ApprovalRejected,
    PolicyBlocked,
    StepLimitReached,
    TimeLimitReached,
    Stalled,
    UserStopped,
    InterruptedByRestart,
    InvalidPlan,
    InvalidStep,
    InvalidStateTransition,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileGoalError {
    pub code: MobileGoalErrorCode,
    pub message: String,
}
impl MobileGoalError {
    /// Fixed messages: no model text, private screen text, or secrets in audit/errors.
    pub fn new(code: MobileGoalErrorCode) -> Self {
        let message = match code {
            MobileGoalErrorCode::InvalidStateTransition => {
                "Mobile execution state transition rejected"
            }
            MobileGoalErrorCode::InvalidPlan => "Mobile plan is invalid",
            MobileGoalErrorCode::InvalidStep => "Mobile step or linkage is invalid",
            MobileGoalErrorCode::InterruptedByRestart => {
                "Mobile execution interrupted by restart; no actions replayed"
            }
            MobileGoalErrorCode::ModelNotConfigured => {
                "A real model provider must be configured before planning"
            }
            _ => "Mobile goal operation did not succeed",
        };
        Self {
            code,
            message: message.into(),
        }
    }
}
impl std::fmt::Display for MobileGoalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for MobileGoalError {}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileGoalBudget {
    pub max_steps: u32,
    #[serde(default = "default_model_calls")]
    pub max_model_calls: u32,
    #[serde(default = "default_execution_actions")]
    pub max_execution_actions: u32,
    pub max_runtime_ms: u32,
    pub max_consecutive_failures: u32,
    pub max_identical_observations: u32,
}
impl Default for MobileGoalBudget {
    fn default() -> Self {
        Self {
            max_steps: 8,
            max_model_calls: default_model_calls(),
            max_execution_actions: default_execution_actions(),
            max_runtime_ms: 120000,
            max_consecutive_failures: 2,
            max_identical_observations: 3,
        }
    }
}
impl MobileGoalBudget {
    pub fn validate(&self) -> Result<(), MobileGoalError> {
        if !(1..=100).contains(&self.max_steps)
            || !(1..=202).contains(&self.max_model_calls)
            || !(1..=100).contains(&self.max_execution_actions)
            || !(1..=3_600_000).contains(&self.max_runtime_ms)
            || !(1..=self.max_steps).contains(&self.max_consecutive_failures)
            || !(1..=100).contains(&self.max_identical_observations)
        {
            return Err(MobileGoalError::new(MobileGoalErrorCode::InvalidPlan));
        }
        Ok(())
    }
}
fn default_model_calls() -> u32 {
    18
}
fn default_execution_actions() -> u32 {
    8
}

/// Owner-supplied completion truth, persisted before any model request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "SCREAMING_SNAKE_CASE",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MobileCompletionTarget {
    ActivityEquals {
        package: String,
        activity: String,
    },
    InputTextEquals {
        package: String,
        activity: String,
        resource_id: String,
        value: String,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileGoal {
    pub id: MobileGoalId,
    pub objective: String,
    pub normalized_objective: Option<String>,
    pub status: MobileGoalStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub active_plan_id: Option<MobilePlanId>,
    pub step_budget: MobileGoalBudget,
    pub runtime_deadline: Option<DateTime<Utc>>,
    pub consecutive_failure_count: u32,
    pub identical_observation_count: u32,
    pub last_error_code: Option<MobileGoalErrorCode>,
    pub last_error_message: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobilePlan {
    pub id: MobilePlanId,
    pub goal_id: MobileGoalId,
    pub revision: u32,
    pub objective: String,
    pub status: MobilePlanStatus,
    pub created_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub superseded_at: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewMobileStep {
    pub sequence: u32,
    pub step_type: MobileStepType,
    pub reason: String,
    pub risk: MobileStepRisk,
    pub target_ref: Option<String>,
    pub input_text: Option<String>,
    pub expected_result: Option<ExpectedStepResult>,
    #[serde(default)]
    pub wait_ms: Option<u32>,
    #[serde(default)]
    pub extraction_intent: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobilePlanStep {
    pub id: MobilePlanStepId,
    pub plan_id: MobilePlanId,
    pub sequence: u32,
    pub step_type: MobileStepType,
    pub status: MobileStepStatus,
    pub reason: String,
    pub risk: MobileStepRisk,
    pub target_ref: Option<String>,
    pub input_text: Option<String>,
    pub expected_result: Option<ExpectedStepResult>,
    #[serde(default)]
    pub wait_ms: Option<u32>,
    #[serde(default)]
    pub extraction_intent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub observation_before_id: Option<String>,
    pub observation_after_id: Option<String>,
    pub action_id: Option<String>,
    pub evidence_id: Option<String>,
    pub error_code: Option<MobileGoalErrorCode>,
    pub error_message: Option<String>,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MobileStepOutcome {
    Verified,
    Failed,
    Stopped,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileStepResult {
    pub step_id: MobilePlanStepId,
    pub outcome: MobileStepOutcome,
    pub verified: bool,
    pub observation_before_id: Option<String>,
    pub observation_after_id: Option<String>,
    pub action_receipt_id: Option<String>,
    pub evidence_ids: Vec<String>,
    pub error: Option<MobileGoalError>,
    pub created_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileGoalPlan {
    pub plan: MobilePlan,
    pub steps: Vec<MobilePlanStep>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileExecutionReferences {
    pub goal_id: Option<MobileGoalId>,
    pub plan_id: Option<MobilePlanId>,
    pub step_id: Option<MobilePlanStepId>,
    pub observation_id: Option<String>,
    pub action_id: Option<String>,
    pub evidence_id: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileApprovalSubject {
    pub goal_id: MobileGoalId,
    pub plan_id: MobilePlanId,
    pub step_id: MobilePlanStepId,
    pub action_type: MobileStepType,
    pub target_hash: Option<String>,
    pub observation_id: Option<String>,
}
impl MobileApprovalSubject {
    /// Versioned domain separation; not an issued capability and not accepted by Trade Approval.
    pub fn canonical_hash(&self) -> Result<String, serde_json::Error> {
        let mut digest = Sha256::new();
        digest.update(b"ordinconn.mobile-approval-subject.v1\0");
        digest.update(serde_json::to_vec(self)?);
        Ok(format!("sha256:{:x}", digest.finalize()))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobileStepEvidenceLink {
    pub step_id: MobilePlanStepId,
    pub evidence_id: String,
    pub observation_id: String,
}
