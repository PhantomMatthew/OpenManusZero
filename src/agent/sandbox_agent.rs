//! Sandbox Agent implementation
//!
//! An agent that operates in sandboxed environments with tools for:
//! - Shell command execution
//! - File operations
//! - Browser automation
//! - Vision/screen capture

use crate::agent::base::BaseAgent;
use crate::context::Context;
use crate::error::{Error, Result};
use crate::llm::{LlmClient, LlmRequest, ToolChoice};
use crate::schema::{AgentState, Message, ToolCall};
use crate::tool::{
    SandboxBrowserTool, SandboxFileTool, SandboxShellTool, SandboxVisionTool, TerminateTool,
    ToolCollection,
};
use async_trait::async_trait;
use std::sync::Arc;

/// System prompt for sandbox agent
const SYSTEM_PROMPT: &str = r#"You are an AI agent designed to operate in sandboxed environments. You have access to various tools for executing commands, manipulating files, browsing the web, and capturing screenshots.

# Your Capabilities
1. **Shell Execution**: Execute shell commands in an isolated sandbox environment
2. **File Operations**: Read, write, list, and remove files in the sandbox
3. **Browser Automation**: Open browsers, click elements, and type text
4. **Vision**: Capture screenshots of the sandbox environment

# Available Tools
- `sandbox_shell`: Execute shell commands in the sandbox
- `sandbox_file`: File operations (read, write, list, remove)
- `sandbox_browser`: Browser automation (open, click, type)
- `sandbox_vision`: Capture screenshots
- `terminate`: End the execution when task is complete

# Guidelines
1. Always verify the sandbox environment before executing commands
2. Handle file paths carefully - use absolute paths when possible
3. Check command outputs for errors before proceeding
4. Use browser tools for web-related tasks
5. Capture screenshots to verify visual states
6. Clean up temporary files when done

# Workflow
1. **Understand**: Analyze the user's request and plan the steps
2. **Execute**: Use appropriate tools to complete each step
3. **Verify**: Check outputs and screenshots to confirm success
4. **Report**: Provide clear summary of actions taken

Remember: Work carefully in the sandbox environment and report any issues encountered."#;

/// Next step prompt
const NEXT_STEP_PROMPT: &str = r#"Based on the current state, determine the next best action.

# Decision Guide
1. If shell command needed -> Use sandbox_shell tool
2. If file operations needed -> Use sandbox_file tool
3. If browser automation needed -> Use sandbox_browser tool
4. If visual verification needed -> Use sandbox_vision tool
5. If task is complete -> Use terminate tool

# Note
1. Each step should select the most appropriate tool (ONLY ONE)
2. Review outputs carefully before proceeding to next step
3. Handle errors gracefully and retry with corrections
4. Keep track of overall progress toward the goal"#;

/// Sandbox Agent
///
/// An agent that operates in sandboxed environments with tools for
/// shell execution, file operations, browser automation, and vision.
pub struct SandboxAgent {
    /// Base agent
    base: BaseAgent,
    /// Tool collection
    tools: Arc<ToolCollection>,
    /// Current tool calls from last think
    current_tool_calls: Vec<ToolCall>,
    /// Maximum observation length
    max_observe: Option<usize>,
    /// Sandbox ID (if using a specific sandbox)
    sandbox_id: Option<String>,
}

impl SandboxAgent {
    /// Create a new sandbox agent
    pub fn new(llm: Arc<dyn LlmClient>) -> Self {
        Self::with_sandbox_id(llm, None)
    }

    /// Create a new sandbox agent with a specific sandbox ID
    pub fn with_sandbox_id(llm: Arc<dyn LlmClient>, sandbox_id: Option<String>) -> Self {
        let tools = Arc::new(Self::create_default_tools());

        let base = BaseAgent::new(
            "sandbox_agent",
            "An agent that operates in sandboxed environments with shell, file, browser, and vision tools",
            llm,
        )
        .with_system_prompt(SYSTEM_PROMPT)
        .with_next_step_prompt(NEXT_STEP_PROMPT)
        .with_max_steps(30);

        Self {
            base,
            tools,
            current_tool_calls: Vec::new(),
            max_observe: Some(10000),
            sandbox_id,
        }
    }

    /// Create with custom tools
    pub fn with_tools(llm: Arc<dyn LlmClient>, tools: Arc<ToolCollection>) -> Self {
        let base = BaseAgent::new(
            "sandbox_agent",
            "An agent that operates in sandboxed environments with shell, file, browser, and vision tools",
            llm,
        )
        .with_system_prompt(SYSTEM_PROMPT)
        .with_next_step_prompt(NEXT_STEP_PROMPT)
        .with_max_steps(30);

        Self {
            base,
            tools,
            current_tool_calls: Vec::new(),
            max_observe: Some(10000),
            sandbox_id: None,
        }
    }

