# SweAgent Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a specialized agent for autonomous programming tasks using Bash, StrReplaceEditor, and Terminate tools.

**Architecture:** SweAgent wraps ToolCallAgent with a specialized system prompt for autonomous programming. Follows the same pattern as Manus agent but with different tools (bash, str_replace_editor, terminate) and an empty next_step_prompt.

**Tech Stack:** Rust, tokio (async), async-trait, std::sync::Arc

---

## Task 1: Create SweAgent Skeleton

**Files:**
- Create: `src/agent/swe.rs`
- Modify: `src/agent/mod.rs`

**Step 1: Create src/agent/swe.rs with skeleton**

```rust
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
```

**Step 2: Modify src/agent/mod.rs to export SweAgent**

Add after the existing imports:
```rust
pub mod swe;
pub use swe::SweAgent;
```

Update the prelude module:
```rust
pub mod prelude {
    pub use super::base::{Agent, BaseAgent};
    pub use super::manus::Manus;
    pub use super::react::ReActAgent;
    pub use super::toolcall::ToolCallAgent;
    pub use super::swe::SweAgent;  // Add this line
    #[cfg(feature = "mcp")]
    pub use super::mcp::McpAgent;
    pub use crate::schema::AgentState;
}
```

**Step 3: Run cargo check to verify compilation**

Run: `cargo check 2>&1 | head -30`
Expected: No errors (warnings about unused code are OK)

**Step 4: Commit**

```bash
git add src/agent/swe.rs src/agent/mod.rs && git commit -m "feat(agent): add SweAgent skeleton"
```

---

## Task 2: Add Unit Tests

**Files:**
- Modify: `src/agent/swe.rs`

**Step 1: Add test module to swe.rs**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::llm::MockLlmClient;

    #[tokio::test]
    async fn test_swe_new() {
        let swe = SweAgent::new().await;
        assert!(swe.is_ok());
        let swe = swe.unwrap();
        assert_eq!(swe.name(), "swe");
    }

    #[test]
    fn test_swe_with_llm() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let swe = SweAgent::with_llm(llm);
        assert_eq!(swe.name(), "swe");
        assert_eq!(swe.state(), AgentState::Idle);
    }

    #[test]
    fn test_swe_has_tools() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let swe = SweAgent::with_llm(llm);
        assert!(!swe.agent.tools().is_empty());
        assert!(swe.agent.tools().has_tool("bash"));
        assert!(swe.agent.tools().has_tool("str_replace_editor"));
        assert!(swe.agent.tools().has_tool("terminate"));
    }

    #[tokio::test]
    async fn test_swe_run() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        llm.set_text_response("I will help you fix the bug.");

        let mut swe = SweAgent::with_llm(llm);
        let result = swe.run("Fix the bug in main.rs").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_swe_cleanup() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let mut swe = SweAgent::with_llm(llm);
        swe.agent.base_mut().update_memory(crate::schema::Message::user("test"));

        swe.cleanup().await.unwrap();
        assert!(swe.agent.base().memory().is_empty());
    }
}
```

**Step 2: Run tests to verify they pass**

Run: `cargo test swe -- --nocapture`
Expected: All 5 tests pass

**Step 3: Commit**

```bash
git add src/agent/swe.rs && git commit -m "feat(agent): add SweAgent unit tests"
```

---

## Task 3: Update pending_tasks.md

**Files:**
- Modify: `pending_tasks.md`

**Step 1: Mark SWEAgent as complete**

Change the line:
```
| SWEAgent | `app/agent/swe.py` | 软件工程代理，代码修复 | 高 |
```
to:
```
| SWEAgent | `app/agent/swe.py` | 软件工程代理，代码修复 | ✅ |
```

**Step 2: Commit**

```bash
git add pending_tasks.md && git commit -m "docs: mark SWEAgent as complete"
```

---

## Summary

| Task | Description | Tests |
|------|-------------|-------|
| 1 | Create SweAgent skeleton | 0 |
| 2 | Add unit tests | 5 |
| 3 | Update documentation | 0 |

**Total: 5 tests**
