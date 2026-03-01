//! ToolCall agent implementation

use crate::agent::base::BaseAgent;
use crate::context::Context;
use crate::error::{Error, Result};
use crate::llm::{LlmClient, LlmRequest, ToolChoice};
use crate::schema::{AgentState, Message, ToolCall};
use crate::tool::ToolCollection;
use async_trait::async_trait;
use std::sync::Arc;

/// Agent that can call tools
pub struct ToolCallAgent {
    /// Base agent
    base: BaseAgent,
    /// Tool collection
    tools: Arc<ToolCollection>,
    /// Special tool names (tools that can finish execution)
    special_tool_names: Vec<String>,
    /// Current tool calls from last think
    current_tool_calls: Vec<ToolCall>,
    /// Maximum observation length
    max_observe: Option<usize>,
}

impl ToolCallAgent {
    /// Create a new tool call agent
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        llm: Arc<dyn LlmClient>,
        tools: Arc<ToolCollection>,
    ) -> Self {
        Self {
            base: BaseAgent::new(name, description, llm)
                .with_system_prompt(
                    "You are a helpful assistant that can use tools to complete tasks.",
                )
                .with_next_step_prompt("What should I do next?")
                .with_max_steps(30),
            tools,
            special_tool_names: vec!["terminate".to_string()],
            current_tool_calls: Vec::new(),
            max_observe: Some(10000),
        }
    }

    /// Set special tool names
    pub fn with_special_tool_names(mut self, names: Vec<String>) -> Self {
        self.special_tool_names = names;
        self
    }

    /// Set max observe length
    pub fn with_max_observe(mut self, max: usize) -> Self {
        self.max_observe = Some(max);
        self
    }

    /// Set max steps
    pub fn with_max_steps(mut self, steps: usize) -> Self {
        self.base = self.base.with_max_steps(steps);
        self
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

    /// Set next step prompt (mutable version)
    pub fn set_next_step_prompt(&mut self, prompt: impl Into<String>) {
        self.base.set_next_step_prompt(prompt);
    }

    /// Get reference to tools
    pub fn tools(&self) -> &ToolCollection {
        &self.tools
    }

    /// Get reference to base agent
    pub fn base(&self) -> &BaseAgent {
        &self.base
    }

    /// Get mutable reference to base agent
    pub fn base_mut(&mut self) -> &mut BaseAgent {
        &mut self.base
    }

    /// Think step - decide which tools to call
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

        // Build request with tools
        let request = LlmRequest::new(self.base.llm().model(), messages)
            .with_tools(self.tools.to_definitions())
            .with_tool_choice(ToolChoice::Auto);

        // Make LLM request
        let response = self
            .base
            .llm()
            .completion(&request)
            .await
            .map_err(Error::Llm)?;

        // Extract tool calls and content
        self.current_tool_calls = response.tool_calls().cloned().unwrap_or_default();
        let content = response.content().unwrap_or("").to_string();
        let has_content = !content.is_empty();
        let has_tool_calls = !self.current_tool_calls.is_empty();

        tracing::info!("Agent thoughts: {}", content);
        tracing::info!(
            "Selected {} tools: {:?}",
            self.current_tool_calls.len(),
            self.current_tool_calls
                .iter()
                .map(|t| &t.function.name)
                .collect::<Vec<_>>()
        );

        // Add response to memory
        if has_tool_calls {
            self.base.update_memory(Message::assistant_with_tools(
                if content.is_empty() {
                    None
                } else {
                    Some(content)
                },
                self.current_tool_calls.clone(),
            ));
        } else {
            self.base.update_memory(Message::assistant(content));
        }

        Ok(has_tool_calls || has_content)
    }

    /// Act step - execute tool calls
    pub async fn act(&mut self) -> Result<String> {
        if self.current_tool_calls.is_empty() {
            let messages = self.base.memory().messages();
            let last = messages.last();
            return Ok(last
                .and_then(|m| m.content.clone())
                .unwrap_or_else(|| "No content or commands to execute".to_string()));
        }

        let mut results = Vec::new();
        let mut ctx = Context::new();
        let mut should_finish = false;

        for tool_call in &self.current_tool_calls {
            let result = self.execute_tool(tool_call, &mut ctx).await?;

            // Truncate if needed
            let result_str = if let Some(max) = self.max_observe {
                if result.len() > max {
                    format!("{}...[truncated]", &result[..max])
                } else {
                    result
                }
            } else {
                result
            };

            tracing::info!("Tool '{}' completed", tool_call.function.name);

            // Add tool result to memory
            self.base.update_memory(Message::tool_result(
                &result_str,
                &tool_call.function.name,
                &tool_call.id,
            ));

            results.push(result_str);

            // Check if this is a special tool
            if self.is_special_tool(&tool_call.function.name) {
                should_finish = true;
            }
        }

        if should_finish {
            self.base.set_state(AgentState::Finished);
        }

        Ok(results.join("\n\n"))
    }

    /// Execute a single tool call
    async fn execute_tool(&self, tool_call: &ToolCall, ctx: &mut Context) -> Result<String> {
        let name = &tool_call.function.name;
        let args = &tool_call.function.arguments;

        tracing::info!("Executing tool: '{}' with args: {}", name, args);

        let result = self
            .tools
            .execute(name, args, ctx)
            .await
            .map_err(Error::Tool)?;

        Ok(format!(
            "Observed output of cmd `{}`:\n{}",
            name,
            if result.is_empty() {
                "Cmd completed with no output".to_string()
            } else {
                result.to_string()
            }
        ))
    }

    /// Check if tool name is in special tools list
    fn is_special_tool(&self, name: &str) -> bool {
        self.special_tool_names
            .iter()
            .any(|n| n.to_lowercase() == name.to_lowercase())
    }
}