    /// Set max steps
    pub fn with_max_steps(mut self, steps: usize) -> Self {
        self.base = self.base.with_max_steps(steps);
        self
    }

    /// Set max observe length
    pub fn with_max_observe(mut self, max: usize) -> Self {
        self.max_observe = Some(max);
        self
    }

    /// Set system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.base = self.base.with_system_prompt(prompt);
        self
    }

    /// Set the sandbox ID
    pub fn set_sandbox_id(&mut self, id: impl Into<String>) {
        self.sandbox_id = Some(id.into());
    }

    /// Get the sandbox ID
    pub fn sandbox_id(&self) -> Option<&str> {
        self.sandbox_id.as_deref()
    }

    /// Create default tools for sandbox operations
    fn create_default_tools() -> ToolCollection {
        let mut collection = ToolCollection::new();
        collection.add_tool(Arc::new(SandboxShellTool::new()));
        collection.add_tool(Arc::new(SandboxFileTool::new()));
        collection.add_tool(Arc::new(SandboxBrowserTool::new()));
        collection.add_tool(Arc::new(SandboxVisionTool::new()));
        collection.add_tool(Arc::new(TerminateTool::new()));
        collection
    }

    /// Get reference to tools
    pub fn tools(&self) -> &ToolCollection {
        &self.tools
    }

    /// Get reference to base agent
    pub fn base(&self) -> &BaseAgent {
        &self.base
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
            // Include sandbox ID in system prompt if available
            let system_content = if let Some(id) = &self.sandbox_id {
                format!("{}\n\n# Current Sandbox\nSandbox ID: {}", system, id)
            } else {
                system.to_string()
            };
            messages.push(Message::system(&system_content));
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

        tracing::info!("Sandbox Agent thoughts: {}", content);
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

        // Set sandbox ID in context if available
        if let Some(id) = &self.sandbox_id {
            ctx.set_state("sandbox_id", id.clone()).ok();
        }

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

            tracing::info!("Sandbox Tool '{}' completed", tool_call.function.name);

            // Add tool result to memory
            self.base.update_memory(Message::tool_result(
                &result_str,
                &tool_call.function.name,
                &tool_call.id,
            ));

            results.push(result_str);

            // Check if this is a terminate tool
            if tool_call.function.name.to_lowercase() == "terminate" {
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

        tracing::info!("Executing Sandbox tool: '{}' with args: {}", name, args);

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
}

#[async_trait]
impl super::Agent for SandboxAgent {
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::llm::MockLlmClient;

    fn create_test_agent() -> SandboxAgent {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        SandboxAgent::new(llm)
    }

    #[test]
    fn test_sandbox_agent_new() {
        let agent = create_test_agent();
        assert_eq!(agent.name(), "sandbox_agent");
        assert_eq!(
            agent.description(),
            "An agent that operates in sandboxed environments with shell, file, browser, and vision tools"
        );
        assert_eq!(agent.state(), AgentState::Idle);
        assert!(agent.sandbox_id().is_none());
    }

    #[test]
    fn test_sandbox_agent_with_sandbox_id() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let agent = SandboxAgent::with_sandbox_id(llm, Some("sandbox-123".to_string()));

        assert_eq!(agent.sandbox_id(), Some("sandbox-123"));
    }

    #[test]
    fn test_sandbox_agent_set_sandbox_id() {
        let mut agent = create_test_agent();
        agent.set_sandbox_id("new-sandbox-456");

        assert_eq!(agent.sandbox_id(), Some("new-sandbox-456"));
    }

    #[test]
    fn test_sandbox_agent_has_tools() {
        let agent = create_test_agent();
        // Should have 5 default tools
        assert!(!agent.tools().is_empty());
    }

    #[test]
    fn test_sandbox_agent_config_builders() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let agent = SandboxAgent::new(llm)
            .with_max_steps(50)
            .with_max_observe(20000);

        assert_eq!(agent.base().max_steps(), 50);
    }

    #[test]
    fn test_create_default_tools() {
        let tools = SandboxAgent::create_default_tools();
        assert!(tools.get_tool("sandbox_shell").is_some());
        assert!(tools.get_tool("sandbox_file").is_some());
        assert!(tools.get_tool("sandbox_browser").is_some());
        assert!(tools.get_tool("sandbox_vision").is_some());
        assert!(tools.get_tool("terminate").is_some());
    }

    #[tokio::test]
    async fn test_sandbox_agent_cleanup() {
        let mut agent = create_test_agent();
        agent.base.update_memory(Message::user("test"));
        agent
            .current_tool_calls
            .push(ToolCall::new("id1", "sandbox_shell", r#"{"command": "ls"}"#));

        agent.cleanup().await.unwrap();
        assert!(agent.base.memory().is_empty());
        assert!(agent.current_tool_calls.is_empty());
    }
}
