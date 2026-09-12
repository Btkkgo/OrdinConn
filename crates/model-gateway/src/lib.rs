mod openai_compatible;

pub use openai_compatible::{
    ChatStreamNormalizer, OpenAiCompatibleChatAdapter, build_chat_request,
    normalize_chat_completion,
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilities {
    pub chat_completions: bool,
    pub responses: bool,
    pub streaming: bool,
    pub tool_calling: bool,
    pub reasoning: bool,
    pub vision: bool,
    pub structured_output: bool,
    pub json_mode: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnifiedMessage {
    pub role: ModelRole,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

impl ModelTool {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedModelRequest {
    pub model: String,
    pub messages: Vec<UnifiedMessage>,
    pub tools: Vec<ModelTool>,
    pub context: std::collections::BTreeMap<String, String>,
    pub temperature: f32,
}

impl UnifiedModelRequest {
    pub fn with_user_text(text: impl Into<String>) -> Self {
        Self {
            model: "mock-model".into(),
            messages: vec![UnifiedMessage {
                role: ModelRole::User,
                content: text.into(),
            }],
            tools: Vec::new(),
            context: Default::default(),
            temperature: 0.2,
        }
    }

    pub fn with_tool(mut self, tool: ModelTool) -> Self {
        self.tools.push(tool);
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModelEvent {
    MessageDelta {
        text: String,
    },
    ReasoningDelta {
        text: String,
    },
    ToolCallStarted {
        id: String,
        name: String,
    },
    ToolCallDelta {
        id: String,
        arguments_delta: String,
    },
    ToolCallCompleted {
        id: String,
        name: String,
        arguments_json: String,
    },
    Usage {
        input_tokens: u64,
        output_tokens: u64,
        total_tokens: u64,
    },
    Completed,
    Error {
        code: String,
        message: String,
        retryable: bool,
    },
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ModelError {
    #[error("provider does not support {0}")]
    UnsupportedCapability(&'static str),
    #[error("provider authentication failed")]
    Authentication,
    #[error("provider rate limited the request")]
    RateLimited,
    #[error("provider request timed out")]
    Timeout,
    #[error("provider returned malformed data: {0}")]
    MalformedResponse(String),
    #[error("provider network error: {0}")]
    Network(String),
    #[error("provider error: {0}")]
    Provider(String),
}

#[async_trait]
pub trait ModelProviderAdapter: Send + Sync {
    fn capabilities(&self) -> &ProviderCapabilities;
    async fn complete(
        &self,
        request: &UnifiedModelRequest,
        api_key: Option<&str>,
    ) -> Result<Vec<ModelEvent>, ModelError>;
    async fn stream(
        &self,
        request: &UnifiedModelRequest,
        api_key: Option<&str>,
        sender: tokio::sync::mpsc::Sender<ModelEvent>,
    ) -> Result<(), ModelError>;
}

/// Reserved V0.1 boundary for a future OpenAI Responses implementation.
/// It deliberately advertises no capability until the adapter is implemented.
pub struct OpenAiResponsesAdapter {
    capabilities: ProviderCapabilities,
}

impl OpenAiResponsesAdapter {
    pub fn reserved() -> Self {
        Self {
            capabilities: ProviderCapabilities::default(),
        }
    }
}

#[async_trait]
impl ModelProviderAdapter for OpenAiResponsesAdapter {
    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn complete(
        &self,
        _request: &UnifiedModelRequest,
        _api_key: Option<&str>,
    ) -> Result<Vec<ModelEvent>, ModelError> {
        Err(ModelError::UnsupportedCapability("responses"))
    }

    async fn stream(
        &self,
        _request: &UnifiedModelRequest,
        _api_key: Option<&str>,
        _sender: tokio::sync::mpsc::Sender<ModelEvent>,
    ) -> Result<(), ModelError> {
        Err(ModelError::UnsupportedCapability("responses"))
    }
}

pub struct MockModelAdapter {
    capabilities: ProviderCapabilities,
}

impl Default for MockModelAdapter {
    fn default() -> Self {
        Self {
            capabilities: ProviderCapabilities {
                chat_completions: true,
                streaming: true,
                tool_calling: true,
                ..Default::default()
            },
        }
    }
}

#[async_trait]
impl ModelProviderAdapter for MockModelAdapter {
    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn complete(
        &self,
        request: &UnifiedModelRequest,
        _api_key: Option<&str>,
    ) -> Result<Vec<ModelEvent>, ModelError> {
        let asset = request
            .context
            .get("asset")
            .map(String::as_str)
            .unwrap_or("the selected asset");
        Ok(vec![
            ModelEvent::MessageDelta {
                text: format!(
                    "Mock Model: {asset} is in watch status because the linked evidence remains directional but limited to demonstration sources. Review risks and invalidation before any proposal."
                ),
            },
            ModelEvent::Completed,
        ])
    }

    async fn stream(
        &self,
        request: &UnifiedModelRequest,
        api_key: Option<&str>,
        sender: tokio::sync::mpsc::Sender<ModelEvent>,
    ) -> Result<(), ModelError> {
        for event in self.complete(request, api_key).await? {
            if sender.send(event).await.is_err() {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_non_streaming_chat_completion() {
        let events = normalize_chat_completion(&json!({
            "choices": [{
                "message": {
                    "content": "Evidence supports a watch stance.",
                    "tool_calls": [{
                        "id": "call-1",
                        "type": "function",
                        "function": {"name": "read_evidence", "arguments": "{\"signalId\":\"sig-1\"}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 8, "total_tokens": 18}
        })).unwrap();

        assert!(events.contains(&ModelEvent::MessageDelta {
            text: "Evidence supports a watch stance.".into()
        }));
        assert!(events.contains(&ModelEvent::ToolCallCompleted {
            id: "call-1".into(),
            name: "read_evidence".into(),
            arguments_json: "{\"signalId\":\"sig-1\"}".into(),
        }));
        assert!(events.contains(&ModelEvent::Usage {
            input_tokens: 10,
            output_tokens: 8,
            total_tokens: 18
        }));
        assert!(events.contains(&ModelEvent::Completed));
    }

    #[test]
    fn assembles_streamed_tool_call_deltas() {
        let mut normalizer = ChatStreamNormalizer::default();
        let first = normalizer.ingest(&json!({"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call-1","function":{"name":"read_signal","arguments":"{\"id\":"}}]}}]})).unwrap();
        let second = normalizer.ingest(&json!({"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"sig-1\"}"}}]},"finish_reason":"tool_calls"}]})).unwrap();

        assert!(first.contains(&ModelEvent::ToolCallStarted {
            id: "call-1".into(),
            name: "read_signal".into()
        }));
        assert!(second.contains(&ModelEvent::ToolCallCompleted {
            id: "call-1".into(),
            name: "read_signal".into(),
            arguments_json: "{\"id\":\"sig-1\"}".into(),
        }));
    }

    #[test]
    fn unsupported_tool_calling_is_removed_from_request() {
        let request = UnifiedModelRequest::with_user_text("Explain the signal").with_tool(
            ModelTool::new("read_signal", "Read one signal", json!({"type":"object"})),
        );
        let capabilities = ProviderCapabilities {
            chat_completions: true,
            ..Default::default()
        };
        let wire = build_chat_request(&request, &capabilities).unwrap();
        assert!(wire.get("tools").is_none());
    }

    #[tokio::test]
    async fn responses_adapter_boundary_is_reserved_but_unavailable() {
        let adapter = OpenAiResponsesAdapter::reserved();
        assert!(!adapter.capabilities().responses);
        let error = adapter
            .complete(&UnifiedModelRequest::with_user_text("hello"), None)
            .await
            .unwrap_err();
        assert_eq!(error, ModelError::UnsupportedCapability("responses"));
    }
}
