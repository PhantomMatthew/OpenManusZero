//! MCP Agent implementation
//!
//! Agent that connects to MCP servers and uses their tools.

use crate::agent::base::BaseAgent;
use crate::context::Context;
use crate::error::{Error, Result};
use crate::llm::{LlmClient, LlmRequest, ToolChoice};
use crate::mcp::client::McpClientManager;
use crate::mcp::types::{McpClientConfig, TransportType};
use crate::schema::{AgentState, Message, ToolCall};
use crate::tool::ToolCollection;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// System prompt for MCP agent
const SYSTEM_PROMPT: &str = r#"You are an AI agent connected to MCP (Model Context Protocol) servers.
You have access to tools provided by these servers to complete tasks.
Use the available tools effectively and report any issues with tool execution."#;

const NEXT_STEP_PROMPT: &str = "What should I do next?";

/// Agent that connects to MCP servers and uses their tools
pub struct McpAgent {
    /// Base agent
    base: BaseAgent,
    /// MCP client manager
    mcp_manager: McpClientManager,
    /// Tool collection from MCP servers
    tools: Arc<ToolCollection>,
    /// Tool schemas for change detection
    tool_schemas: HashMap<String, serde_json::Value>,
    /// Current tool calls from last think
    current_tool_calls: Vec<ToolCall>,
    /// Refresh tools every N steps
    refresh_tools_interval: usize,
    /// Maximum observation length
    max_observe: Option<usize>,
}

