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
async fn retry_after_crosses_the_real_http_adapter_only_as_a_bounded_interval() {
    for (header, expected) in [
        ("3", Some(3000)),
        ("999999999999999999999", Some(10_000)),
        ("invalid", None),
    ] {
        let body = r#"{"error":{"code":429,"message":"fixture-private-value"}}"#;
        let response = format!(
            "HTTP/1.1 429 Too Many Requests\r\nRetry-After: {header}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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
        let (result, diagnostics) = adapter.complete_with_diagnostics(&request(), None).await;
        let error = result.unwrap_err();
        assert!(
            matches!(&error,ModelError::HttpRejected{status:429,retry_after_ms,..} if *retry_after_ms==expected)
        );
        assert_eq!(error.diagnostic_class(), "RATE_LIMITED");
        assert_eq!(diagnostics.retry_after_ms, expected);
        assert!(!format!("{error:?}{diagnostics:?}").contains("fixture-private-value"));
        server.await.unwrap();
    }
}

#[tokio::test]
async fn gemini_extra_metadata_is_ignored_and_diagnostics_never_retain_content() {
    for finish in ["stop", "length", "private-vendor-value"] {
        let payload = json!({"choices":[{"finish_reason":finish,"message":{
            "content":"{\"private_model_text\":true}",
            "extra_content":{"google":{"thought_signature":"private-signature"}}
        }}]});
        let (url, server) = serve(http(&payload.to_string()), std::time::Duration::ZERO).await;
        let adapter = OpenAiCompatibleChatAdapter::new(
            url,
            ProviderCapabilities {
                chat_completions: true,
                ..Default::default()
            },
        );
        let (result, diagnostics) = adapter.complete_with_diagnostics(&request(), None).await;
        assert_eq!(diagnostics.http_status, Some(200));
        assert!(diagnostics.assistant_content_present);
        assert_eq!(diagnostics.assistant_content_bytes, 27);
        assert_eq!(
            diagnostics.finish_reason.as_deref(),
            Some(if finish == "private-vendor-value" {
                "[REDACTED]"
            } else {
                finish
            })
        );
        let safe = serde_json::to_string(&diagnostics).unwrap();
        for forbidden in [
            "private_model_text",
            "private-signature",
            "extra_content",
            "private-vendor-value",
        ] {
            assert!(!safe.contains(forbidden));
        }
        if finish == "stop" {
            let events = result.unwrap();
            assert!(events.contains(&ModelEvent::MessageDelta {
                text: "{\"private_model_text\":true}".into()
            }));
        } else {
            assert!(matches!(result, Err(ModelError::MalformedResponse(_))));
        }
        server.await.unwrap();
    }
}

#[tokio::test]
async fn rejection_reports_status_code_and_only_safe_provider_message() {
    for (status, payload, expected) in [
        (
            400,
            json!([{"error":{"code":400,"message":"Missing or invalid Authorization header.","status":"INVALID_ARGUMENT"}}]),
            "Missing or invalid Authorization header.",
        ),
        (
            400,
            json!({"error":{"code":400,"message":"User location is not supported for the API use.","status":"FAILED_PRECONDITION"}}),
            "User location is not supported for the API use.",
        ),
        (
            401,
            json!({"error":{"code":"invalid_api_key","message":"Bearer fixture-credential; password=private-value"}}),
            "[REDACTED]",
        ),
        (
            429,
            json!({"error":{"code":"rate_limit_exceeded","message":"private prompt echoed here"}}),
            "[REDACTED]",
        ),
        (
            500,
            json!({"error":{"code":"private-value","message":"private-value"},"debug":"fixture-credential"}),
            "[REDACTED]",
        ),
        (
            503,
            json!({"error":{"code":503,"message":"private provider details","status":"UNAVAILABLE"}}),
            "[REDACTED]",
        ),
    ] {
        let response = http(&payload.to_string()).replacen("200 OK", &format!("{status} Error"), 1);
        let (url, server) = serve(response, std::time::Duration::ZERO).await;
        let adapter = OpenAiCompatibleChatAdapter::new(
            url,
            ProviderCapabilities {
                chat_completions: true,
                ..Default::default()
            },
        );
        let (result, diagnostics) = adapter
            .complete_with_diagnostics(&request(), Some("fixture-credential"))
            .await;
        assert_eq!(diagnostics.http_status, Some(status));
        assert_eq!(diagnostics.finish_reason, None);
        assert!(!diagnostics.assistant_content_present);
        let error = result.unwrap_err().to_string();
        assert!(
            error.contains(&format!("HTTP {status}")),
            "missing HTTP status"
        );
        assert!(error.contains(expected), "missing safe message");
        if status == 400 {
            assert!(error.contains("code=400"));
        }
        if status == 401 {
            assert!(error.contains("code=invalid_api_key"));
        }
        for private in [
            "fixture-credential",
            "private-value",
            "private prompt",
            "Bearer",
        ] {
            assert!(!error.contains(private), "untrusted data escaped");
        }
        server.await.unwrap();
    }
}

#[tokio::test]
async fn streaming_rejection_uses_the_same_safe_diagnostics() {
    let payload = json!({"error":{"code":"PERMISSION_DENIED","message":"The caller does not have permission"}});
    let response = http(&payload.to_string()).replacen("200 OK", "403 Forbidden", 1);
    let (url, server) = serve(response, std::time::Duration::ZERO).await;
    let adapter = OpenAiCompatibleChatAdapter::new(
        url,
        ProviderCapabilities {
            chat_completions: true,
            streaming: true,
            ..Default::default()
        },
    );
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
    let error = adapter
        .stream(&request(), None, sender)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("HTTP 403"));
    assert!(error.contains("code=PERMISSION_DENIED"));
    assert!(error.contains("The caller does not have permission"));
    assert!(receiver.recv().await.is_none());
    server.await.unwrap();
}

