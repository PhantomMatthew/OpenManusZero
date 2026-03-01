//! LLM types and data structures

use crate::schema::{Message, ToolCall};
use crate::tool::ToolDefinition;
use serde::{Deserialize, Serialize};

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmUsage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u64,
    /// Number of tokens in the completion
    pub completion_tokens: u64,
    /// Total tokens used
    pub total_tokens: u64,
}

/// Tool choice options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ToolChoice {
    None,
    #[default]
    Auto,
    Required,
}

/// LLM request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    /// Model to use
    pub model: String,
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Maximum tokens in the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// Sampling temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Tools available for the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// Tool choice strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

impl LlmRequest {
    /// Create a new LLM request
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            max_tokens: None,
            temperature: None,
            tools: None,
            tool_choice: None,
            stream: None,
            stop: None,
        }
    }

    /// Set the maximum tokens
    pub fn with_max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the tools
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the tool choice
    pub fn with_tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    /// Enable streaming
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Add a message
    pub fn add_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add a system message at the beginning
    pub fn with_system_message(mut self, content: impl Into<String>) -> Self {
        self.messages.insert(0, Message::system(content));
        self
    }
}

/// LLM response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponseMessage {
    /// Message role
    pub role: String,
    /// Message content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// LLM response choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponseChoice {
    /// Index of the choice
    pub index: u32,
    /// The message
    pub message: LlmResponseMessage,
    /// Finish reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// Response ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Model used
    pub model: String,
    /// Response choices
    pub choices: Vec<LlmResponseChoice>,
    /// Token usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<LlmUsage>,
}

impl LlmResponse {
    /// Get the first message content
    pub fn content(&self) -> Option<&str> {
        self.choices.first()?.message.content.as_deref()
    }

    /// Get the tool calls from the first choice
    pub fn tool_calls(&self) -> Option<&Vec<ToolCall>> {
        self.choices.first()?.message.tool_calls.as_ref()
    }

    /// Check if the response has tool calls
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls().is_some_and(|tc| !tc.is_empty())
    }
}

/// Streaming chunk from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChunk {
    /// Response ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Model used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Chunk choices
    pub choices: Vec<LlmChunkChoice>,
}

/// Choice in a streaming chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChunkChoice {
    /// Index
    pub index: u32,
    /// Delta content
    pub delta: LlmChunkDelta,
    /// Finish reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Delta content in a streaming chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChunkDelta {
    /// Role (only in first chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Content delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool calls delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

/// Tool call delta in streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDelta {
    /// Index
    pub index: u32,
    /// Tool call ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Type (always "function")
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub call_type: Option<String>,
    /// Function delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionDelta>,
}

/// Function delta in streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDelta {
    /// Function name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Arguments delta (JSON string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_usage_default() {
        let usage = LlmUsage::default();
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn test_llm_request_new() {
        let req = LlmRequest::new("gpt-4", vec![Message::user("Hello")]);
        assert_eq!(req.model, "gpt-4");
        assert_eq!(req.messages.len(), 1);
    }

    #[test]
    fn test_llm_request_builder() {
        let req = LlmRequest::new("gpt-4", vec![])
            .with_max_tokens(1000)
            .with_temperature(0.5)
            .with_system_message("You are helpful");

        assert_eq!(req.max_tokens, Some(1000));
        assert_eq!(req.temperature, Some(0.5));
        assert_eq!(req.messages.len(), 1);
        assert_eq!(
            req.messages[0].role,
            super::super::super::schema::Role::System
        );
    }

    #[test]
    fn test_llm_response_content() {
        let response = LlmResponse {
            id: Some("test-id".to_string()),
            model: "gpt-4".to_string(),
            choices: vec![LlmResponseChoice {
                index: 0,
                message: LlmResponseMessage {
                    role: "assistant".to_string(),
                    content: Some("Hello, world!".to_string()),
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(LlmUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };

        assert_eq!(response.content(), Some("Hello, world!"));
        assert!(!response.has_tool_calls());
    }

    #[test]
    fn test_llm_response_tool_calls() {
        let response = LlmResponse {
            id: None,
            model: "gpt-4".to_string(),
            choices: vec![LlmResponseChoice {
                index: 0,
                message: LlmResponseMessage {
                    role: "assistant".to_string(),
                    content: None,
                    tool_calls: Some(vec![ToolCall::new("id1", "bash", "{}")]),
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            usage: None,
        };

        assert!(response.has_tool_calls());
        let tool_calls = response.tool_calls().unwrap();
        assert_eq!(tool_calls.len(), 1);
    }

    #[test]
    fn test_tool_choice_default() {
        let choice = ToolChoice::default();
        assert!(matches!(choice, ToolChoice::Auto));
    }

    #[test]
    fn test_llm_request_serialization() {
        let req = LlmRequest::new("gpt-4", vec![Message::user("Hello")]).with_max_tokens(100);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"model\":\"gpt-4\""));
        assert!(json.contains("\"max_tokens\":100"));
    }
}
