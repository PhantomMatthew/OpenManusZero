//! SweAgent - autonomous software engineering agent

use crate::agent::toolcall::ToolCallAgent;
use crate::error::Result;
use crate::llm::LlmClient;
use crate::schema::AgentState;
use crate::tool::{Tool, ToolCollection};
use async_trait::async_trait;
use std::sync::Arc;

/// System prompt for SWE agent
const SWE_SYSTEM_PROMPT: &str = r#"SETTING: You are an autonomous programmer, and you're working directly in the command line with a special interface.

The special interface consists of a file editor that shows you 100 lines of a file at a time.
In addition to typical bash commands, you can also use specific commands to help you navigate and edit files.
To call a command, you need to invoke it with a function call/tool call.

Please note that THE EDIT COMMAND REQUIRES PROPER INDENTATION.
If you'd like to add the line '        print(x)' you must fully write that out, with all those spaces before the code! Indentation is important and code that is not indented correctly will fail and require fixing before it can be run.

RESPONSE FORMAT:
Your shell prompt is formatted as follows:
(Open file: <path>)
(Current directory: <cwd>)
bash-$

First, you should _always_ include a general thought about what you're going to do next.
Then, for every response, you must include exactly _ONE_ tool call/function call.

Remember, you should always include a _SINGLE_ tool call/function call and then wait for a response from the shell before continuing with more discussion and commands. Everything you include in the DISCUSSION section will be saved for future reference.
If you'd like to issue two commands at once, PLEASE DO NOT DO THAT! Please instead first submit just the first tool call, and then after receiving a response you'll be able to issue the second tool call.
Note that the environment does NOT support interactive session commands (e.g. python, vim), so please do not invoke them.
"#;

/// SweAgent - autonomous software engineering agent
pub struct SweAgent {
    agent: ToolCallAgent,
}

impl SweAgent {
    /// Create a new SweAgent with default configuration
    pub async fn new() -> Result<Self> {
        let tools = Self::create_default_tools();
        let llm = Arc::new(crate::llm::MockLlmClient::new("gpt-4"));

        let agent = ToolCallAgent::new(
            "swe",
            "an autonomous AI programmer that interacts directly with the computer to solve tasks",
            llm,
            tools,
        )
        .with_system_prompt(SWE_SYSTEM_PROMPT)
        .with_next_step_prompt("")
        .with_max_steps(20);

        Ok(Self { agent })
    }

    /// Create a SweAgent with a custom LLM client
    pub fn with_llm(llm: Arc<dyn LlmClient>) -> Self {
        let tools = Self::create_default_tools();

        let agent = ToolCallAgent::new(
            "swe",
            "an autonomous AI programmer that interacts directly with the computer to solve tasks",
            llm,
            tools,
        )
        .with_system_prompt(SWE_SYSTEM_PROMPT)
        .with_next_step_prompt("")
        .with_max_steps(20);

        Self { agent }
    }

    /// Create the default tool collection for SWE agent
    fn create_default_tools() -> Arc<ToolCollection> {
        let mut collection = ToolCollection::new();

        collection.add_tool(Arc::new(crate::tool::bash::BashTool::new()) as Arc<dyn Tool>);
        collection.add_tool(Arc::new(crate::tool::str_replace_editor::StrReplaceEditor::new()) as Arc<dyn Tool>);
        collection.add_tool(Arc::new(crate::tool::terminate::TerminateTool::new()) as Arc<dyn Tool>);

        Arc::new(collection)
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
impl super::Agent for SweAgent {
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
        tracing::info!("SweAgent starting with request: {}", request);
        self.agent.run(request).await
    }

    async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("SweAgent cleaning up");
        self.agent.cleanup().await
    }
}