#[tokio::test]
async fn rejection_bounds_and_suppresses_unknown_or_oversized_bodies() {
    for payload in [
        "<html>private-value</html>".into(),
        json!({"error":{"code":"fixture-credential","message":"fixture-credential private-value"}}).to_string(),
        json!({"error":{"code":400,"message":"User location is not supported for the API use."},"padding":"x".repeat(17000)}).to_string(),
        json!([{"error":{"code":400,"message":"User location is not supported for the API use."}},{"error":{"message":"private-value"}}]).to_string(),
    ] {
        let response = http(&payload).replacen("200 OK", "502 Error", 1);
        let (url, server) = serve(response, std::time::Duration::ZERO).await;
        let adapter = OpenAiCompatibleChatAdapter::new(url, ProviderCapabilities { chat_completions: true, ..Default::default() });
        let error = adapter.complete(&request(), Some("fixture-credential")).await.unwrap_err().to_string();
        assert!(error.contains("HTTP 502"));
        assert!(error.contains("[REDACTED]"));
        assert!(!error.contains("fixture-credential"));
        assert!(!error.contains("private-value"));
        assert!(error.len() < 200);
        server.await.unwrap();
    }
}

#[tokio::test]
async fn minimal_chat_preserves_gemini_path_and_omits_optional_features() {
    let (url, server) = serve(
        http(r#"{"choices":[{"message":{"content":"ORDINCONN_OK"},"finish_reason":"stop"}]}"#),
        std::time::Duration::ZERO,
    )
    .await;
    let url = url.replace("/v1", "/v1beta/openai/");
    let adapter = OpenAiCompatibleChatAdapter::new(
        url,
        ProviderCapabilities {
            chat_completions: true,
            streaming: true,
            structured_output: true,
            json_mode: true,
            tool_calling: true,
            ..Default::default()
        },
    );
    let mut request = UnifiedModelRequest::with_user_text("Reply with exactly ORDINCONN_OK.");
    request.model = "gemini-3.6-flash".into();
    let events = adapter
        .complete(&request, Some("fixture-credential"))
        .await
        .unwrap();
    assert!(events.contains(&ModelEvent::MessageDelta {
        text: "ORDINCONN_OK".into()
    }));
    let wire = server.await.unwrap();
    assert!(wire.starts_with("POST /v1beta/openai/chat/completions HTTP/1.1"));
    let (head, body) = wire.split_once("\r\n\r\n").unwrap();
    assert!(
        head.to_ascii_lowercase()
            .contains("content-type: application/json")
    );
    assert!(
        head.to_ascii_lowercase()
            .contains("authorization: bearer fixture-credential")
    );
    let body: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(body.as_object().unwrap().len(), 3);
    assert_eq!(body["model"], "gemini-3.6-flash");
    assert_eq!(
        body["messages"],
        json!([{"role":"user","content":"Reply with exactly ORDINCONN_OK."}])
    );
    assert!((body["temperature"].as_f64().unwrap() - 0.2).abs() < 0.00001);
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
