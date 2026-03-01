//! A2A Protocol Types
//!
//! Based on Google's A2A (Agent-to-Agent) Protocol
//! https://google.github.io/A2A/#/documentation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Agent capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ACapabilities {
    /// Whether the agent supports streaming responses
    #[serde(default)]
    pub streaming: bool,
    /// Whether the agent supports push notifications
    #[serde(default = "default_true")]
    pub push_notifications: bool,
}

fn default_true() -> bool {
    true
}

impl Default for A2ACapabilities {
    fn default() -> Self {
        Self {
            streaming: false,
            push_notifications: true,
        }
    }
}

/// Agent skill description
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ASkill {
    /// Unique skill identifier
    pub id: String,
    /// Human-readable skill name
    pub name: String,
    /// Detailed skill description
    pub description: String,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Example usage
    #[serde(default)]
    pub examples: Vec<String>,
}

impl A2ASkill {
    /// Create a new skill
    pub fn new(id: impl Into<String>, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            tags: Vec::new(),
            examples: Vec::new(),
        }
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add examples
    pub fn with_examples(mut self, examples: Vec<String>) -> Self {
        self.examples = examples;
        self
    }
}

/// Agent card - metadata describing the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ACard {
    /// Agent name
    pub name: String,
    /// Agent description
    pub description: String,
    /// Base URL for the agent
    pub url: String,
    /// Agent version
    pub version: String,
    /// Supported input content types
    #[serde(default = "default_text_types")]
    pub default_input_modes: Vec<String>,
    /// Supported output content types
    #[serde(default = "default_text_types")]
    pub default_output_modes: Vec<String>,
    /// Agent capabilities
    pub capabilities: A2ACapabilities,
    /// Agent skills
    #[serde(default)]
    pub skills: Vec<A2ASkill>,
}

fn default_text_types() -> Vec<String> {
    vec!["text".to_string(), "text/plain".to_string()]
}

impl A2ACard {
    /// Create a new agent card
    pub fn new(name: impl Into<String>, description: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            url: url.into(),
            version: "1.0.0".to_string(),
            default_input_modes: default_text_types(),
            default_output_modes: default_text_types(),
            capabilities: A2ACapabilities::default(),
            skills: Vec::new(),
        }
    }

    /// Set version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set capabilities
    pub fn with_capabilities(mut self, capabilities: A2ACapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Add a skill
    pub fn with_skill(mut self, skill: A2ASkill) -> Self {
        self.skills.push(skill);
        self
    }

    /// Set skills
    pub fn with_skills(mut self, skills: Vec<A2ASkill>) -> Self {
        self.skills = skills;
        self
    }
}

/// Message part (text, data, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum A2APart {
    /// Text content
    Text { text: String },
    /// Data content
    Data { data: HashMap<String, serde_json::Value> },
    /// File content
    File { file: A2AFilePart },
}

impl A2APart {
    /// Create a text part
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            text: content.into(),
        }
    }

    /// Create a data part
    pub fn data(data: HashMap<String, serde_json::Value>) -> Self {
        Self::Data { data }
    }

    /// Get text content if this is a text part
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }
}

/// File part for A2A messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AFilePart {
    /// File name
    pub name: String,
    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// File content (base64 for binary)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// URL to file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A2A Message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    /// Message ID
    #[serde(default = "generate_message_id")]
    pub message_id: String,
    /// Context ID (session)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    /// Task ID this message belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Role (user or agent)
    pub role: A2ARole,
    /// Message parts
    pub parts: Vec<A2APart>,
    /// Metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn generate_message_id() -> String {
    Uuid::new_v4().to_string()
}

/// Message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum A2ARole {
    User,
    Agent,
}

impl A2AMessage {
    /// Create a new user message
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            message_id: generate_message_id(),
            context_id: None,
            task_id: None,
            role: A2ARole::User,
            parts: vec![A2APart::text(text)],
            metadata: HashMap::new(),
        }
    }

    /// Create a new agent message
    pub fn agent(text: impl Into<String>) -> Self {
        Self {
            message_id: generate_message_id(),
            context_id: None,
            task_id: None,
            role: A2ARole::Agent,
            parts: vec![A2APart::text(text)],
            metadata: HashMap::new(),
        }
    }

    /// Set context ID
    pub fn with_context(mut self, context_id: impl Into<String>) -> Self {
        self.context_id = Some(context_id.into());
        self
    }

    /// Set task ID
    pub fn with_task(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    /// Get text content from parts
    pub fn get_text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| p.as_text())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Artifact - output from a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AArtifact {
    /// Artifact ID
    #[serde(default = "generate_artifact_id")]
    pub artifact_id: String,
    /// Artifact name
    pub name: String,
    /// Artifact description
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Artifact parts
    pub parts: Vec<A2APart>,
    /// Metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn generate_artifact_id() -> String {
    Uuid::new_v4().to_string()
}

impl A2AArtifact {
    /// Create a new artifact
    pub fn new(name: impl Into<String>, parts: Vec<A2APart>) -> Self {
        Self {
            artifact_id: generate_artifact_id(),
            name: name.into(),
            description: String::new(),
            parts,
            metadata: HashMap::new(),
        }
    }

