//! ReAct-style agent implementation

use crate::agent::base::BaseAgent;
use crate::error::{Error, Result};
use crate::llm::LlmRequest;
use crate::schema::{AgentState, Message};
use async_trait::async_trait;
use std::sync::Arc;

/// ReAct (Reasoning and Acting) agent
pub struct ReActAgent {
    base: BaseAgent,
}

impl ReActAgent {
    /// Create a new ReAct agent
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        llm: Arc<dyn crate::llm::LlmClient>,
    ) -> Self {
        Self {
            base: BaseAgent::new(name, description, llm)
                .with_system_prompt("You are a helpful assistant that reasons step by step.")
                .with_next_step_prompt("What should I do next?"),
        }
    }

    /// Set system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.base = self.base.with_system_prompt(prompt);
        self
    }

    /// Set next step prompt
    pub fn with_next_step_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.base = self.base.with_next_step_prompt(prompt);
        self
    }

    /// Set max steps
    pub fn with_max_steps(mut self, steps: usize) -> Self {
        self.base = self.base.with_max_steps(steps);
        self
    }

    /// Think step - process current state and decide next action
    pub async fn think(&mut self) -> Result<bool> {
        // Add next step prompt if available
        if let Some(prompt) = self.base.next_step_prompt() {
            self.base.update_memory(Message::user(prompt));
        }

        // Build messages for LLM
        let mut messages = Vec::new();
        if let Some(system) = self.base.system_prompt() {
            messages.push(Message::system(system));
        }
        messages.extend(self.base.memory().messages());

        // Make LLM request
        let request = LlmRequest::new(self.base.llm().model(), messages);
        let response = self
            .base
            .llm()
            .completion(&request)
            .await
            .map_err(Error::Llm)?;

        // Add response to memory
        let content = response.content().unwrap_or("").to_string();
        let is_empty = content.is_empty();
        self.base.update_memory(Message::assistant(content));

        Ok(!is_empty)
    }

    /// Act step - execute the decided action
    pub async fn act(&mut self) -> Result<String> {
        let messages = self.base.memory().messages();
        let last_message = messages
            .last()
            .ok_or_else(|| Error::Agent("No messages in memory".to_string()))?;

        Ok(last_message.content.clone().unwrap_or_default())
    }

    /// Get mutable reference to base agent
    pub fn base_mut(&mut self) -> &mut BaseAgent {
        &mut self.base
    }

    /// Get reference to base agent
    pub fn base(&self) -> &BaseAgent {
        &self.base
    }
}

#[async_trait]
impl super::Agent for ReActAgent {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn description(&self) -> &str {
        self.base.description()
    }

    fn state(&self) -> AgentState {
        self.base.state()
    }

    async fn run(&mut self, request: &str) -> Result<String> {
        if self.base.state() != AgentState::Idle {
            return Err(Error::Agent(format!(
                "Cannot run agent from state: {:?}",
                self.base.state()
            )));
        }

        self.base.update_memory(Message::user(request));
        self.base.set_state(AgentState::Running);
        let mut results = Vec::new();

        while self.base.current_step() < self.base.max_steps()
            && self.base.state() != AgentState::Finished
        {
            self.base.increment_step();
            let step = self.base.current_step();
            let max = self.base.max_steps();
            tracing::info!("Executing step {}/{}", step, max);

            // Think
            let thinking = self.think().await?;
            if !thinking {
                tracing::info!("Agent finished thinking");
                break;
            }

            // Act
            let action_result = self.act().await?;
            results.push(format!("Step {}: {}", step, action_result));

            // Check for stuck state
            if self.base.is_stuck() {
                self.base.handle_stuck_state();
            }
        }

        if self.base.current_step() >= self.base.max_steps() {
            results.push(format!(
                "Terminated: Reached max steps ({})",
                self.base.max_steps()
            ));
        }

        self.base.reset_steps();
        self.base.set_state(AgentState::Idle);
        Ok(results.join("\n"))
    }

    async fn cleanup(&mut self) -> Result<()> {
        self.base.clear_memory();
        self.base.set_state(AgentState::Idle);
        self.base.reset_steps();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::llm::MockLlmClient;

    fn create_test_agent() -> ReActAgent {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        ReActAgent::new("react_test", "A ReAct test agent", llm)
    }

    #[test]
    fn test_react_agent_new() {
        let agent = create_test_agent();
        assert_eq!(agent.name(), "react_test");
        assert_eq!(agent.description(), "A ReAct test agent");
        assert_eq!(agent.state(), AgentState::Idle);
    }

    #[test]
    fn test_react_agent_has_system_prompt() {
        let agent = create_test_agent();
        assert!(agent.base().system_prompt().is_some());
    }

    #[tokio::test]
    async fn test_react_agent_think() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        llm.set_text_response("I should help the user.");

        let mut agent = ReActAgent::new("test", "test agent", llm);
        agent.base_mut().update_memory(Message::user("Hello"));

        let result = agent.think().await;
        assert!(result.is_ok());
        assert!(agent.base().memory().len() > 1);
    }

    #[tokio::test]
    async fn test_react_agent_act() {
        let mut agent = create_test_agent();
        agent
            .base_mut()
            .update_memory(Message::assistant("Test response"));

        let result = agent.act().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Test response");
    }

    #[tokio::test]
    async fn test_react_agent_cleanup() {
        let mut agent = create_test_agent();
        agent.base_mut().update_memory(Message::user("test"));
        agent.base_mut().set_state(AgentState::Running);

        agent.cleanup().await.unwrap();
        assert!(agent.base().memory().is_empty());
        assert_eq!(agent.state(), AgentState::Idle);
    }
}
