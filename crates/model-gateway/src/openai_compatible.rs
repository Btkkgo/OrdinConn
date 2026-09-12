use crate::{
    ModelError, ModelEvent, ModelProviderAdapter, ModelRole, ProviderCapabilities,
    UnifiedModelRequest,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub fn build_chat_request(
    request: &UnifiedModelRequest,
    capabilities: &ProviderCapabilities,
) -> Result<Value, ModelError> {
    if !capabilities.chat_completions {
        return Err(ModelError::UnsupportedCapability("chatCompletions"));
    }
    let messages: Vec<Value> = request
        .messages
        .iter()
        .map(|message| {
            json!({
                "role": match message.role {
                    ModelRole::System => "system",
                    ModelRole::User => "user",
                    ModelRole::Assistant => "assistant",
                    ModelRole::Tool => "tool",
                },
                "content": message.content,
            })
        })
        .collect();
    let mut body = json!({
        "model": request.model,
        "messages": messages,
        "temperature": request.temperature,
    });
    if capabilities.tool_calling && !request.tools.is_empty() {
        body["tools"] = Value::Array(
            request
                .tools
                .iter()
                .map(|tool| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": tool.input_schema,
                        }
                    })
                })
                .collect(),
        );
    }
    Ok(body)
}

pub fn normalize_chat_completion(value: &Value) -> Result<Vec<ModelEvent>, ModelError> {
    let choice = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or_else(|| ModelError::MalformedResponse("missing choices[0]".into()))?;
    let message = choice
        .get("message")
        .ok_or_else(|| ModelError::MalformedResponse("missing message".into()))?;
    let mut events = Vec::new();
    if let Some(content) = message.get("content").and_then(Value::as_str)
        && !content.is_empty()
    {
        events.push(ModelEvent::MessageDelta {
            text: content.into(),
        });
    }
    if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
        for call in tool_calls {
            let id = required_string(call, "id")?;
            let function = call
                .get("function")
                .ok_or_else(|| ModelError::MalformedResponse("missing tool function".into()))?;
            let name = required_string(function, "name")?;
            let arguments_json = required_string(function, "arguments")?;
            events.push(ModelEvent::ToolCallStarted {
                id: id.clone(),
                name: name.clone(),
            });
            events.push(ModelEvent::ToolCallCompleted {
                id,
                name,
                arguments_json,
            });
        }
    }
    if let Some(usage) = value.get("usage") {
        events.push(ModelEvent::Usage {
            input_tokens: usage
                .get("prompt_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            output_tokens: usage
                .get("completion_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            total_tokens: usage
                .get("total_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        });
    }
    events.push(ModelEvent::Completed);
    Ok(events)
}

fn required_string(value: &Value, key: &str) -> Result<String, ModelError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ModelError::MalformedResponse(format!("missing {key}")))
}

#[derive(Default)]
pub struct ChatStreamNormalizer {
    calls: BTreeMap<u64, PendingToolCall>,
}

#[derive(Default)]
struct PendingToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl ChatStreamNormalizer {
    pub fn ingest(&mut self, value: &Value) -> Result<Vec<ModelEvent>, ModelError> {
        let choice = value
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .ok_or_else(|| ModelError::MalformedResponse("missing streaming choice".into()))?;
        let delta = choice.get("delta").cloned().unwrap_or_else(|| json!({}));
        let mut events = Vec::new();
        if let Some(content) = delta.get("content").and_then(Value::as_str)
            && !content.is_empty()
        {
            events.push(ModelEvent::MessageDelta {
                text: content.into(),
            });
        }
        if let Some(reasoning) = delta
            .get("reasoning_content")
            .or_else(|| delta.get("reasoning"))
            .and_then(Value::as_str)
            && !reasoning.is_empty()
        {
            events.push(ModelEvent::ReasoningDelta {
                text: reasoning.into(),
            });
        }
        if let Some(calls) = delta.get("tool_calls").and_then(Value::as_array) {
            for call in calls {
                let index = call.get("index").and_then(Value::as_u64).unwrap_or(0);
                let pending = self.calls.entry(index).or_default();
                if let Some(id) = call.get("id").and_then(Value::as_str) {
                    pending.id = id.into();
                }
                if let Some(function) = call.get("function") {
                    if let Some(name) = function.get("name").and_then(Value::as_str) {
                        pending.name.push_str(name);
                        if !pending.id.is_empty() {
                            events.push(ModelEvent::ToolCallStarted {
                                id: pending.id.clone(),
                                name: pending.name.clone(),
                            });
                        }
                    }
                    if let Some(arguments) = function.get("arguments").and_then(Value::as_str) {
                        pending.arguments.push_str(arguments);
                        events.push(ModelEvent::ToolCallDelta {
                            id: pending.id.clone(),
                            arguments_delta: arguments.into(),
                        });
                    }
                }
            }
        }
        if choice.get("finish_reason").and_then(Value::as_str) == Some("tool_calls") {
            for pending in self.calls.values() {
                events.push(ModelEvent::ToolCallCompleted {
                    id: pending.id.clone(),
                    name: pending.name.clone(),
                    arguments_json: pending.arguments.clone(),
                });
            }
        }
        Ok(events)
    }
}

pub struct OpenAiCompatibleChatAdapter {
    client: reqwest::Client,
    base_url: String,
    capabilities: ProviderCapabilities,
}

impl OpenAiCompatibleChatAdapter {
    pub fn new(base_url: impl Into<String>, capabilities: ProviderCapabilities) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            capabilities,
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    fn request_builder(&self, api_key: Option<&str>) -> reqwest::RequestBuilder {
        let builder = self.client.post(self.endpoint());
        if let Some(key) = api_key.filter(|key| !key.is_empty()) {
            builder.bearer_auth(key)
        } else {
            builder
        }
    }

    async fn validate_response(
        response: reqwest::Response,
    ) -> Result<reqwest::Response, ModelError> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "provider request failed".into());
        if status.as_u16() == 401 || status.as_u16() == 403 {
            Err(ModelError::Authentication)
        } else if status.as_u16() == 429 {
            Err(ModelError::RateLimited)
        } else {
            Err(ModelError::Provider(redact_provider_error(&message)))
        }
    }
}

