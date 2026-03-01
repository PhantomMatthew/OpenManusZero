//! Base agent implementation

use crate::context::Memory;
use crate::error::{Error, Result};
use crate::llm::LlmClient;
use crate::schema::{AgentState, Message};
use async_trait::async_trait;
use std::sync::Arc;

/// Core agent trait that all agents must implement
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get the agent name
    fn name(&self) -> &str;

    /// Get the agent description
    fn description(&self) -> &str;

    /// Get the current agent state
    fn state(&self) -> AgentState;

    /// Run the agent with a request
    async fn run(&mut self, request: &str) -> Result<String>;

    /// Cleanup agent resources
    async fn cleanup(&mut self) -> Result<()>;
}

/// Base agent implementation with common functionality
pub struct BaseAgent {
    /// Agent name
    name: String,
    /// Agent description
    description: String,
    /// System prompt
    system_prompt: Option<String>,
    /// Next step prompt
    next_step_prompt: Option<String>,
    /// LLM client
    llm: Arc<dyn LlmClient>,
    /// Memory for conversation history
    memory: Memory,
    /// Current agent state
    state: AgentState,
    /// Maximum steps before termination
    max_steps: usize,
    /// Current step count
    current_step: usize,
    /// Duplicate threshold for stuck detection
    duplicate_threshold: usize,
}

impl BaseAgent {
    /// Create a new base agent
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        llm: Arc<dyn LlmClient>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            system_prompt: None,
            next_step_prompt: None,
            llm,
            memory: Memory::new(),
            state: AgentState::Idle,
            max_steps: 10,
            current_step: 0,
            duplicate_threshold: 2,
        }
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the next step prompt
    pub fn with_next_step_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.next_step_prompt = Some(prompt.into());
        self
    }

    /// Set the maximum steps
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Set the duplicate threshold for stuck detection
    pub fn with_duplicate_threshold(mut self, threshold: usize) -> Self {
        self.duplicate_threshold = threshold;
        self
    }

    /// Get the system prompt
    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    /// Set the system prompt (mutable version)
    pub fn set_system_prompt(&mut self, prompt: impl Into<String>) {
        self.system_prompt = Some(prompt.into());
    }

    /// Get the next step prompt
    pub fn next_step_prompt(&self) -> Option<&str> {
        self.next_step_prompt.as_deref()
    }

    /// Set the next step prompt (mutable version)
    pub fn set_next_step_prompt(&mut self, prompt: impl Into<String>) {
        self.next_step_prompt = Some(prompt.into());
    }

    /// Get a reference to the LLM client
    pub fn llm(&self) -> &dyn LlmClient {
        self.llm.as_ref()
    }

    /// Get a reference to the memory
    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    /// Get a mutable reference to the memory
    pub fn memory_mut(&mut self) -> &mut Memory {
        &mut self.memory
    }

    /// Get the maximum steps
    pub fn max_steps(&self) -> usize {
        self.max_steps
    }

    /// Get the current step
    pub fn current_step(&self) -> usize {
        self.current_step
    }

    /// Update memory with a new message
    pub fn update_memory(&mut self, message: Message) {
        self.memory.add(message);
    }

    /// Add multiple messages to memory
    pub fn add_messages(&mut self, messages: Vec<Message>) {
        self.memory.add_all(messages);
    }

    /// Set the agent state
    pub fn set_state(&mut self, state: AgentState) {
        self.state = state;
    }

    /// Increment the step counter
    pub fn increment_step(&mut self) {
        self.current_step += 1;
    }

    /// Reset the step counter
    pub fn reset_steps(&mut self) {
        self.current_step = 0;
    }

    /// Check if the agent has reached max steps
    pub fn is_max_steps_reached(&self) -> bool {
        self.current_step >= self.max_steps
    }

    /// Check if the agent is stuck in a loop
    pub fn is_stuck(&self) -> bool {
        let messages = self.memory.messages();
        if messages.len() < 2 {
            return false;
        }

        let last_message = match messages.last() {
            Some(m) => m,
            None => return false,
        };

        let content = match &last_message.content {
            Some(c) => c,
            None => return false,
        };

        // Count identical content occurrences
        let duplicate_count = messages
            .iter()
            .rev()
            .skip(1)
            .filter(|m| m.role == crate::schema::Role::Assistant)
            .filter(|m| m.content.as_ref() == Some(content))
            .count();

        duplicate_count >= self.duplicate_threshold
    }

    /// Handle stuck state by modifying the next step prompt
    pub fn handle_stuck_state(&mut self) {
        let stuck_prompt = "Observed duplicate responses. Consider new strategies and avoid repeating ineffective paths already attempted.";
        self.next_step_prompt = Some(format!(
            "{}\n{}",
            stuck_prompt,
            self.next_step_prompt.as_deref().unwrap_or("")
        ));
        tracing::warn!("Agent detected stuck state. Added prompt: {}", stuck_prompt);
    }

    /// Clear memory
    pub fn clear_memory(&mut self) {
        self.memory.clear();
    }
}