impl McpAgent {
    /// Create a new MCP agent
    pub fn new(llm: Arc<dyn LlmClient>) -> Self {
        Self {
            base: BaseAgent::new(
                "mcp_agent",
                "An agent that connects to MCP servers and uses their tools",
                llm,
            )
            .with_system_prompt(SYSTEM_PROMPT)
            .with_next_step_prompt(NEXT_STEP_PROMPT)
            .with_max_steps(20),
            mcp_manager: McpClientManager::new(),
            tools: Arc::new(ToolCollection::new()),
            tool_schemas: HashMap::new(),
            current_tool_calls: Vec::new(),
            refresh_tools_interval: 5,
            max_observe: Some(10000),
        }
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

    /// Set refresh tools interval
    pub fn with_refresh_interval(mut self, interval: usize) -> Self {
        self.refresh_tools_interval = interval;
        self
    }

    /// Set max observe length
    pub fn with_max_observe(mut self, max: usize) -> Self {
        self.max_observe = Some(max);
        self
    }

    /// Connect to an MCP server via stdio
    pub async fn connect_stdio(
        &mut self,
        command: impl Into<String>,
        args: Vec<String>,
        server_id: Option<String>,
    ) -> Result<()> {
        let config = McpClientConfig {
            server_id: server_id.unwrap_or_default(),
            transport: TransportType::Stdio {
                command: command.into(),
                args,
            },
            ..Default::default()
        };
        self.connect(config).await
    }

    /// Connect to an MCP server via SSE
    pub async fn connect_sse(
        &mut self,
        url: impl Into<String>,
        server_id: Option<String>,
    ) -> Result<()> {
        let config = McpClientConfig {
            server_id: server_id.unwrap_or_default(),
            transport: TransportType::Sse {
                url: url.into(),
                headers: HashMap::new(),
            },
            ..Default::default()
        };
        self.connect(config).await
    }

    /// Connect to an MCP server via WebSocket
    pub async fn connect_websocket(
        &mut self,
        url: impl Into<String>,
        server_id: Option<String>,
    ) -> Result<()> {
        let config = McpClientConfig {
            server_id: server_id.unwrap_or_default(),
            transport: TransportType::WebSocket {
                url: url.into(),
                headers: HashMap::new(),
            },
            ..Default::default()
        };
        self.connect(config).await
    }

    /// Connect to an MCP server with the given configuration
    pub async fn connect(&mut self, config: McpClientConfig) -> Result<()> {
        self.mcp_manager
            .connect(config)
            .await
            .map_err(|e| Error::Agent(format!("MCP connection failed: {}", e)))?;

        // Refresh tools after connection
        self.refresh_tools().await?;

        // Update system prompt with available tools info
        self.update_tools_info();

        Ok(())
    }

    /// Refresh tools from all connected MCP servers
    async fn refresh_tools(&mut self) -> Result<(Vec<String>, Vec<String>)> {
        // Get current tool schemas
        let all_tools = self
            .mcp_manager
            .list_all_tools()
            .await
            .map_err(|e| Error::Agent(format!("Failed to list tools: {}", e)))?;

        let mut current_schemas = HashMap::new();
        for (server_id, tools) in &all_tools {
            for tool in tools {
                let key = format!("mcp_{}_{}", server_id, tool.name);
                current_schemas.insert(key, tool.input_schema.clone());
            }
        }

        let current_names: std::collections::HashSet<String> =
            current_schemas.keys().cloned().collect();
        let previous_names: std::collections::HashSet<String> =
            self.tool_schemas.keys().cloned().collect();

        let added: Vec<String> = current_names.difference(&previous_names).cloned().collect();
        let removed: Vec<String> = previous_names.difference(&current_names).cloned().collect();

        // Update stored schemas
        self.tool_schemas = current_schemas;

        // Update tool collection
        self.tools = Arc::new(self.mcp_manager.to_tool_collection());

        Ok((added, removed))
    }

    /// Update system prompt with available tools info
    fn update_tools_info(&mut self) {
        let tool_names: Vec<&str> = self.tools.tools().iter().map(|t| t.name()).collect();
        let tools_info = tool_names.join(", ");

        let prompt = format!("{}\n\nAvailable MCP tools: {}", SYSTEM_PROMPT, tools_info);
        self.base.set_system_prompt(&prompt);
    }

    /// Get reference to MCP client manager
    pub fn mcp_manager(&self) -> &McpClientManager {
        &self.mcp_manager
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
        // Check if MCP sessions are still active
        if self.mcp_manager.clients().is_empty() {
            tracing::info!("MCP service is no longer available, ending interaction");
            self.base.set_state(AgentState::Finished);
            return Ok(false);
        }

        // Refresh tools periodically
        if self.base.current_step() > 0
            && self.base.current_step() % self.refresh_tools_interval == 0
        {
            let (added, removed) = self.refresh_tools().await?;

            if !added.is_empty() {
                tracing::info!("Added MCP tools: {:?}", added);
                self.base.update_memory(Message::system(format!(
                    "New tools available: {}",
                    added.join(", ")
                )));
            }
            if !removed.is_empty() {
                tracing::info!("Removed MCP tools: {:?}", removed);
                self.base.update_memory(Message::system(format!(
                    "Tools no longer available: {}",
                    removed.join(", ")
                )));
            }

            // All tools removed indicates shutdown
            if self.tools.is_empty() {
                tracing::info!("MCP service has shut down, ending interaction");
                self.base.set_state(AgentState::Finished);
                return Ok(false);
            }
        }

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

        tracing::info!("MCP Agent thoughts: {}", content);
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

            tracing::info!("MCP Tool '{}' completed", tool_call.function.name);

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

        tracing::info!("Executing MCP tool: '{}' with args: {}", name, args);

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

    /// Disconnect from all MCP servers
    pub async fn disconnect(&mut self) {
        self.mcp_manager.disconnect_all().await;
        tracing::info!("Disconnected from all MCP servers");
    }
}

#[async_trait]
impl super::Agent for McpAgent {
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
        // Disconnect from MCP servers
        self.disconnect().await;

        // Clean up base
        self.base.clear_memory();
        self.base.set_state(AgentState::Idle);
        self.base.reset_steps();
        self.current_tool_calls.clear();
        self.tool_schemas.clear();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::llm::MockLlmClient;

    fn create_test_agent() -> McpAgent {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        McpAgent::new(llm)
    }

    #[test]
    fn test_mcp_agent_new() {
        let agent = create_test_agent();
        assert_eq!(agent.name(), "mcp_agent");
        assert_eq!(
            agent.description(),
            "An agent that connects to MCP servers and uses their tools"
        );
        assert_eq!(agent.state(), AgentState::Idle);
    }

    #[test]
    fn test_mcp_agent_has_no_tools_initially() {
        let agent = create_test_agent();
        assert!(agent.tools().is_empty());
    }

    #[test]
    fn test_mcp_agent_config_builders() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let agent = McpAgent::new(llm)
            .with_max_steps(50)
            .with_max_observe(5000)
            .with_refresh_interval(10);

        assert_eq!(agent.base().max_steps(), 50);
    }
}
