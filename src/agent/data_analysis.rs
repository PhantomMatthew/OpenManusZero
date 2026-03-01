//! Data Analysis Agent implementation
//!
//! An analytical agent that uses Python and visualization tools
//! to solve diverse data analysis tasks.

use crate::agent::base::BaseAgent;
use crate::context::Context;
use crate::error::{Error, Result};
use crate::llm::{LlmClient, LlmRequest, ToolChoice};
use crate::schema::{AgentState, Message, ToolCall};
use crate::tool::{ChartTool, FileTool, PythonTool, TerminateTool, ToolCollection};
use async_trait::async_trait;
use std::sync::Arc;

/// System prompt for data analysis agent
const SYSTEM_PROMPT: &str = r#"You are an AI agent designed to perform data analysis and visualization tasks. You have various tools at your disposal that you can call upon to efficiently complete complex requests.

# Your Capabilities
1. **Data Processing**: Use Python to load, clean, transform, and analyze data
2. **Data Visualization**: Create charts and visualizations to represent data insights
3. **File Operations**: Read from and write files in the workspace
4. **Report Generation**: Generate comprehensive analysis reports

# Available Tools
- `python`: Execute Python code for data analysis, calculations, and processing
- `chart`: Create visualizations (bar, line, pie charts, etc.) from data
- `file`: Read/write files in the workspace
- `terminate`: End the analysis when complete

# Guidelines
1. Always read data files first before processing
2. Clean and validate data before analysis
3. Use appropriate visualization types for different data
4. Generate a comprehensive analysis report at the end
5. Save all outputs (processed data, charts, reports) to the workspace
6. Handle errors gracefully and retry with corrections

# Analysis Workflow
1. **Understand**: Analyze the user's request and identify the data needed
2. **Load**: Read data from files or accept provided data
3. **Explore**: Perform initial data exploration (shape, types, statistics)
4. **Clean**: Handle missing values, outliers, and data type issues
5. **Analyze**: Perform the requested analysis
6. **Visualize**: Create appropriate charts and visualizations
7. **Report**: Generate a comprehensive report with findings

Remember: Always explain your reasoning and the steps you're taking."#;

/// Next step prompt
const NEXT_STEP_PROMPT: &str = r#"Based on the current state of analysis, determine the next best action.

# Decision Guide
1. If data needs to be loaded -> Use file tool to read files
2. If data needs processing -> Use python tool for analysis
3. If visualization is needed -> Use chart tool
4. If results need to be saved -> Use file tool to write files
5. If analysis is complete -> Use terminate tool

# Note
1. Each step should select the most appropriate tool (ONLY ONE)
2. After each tool execution, explain results and suggest next steps
3. If an error occurs, review and fix the issue before retrying
4. Keep track of the overall analysis progress"#;

/// Data Analysis Agent
///
/// An analytical agent that utilizes Python and data visualization tools
/// to solve diverse data analysis tasks.
pub struct DataAnalysisAgent {
    /// Base agent
    base: BaseAgent,
    /// Tool collection
    tools: Arc<ToolCollection>,
    /// Current tool calls from last think
    current_tool_calls: Vec<ToolCall>,
    /// Maximum observation length
    max_observe: Option<usize>,
    /// Working directory
    working_dir: String,
}

impl DataAnalysisAgent {
    /// Create a new data analysis agent
    pub fn new(llm: Arc<dyn LlmClient>) -> Self {
        Self::with_working_dir(llm, "./workspace")
    }

    /// Create a new data analysis agent with custom working directory
    pub fn with_working_dir(llm: Arc<dyn LlmClient>, working_dir: impl Into<String>) -> Self {
        // Create default tool collection for data analysis
        let tools = Arc::new(Self::create_default_tools());

        let base = BaseAgent::new(
            "data_analysis",
            "An analytical agent that utilizes Python and data visualization tools to solve diverse data analysis tasks",
            llm,
        )
        .with_system_prompt(SYSTEM_PROMPT)
        .with_next_step_prompt(NEXT_STEP_PROMPT)
        .with_max_steps(20);

        Self {
            base,
            tools,
            current_tool_calls: Vec::new(),
            max_observe: Some(15000),
            working_dir: working_dir.into(),
        }
    }

    /// Create with custom tools
    pub fn with_tools(llm: Arc<dyn LlmClient>, tools: Arc<ToolCollection>) -> Self {
        let base = BaseAgent::new(
            "data_analysis",
            "An analytical agent that utilizes Python and data visualization tools to solve diverse data analysis tasks",
            llm,
        )
        .with_system_prompt(SYSTEM_PROMPT)
        .with_next_step_prompt(NEXT_STEP_PROMPT)
        .with_max_steps(20);

        Self {
            base,
            tools,
            current_tool_calls: Vec::new(),
            max_observe: Some(15000),
            working_dir: "./workspace".to_string(),
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

    /// Set working directory
    pub fn with_working_dir_path(mut self, path: impl Into<String>) -> Self {
        self.working_dir = path.into();
        self
    }

    /// Create default tools for data analysis
    fn create_default_tools() -> ToolCollection {
        let mut collection = ToolCollection::new();
        collection.add_tool(Arc::new(PythonTool::new()));
        collection.add_tool(Arc::new(ChartTool::new()));
        collection.add_tool(Arc::new(FileTool::new()));
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

    /// Get working directory
    pub fn working_dir(&self) -> &str {
        &self.working_dir
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

        tracing::info!("DataAnalysis Agent thoughts: {}", content);
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
        ctx.working_dir = self.working_dir.clone().into();
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

            tracing::info!("DataAnalysis Tool '{}' completed", tool_call.function.name);

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

        tracing::info!("Executing DataAnalysis tool: '{}' with args: {}", name, args);

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
impl super::Agent for DataAnalysisAgent {
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

    fn create_test_agent() -> DataAnalysisAgent {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        DataAnalysisAgent::new(llm)
    }

    #[test]
    fn test_data_analysis_agent_new() {
        let agent = create_test_agent();
        assert_eq!(agent.name(), "data_analysis");
        assert_eq!(
            agent.description(),
            "An analytical agent that utilizes Python and data visualization tools to solve diverse data analysis tasks"
        );
        assert_eq!(agent.state(), AgentState::Idle);
    }

    #[test]
    fn test_data_analysis_agent_has_tools() {
        let agent = create_test_agent();
        // Should have 4 default tools
        assert!(!agent.tools().is_empty());
    }

    #[test]
    fn test_data_analysis_agent_config_builders() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let agent = DataAnalysisAgent::new(llm)
            .with_max_steps(50)
            .with_max_observe(20000)
            .with_working_dir_path("/data/workspace");

        assert_eq!(agent.base().max_steps(), 50);
        assert_eq!(agent.working_dir(), "/data/workspace");
    }

    #[test]
    fn test_create_default_tools() {
        let tools = DataAnalysisAgent::create_default_tools();
        assert!(tools.get_tool("python").is_some());
        assert!(tools.get_tool("chart").is_some());
        assert!(tools.get_tool("file").is_some());
        assert!(tools.get_tool("terminate").is_some());
    }
}