#[async_trait]
impl Agent for BaseAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn state(&self) -> AgentState {
        self.state
    }

    async fn run(&mut self, request: &str) -> Result<String> {
        if self.state != AgentState::Idle {
            return Err(Error::Agent(format!(
                "Cannot run agent from state: {:?}",
                self.state
            )));
        }

        self.update_memory(Message::user(request));
        self.state = AgentState::Running;
        let mut results = Vec::new();

        while self.current_step < self.max_steps && self.state != AgentState::Finished {
            self.current_step += 1;
            tracing::info!("Executing step {}/{}", self.current_step, self.max_steps);

            // Check for stuck state
            if self.is_stuck() {
                self.handle_stuck_state();
            }

            results.push(format!("Step {} executed", self.current_step));
        }

        if self.current_step >= self.max_steps {
            self.current_step = 0;
            self.state = AgentState::Idle;
            results.push(format!(
                "Terminated: Reached max steps ({})",
                self.max_steps
            ));
        }

        self.state = AgentState::Idle;
        Ok(results.join("\n"))
    }

    async fn cleanup(&mut self) -> Result<()> {
        self.clear_memory();
        self.state = AgentState::Idle;
        self.current_step = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLlmClient;

    fn create_test_agent() -> BaseAgent {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        BaseAgent::new("test_agent", "A test agent", llm)
    }

    #[test]
    fn test_base_agent_new() {
        let agent = create_test_agent();
        assert_eq!(agent.name(), "test_agent");
        assert_eq!(agent.description(), "A test agent");
        assert_eq!(agent.state(), AgentState::Idle);
    }

    #[test]
    fn test_base_agent_with_system_prompt() {
        let agent = create_test_agent().with_system_prompt("You are helpful");
        assert_eq!(agent.system_prompt(), Some("You are helpful"));
    }

    #[test]
    fn test_base_agent_with_max_steps() {
        let agent = create_test_agent().with_max_steps(20);
        assert_eq!(agent.max_steps(), 20);
    }

    #[test]
    fn test_base_agent_update_memory() {
        let mut agent = create_test_agent();
        agent.update_memory(Message::user("Hello"));
        assert_eq!(agent.memory().len(), 1);
    }

    #[test]
    fn test_base_agent_state_transitions() {
        let mut agent = create_test_agent();
        assert_eq!(agent.state(), AgentState::Idle);
        agent.set_state(AgentState::Running);
        assert_eq!(agent.state(), AgentState::Running);
    }

    #[test]
    fn test_base_agent_step_counter() {
        let mut agent = create_test_agent();
        assert_eq!(agent.current_step(), 0);
        agent.increment_step();
        assert_eq!(agent.current_step(), 1);
        agent.reset_steps();
        assert_eq!(agent.current_step(), 0);
    }

    #[test]
    fn test_base_agent_is_max_steps_reached() {
        let mut agent = create_test_agent().with_max_steps(2);
        assert!(!agent.is_max_steps_reached());
        agent.increment_step();
        assert!(!agent.is_max_steps_reached());
        agent.increment_step();
        assert!(agent.is_max_steps_reached());
    }

    #[test]
    fn test_base_agent_is_stuck() {
        let mut agent = create_test_agent().with_duplicate_threshold(2);
        assert!(!agent.is_stuck());

        // Add duplicate messages
        agent.update_memory(Message::assistant("Same response"));
        agent.update_memory(Message::assistant("Same response"));
        agent.update_memory(Message::assistant("Same response"));

        assert!(agent.is_stuck());
    }

    #[test]
    fn test_base_agent_handle_stuck_state() {
        let mut agent = create_test_agent();
        agent.handle_stuck_state();
        assert!(agent
            .next_step_prompt()
            .unwrap()
            .contains("duplicate responses"));
    }

    #[tokio::test]
    async fn test_base_agent_run_from_non_idle_fails() {
        let mut agent = create_test_agent();
        agent.set_state(AgentState::Running);
        let result = agent.run("test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_base_agent_cleanup() {
        let mut agent = create_test_agent();
        agent.update_memory(Message::user("test"));
        agent.set_state(AgentState::Running);
        agent.increment_step();

        agent.cleanup().await.unwrap();
        assert!(agent.memory().is_empty());
        assert_eq!(agent.state(), AgentState::Idle);
        assert_eq!(agent.current_step(), 0);
    }
}
