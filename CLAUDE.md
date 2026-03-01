# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

OpenManus is a Rust-based AI agent framework that enables LLMs to autonomously solve tasks using various tools. It supports browser automation, code execution, file editing, web search, MCP (Model Context Protocol) integration, and computer use (screen capture, mouse/keyboard control).

## Common Commands

### Setup and Installation

```bash
# Toolchain is pinned in rust-toolchain.toml (1.72)
rustup show   # installs toolchain automatically

# Copy and edit config
cp config/config.example.toml config/config.toml
```

### Building

```bash
cargo build                          # debug build
cargo build --release                # release build
cargo build --features computer-use  # with computer use support
cargo build --all-features           # all features
```

### Running

```bash
cargo run -- --help                  # show help
cargo run -- run -p "Your task"      # single prompt
cargo run -- interactive             # interactive mode
RUST_LOG=debug cargo run             # with debug logging
```

### Testing and Code Quality

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt

# Check without building
cargo check --all-features
```

### Configuration

```bash
cp config/config.example.toml config/config.toml
# Edit with your API keys and settings
```

## Architecture

### Agent Hierarchy

The agent system follows an inheritance chain:

```
BaseAgent (src/agent/base.rs)
    └── ReActAgent (src/agent/react.rs)
            └── ToolCallAgent (src/agent/toolcall.rs)
                    └── Manus (src/agent/manus.rs)  - Main general-purpose agent
                    └── McpAgent (src/agent/mcp.rs)
                    └── SweAgent (src/agent/swe.rs)
```

- **BaseAgent**: Core state management (Idle/Running/Finished/Error), memory, step-based execution loop
- **ReActAgent**: Adds reasoning and acting cycle with `think()` and `act()` methods
- **ToolCallAgent**: Implements tool/function calling with OpenAI-compatible tool use
- **Manus**: Primary agent with default tool set (bash, python, file editing, web search, browser)

### Tool System

Tools are defined in `src/tool/` and implement the `Tool` trait:

- `BashTool`: Shell command execution
- `PythonTool`: Python code execution
- `StrReplaceEditor`: File editing with string replacement
- `AskHumanTool`: Request user input
- `TerminateTool`: End execution
- `ComputerUseTool`: Screen capture, mouse/keyboard control (feature-gated: `computer-use`)
- Search tools: `WebSearchTool` with Google, Bing, DuckDuckGo, Baidu backends
- `CrawlTool`: Web page content extraction

Tools are collected in a `ToolCollection` which provides `to_params()` for LLM function calling and `execute()` for dispatch.

### Flow System

Multi-agent orchestration in `src/flow/`:

- `BaseFlow`: Abstract base for orchestrating multiple agents
- `PlanningFlow`: Decomposes tasks into plans, assigns to specialized agents
- `FlowFactory`: Creates flow instances by type

### LLM Integration

`src/llm/` provides the `LlmClient` trait:

- Supports OpenAI, Azure OpenAI, AWS Bedrock, and Ollama
- Methods: `completion()`, `stream_completion()`
- Built-in token counting (tiktoken-rs) and retry logic
- `HttpLlmClient`: Production HTTP client
- `MockLlmClient`: Test mock with configurable responses

### MCP Integration

Model Context Protocol support in `src/protocol/`:

- MCP server: exposes OpenManus tools to external MCP clients
- MCP client: connects to external MCP servers
- Transport types: `stdio` (implemented), `sse`/`websocket` (planned)
- `McpAgent`: agent that uses MCP server tools with LLM reasoning

### Configuration System

`src/config.rs` loads from `config/config.toml`:

- LLM settings (model, API keys, endpoints, temperature)
- Browser settings (headless mode, proxy)
- Search settings (engine selection, fallbacks)
- Sandbox settings (Docker/WASM isolation)
- MCP server references

### Key Data Models

`src/schema.rs` defines:

- `Message`: Chat message with role, content, tool_calls
- `Memory`: Message history container
- `ToolCall`: Function call representation
- `AgentState`: Execution state enum (Idle/Running/Finished/Error)

## Project Structure

```
.
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library root
│   ├── agent/           # Agent implementations
│   ├── tool/            # Tool implementations
│   ├── llm/             # LLM client
│   ├── flow/            # Flow orchestration
│   ├── sandbox/         # Sandboxed execution
│   ├── protocol/        # A2A / MCP protocol
│   ├── config.rs        # Configuration
│   ├── schema.rs        # Data models
│   └── error.rs         # Error types
├── tests/               # Integration tests
├── benches/             # Benchmarks
├── fuzz/                # Fuzzing targets
├── config/              # Configuration files
├── docs/                # Documentation
└── examples/            # Example programs
```

## Features

| Feature | Description | Default |
|---------|-------------|---------|
| `browser` | Browser automation via chromiumoxide | yes |
| `server` | HTTP server via axum | yes |
| `mcp` | MCP protocol support | yes |
| `computer-use` | Screen capture and desktop automation | no |
| `wasm-sandbox` | WASM-based sandboxed execution | no |
| `docker-sandbox` | Docker container sandbox | no |

## Code Style

- Rust edition 2021, MSRV 1.72
- Async/await throughout using Tokio
- `thiserror` for domain errors, `anyhow` only at binary entry points
- `#[async_trait]` for async trait methods
- All public functions/methods have doc comments
- Unit tests in `#[cfg(test)] mod tests` at bottom of each file
- Integration tests in `tests/`
