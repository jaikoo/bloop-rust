use std::sync::{Arc, Mutex};
use crate::event::{Event, IngestEvent};
use crate::signing;

#[cfg(feature = "tracing")]
use crate::tracing::Trace;

#[derive(Debug, Clone)]
pub struct BloopClientBuilder {
    endpoint: Option<String>,
    project_key: Option<String>,
    environment: String,
    release: String,
    source: String,
    max_buffer_size: usize,
}

impl BloopClientBuilder {
    pub fn new() -> Self {
        Self {
            endpoint: None,
            project_key: None,
            environment: "production".into(),
            release: String::new(),
            source: "rust".into(),
            max_buffer_size: 20,
        }
    }

    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn project_key(mut self, key: impl Into<String>) -> Self {
        self.project_key = Some(key.into());
        self
    }

    pub fn environment(mut self, env: impl Into<String>) -> Self {
        self.environment = env.into();
        self
    }

    pub fn release(mut self, release: impl Into<String>) -> Self {
        self.release = release.into();
        self
    }

    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    pub fn build(self) -> Result<BloopClient, String> {
        let endpoint = self.endpoint.ok_or("endpoint is required")?;
        let project_key = self.project_key.ok_or("project_key is required")?;

        let http = reqwest::Client::new();
        let error_buffer = Arc::new(Mutex::new(Vec::new()));

        #[cfg(feature = "tracing")]
        let trace_buffer = Arc::new(Mutex::new(Vec::<Trace>::new()));

        Ok(BloopClient {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            project_key,
            environment: self.environment,
            release: self.release,
            source: self.source,
            max_buffer_size: self.max_buffer_size,
            http,
            error_buffer,
            #[cfg(feature = "tracing")]
            trace_buffer,
        })
    }
}

impl std::fmt::Debug for BloopClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BloopClient")
            .field("endpoint", &self.endpoint)
            .field("environment", &self.environment)
            .field("source", &self.source)
            .finish()
    }
}

pub struct BloopClient {
    endpoint: String,
    project_key: String,
    environment: String,
    release: String,
    source: String,
    max_buffer_size: usize,
    http: reqwest::Client,
    error_buffer: Arc<Mutex<Vec<IngestEvent>>>,
    #[cfg(feature = "tracing")]
    trace_buffer: Arc<Mutex<Vec<Trace>>>,
}

impl BloopClient {
    pub fn builder() -> BloopClientBuilder {
        BloopClientBuilder::new()
    }

    /// Capture a structured error event.
    pub fn capture(&self, event: Event) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let ingest = IngestEvent {
            timestamp: now,
            source: event.source.unwrap_or_else(|| self.source.clone()),
            environment: self.environment.clone(),
            release: self.release.clone(),
            error_type: event.error_type,
            message: event.message,
            route_or_procedure: event.route_or_procedure,
            screen: event.screen,
            stack: event.stack,
            http_status: event.http_status,
            request_id: event.request_id,
            user_id_hash: event.user_id_hash,
            metadata: event.metadata,
        };

        let mut buf = self.error_buffer.lock().unwrap();
        buf.push(ingest);
        if buf.len() >= self.max_buffer_size {
            let batch = std::mem::take(&mut *buf);
            drop(buf);
            let client = self.http.clone();
            let endpoint = self.endpoint.clone();
            let key = self.project_key.clone();
            tokio::spawn(async move {
                let _ = send_error_batch(&client, &endpoint, &key, batch).await;
            });
        }
    }

    /// Capture an error type and message.
    pub fn capture_error(&self, error_type: impl Into<String>, message: impl Into<String>) {
        self.capture(Event {
            error_type: error_type.into(),
            message: message.into(),
            ..Default::default()
        });
    }

    #[cfg(feature = "tracing")]
    pub fn start_trace(&self, name: impl Into<String>) -> crate::tracing::Trace {
        crate::tracing::Trace::new(name)
    }

    #[cfg(feature = "tracing")]
    pub fn send_trace(&self, trace: Trace) {
        let mut buf = self.trace_buffer.lock().unwrap();
        buf.push(trace);
        if buf.len() >= self.max_buffer_size {
            let batch = std::mem::take(&mut *buf);
            drop(buf);
            let client = self.http.clone();
            let endpoint = self.endpoint.clone();
            let key = self.project_key.clone();
            tokio::spawn(async move {
                let _ = send_trace_batch(&client, &endpoint, &key, batch).await;
            });
        }
    }

    /// Flush all buffered events and traces.
    pub async fn flush(&self) {
        // Flush errors
        let errors = {
            let mut buf = self.error_buffer.lock().unwrap();
            std::mem::take(&mut *buf)
        };
        if !errors.is_empty() {
            let _ = send_error_batch(&self.http, &self.endpoint, &self.project_key, errors).await;
        }

        // Flush traces
        #[cfg(feature = "tracing")]
        {
            let traces = {
                let mut buf = self.trace_buffer.lock().unwrap();
                std::mem::take(&mut *buf)
            };
            if !traces.is_empty() {
                let _ = send_trace_batch(&self.http, &self.endpoint, &self.project_key, traces).await;
            }
        }
    }

    /// Flush and shutdown.
    pub async fn shutdown(&self) {
        self.flush().await;
    }
}

async fn send_error_batch(
    http: &reqwest::Client,
    endpoint: &str,
    key: &str,
    events: Vec<IngestEvent>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let body = serde_json::to_vec(&serde_json::json!({ "events": events }))?;
    let signature = signing::sign(key, &body);

    http.post(format!("{endpoint}/v1/ingest/batch"))
        .header("Content-Type", "application/json")
        .header("X-Signature", signature)
        .header("X-Project-Key", key)
        .body(body)
        .send()
        .await?;

    Ok(())
}

#[cfg(feature = "tracing")]
async fn send_trace_batch(
    http: &reqwest::Client,
    endpoint: &str,
    key: &str,
    traces: Vec<Trace>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let body = serde_json::to_vec(&serde_json::json!({ "traces": traces }))?;
    let signature = signing::sign(key, &body);

    http.post(format!("{endpoint}/v1/traces/batch"))
        .header("Content-Type", "application/json")
        .header("X-Signature", signature)
        .header("X-Project-Key", key)
        .body(body)
        .send()
        .await?;

    Ok(())
}
