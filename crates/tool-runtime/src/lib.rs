use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel {
    Read,
    LowWrite,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolAvailability {
    Available,
    RequiresApproval,
    Unavailable,
    Prohibited,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub permission: String,
    pub risk_level: RiskLevel,
    pub availability: ToolAvailability,
}

impl ToolDefinition {
    pub fn new(id: impl Into<String>, risk_level: RiskLevel) -> Self {
        let id = id.into();
        Self {
            name: id.clone(),
            id,
            description: String::new(),
            input_schema: json!({"type": "object", "additionalProperties": false}),
            permission: "runtime".into(),
            risk_level,
            availability: ToolAvailability::Available,
        }
    }

    pub fn unavailable(mut self) -> Self {
        self.availability = ToolAvailability::Unavailable;
        self
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ToolRegistryError {
    #[error("tool is prohibited by OrdinConn safety policy")]
    ProhibitedTool,
    #[error("tool id is already registered")]
    Duplicate,
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, ToolDefinition>,
}

impl ToolRegistry {
    pub fn register(&mut self, mut tool: ToolDefinition) -> Result<(), ToolRegistryError> {
        let normalized = tool.id.to_ascii_lowercase();
        if [
            "private_key",
            "seed_phrase",
            "mnemonic",
            "withdraw",
            "asset_transfer",
        ]
        .iter()
        .any(|blocked| normalized.contains(blocked))
            || tool.risk_level == RiskLevel::Critical
        {
            tool.availability = ToolAvailability::Prohibited;
            return Err(ToolRegistryError::ProhibitedTool);
        }
        if self.tools.contains_key(&tool.id) {
            return Err(ToolRegistryError::Duplicate);
        }
        self.tools.insert(tool.id.clone(), tool);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&ToolDefinition> {
        self.tools.get(id)
    }

    pub fn list(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    pub fn v0_1() -> Self {
        let mut registry = Self::default();
        for id in [
            "read_market_context",
            "read_signal",
            "read_evidence",
            "search_local_data",
            "create_signal_report",
            "create_watch",
            "open_url",
            "computer_observe",
            "mobile_observe",
            "mobile_read_ui_tree",
            "mobile_read_frame",
            "mobile_inspect_element",
        ] {
            registry
                .register(ToolDefinition::new(id, RiskLevel::Read))
                .expect("safe built-in tool");
        }
        registry
            .register(ToolDefinition::new("computer_click", RiskLevel::Medium).unavailable())
            .expect("bounded unavailable tool");
        registry
            .register(ToolDefinition::new("computer_type", RiskLevel::High).unavailable())
            .expect("bounded unavailable tool");
        for id in [
            "mobile_tap",
            "mobile_swipe",
            "mobile_type",
            "mobile_back",
            "mobile_home",
            "mobile_open_app",
        ] {
            registry
                .register(ToolDefinition::new(id, RiskLevel::Medium).unavailable())
                .expect("bounded unavailable mobile tool");
        }
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_and_type_are_truthfully_unavailable_in_v0_1() {
        let registry = ToolRegistry::v0_1();
        assert_eq!(
            registry.get("computer_click").unwrap().availability,
            ToolAvailability::Unavailable
        );
        assert_eq!(
            registry.get("computer_type").unwrap().availability,
            ToolAvailability::Unavailable
        );
        assert_eq!(
            registry.get("read_evidence").unwrap().risk_level,
            RiskLevel::Read
        );
    }

    #[test]
    fn prohibited_secret_tools_cannot_be_registered() {
        let mut registry = ToolRegistry::default();
        let result =
            registry.register(ToolDefinition::new("read_private_key", RiskLevel::Critical));
        assert_eq!(result.unwrap_err(), ToolRegistryError::ProhibitedTool);
    }

    #[test]
    fn mobile_tools_are_observe_only() {
        let registry = ToolRegistry::v0_1();
        for id in [
            "mobile_observe",
            "mobile_read_ui_tree",
            "mobile_read_frame",
            "mobile_inspect_element",
        ] {
            assert_eq!(
                registry.get(id).unwrap().availability,
                ToolAvailability::Available
            );
            assert_eq!(registry.get(id).unwrap().risk_level, RiskLevel::Read);
        }
        for id in [
            "mobile_tap",
            "mobile_swipe",
            "mobile_type",
            "mobile_back",
            "mobile_home",
            "mobile_open_app",
        ] {
            assert_eq!(
                registry.get(id).unwrap().availability,
                ToolAvailability::Unavailable
            );
        }
    }
}
