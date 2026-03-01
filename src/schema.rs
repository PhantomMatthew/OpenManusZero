//! Core data structures for OpenManus

use serde::{Deserialize, Serialize};
use std::fmt;

/// Message role in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Role {
    System,
    #[default]
    User,
    Assistant,
    Tool,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// Agent execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AgentState {
    #[default]
    Idle,
    Running,
    Finished,
    Error,
}

/// Function call in a tool invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub arguments: String,
}

/// Represents a tool/function call in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: Function,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            call_type: "function".to_string(),
            function: Function {
                name: name.into(),
                arguments: arguments.into(),
            },
        }
    }
}

/// Represents a chat message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64_image: Option<String>,
}

impl Message {
    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: Some(content.into()),
            tool_calls: None,
            name: None,
            tool_call_id: None,
            base64_image: None,
        }
    }

    /// Create a user message with an image
    pub fn user_with_image(content: impl Into<String>, base64_image: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: Some(content.into()),
            tool_calls: None,
            name: None,
            tool_call_id: None,
            base64_image: Some(base64_image.into()),
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: Some(content.into()),
            tool_calls: None,
            name: None,
            tool_call_id: None,
            base64_image: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: Some(content.into()),
            tool_calls: None,
            name: None,
            tool_call_id: None,
            base64_image: None,
        }
    }

    /// Create an assistant message with tool calls
    pub fn assistant_with_tools(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content,
            tool_calls: Some(tool_calls),
            name: None,
            tool_call_id: None,
            base64_image: None,
        }
    }

    /// Create a tool result message
    pub fn tool_result(
        content: impl Into<String>,
        name: impl Into<String>,
        tool_call_id: impl Into<String>,
    ) -> Self {
        Self {
            role: Role::Tool,
            content: Some(content.into()),
            tool_calls: None,
            name: Some(name.into()),
            tool_call_id: Some(tool_call_id.into()),
            base64_image: None,
        }
    }

    /// Create a tool result message with an image
    pub fn tool_result_with_image(
        content: impl Into<String>,
        name: impl Into<String>,
        tool_call_id: impl Into<String>,
        base64_image: impl Into<String>,
    ) -> Self {
        Self {
            role: Role::Tool,
            content: Some(content.into()),
            tool_calls: None,
            name: Some(name.into()),
            tool_call_id: Some(tool_call_id.into()),
            base64_image: Some(base64_image.into()),
        }
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::user("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_display() {
        assert_eq!(Role::System.to_string(), "system");
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Assistant.to_string(), "assistant");
        assert_eq!(Role::Tool.to_string(), "tool");
    }

    #[test]
    fn test_role_serialize() {
        let role = Role::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");
    }

    #[test]
    fn test_role_deserialize() {
        let role: Role = serde_json::from_str("\"assistant\"").unwrap();
        assert_eq!(role, Role::Assistant);
    }

    #[test]
    fn test_message_user() {
        let msg = Message::user("Hello, world!");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, Some("Hello, world!".to_string()));
        assert!(msg.tool_calls.is_none());
    }

    #[test]
    fn test_message_system() {
        let msg = Message::system("You are a helpful assistant");
        assert_eq!(msg.role, Role::System);
        assert_eq!(msg.content, Some("You are a helpful assistant".to_string()));
    }

    #[test]
    fn test_message_assistant() {
        let msg = Message::assistant("I can help you with that");
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, Some("I can help you with that".to_string()));
    }

    #[test]
    fn test_message_tool_result() {
        let msg = Message::tool_result("Result content", "bash", "call_123");
        assert_eq!(msg.role, Role::Tool);
        assert_eq!(msg.content, Some("Result content".to_string()));
        assert_eq!(msg.name, Some("bash".to_string()));
        assert_eq!(msg.tool_call_id, Some("call_123".to_string()));
    }

    #[test]
    fn test_tool_call_new() {
        let tc = ToolCall::new("id_1", "bash", "{\"command\": \"ls\"}");
        assert_eq!(tc.id, "id_1");
        assert_eq!(tc.call_type, "function");
        assert_eq!(tc.function.name, "bash");
        assert_eq!(tc.function.arguments, "{\"command\": \"ls\"}");
    }

    #[test]
    fn test_message_serialize() {
        let msg = Message::user("Hello");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_message_with_image() {
        let msg = Message::user_with_image("What's in this image?", "base64imagedata");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.base64_image, Some("base64imagedata".to_string()));
    }

    #[test]
    fn test_agent_state_default() {
        let state = AgentState::default();
        assert_eq!(state, AgentState::Idle);
    }
}
