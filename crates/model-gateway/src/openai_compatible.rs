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
    if let Some(output) = &request.structured_output {
        if capabilities.structured_output {
            body["response_format"] = json!({"type":"json_schema","json_schema":{"name":output.name,"strict":true,"schema":output.schema}});
        } else if capabilities.json_mode {
            body["response_format"] = json!({"type":"json_object"});
        }
    }
    if let Some(limit) = request.max_output_tokens {
        body["max_tokens"] = json!(limit);
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
        let retry_after_ms = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| parse_retry_after(value, chrono::Utc::now()));
        // Never surface raw bodies: even error.code/message can echo secrets or prompts.
        // Read a bounded envelope and project only recognized public diagnostics.
        let mut bytes = Vec::new();
        let mut chunks = response.bytes_stream();
        while let Some(chunk) = chunks.next().await {
            let Ok(chunk) = chunk else { break };
            if bytes.len() + chunk.len() > 16_384 {
                bytes.clear();
                break;
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        // Gemini can return a single-element error array instead of an object.
        let envelope = match value.as_array() {
            Some(errors) if errors.len() == 1 => &errors[0],
            Some(_) => &Value::Null,
            None => &value,
        };
        let error = &envelope["error"];
        let code = match &error["code"] {
            Value::Number(n) if n.as_u64().is_some_and(|n| (100..=599).contains(&n)) => {
                n.to_string()
            }
            Value::String(s)
                if matches!(
                    s.as_str(),
                    "invalid_api_key"
                        | "invalid_request_error"
                        | "rate_limit_exceeded"
                        | "model_not_found"
                        | "insufficient_quota"
                        | "unsupported_parameter"
                        | "INVALID_ARGUMENT"
                        | "API_KEY_INVALID"
                        | "UNAUTHENTICATED"
                        | "PERMISSION_DENIED"
                        | "RESOURCE_EXHAUSTED"
                        | "NOT_FOUND"
                        | "FAILED_PRECONDITION"
                        | "INTERNAL"
                        | "UNAVAILABLE"
                        | "DEADLINE_EXCEEDED"
                ) =>
            {
                s.clone()
            }
            Value::Null => "none".into(),
            _ => "[REDACTED]".into(),
        };
        let message = match error["message"].as_str() {
            Some("Missing or invalid Authorization header.") => {
                "Missing or invalid Authorization header."
            }
            Some("User location is not supported for the API use.") => {
                "User location is not supported for the API use."
            }
            Some("API key not valid. Please pass a valid API key.") => {
                "API key not valid. Please pass a valid API key."
            }
            Some(
                "Request had invalid authentication credentials. Expected OAuth 2 access token, login cookie or other valid authentication credential.",
            ) => "Request had invalid authentication credentials.",
            Some("The caller does not have permission") => "The caller does not have permission",
            Some("Request contains an invalid argument.") => {
                "Request contains an invalid argument."
            }
            Some("Resource has been exhausted (e.g. check quota).") => {
                "Resource has been exhausted (e.g. check quota)."
            }
            Some("Internal error encountered.") => "Internal error encountered.",
            Some("The model is overloaded. Please try again later.") => {
                "The model is overloaded. Please try again later."
            }
            _ => "[REDACTED] unrecognized provider message",
        };
        Err(ModelError::HttpRejected {
            status: status.as_u16(),
            code,
            message: message.into(),
            retry_after_ms,
        })
    }
}

/// Only an interval is retained, never raw header text. Bound before arithmetic/sleep.
fn parse_retry_after(value: &str, now: chrono::DateTime<chrono::Utc>) -> Option<u64> {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 {
        return None;
    }
    if value.bytes().all(|c| c.is_ascii_digit()) {
        let seconds = value.bytes().fold(0u64, |n, c| {
            n.saturating_mul(10).saturating_add((c - b'0') as u64)
        });
        return Some(seconds.saturating_mul(1000).min(10_000));
    }
    let date = chrono::DateTime::parse_from_rfc2822(value).ok()?;
    Some(
        date.signed_duration_since(now)
            .num_milliseconds()
            .clamp(0, 10_000) as u64,
    )
}

