//! LLM client implementations
//!
//! This module provides the `LlmClient` trait and implementations for
//! interacting with LLM APIs.

use crate::error::LlmError;
use crate::llm::token::TokenCounter;
use crate::llm::{LlmChunk, LlmRequest, LlmResponse, ToolChoice};
use crate::schema::Message;
use crate::tool::ToolDefinition;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use std::time::Duration;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// Boxed stream type for LLM chunks
pub type LlmStream = Pin<Box<dyn Stream<Item = Result<LlmChunk, LlmError>> + Send>>;

/// LLM client trait
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Get the model name
    fn model(&self) -> &str;

    /// Get a completion (non-streaming)
    async fn completion(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError>;

    /// Get a streaming completion (optional, default returns error)
    async fn stream_completion(&self, _request: &LlmRequest) -> Result<LlmStream, LlmError> {
        Err(LlmError::StreamError("Streaming not supported".to_string()))
    }

    /// Get the token counter
    fn token_counter(&self) -> &TokenCounter;

    /// Convenience method: simple text conversation
    async fn ask(&self, prompt: &str) -> Result<String, LlmError> {
        let request = LlmRequest::new(self.model(), vec![Message::user(prompt)]);
        let response = self.completion(&request).await?;
        Ok(response.content().unwrap_or_default().to_string())
    }

    /// Convenience method: conversation with system message
    async fn ask_with_system(&self, system: &str, prompt: &str) -> Result<String, LlmError> {
        let request = LlmRequest::new(
            self.model(),
            vec![Message::system(system), Message::user(prompt)],
        );
        let response = self.completion(&request).await?;
        Ok(response.content().unwrap_or_default().to_string())
    }

    /// Convenience method: with tool calls
    async fn ask_with_tools(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        tool_choice: ToolChoice,
    ) -> Result<LlmResponse, LlmError> {
        let request = LlmRequest::new(self.model(), messages)
            .with_tools(tools)
            .with_tool_choice(tool_choice);
        self.completion(&request).await
    }
}

/// Mock LLM client for testing
pub struct MockLlmClient {
    model: String,
    responses: parking_lot::RwLock<Vec<LlmResponse>>,
    chunks: parking_lot::RwLock<Vec<LlmChunk>>,
    token_counter: TokenCounter,
}

impl Clone for MockLlmClient {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            responses: parking_lot::RwLock::new(self.responses.read().clone()),
            chunks: parking_lot::RwLock::new(self.chunks.read().clone()),
            token_counter: self.token_counter.clone(),
        }
    }
}

impl MockLlmClient {
    /// Create a new mock client
    pub fn new(model: impl Into<String>) -> Self {
        let model = model.into();
        let token_counter = TokenCounter::for_model(&model);
        Self {
            model,
            responses: parking_lot::RwLock::new(Vec::new()),
            chunks: parking_lot::RwLock::new(Vec::new()),
            token_counter,
        }
    }

    /// Add a response to the queue
    pub fn add_response(&self, response: LlmResponse) {
        self.responses.write().push(response);
    }

    /// Add a chunk to the queue
    pub fn add_chunk(&self, chunk: LlmChunk) {
        self.chunks.write().push(chunk);
    }

