//! Token counting utilities for LLM operations
//!
//! This module provides token counting functionality compatible with
//! OpenAI's tokenization, using the tiktoken-rs library.

use crate::schema::Message;
use tiktoken_rs::CoreBPE;

/// Token counting constants (matching Python version)
const BASE_MESSAGE_TOKENS: usize = 4;
const FORMAT_TOKENS: usize = 2;

/// Token counter for LLM operations
///
/// Uses tiktoken-rs to count tokens in text and messages,
/// compatible with OpenAI's tokenization.
#[derive(Debug, Clone)]
pub struct TokenCounter {
    bpe: CoreBPE,
    model: String,
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCounter {
    /// Create a new TokenCounter with the default encoder (cl100k_base)
    ///
    /// This is suitable for GPT-4, GPT-3.5-turbo, and text-embedding-ada-002.
    pub fn new() -> Self {
        Self::for_model("gpt-4")
    }

    /// Create a TokenCounter for a specific model
    ///
    /// If the model is not recognized, falls back to cl100k_base encoder.
    ///
    /// # Arguments
    /// * `model` - The model name (e.g., "gpt-4", "gpt-4o", "gpt-3.5-turbo")
    pub fn for_model(model: &str) -> Self {
        let bpe = tiktoken_rs::get_bpe_from_model(model)
            .unwrap_or_else(|_| tiktoken_rs::cl100k_base().unwrap());
        Self {
            bpe,
            model: model.to_string(),
        }
    }

    /// Get the model name this counter was created for
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Count tokens in a text string
    ///
    /// # Arguments
    /// * `text` - The text to count tokens for
    ///
    /// # Returns
    /// The number of tokens in the text
    pub fn count_text(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        self.bpe.encode_with_special_tokens(text).len()
    }

    /// Count tokens in a list of messages
    ///
    /// This follows the same calculation as the Python version:
    /// - Base format tokens (2)
    /// - Per message: base tokens (4) + role tokens + content tokens
    /// - Additional tokens for tool calls, name, tool_call_id
    ///
    /// # Arguments
    /// * `messages` - The messages to count tokens for
    ///
    /// # Returns
    /// The total number of tokens
    pub fn count_messages(&self, messages: &[Message]) -> usize {
        let mut total = FORMAT_TOKENS;

        for message in messages {
            let mut tokens = BASE_MESSAGE_TOKENS;

            // Add role tokens
            tokens += self.count_text(&message.role.to_string());

            // Add content tokens
            if let Some(content) = &message.content {
                tokens += self.count_text(content);
            }

            // Add tool calls tokens
            if let Some(tool_calls) = &message.tool_calls {
                for tool_call in tool_calls {
                    tokens += self.count_text(&tool_call.function.name);
                    tokens += self.count_text(&tool_call.function.arguments);
                }
            }

            // Add name tokens
            if let Some(name) = &message.name {
                tokens += self.count_text(name);
            }

            // Add tool_call_id tokens
            if let Some(tool_call_id) = &message.tool_call_id {
                tokens += self.count_text(tool_call_id);
            }

            total += tokens;
        }

        total
    }

    /// Estimate tokens for a streaming response
    ///
    /// For streaming responses, we don't get exact token counts from the API,
    /// so we estimate based on the text length.
    ///
    /// # Arguments
    /// * `text` - The response text
    ///
    /// # Returns
    /// Estimated number of tokens
    pub fn estimate_completion_tokens(&self, text: &str) -> usize {
        self.count_text(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Role, ToolCall};

    #[test]
    fn test_token_counter_new() {
        let counter = TokenCounter::new();
        assert_eq!(counter.model(), "gpt-4");
    }

    #[test]
    fn test_token_counter_for_model() {
        let counter = TokenCounter::for_model("gpt-4o");
        assert_eq!(counter.model(), "gpt-4o");

        // Unknown model should fallback to cl100k_base
        let counter = TokenCounter::for_model("unknown-model");
        assert_eq!(counter.model(), "unknown-model");
    }

    #[test]
    fn test_count_text_empty() {
        let counter = TokenCounter::new();
        assert_eq!(counter.count_text(""), 0);
    }

    #[test]
    fn test_count_text_simple() {
        let counter = TokenCounter::new();
        let tokens = counter.count_text("Hello, world!");
        assert!(tokens > 0);
        // "Hello, world!" is typically 4 tokens
        assert_eq!(tokens, 4);
    }

    #[test]
    fn test_count_text_longer() {
        let counter = TokenCounter::new();
        let text = "This is a longer piece of text that should have more tokens.";
        let tokens = counter.count_text(text);
        assert!(tokens > 5);
    }

    #[test]
    fn test_count_messages_single() {
        let counter = TokenCounter::new();
        let messages = vec![Message::user("Hello")];
        let tokens = counter.count_messages(&messages);

        // Should include format tokens (2) + message tokens
        // message = base (4) + role tokens (~1) + content tokens (~1)
        assert!(tokens > 5);
    }

    #[test]
    fn test_count_messages_multiple() {
        let counter = TokenCounter::new();
        let messages = vec![
            Message::system("You are a helpful assistant."),
            Message::user("Hello"),
        ];
        let tokens = counter.count_messages(&messages);
        assert!(tokens > 10);
    }

    #[test]
    fn test_count_messages_with_tool_calls() {
        let counter = TokenCounter::new();
        let tool_call = ToolCall::new("call_123", "bash", r#"{"command": "ls"}"#);
        let messages = vec![Message {
            role: Role::Assistant,
            content: None,
            tool_calls: Some(vec![tool_call]),
            name: None,
            tool_call_id: None,
            base64_image: None,
        }];
        let tokens = counter.count_messages(&messages);
        assert!(tokens > 5);
    }

    #[test]
    fn test_estimate_completion_tokens() {
        let counter = TokenCounter::new();
        let text = "This is a response from the LLM.";
        let tokens = counter.estimate_completion_tokens(text);
        assert_eq!(tokens, counter.count_text(text));
    }
}
