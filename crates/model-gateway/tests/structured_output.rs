use model_gateway::*;
use serde_json::json;
#[test]
fn schema_capability_controls_native_format_and_never_sends_tools() {
    let mut request = UnifiedModelRequest::with_user_text("JSON only");
    request.structured_output = Some(StructuredOutputRequest {
        name: "next_action".into(),
        schema: json!({"type":"object","additionalProperties":false,"properties":{},"required":[]}),
    });
    request.max_output_tokens = Some(512);
    let native = ProviderCapabilities {
        chat_completions: true,
        structured_output: true,
        ..Default::default()
    };
    let wire = build_chat_request(&request, &native).unwrap();
    assert_eq!(wire["response_format"]["type"], "json_schema");
    assert_eq!(wire["response_format"]["json_schema"]["strict"], true);
    assert_eq!(wire["max_tokens"], 512);
    assert!(wire.get("tools").is_none());
    let json_only = ProviderCapabilities {
        chat_completions: true,
        json_mode: true,
        ..Default::default()
    };
    assert_eq!(
        build_chat_request(&request, &json_only).unwrap()["response_format"]["type"],
        "json_object"
    );
    assert!(
        build_chat_request(
            &request,
            &ProviderCapabilities {
                chat_completions: true,
                ..Default::default()
            }
        )
        .unwrap()
        .get("response_format")
        .is_none()
    );
}

async fn serve(
    response: String,
    delay: std::time::Duration,
) -> (String, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = vec![];
        let mut buffer = [0; 4096];
        loop {
            let n = socket.read(&mut buffer).await.unwrap();
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..n]);
            if let Some(pos) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&bytes[..pos]);
                let len = head
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|n| n.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if bytes.len() >= pos + 4 + len {
                    break;
                }
            }
        }
        tokio::time::sleep(delay).await;
        let _ = socket.write_all(response.as_bytes()).await;
        String::from_utf8(bytes).unwrap()
    });
    (url, handle)
}
fn request() -> UnifiedModelRequest {
    let mut r = UnifiedModelRequest::with_user_text("JSON only");
    r.structured_output = Some(StructuredOutputRequest {
        name: "decision".into(),
        schema: json!({"type":"object","properties":{},"required":[],"additionalProperties":false}),
    });
    r.timeout_ms = Some(1000);
    r.max_response_bytes = Some(512);
    r
}
fn http(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}
#[tokio::test]
async fn real_gateway_sends_native_schema_and_normalizes_one_text_reply() {
    let (url, server) = serve(
        http(r#"{"choices":[{"message":{"content":"{}"},"finish_reason":"stop"}]}"#),
        std::time::Duration::ZERO,
    )
    .await;
    let adapter = OpenAiCompatibleChatAdapter::new(
        url,
        ProviderCapabilities {
            chat_completions: true,
            structured_output: true,
            ..Default::default()
        },
    );
    let events = adapter.complete(&request(), None).await.unwrap();
    assert!(events.contains(&ModelEvent::MessageDelta { text: "{}".into() }));
    let wire = server.await.unwrap();
    let body: serde_json::Value =
        serde_json::from_str(wire.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["response_format"]["json_schema"]["strict"], true);
}
#[tokio::test]
async fn gateway_rejects_truncated_refusal_multiple_choices_and_tool_output() {
    for body in [
        r#"{"choices":[{"message":{"content":"{}"},"finish_reason":"length"}]}"#,
        r#"{"choices":[{"message":{"content":"{}","refusal":"denied"},"finish_reason":"stop"}]}"#,
        r#"{"choices":[{"message":{"content":"{}"},"finish_reason":"stop"},{"message":{"content":"{}"},"finish_reason":"stop"}]}"#,
        r#"{"choices":[{"message":{"content":"{}","tool_calls":[]},"finish_reason":"stop"}]}"#,
    ] {
        let (url, server) = serve(http(body), std::time::Duration::ZERO).await;
        let adapter = OpenAiCompatibleChatAdapter::new(
            url,
            ProviderCapabilities {
                chat_completions: true,
                ..Default::default()
            },
        );
        assert!(adapter.complete(&request(), None).await.is_err());
        server.await.unwrap();
    }
}
#[tokio::test]
async fn gateway_bounds_chunked_response_without_content_length() {
    let body = "x".repeat(600);
    let response = format!(
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{body}\r\n0\r\n\r\n",
        body.len()
    );
    let (url, server) = serve(response, std::time::Duration::ZERO).await;
    let adapter = OpenAiCompatibleChatAdapter::new(
        url,
        ProviderCapabilities {
            chat_completions: true,
            ..Default::default()
        },
    );
    assert!(matches!(
        adapter.complete(&request(), None).await,
        Err(ModelError::MalformedResponse(_))
    ));
    server.await.unwrap();
}
#[tokio::test]
async fn gateway_times_out_and_never_returns_provider_error_body() {
    let (url,server)=serve("HTTP/1.1 500 Internal Server Error\r\nContent-Length: 18\r\nConnection: close\r\n\r\nprivate test value".into(),std::time::Duration::ZERO).await;
    let adapter = OpenAiCompatibleChatAdapter::new(
        url,
        ProviderCapabilities {
            chat_completions: true,
            ..Default::default()
        },
    );
    let e = adapter.complete(&request(), None).await.unwrap_err();
    assert!(!e.to_string().contains("private test value"));
    server.await.unwrap();
    let (url, server) = serve(http("{}"), std::time::Duration::from_millis(80)).await;
    let adapter = OpenAiCompatibleChatAdapter::new(
        url,
        ProviderCapabilities {
            chat_completions: true,
            ..Default::default()
        },
    );
    let mut r = request();
    r.timeout_ms = Some(5);
    assert_eq!(
        adapter.complete(&r, None).await.unwrap_err(),
        ModelError::Timeout
    );
    server.await.unwrap();
}