#[async_trait]
impl super::Agent for ToolCallAgent {
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
        self.current_tool_calls.clear();
        self.tools.cleanup().await.map_err(Error::Tool)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::error::ToolError;
    use crate::llm::MockLlmClient;
    use crate::tool::Tool;
    use crate::tool::ToolResult;
    use async_trait::async_trait;

    // Mock tool for testing
    struct MockTool {
        name: &'static str,
        description: &'static str,
    }

    impl MockTool {
        fn new(name: &'static str, description: &'static str) -> Self {
            Self { name, description }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &'static str {
            self.name
        }

        fn description(&self) -> &'static str {
            self.description
        }

        async fn execute(
            &self,
            input: &str,
            _ctx: &mut Context,
        ) -> std::result::Result<ToolResult, ToolError> {
            Ok(ToolResult::success(format!("Mock execution: {}", input)))
        }
    }

    fn create_test_tools() -> Arc<ToolCollection> {
        let mut collection = ToolCollection::new();
        collection.add_tool(Arc::new(MockTool::new("bash", "Execute bash commands")));
        collection.add_tool(Arc::new(MockTool::new("terminate", "End execution")));
        Arc::new(collection)
    }

    fn create_test_agent() -> ToolCallAgent {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let tools = create_test_tools();
        ToolCallAgent::new("toolcall_test", "A tool call test agent", llm, tools)
    }

    #[test]
    fn test_toolcall_agent_new() {
        let agent = create_test_agent();
        assert_eq!(agent.name(), "toolcall_test");
        assert_eq!(agent.description(), "A tool call test agent");
        assert_eq!(agent.state(), AgentState::Idle);
    }

    #[test]
    fn test_toolcall_agent_has_tools() {
        let agent = create_test_agent();
        assert!(!agent.tools().is_empty());
        assert!(agent.tools().has_tool("bash"));
        assert!(agent.tools().has_tool("terminate"));
    }

    #[test]
    fn test_toolcall_agent_special_tool_names() {
        let agent = create_test_agent();
        assert!(agent.special_tool_names.contains(&"terminate".to_string()));
    }

    #[tokio::test]
    async fn test_toolcall_agent_think_no_tools() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        llm.set_text_response("I will help you.");

        let tools = create_test_tools();
        let mut agent = ToolCallAgent::new("test", "test agent", llm, tools);
        agent.base.update_memory(Message::user("Hello"));

        let result = agent.think().await;
        assert!(result.is_ok());
        assert!(agent.current_tool_calls.is_empty());
    }

    #[tokio::test]
    async fn test_toolcall_agent_act_no_tools() {
        let mut agent = create_test_agent();
        agent
            .base
            .update_memory(Message::assistant("Just a response"));

        let result = agent.act().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Just a response");
    }

    #[tokio::test]
    async fn test_toolcall_agent_cleanup() {
        let mut agent = create_test_agent();
        agent.base.update_memory(Message::user("test"));
        agent
            .current_tool_calls
            .push(ToolCall::new("id1", "bash", "{}"));

        agent.cleanup().await.unwrap();
        assert!(agent.base.memory().is_empty());
        assert!(agent.current_tool_calls.is_empty());
    }

    #[test]
    fn test_is_special_tool() {
        let agent = create_test_agent();
        assert!(agent.is_special_tool("terminate"));
        assert!(agent.is_special_tool("TERMINATE")); // Case insensitive
        assert!(!agent.is_special_tool("bash"));
    }
}