    /// Set a simple text response
    pub fn set_text_response(&self, text: impl Into<String>) {
        let response = LlmResponse {
            id: Some("mock-id".to_string()),
            model: self.model.clone(),
            choices: vec![crate::llm::LlmResponseChoice {
                index: 0,
                message: crate::llm::LlmResponseMessage {
                    role: "assistant".to_string(),
                    content: Some(text.into()),
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(crate::llm::LlmUsage {
                prompt_tokens: 10,
                completion_tokens: 10,
                total_tokens: 20,
            }),
        };
        self.add_response(response);
    }

    /// Set a tool call response
    pub fn set_tool_call_response(&self, tool_name: &str, arguments: &str) {
        use crate::schema::ToolCall;
        let response = LlmResponse {
            id: Some("mock-id".to_string()),
            model: self.model.clone(),
            choices: vec![crate::llm::LlmResponseChoice {
                index: 0,
                message: crate::llm::LlmResponseMessage {
                    role: "assistant".to_string(),
                    content: None,
                    tool_calls: Some(vec![ToolCall::new("call_123", tool_name, arguments)]),
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            usage: None,
        };
        self.add_response(response);
    }

    /// Clear all queued responses and chunks
    pub fn clear(&self) {
        self.responses.write().clear();
        self.chunks.write().clear();
    }
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn completion(&self, _request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let mut responses = self.responses.write();
        if responses.is_empty() {
            // Return a default response
            Ok(LlmResponse {
                id: Some("mock-id".to_string()),
                model: self.model.clone(),
                choices: vec![crate::llm::LlmResponseChoice {
                    index: 0,
                    message: crate::llm::LlmResponseMessage {
                        role: "assistant".to_string(),
                        content: Some("Mock response".to_string()),
                        tool_calls: None,
                    },
                    finish_reason: Some("stop".to_string()),
                }],
                usage: None,
            })
        } else {
            Ok(responses.remove(0))
        }
    }

    async fn stream_completion(&self, _request: &LlmRequest) -> Result<LlmStream, LlmError> {
        let chunks = self.chunks.read().clone();
        let stream = futures::stream::iter(chunks.into_iter().map(Ok));
        Ok(Box::pin(stream))
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn token_counter(&self) -> &TokenCounter {
        &self.token_counter
    }
}

/// HTTP-based LLM client (OpenAI-compatible API)
#[derive(Clone, Debug)]
pub struct HttpLlmClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    max_tokens: Option<u64>,
    temperature: Option<f32>,
    max_retries: u32,
    token_counter: TokenCounter,
}

impl HttpLlmClient {
    /// Create a new builder for HttpLlmClient
    pub fn builder() -> HttpLlmClientBuilder {
        HttpLlmClientBuilder::new()
    }

    /// Create a new HTTP LLM client (simple constructor)
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self::builder()
            .base_url(base_url)
            .api_key(api_key)
            .model(model)
            .build()
            .expect("Failed to build HttpLlmClient")
    }

    /// Create a new HTTP LLM client with custom reqwest client
    pub fn with_client(
        client: reqwest::Client,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let model = model.into();
        Self {
            client,
            base_url: base_url.into(),
            api_key: api_key.into(),
            token_counter: TokenCounter::for_model(&model),
            model,
            max_tokens: None,
            temperature: None,
            max_retries: 3,
        }
    }

    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Build the request URL
    fn build_url(&self, endpoint: &str) -> String {
        format!("{}/{}", self.base_url.trim_end_matches('/'), endpoint)
    }

    /// Apply default parameters to a request
    fn apply_defaults(&self, mut request: LlmRequest) -> LlmRequest {
        if request.max_tokens.is_none() && self.max_tokens.is_some() {
            request.max_tokens = self.max_tokens;
        }
        if request.temperature.is_none() && self.temperature.is_some() {
            request.temperature = self.temperature;
        }
        request
    }

    /// Internal completion implementation
    async fn completion_inner(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let url = self.build_url("chat/completions");
        let request = self.apply_defaults(request.clone());

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            response
                .json::<LlmResponse>()
                .await
                .map_err(|e| LlmError::InvalidResponse(e.to_string()))
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // Handle specific error codes
            match status.as_u16() {
                401 => Err(LlmError::AuthFailed(format!(
                    "Authentication failed: {}",
                    body
                ))),
                429 => Err(LlmError::RateLimitExceeded(format!("Rate limit: {}", body))),
                404 => Err(LlmError::ModelNotFound(format!(
                    "Model not found: {}",
                    body
                ))),
                400 if body.contains("context_length") => {
                    Err(LlmError::ContextLengthExceeded(body))
                }
                _ => Err(LlmError::ApiError(format!("HTTP {}: {}", status, body))),
            }
        }
    }
}

#[async_trait]
impl LlmClient for HttpLlmClient {
    async fn completion(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let mut attempt = 0;
        let max_retries = self.max_retries;

        loop {
            match self.completion_inner(request).await {
                Ok(response) => return Ok(response),
                Err(LlmError::RateLimitExceeded(_)) if attempt < max_retries => {
                    attempt += 1;
                    let delay = Duration::from_millis(100 * 2u64.pow(attempt));
                    tracing::warn!(
                        "Rate limit hit, retrying in {:?} (attempt {}/{})",
                        delay,
                        attempt,
                        max_retries
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(LlmError::NetworkError(_)) if attempt < max_retries => {
                    attempt += 1;
                    let delay = Duration::from_millis(500);
                    tracing::warn!(
                        "Network error, retrying in {:?} (attempt {}/{})",
                        delay,
                        attempt,
                        max_retries
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn stream_completion(&self, request: &LlmRequest) -> Result<LlmStream, LlmError> {
        let mut req = self.apply_defaults(request.clone());
        req.stream = Some(true);

        let url = self.build_url("chat/completions");

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!("HTTP {}: {}", status, body)));
        }

        // Create channel for streaming chunks
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        // Spawn task to process SSE stream
        tokio::spawn(async move {
            use futures::StreamExt;

            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        if let Ok(text) = std::str::from_utf8(&bytes) {
                            buffer.push_str(text);

                            // Process complete lines
                            while let Some(newline_pos) = buffer.find('\n') {
                                let line = buffer[..newline_pos].trim().to_string();
                                buffer = buffer[newline_pos + 1..].to_string();

                                if let Some(data) = line.strip_prefix("data: ") {
                                    if data == "[DONE]" {
                                        let _ = tx
                                            .send(Err(LlmError::StreamError("[DONE]".to_string())));
                                        return;
                                    }

                                    match serde_json::from_str::<LlmChunk>(data) {
                                        Ok(chunk) => {
                                            if tx.send(Ok(chunk)).is_err() {
                                                return; // Receiver dropped
                                            }
                                        }
                                        Err(e) => {
                                            tracing::debug!(
                                                "Failed to parse chunk: {} - {}",
                                                e,
                                                data
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(LlmError::NetworkError(e.to_string())));
                        return;
                    }
                }
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn token_counter(&self) -> &TokenCounter {
        &self.token_counter
    }
}

/// Builder for HttpLlmClient
pub struct HttpLlmClientBuilder {
    base_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    max_tokens: Option<u64>,
    temperature: Option<f32>,
    max_retries: u32,
    timeout: Duration,
}

impl Default for HttpLlmClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpLlmClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            base_url: None,
            api_key: None,
            model: None,
            max_tokens: None,
            temperature: None,
            max_retries: 3,
            timeout: Duration::from_secs(60),
        }
    }

    /// Create builder from environment variables
    ///
    /// Reads:
    /// - `OPENAI_API_KEY` or `API_KEY` for the API key
    /// - `OPENAI_BASE_URL` or `BASE_URL` for the base URL (defaults to OpenAI API)
    /// - `OPENAI_MODEL` or `MODEL` for the model name (defaults to gpt-4)
    pub fn from_env() -> Self {
        let api_key = std::env::var("OPENAI_API_KEY")
            .or_else(|_| std::env::var("API_KEY"))
            .unwrap_or_default();

        let base_url = std::env::var("OPENAI_BASE_URL")
            .or_else(|_| std::env::var("BASE_URL"))
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let model = std::env::var("OPENAI_MODEL")
            .or_else(|_| std::env::var("MODEL"))
            .unwrap_or_else(|_| "gpt-4".to_string());

        Self::new().base_url(base_url).api_key(api_key).model(model)
    }

    /// Set the base URL
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the API key
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the model name
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the maximum tokens for responses
    pub fn max_tokens(mut self, tokens: u64) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Set the temperature
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set the maximum number of retries
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set the request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Build the HttpLlmClient
    pub fn build(self) -> Result<HttpLlmClient, LlmError> {
        let base_url = self
            .base_url
            .ok_or_else(|| LlmError::BuilderError("base_url is required".to_string()))?;
        let api_key = self
            .api_key
            .ok_or_else(|| LlmError::BuilderError("api_key is required".to_string()))?;
        let model = self
            .model
            .ok_or_else(|| LlmError::BuilderError("model is required".to_string()))?;

        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| LlmError::BuilderError(e.to_string()))?;

        Ok(HttpLlmClient {
            client,
            base_url,
            api_key,
            token_counter: TokenCounter::for_model(&model),
            model,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            max_retries: self.max_retries,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_llm_client_completion() {
        let client = MockLlmClient::new("gpt-4");
        client.set_text_response("Hello, world!");

        let request = LlmRequest::new("gpt-4", vec![Message::user("Hi")]);
        let response = client.completion(&request).await.unwrap();

        assert_eq!(response.content(), Some("Hello, world!"));
    }

    #[tokio::test]
    async fn test_mock_llm_client_queued_responses() {
        let client = MockLlmClient::new("gpt-4");
        client.set_text_response("First response");
        client.set_text_response("Second response");

        let request = LlmRequest::new("gpt-4", vec![Message::user("Hi")]);

        let response1 = client.completion(&request).await.unwrap();
        assert_eq!(response1.content(), Some("First response"));

        let response2 = client.completion(&request).await.unwrap();
        assert_eq!(response2.content(), Some("Second response"));
    }

    #[tokio::test]
    async fn test_mock_llm_client_default_response() {
        let client = MockLlmClient::new("gpt-4");

        let request = LlmRequest::new("gpt-4", vec![Message::user("Hi")]);
        let response = client.completion(&request).await.unwrap();

        assert_eq!(response.content(), Some("Mock response"));
    }

    #[test]
    fn test_http_llm_client_new() {
        let client = HttpLlmClient::new("https://api.openai.com/v1", "test-key", "gpt-4");
        assert_eq!(client.model(), "gpt-4");
        assert_eq!(client.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn test_http_llm_client_build_url() {
        let client = HttpLlmClient::new("https://api.openai.com/v1/", "test-key", "gpt-4");
        assert_eq!(
            client.build_url("chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_http_llm_client_builder() {
        let client = HttpLlmClient::builder()
            .base_url("https://api.openai.com/v1")
            .api_key("test-key")
            .model("gpt-4o")
            .max_tokens(4096)
            .temperature(0.7)
            .max_retries(5)
            .build()
            .unwrap();

        assert_eq!(client.model(), "gpt-4o");
        assert_eq!(client.max_tokens, Some(4096));
        assert_eq!(client.temperature, Some(0.7));
        assert_eq!(client.max_retries, 5);
    }

    #[test]
    fn test_http_llm_client_builder_missing_fields() {
        let result = HttpLlmClient::builder().build();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LlmError::BuilderError(_)));
    }

    #[tokio::test]
    async fn test_mock_llm_client_model() {
        let client = MockLlmClient::new("gpt-4-turbo");
        assert_eq!(client.model(), "gpt-4-turbo");
    }

    #[tokio::test]
    async fn test_mock_llm_client_clear() {
        let client = MockLlmClient::new("gpt-4");
        client.set_text_response("Response 1");
        client.set_text_response("Response 2");
        client.clear();

        let request = LlmRequest::new("gpt-4", vec![Message::user("Hi")]);
        let response = client.completion(&request).await.unwrap();
        assert_eq!(response.content(), Some("Mock response")); // Default response
    }

    #[tokio::test]
    async fn test_mock_llm_client_tool_call() {
        let client = MockLlmClient::new("gpt-4");
        client.set_tool_call_response("bash", r#"{"command": "ls"}"#);

        let request = LlmRequest::new("gpt-4", vec![Message::user("List files")]);
        let response = client.completion(&request).await.unwrap();

        assert!(response.has_tool_calls());
        let tool_calls = response.tool_calls().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].function.name, "bash");
    }

    #[tokio::test]
    async fn test_ask_convenience_method() {
        let client = MockLlmClient::new("gpt-4");
        client.set_text_response("Rust is a systems programming language.");

        let answer = client.ask("What is Rust?").await.unwrap();
        assert_eq!(answer, "Rust is a systems programming language.");
    }

    #[tokio::test]
    async fn test_ask_with_system() {
        let client = MockLlmClient::new("gpt-4");
        client.set_text_response("Hello! I'm a helpful assistant.");

        let answer = client
            .ask_with_system("You are a helpful assistant.", "Hello!")
            .await
            .unwrap();
        assert_eq!(answer, "Hello! I'm a helpful assistant.");
    }

    #[test]
    fn test_token_counter_in_client() {
        let client = MockLlmClient::new("gpt-4");
        assert_eq!(client.token_counter().model(), "gpt-4");

        let http_client = HttpLlmClient::new("https://api.openai.com/v1", "test-key", "gpt-4o");
        assert_eq!(http_client.token_counter().model(), "gpt-4o");
    }
}
