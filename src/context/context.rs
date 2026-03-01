//! Context for tool execution

use crate::schema::Message;
use crate::tool::ToolCollection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Execution context passed to tools
pub struct Context {
    /// Current working directory
    pub working_dir: PathBuf,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Shared state (key-value store)
    pub state: HashMap<String, serde_json::Value>,
    /// Reference to available tools
    pub tools: Option<Arc<ToolCollection>>,
    /// Current conversation messages (read-only access for context)
    pub messages: Vec<Message>,
    /// Maximum allowed execution time in seconds
    pub timeout_secs: u64,
    /// Whether network access is allowed
    pub network_enabled: bool,
    /// Maximum memory usage allowed (in bytes)
    pub memory_limit: Option<usize>,
}

impl Context {
    /// Create a new context with default settings
    pub fn new() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            env: std::env::vars().collect(),
            state: HashMap::new(),
            tools: None,
            messages: Vec::new(),
            timeout_secs: 300,
            network_enabled: true,
            memory_limit: None,
        }
    }

    /// Create a context with a specific working directory
    pub fn with_working_dir(path: impl Into<PathBuf>) -> Self {
        let mut ctx = Self::new();
        ctx.working_dir = path.into();
        ctx
    }

    /// Set the working directory
    pub fn set_working_dir(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.working_dir = path.into();
        self
    }

    /// Set environment variable
    pub fn set_env(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Get environment variable
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env.get(key)
    }

    /// Set a state value
    pub fn set_state<T: serde::Serialize>(
        &mut self,
        key: impl Into<String>,
        value: T,
    ) -> Result<(), serde_json::Error> {
        self.state.insert(key.into(), serde_json::to_value(value)?);
        Ok(())
    }

    /// Get a state value
    pub fn get_state<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Option<Result<T, serde_json::Error>> {
        self.state
            .get(key)
            .map(|v| serde_json::from_value(v.clone()))
    }

    /// Set the tool collection
    pub fn set_tools(&mut self, tools: Arc<ToolCollection>) -> &mut Self {
        self.tools = Some(tools);
        self
    }

    /// Set the messages
    pub fn set_messages(&mut self, messages: Vec<Message>) -> &mut Self {
        self.messages = messages;
        self
    }

    /// Set the timeout
    pub fn set_timeout(&mut self, secs: u64) -> &mut Self {
        self.timeout_secs = secs;
        self
    }

    /// Enable or disable network access
    pub fn set_network_enabled(&mut self, enabled: bool) -> &mut Self {
        self.network_enabled = enabled;
        self
    }

    /// Set memory limit
    pub fn set_memory_limit(&mut self, bytes: usize) -> &mut Self {
        self.memory_limit = Some(bytes);
        self
    }

    /// Get recent messages (last n)
    pub fn recent_messages(&self, n: usize) -> &[Message] {
        let start = self.messages.len().saturating_sub(n);
        &self.messages[start..]
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_new() {
        let ctx = Context::new();
        assert!(ctx.working_dir.exists() || ctx.working_dir == PathBuf::from("."));
        assert!(ctx.timeout_secs > 0);
        assert!(ctx.network_enabled);
    }

    #[test]
    fn test_context_with_working_dir() {
        let ctx = Context::with_working_dir("/tmp");
        assert_eq!(ctx.working_dir, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_context_set_env() {
        let mut ctx = Context::new();
        ctx.set_env("TEST_VAR", "test_value");
        assert_eq!(ctx.get_env("TEST_VAR"), Some(&"test_value".to_string()));
    }

    #[test]
    fn test_context_set_state() {
        let mut ctx = Context::new();
        ctx.set_state("counter", 42).unwrap();
        let result: Result<i32, _> = ctx.get_state("counter").unwrap();
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_context_set_timeout() {
        let mut ctx = Context::new();
        ctx.set_timeout(60);
        assert_eq!(ctx.timeout_secs, 60);
    }

    #[test]
    fn test_context_set_network_enabled() {
        let mut ctx = Context::new();
        ctx.set_network_enabled(false);
        assert!(!ctx.network_enabled);
    }

    #[test]
    fn test_context_set_memory_limit() {
        let mut ctx = Context::new();
        ctx.set_memory_limit(1024 * 1024);
        assert_eq!(ctx.memory_limit, Some(1024 * 1024));
    }

    #[test]
    fn test_context_recent_messages() {
        let mut ctx = Context::new();
        ctx.messages = vec![
            Message::user("msg1"),
            Message::user("msg2"),
            Message::user("msg3"),
        ];
        let recent = ctx.recent_messages(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].content, Some("msg2".to_string()));
    }

    #[test]
    fn test_context_recent_messages_empty() {
        let ctx = Context::new();
        let recent = ctx.recent_messages(5);
        assert!(recent.is_empty());
    }

    #[test]
    fn test_context_state_missing_key() {
        let ctx = Context::new();
        let result: Option<Result<i32, _>> = ctx.get_state("nonexistent");
        assert!(result.is_none());
    }
}
