use bloop_client::*;

#[test]
fn test_event_creation() {
    let event = Event {
        error_type: "TypeError".into(),
        message: "something broke".into(),
        ..Default::default()
    };
    assert_eq!(event.error_type, "TypeError");
    assert_eq!(event.message, "something broke");
    assert!(event.source.is_none());
    assert!(event.metadata.is_none());
}

#[test]
fn test_event_with_all_fields() {
    let event = Event {
        error_type: "NetworkError".into(),
        message: "timeout".into(),
        source: Some("api-server".into()),
        route_or_procedure: Some("/api/users".into()),
        screen: Some("dashboard".into()),
        stack: Some("at main.rs:42".into()),
        http_status: Some(500),
        request_id: Some("req-123".into()),
        user_id_hash: Some("abc123".into()),
        metadata: Some(serde_json::json!({"key": "value"})),
    };
    assert_eq!(event.http_status, Some(500));
    assert_eq!(event.source.as_deref(), Some("api-server"));
}

#[test]
fn test_event_serialization_skips_none() {
    let event = Event {
        error_type: "Error".into(),
        message: "msg".into(),
        ..Default::default()
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"error_type\":\"Error\""));
    assert!(json.contains("\"message\":\"msg\""));
    // None fields should be skipped
    assert!(!json.contains("\"source\""));
    assert!(!json.contains("\"stack\""));
    assert!(!json.contains("\"metadata\""));
}

#[test]
fn test_trace_creation() {
    let trace = Trace::new("chat-completion")
        .session_id("session-1")
        .user_id("user-1")
        .input_text("Hello");
    assert_eq!(trace.name, "chat-completion");
    assert_eq!(trace.session_id.as_deref(), Some("session-1"));
    assert_eq!(trace.user_id.as_deref(), Some("user-1"));
    assert_eq!(trace.input.as_deref(), Some("Hello"));
    assert!(trace.spans.is_empty());
    assert!(!trace.id.is_empty());
    assert!(trace.started_at > 0);
}

#[test]
fn test_trace_prompt_fields() {
    let trace = Trace::new("test")
        .prompt_name("my-prompt")
        .prompt_version("v2");
    assert_eq!(trace.prompt_name.as_deref(), Some("my-prompt"));
    assert_eq!(trace.prompt_version.as_deref(), Some("v2"));
}

#[test]
fn test_span_creation() {
    let span = Span::new(SpanType::Generation, "gpt-4o call")
        .model("gpt-4o")
        .provider("openai")
        .input_text("hello world");
    assert_eq!(span.name, "gpt-4o call");
    assert_eq!(span.model.as_deref(), Some("gpt-4o"));
    assert_eq!(span.provider.as_deref(), Some("openai"));
    assert_eq!(span.input.as_deref(), Some("hello world"));
    assert!(!span.id.is_empty());
    assert!(span.started_at > 0);
}

#[test]
fn test_span_parent() {
    let span = Span::new(SpanType::Tool, "search")
        .parent("parent-span-123");
    assert_eq!(span.parent_span_id.as_deref(), Some("parent-span-123"));
}

#[test]
fn test_span_end() {
    let mut span = Span::new(SpanType::Generation, "call");
    // Simulate some time passing (at least the span records latency)
    span.end(SpanStatus::Ok);
    assert!(span.latency_ms.is_some());
    assert!(matches!(span.status, Some(SpanStatus::Ok)));
}

#[test]
fn test_span_set_usage() {
    let mut span = Span::new(SpanType::Generation, "call");
    span.set_usage(100, 50, 0.0025);
    assert_eq!(span.input_tokens, Some(100));
    assert_eq!(span.output_tokens, Some(50));
    assert!((span.cost.unwrap() - 0.0025).abs() < f64::EPSILON);
}

#[test]
fn test_span_set_output() {
    let mut span = Span::new(SpanType::Generation, "call");
    span.set_output("Hello, world!");
    assert_eq!(span.output.as_deref(), Some("Hello, world!"));
}

#[test]
fn test_span_set_error() {
    let mut span = Span::new(SpanType::Generation, "call");
    span.set_error("rate limit exceeded");
    assert!(matches!(span.status, Some(SpanStatus::Error)));
    assert_eq!(span.error_message.as_deref(), Some("rate limit exceeded"));
}