#[async_trait]
impl ModelProviderAdapter for OpenAiCompatibleChatAdapter {
    fn capabilities(&self) -> &ProviderCapabilities {
        &self.capabilities
    }

    async fn complete(
        &self,
        request: &UnifiedModelRequest,
        api_key: Option<&str>,
    ) -> Result<Vec<ModelEvent>, ModelError> {
        let body = build_chat_request(request, &self.capabilities)?;
        let response = self
            .request_builder(api_key)
            .json(&body)
            .send()
            .await
            .map_err(normalize_reqwest_error)?;
        let response = Self::validate_response(response).await?;
        let value = response
            .json::<Value>()
            .await
            .map_err(|error| ModelError::MalformedResponse(error.to_string()))?;
        normalize_chat_completion(&value)
    }

    async fn stream(
        &self,
        request: &UnifiedModelRequest,
        api_key: Option<&str>,
        sender: tokio::sync::mpsc::Sender<ModelEvent>,
    ) -> Result<(), ModelError> {
        if !self.capabilities.streaming {
            return Err(ModelError::UnsupportedCapability("streaming"));
        }
        let mut body = build_chat_request(request, &self.capabilities)?;
        body["stream"] = Value::Bool(true);
        let response = self
            .request_builder(api_key)
            .json(&body)
            .send()
            .await
            .map_err(normalize_reqwest_error)?;
        let response = Self::validate_response(response).await?;
        let mut chunks = response.bytes_stream();
        let mut buffer = String::new();
        let mut normalizer = ChatStreamNormalizer::default();

        while let Some(chunk) = chunks.next().await {
            let bytes = chunk.map_err(normalize_reqwest_error)?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(newline) = buffer.find('\n') {
                let line = buffer[..newline].trim().to_owned();
                buffer.drain(..=newline);
                let Some(data) = line.strip_prefix("data:").map(str::trim) else {
                    continue;
                };
                if data == "[DONE]" {
                    let _ = sender.send(ModelEvent::Completed).await;
                    return Ok(());
                }
                if data.is_empty() {
                    continue;
                }
                let value: Value = serde_json::from_str(data)
                    .map_err(|error| ModelError::MalformedResponse(error.to_string()))?;
                for event in normalizer.ingest(&value)? {
                    if sender.send(event).await.is_err() {
                        return Ok(());
                    }
                }
            }
        }
        let _ = sender.send(ModelEvent::Completed).await;
        Ok(())
    }
}

fn normalize_reqwest_error(error: reqwest::Error) -> ModelError {
    if error.is_timeout() {
        ModelError::Timeout
    } else {
        ModelError::Network(error.without_url().to_string())
    }
}

fn redact_provider_error(message: &str) -> String {
    let shortened: String = message.chars().take(240).collect();
    if shortened.to_ascii_lowercase().contains("api key") {
        "provider rejected the request; credentials were redacted".into()
    } else {
        shortened
    }
}
