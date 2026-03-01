//! Manus agent - the main general-purpose agent

use crate::agent::toolcall::ToolCallAgent;
use crate::error::Result;
use crate::llm::LlmClient;
use crate::schema::AgentState;
use crate::tool::{Tool, ToolCollection};
use async_trait::async_trait;
use std::sync::Arc;

/// Default system prompt for Manus
const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a helpful AI assistant that can solve various tasks.
You have access to tools that allow you to execute code, browse the web, edit files, and more.
Always think step by step and use tools when appropriate.
If you need information from the user, use the ask_human tool.
When you have completed the task, use the terminate tool to finish."#;

/// Default next step prompt
const DEFAULT_NEXT_STEP_PROMPT: &str = "What should I do next to accomplish the task?";

/// Manus - the main general-purpose agent
pub struct Manus {
    agent: ToolCallAgent,
}

impl Manus {
    /// Create a new Manus agent with default configuration
    pub async fn new() -> Result<Self> {
        let tools = Self::create_default_tools();

        // Create a placeholder LLM client - in production this would be configured
        // from config file or environment
        let llm = Arc::new(crate::llm::MockLlmClient::new("gpt-4"));

        let agent = ToolCallAgent::new(
            "Manus",
            "A versatile agent that can solve various tasks using multiple tools",
            llm,
            tools,
        )
        .with_system_prompt(DEFAULT_SYSTEM_PROMPT)
        .with_next_step_prompt(DEFAULT_NEXT_STEP_PROMPT)
        .with_max_steps(20);

        Ok(Self { agent })
    }

    /// Create a Manus agent with a custom LLM client
    pub fn with_llm(llm: Arc<dyn LlmClient>) -> Self {
        let tools = Self::create_default_tools();

        let agent = ToolCallAgent::new(
            "Manus",
            "A versatile agent that can solve various tasks using multiple tools",
            llm,
            tools,
        )
        .with_system_prompt(DEFAULT_SYSTEM_PROMPT)
        .with_next_step_prompt(DEFAULT_NEXT_STEP_PROMPT)
        .with_max_steps(20);

        Self { agent }
    }

    /// Create the default tool collection
    fn create_default_tools() -> Arc<ToolCollection> {
        let mut collection = ToolCollection::new();

        // Add basic tools
        collection
            .add_tool(Arc::new(crate::tool::terminate::TerminateTool::new()) as Arc<dyn Tool>);
        collection.add_tool(Arc::new(crate::tool::ask_human::AskHumanTool::new()) as Arc<dyn Tool>);

        #[cfg(feature = "browser")]
        {
            // Browser tool would be added here
        }

        Arc::new(collection)
    }

    /// Add a tool to the agent
    pub fn add_tool(&mut self, tool: Arc<dyn Tool>) {
        // Create new collection with the added tool
        let mut new_tools = (*self.agent.tools()).clone();
        new_tools.add_tool(tool);
    }

    /// Get the underlying ToolCallAgent
    pub fn inner(&self) -> &ToolCallAgent {
        &self.agent
    }

    /// Get mutable reference to underlying ToolCallAgent
    pub fn inner_mut(&mut self) -> &mut ToolCallAgent {
        &mut self.agent
    }
}

#[async_trait]
impl super::Agent for Manus {
    fn name(&self) -> &str {
        self.agent.name()
    }

    fn description(&self) -> &str {
        self.agent.description()
    }

    fn state(&self) -> AgentState {
        self.agent.state()
    }

    async fn run(&mut self, request: &str) -> Result<String> {
        tracing::info!("Manus agent starting with request: {}", request);
        self.agent.run(request).await
    }

    async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Manus agent cleaning up");
        self.agent.cleanup().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::llm::MockLlmClient;

    #[tokio::test]
    async fn test_manus_new() {
        let manus = Manus::new().await;
        assert!(manus.is_ok());
        let manus = manus.unwrap();
        assert_eq!(manus.name(), "Manus");
    }

    #[test]
    fn test_manus_with_llm() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let manus = Manus::with_llm(llm);
        assert_eq!(manus.name(), "Manus");
        assert_eq!(manus.state(), AgentState::Idle);
    }

    #[test]
    fn test_manus_has_tools() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let manus = Manus::with_llm(llm);
        assert!(!manus.agent.tools().is_empty());
        assert!(manus.agent.tools().has_tool("terminate"));
        assert!(manus.agent.tools().has_tool("ask_human"));
    }

    #[tokio::test]
    async fn test_manus_run() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        llm.set_text_response("I will help you with that.");

        let mut manus = Manus::with_llm(llm);
        let result = manus.run("Hello").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_manus_cleanup() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let mut manus = Manus::with_llm(llm);
        manus
            .agent
            .base_mut()
            .update_memory(crate::schema::Message::user("test"));

        manus.cleanup().await.unwrap();
        assert!(manus.agent.base().memory().is_empty());
    }
}