    /// Create a text artifact
    pub fn text(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(name, vec![A2APart::text(content)])
    }
}

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum A2ATaskState {
    Submitted,
    Working,
    InputRequired,
    Completed,
    Canceled,
    Failed,
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATaskStatus {
    /// Current state
    pub state: A2ATaskState,
    /// Status message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<A2AMessage>,
    /// Timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

impl A2ATaskStatus {
    /// Create a new status
    pub fn new(state: A2ATaskState) -> Self {
        Self {
            state,
            message: None,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Create submitted status
    pub fn submitted() -> Self {
        Self::new(A2ATaskState::Submitted)
    }

    /// Create working status
    pub fn working() -> Self {
        Self::new(A2ATaskState::Working)
    }

    /// Create input-required status
    pub fn input_required() -> Self {
        Self::new(A2ATaskState::InputRequired)
    }

    /// Create completed status
    pub fn completed() -> Self {
        Self::new(A2ATaskState::Completed)
    }

    /// Create failed status
    pub fn failed() -> Self {
        Self::new(A2ATaskState::Failed)
    }
}

/// A2A Task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATask {
    /// Task kind
    #[serde(default = "default_task_kind")]
    pub kind: String,
    /// Task ID
    pub id: String,
    /// Context ID (session)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    /// Task status
    pub status: A2ATaskStatus,
    /// Task history
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<A2AMessage>,
    /// Task artifacts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<A2AArtifact>,
    /// Metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_task_kind() -> String {
    "task".to_string()
}

impl A2ATask {
    /// Create a new task
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            kind: default_task_kind(),
            id: id.into(),
            context_id: None,
            status: A2ATaskStatus::submitted(),
            history: Vec::new(),
            artifacts: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Generate a task with random ID
    pub fn generate() -> Self {
        Self::new(Uuid::new_v4().to_string())
    }

    /// Set context ID
    pub fn with_context(mut self, context_id: impl Into<String>) -> Self {
        self.context_id = Some(context_id.into());
        self
    }

    /// Set status
    pub fn with_status(mut self, status: A2ATaskStatus) -> Self {
        self.status = status;
        self
    }

    /// Add to history
    pub fn with_history(mut self, message: A2AMessage) -> Self {
        self.history.push(message);
        self
    }

    /// Add artifact
    pub fn with_artifact(mut self, artifact: A2AArtifact) -> Self {
        self.artifacts.push(artifact);
        self
    }

    /// Complete the task with a response
    pub fn complete(mut self, response: impl Into<String>) -> Self {
        self.status = A2ATaskStatus::completed();
        self.artifacts
            .push(A2AArtifact::text("response", response.into()));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a2a_capabilities_default() {
        let caps = A2ACapabilities::default();
        assert!(!caps.streaming);
        assert!(caps.push_notifications);
    }

    #[test]
    fn test_a2a_skill_new() {
        let skill = A2ASkill::new("test", "Test Skill", "A test skill")
            .with_tags(vec!["test".to_string()])
            .with_examples(vec!["example".to_string()]);

        assert_eq!(skill.id, "test");
        assert_eq!(skill.name, "Test Skill");
        assert_eq!(skill.tags.len(), 1);
    }

    #[test]
    fn test_a2a_card_new() {
        let card = A2ACard::new("Test Agent", "A test agent", "http://localhost:8080");
        assert_eq!(card.name, "Test Agent");
        assert_eq!(card.version, "1.0.0");
    }

    #[test]
    fn test_a2a_part_text() {
        let part = A2APart::text("Hello");
        assert_eq!(part.as_text(), Some("Hello"));
    }

    #[test]
    fn test_a2a_message_user() {
        let msg = A2AMessage::user("Hello");
        assert_eq!(msg.role, A2ARole::User);
        assert_eq!(msg.get_text(), "Hello");
    }

    #[test]
    fn test_a2a_message_agent() {
        let msg = A2AMessage::agent("Hi there");
        assert_eq!(msg.role, A2ARole::Agent);
        assert_eq!(msg.get_text(), "Hi there");
    }

    #[test]
    fn test_a2a_artifact_text() {
        let artifact = A2AArtifact::text("test", "content");
        assert_eq!(artifact.name, "test");
        assert_eq!(artifact.parts.len(), 1);
    }

    #[test]
    fn test_a2a_task_new() {
        let task = A2ATask::new("task-123");
        assert_eq!(task.id, "task-123");
        assert_eq!(task.kind, "task");
        assert_eq!(task.status.state, A2ATaskState::Submitted);
    }

    #[test]
    fn test_a2a_task_complete() {
        let task = A2ATask::generate().complete("Done!");
        assert_eq!(task.status.state, A2ATaskState::Completed);
        assert_eq!(task.artifacts.len(), 1);
    }

    #[test]
    fn test_a2a_task_status() {
        assert_eq!(A2ATaskStatus::submitted().state, A2ATaskState::Submitted);
        assert_eq!(A2ATaskStatus::working().state, A2ATaskState::Working);
        assert_eq!(A2ATaskStatus::completed().state, A2ATaskState::Completed);
        assert_eq!(A2ATaskStatus::failed().state, A2ATaskState::Failed);
    }

    #[test]
    fn test_a2a_message_json() {
        let msg = A2AMessage::user("Hello");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_a2a_task_json() {
        let task = A2ATask::new("task-1").complete("Done");
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("task-1"));
        assert!(json.contains("completed"));
    }
}