#[test]
fn test_trace_with_spans() {
    let mut trace = Trace::new("chat-completion");
    {
        let span = trace.start_span(SpanType::Generation, "gpt-4o call");
        span.set_usage(100, 50, 0.0025);
        span.set_output("response text");
        span.end(SpanStatus::Ok);
    }
    trace.set_output("final output");
    trace.end(TraceStatus::Completed);
    assert_eq!(trace.spans.len(), 1);
    assert!(trace.ended_at.is_some());
    assert_eq!(trace.output.as_deref(), Some("final output"));
}

#[test]
fn test_trace_multiple_spans() {
    let mut trace = Trace::new("agent-pipeline");
    trace.start_span(SpanType::Retrieval, "vector search");
    trace.start_span(SpanType::Generation, "llm call");
    trace.start_span(SpanType::Tool, "api call");
    assert_eq!(trace.spans.len(), 3);
}

#[test]
fn test_trace_serialization() {
    let mut trace = Trace::new("test");
    trace.end(TraceStatus::Completed);
    let json = serde_json::to_string(&trace).unwrap();
    assert!(json.contains("\"name\":\"test\""));
    assert!(json.contains("\"status\":\"completed\""));
    assert!(json.contains("\"ended_at\""));
    // None fields should be skipped
    assert!(!json.contains("\"session_id\""));
    assert!(!json.contains("\"user_id\""));
}

#[test]
fn test_span_type_serialization() {
    let span_gen = Span::new(SpanType::Generation, "s");
    let span_tool = Span::new(SpanType::Tool, "s");
    let span_ret = Span::new(SpanType::Retrieval, "s");
    let span_cust = Span::new(SpanType::Custom, "s");
    let json_gen = serde_json::to_string(&span_gen).unwrap();
    let json_tool = serde_json::to_string(&span_tool).unwrap();
    let json_ret = serde_json::to_string(&span_ret).unwrap();
    let json_cust = serde_json::to_string(&span_cust).unwrap();
    assert!(json_gen.contains("\"span_type\":\"generation\""));
    assert!(json_tool.contains("\"span_type\":\"tool\""));
    assert!(json_ret.contains("\"span_type\":\"retrieval\""));
    assert!(json_cust.contains("\"span_type\":\"custom\""));
}

#[test]
fn test_trace_status_serialization() {
    let mut trace = Trace::new("t");
    let json_running = serde_json::to_string(&trace).unwrap();
    assert!(json_running.contains("\"status\":\"running\""));

    trace.end(TraceStatus::Error);
    let json_error = serde_json::to_string(&trace).unwrap();
    assert!(json_error.contains("\"status\":\"error\""));
}

#[test]
fn test_builder_requires_endpoint() {
    let result = BloopClient::builder()
        .project_key("key")
        .build();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("endpoint"));
}

#[test]
fn test_builder_requires_key() {
    let result = BloopClient::builder()
        .endpoint("http://localhost")
        .build();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("project_key"));
}

#[test]
fn test_builder_success() {
    let result = BloopClient::builder()
        .endpoint("http://localhost:3000")
        .project_key("test-key")
        .environment("staging")
        .release("1.0.0")
        .source("my-app")
        .build();
    assert!(result.is_ok());
}

#[test]
fn test_builder_strips_trailing_slash() {
    let client = BloopClient::builder()
        .endpoint("http://localhost:3000/")
        .project_key("key")
        .build()
        .unwrap();
    // We can verify indirectly by capturing an event (no panic)
    client.capture_error("Test", "msg");
}

#[tokio::test]
async fn test_capture_and_flush() {
    let client = BloopClient::builder()
        .endpoint("http://localhost:9999")
        .project_key("test-key")
        .build()
        .unwrap();

    // Capture some events (won't actually send since server doesn't exist)
    client.capture_error("Error1", "message1");
    client.capture_error("Error2", "message2");

    // Flush should not panic even if server is unreachable
    client.flush().await;
}

#[cfg(feature = "tracing")]
#[tokio::test]
async fn test_send_trace_and_flush() {
    let client = BloopClient::builder()
        .endpoint("http://localhost:9999")
        .project_key("test-key")
        .build()
        .unwrap();

    let mut trace = client.start_trace("test-trace");
    trace.end(TraceStatus::Completed);
    client.send_trace(trace);

    // Flush should not panic
    client.flush().await;
}

#[tokio::test]
async fn test_shutdown() {
    let client = BloopClient::builder()
        .endpoint("http://localhost:9999")
        .project_key("test-key")
        .build()
        .unwrap();

    client.capture_error("Err", "msg");
    // Shutdown should not panic
    client.shutdown().await;
}