#[cfg(test)]
mod retry_after_tests {
    use super::*;
    #[test]
    fn delta_seconds_dates_and_excessive_values_are_bounded_without_retaining_headers() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-01T00:00:00Z")
            .unwrap()
            .to_utc();
        assert_eq!(parse_retry_after("3", now), Some(3000));
        assert_eq!(
            parse_retry_after("Thu, 01 Oct 2026 00:00:04 GMT", now),
            Some(4000)
        );
        assert_eq!(
            parse_retry_after("Wed, 30 Sep 2026 00:00:00 GMT", now),
            Some(0)
        );
        assert_eq!(
            parse_retry_after("999999999999999999999999999999", now),
            Some(10_000)
        );
        for invalid in ["", "-1", "1.5", "not-a-date", "3\r\nInvalid: value"] {
            assert_eq!(parse_retry_after(invalid, now), None);
        }
    }
}

/// Safe transport facts only. No headers, body, model text or vendor metadata are retained.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct ChatCompletionDiagnostics {
    pub http_status: Option<u16>,
    pub retry_after_ms: Option<u64>,
    pub finish_reason: Option<String>,
    pub assistant_content_present: bool,
    pub assistant_content_bytes: usize,
}
impl OpenAiCompatibleChatAdapter {
    pub async fn complete_with_diagnostics(
        &self,
        request: &UnifiedModelRequest,
        api_key: Option<&str>,
    ) -> (
        Result<Vec<ModelEvent>, ModelError>,
        ChatCompletionDiagnostics,
    ) {
        let mut diagnostics = ChatCompletionDiagnostics::default();
        let outcome = self
            .complete_observed(request, api_key, &mut diagnostics)
            .await;
        (outcome, diagnostics)
    }
    async fn complete_observed(
        &self,
        request: &UnifiedModelRequest,
        api_key: Option<&str>,
        diagnostics: &mut ChatCompletionDiagnostics,
    ) -> Result<Vec<ModelEvent>, ModelError> {
        let body = build_chat_request(request, &self.capabilities)?;
        let response = self
            .request_builder(api_key)
            .json(&body)
            .timeout(std::time::Duration::from_millis(
                request.timeout_ms.unwrap_or(30_000).clamp(1, 120_000),
            ))
            .send()
            .await
            .map_err(normalize_reqwest_error)?;
        diagnostics.http_status = Some(response.status().as_u16());
        diagnostics.retry_after_ms = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| parse_retry_after(value, chrono::Utc::now()));
        let response = Self::validate_response(response).await?;
        let limit = request
            .max_response_bytes
            .unwrap_or(1_048_576)
            .clamp(1, 1_048_576);
        if response
            .content_length()
            .is_some_and(|size| size > limit as u64)
        {
            return Err(ModelError::MalformedResponse(
                "response exceeds byte limit".into(),
            ));
        }
        let mut bytes = Vec::new();
        let mut chunks = response.bytes_stream();
        while let Some(chunk) = chunks.next().await {
            let chunk = chunk.map_err(normalize_reqwest_error)?;
            if chunk.len() > limit.saturating_sub(bytes.len()) {
                return Err(ModelError::MalformedResponse(
                    "response exceeds byte limit".into(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| ModelError::MalformedResponse("invalid response JSON".into()))?;
        let choice = &value["choices"][0];
        diagnostics.finish_reason = choice["finish_reason"].as_str().map(|reason| match reason {
            "stop" | "length" | "tool_calls" | "function_call" | "content_filter" => {
                reason.to_owned()
            }
            _ => "[REDACTED]".to_owned(),
        });
        let content = choice["message"]["content"].as_str();
        diagnostics.assistant_content_present = content.is_some();
        diagnostics.assistant_content_bytes = content.map(str::len).unwrap_or(0);
        if request.structured_output.is_some() {
            let choices = value["choices"]
                .as_array()
                .ok_or_else(|| ModelError::MalformedResponse("missing choice".into()))?;
            if choices.len() != 1
                || choices[0]["finish_reason"] != "stop"
                || choices[0]["message"]
                    .get("refusal")
                    .is_some_and(|v| !v.is_null())
                || choices[0]["message"]
                    .get("tool_calls")
                    .is_some_and(|v| !v.is_null())
            {
                return Err(ModelError::MalformedResponse(
                    "structured response rejected".into(),
                ));
            }
        }
        normalize_chat_completion(&value)
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
        self.complete_observed(request, api_key, &mut ChatCompletionDiagnostics::default())
            .await
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
            .timeout(std::time::Duration::from_millis(
                request.timeout_ms.unwrap_or(30_000).clamp(1, 120_000),
            ))
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
