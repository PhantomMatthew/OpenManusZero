# SweAgent Design

## Overview

SweAgent is a specialized agent for autonomous programming tasks. It extends ToolCallAgent with tools for file editing and command execution, enabling it to solve software engineering problems independently.

## Structure

```rust
pub struct SweAgent {
    agent: ToolCallAgent,
}
```

## Configuration

| Aspect | Value |
|--------|-------|
| Name | "swe" |
| Description | "an autonomous AI programmer that interacts directly with the computer to solve tasks" |
| Tools | BashTool, StrReplaceEditor, TerminateTool |
| Max Steps | 20 |
| Next Step Prompt | "" (empty) |
| Special Tool Names | ["terminate"] |

## System Prompt

```
SETTING: You are an autonomous programmer, and you're working directly in the command line with a special interface.

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
```

## Files

- Create: `src/agent/swe.rs`
- Modify: `src/agent/mod.rs`

## Constructor Pattern

```rust
impl SweAgent {
    pub async fn new() -> Result<Self>;
    pub fn with_llm(llm: Arc<dyn LlmClient>) -> Self;
    fn create_default_tools() -> Arc<ToolCollection>;
    pub fn inner(&self) -> &ToolCallAgent;
    pub fn inner_mut(&mut self) -> &mut ToolCallAgent;
}
```

## Agent Trait Implementation

```rust
#[async_trait]
impl Agent for SweAgent {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn state(&self) -> AgentState;
    async fn run(&mut self, request: &str) -> Result<String>;
    async fn cleanup(&mut self) -> Result<()>;
}
```

## Tests

- `test_swe_new` - creates agent with default config
- `test_swe_with_llm` - creates agent with custom LLM
- `test_swe_has_tools` - verifies all 3 tools are present

## Dependencies

- `crate::agent::toolcall::ToolCallAgent`
- `crate::tool::{BashTool, StrReplaceEditor, TerminateTool, ToolCollection}`
- `crate::llm::LlmClient`
- `async_trait`
- `std::sync::Arc`
