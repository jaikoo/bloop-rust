use serde::Serialize;
use crate::tracing_types::{SpanType, SpanStatus, TraceStatus};

#[derive(Debug, Clone, Serialize)]
pub struct Span {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    pub span_type: SpanType,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_to_first_token_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<SpanStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Span {
    pub fn new(span_type: SpanType, name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            parent_span_id: None,
            span_type,
            name: name.into(),
            model: None,
            provider: None,
            started_at: chrono_millis(),
            input_tokens: None,
            output_tokens: None,
            cost: None,
            latency_ms: None,
            time_to_first_token_ms: None,
            status: None,
            error_message: None,
            input: None,
            output: None,
            metadata: None,
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn input_text(mut self, input: impl Into<String>) -> Self {
        self.input = Some(input.into());
        self
    }

    pub fn parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_span_id = Some(parent_id.into());
        self
    }

    pub fn end(&mut self, status: SpanStatus) {
        self.latency_ms = Some(chrono_millis() - self.started_at);
        self.status = Some(status);
    }

    pub fn set_usage(&mut self, input_tokens: i64, output_tokens: i64, cost: f64) {
        self.input_tokens = Some(input_tokens);
        self.output_tokens = Some(output_tokens);
        self.cost = Some(cost);
    }

    pub fn set_output(&mut self, output: impl Into<String>) {
        self.output = Some(output.into());
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.status = Some(SpanStatus::Error);
        self.error_message = Some(message.into());
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Trace {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub status: TraceStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_version: Option<String>,
    pub started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    pub spans: Vec<Span>,
}

impl Trace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            session_id: None,
            user_id: None,
            status: TraceStatus::Running,
            input: None,
            output: None,
            metadata: None,
            prompt_name: None,
            prompt_version: None,
            started_at: chrono_millis(),
            ended_at: None,
            spans: Vec::new(),
        }
    }

    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }

    pub fn user_id(mut self, id: impl Into<String>) -> Self {
        self.user_id = Some(id.into());
        self
    }

    pub fn input_text(mut self, input: impl Into<String>) -> Self {
        self.input = Some(input.into());
        self
    }

    pub fn prompt_name(mut self, name: impl Into<String>) -> Self {
        self.prompt_name = Some(name.into());
        self
    }

    pub fn prompt_version(mut self, version: impl Into<String>) -> Self {
        self.prompt_version = Some(version.into());
        self
    }

    pub fn start_span(&mut self, span_type: SpanType, name: impl Into<String>) -> &mut Span {
        let span = Span::new(span_type, name);
        self.spans.push(span);
        self.spans.last_mut().unwrap()
    }

    pub fn end(&mut self, status: TraceStatus) {
        self.ended_at = Some(chrono_millis());
        self.status = status;
    }

    pub fn set_output(&mut self, output: impl Into<String>) {
        self.output = Some(output.into());
    }
}

fn chrono_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}
