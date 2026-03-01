# OpenManus Rust Architecture

## Overview

OpenManus Rust is a complete rewrite of the Python OpenManus project, implementing an AI agent framework with ReAct-style reasoning, tool calling, and multi-agent orchestration.

## Core Design Principles

1. **Async-first**: All I/O operations are async using Tokio
2. **Trait-based abstractions**: Core components are defined as traits for testability
3. **Thread-safe**: All shared state uses Arc<RwLock> or DashMap
4. **Error handling**: Using anyhow/thiserror for rich error context
5. **Testability**: All components are mockable for unit testing

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                          CLI Layer                               │
│                         (main.rs)                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────┐                                         │
│  │     Manus        │     ┌──────────────────┐               │
│  │     Agent        │────▶│  Flow            │               │
│  └──────────────────┘     │  Orchestrator    │               │
│          │                └──────────────────┘               │
│          │                         │                          │
│          ▼                         ▼                          │
│  ┌──────────────────┐     ┌──────────────────┐               │
│  │   ToolCall       │     │   LLM Client     │               │
│  │   Agent          │     │   (trait)        │               │
│  └──────────────────┘     └──────────────────┘               │
│          │                         │                          │
│          ▼                         ├── HttpLlmClient          │
│  ┌──────────────────┐              └── MockLlmClient          │
│  │   Tool           │                                         │
│  │   Collection     │                                         │
│  └──────────────────┘                                         │
│          │                                                     │
│          ├── BashTool                                         │
│          ├── PythonTool                                       │
│          ├── FileTool                                         │
│          ├── TerminateTool                                    │
│          └── AskHumanTool                                     │
│                                                                │
├─────────────────────────────────────────────────────────────────┤
│                      Core Components                            │
│                                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐      │
│  │  Schema  │  │ Context  │  │  Memory  │  │  Error   │      │
│  │          │  │          │  │          │  │          │      │
│  │ Message  │  │ Working  │  │ Short    │  │ Tool     │      │
│  │ Role     │  │ Directory│  │ Term     │  │ Error    │      │
│  │ ToolCall │  │ Env      │  │ Long     │  │ LLM      │      │
│  │ Agent    │  │ State    │  │ Term     │  │ Error    │      │
│  │ State    │  │          │  │          │  │          │      │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘      │
│                                                                │
└─────────────────────────────────────────────────────────────────┘
```

## Agent Hierarchy

```rust
// Base trait for all agents
trait Agent {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn state(&self) -> AgentState;
    async fn run(&mut self, request: &str) -> Result<String>;
    async fn cleanup(&mut self) -> Result<()>;
}

// Implementation hierarchy
BaseAgent          // Core state management, memory, step counting
  └── ReActAgent   // Adds think() and act() cycle
        └── ToolCallAgent  // Adds tool calling capability
              └── Manus    // Main agent with default tools
```

## Tool System

```rust
// All tools implement this trait
#[async_trait]
trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self) -> Option<ToolSchema>;
    async fn execute(&self, input: &str, ctx: &mut Context) -> Result<ToolResult, ToolError>;
}
```

Tools are collected in a `ToolCollection` which provides:
- Tool registration and lookup
- Conversion to OpenAI function calling format
- Execution with error handling

## LLM Client

```rust
// LLM client abstraction
#[async_trait]
trait LlmClient: Send + Sync {
    async fn completion(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError>;
    async fn stream_completion(&self, request: &LlmRequest) -> Result<LlmStream, LlmError>;
    fn model(&self) -> &str;
}
```

Implementations:
- `HttpLlmClient`: OpenAI-compatible HTTP client with retry logic
- `MockLlmClient`: Test mock with configurable responses

## Context and Memory

### Context
Execution context passed to tools containing:
- Working directory
- Environment variables
- Shared state (key-value store)
- Reference to tools
- Conversation messages
- Timeout and resource limits

### Memory
Two-tier memory system:
- `ShortTermMemory`: Ring buffer for recent messages
- `LongTermMemory`: Persistent storage (trait, can be file/DB)

## Flow Orchestration

The `PlanningFlow` decomposes complex tasks:
1. Creates a plan using LLM
2. Assigns steps to appropriate agents
3. Executes steps sequentially
4. Aggregates results

## Sandboxed Execution

Two sandbox options:
1. `LocalSandbox`: Process isolation with resource limits
2. `WasmSandbox` (optional): WASM-based isolation using wasmtime

## Error Handling

```rust
// Hierarchical error types
enum Error {
    Tool(ToolError),
    Llm(LlmError),
    Agent(String),
    Context(String),
    Sandbox(String),
    // ...
}

enum ToolError {
    NotFound(String),
    ExecutionFailed(String),
    InvalidInput(String),
    // ...
}

enum LlmError {
    ApiError(String),
    AuthFailed(String),
    RateLimitExceeded(String),
    // ...
}
```

## Configuration

Configuration is managed via:
1. Environment variables (OPENMANUS_*)
2. Command-line arguments
3. Config file (future: TOML-based)

## Testing Strategy

1. **Unit Tests**: Each module has `#[cfg(test)]` tests
2. **Integration Tests**: `tests/` directory with mock LLM
3. **E2E Tests**: Full agent runs with headless browser
4. **Fuzz Tests**: `fuzz/` directory with cargo-fuzz targets
5. **Property Tests**: Using proptest for invariants

## Performance Considerations

1. **Async I/O**: All network calls are async
2. **Streaming**: LLM responses support streaming
3. **Connection Pooling**: reqwest client reused
4. **Memory Limits**: Sandbox enforces limits
5. **Lazy Loading**: Browser only loaded when needed

## Security

1. **Sandbox Isolation**: Untrusted code runs in sandbox
2. **Path Validation**: File tool validates paths
3. **Network Control**: Sandbox can disable network
4. **Audit**: cargo-audit in CI pipeline
