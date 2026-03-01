# OpenManusZero

A versatile AI agent framework written in Rust. This is a Rust rewrite of the Python [OpenManus](https://github.com/FoundationAgents/OpenManus) project. Currently it is in early stage for overall testing.

## Features

- **ReAct-style Agent Loop**: Reasoning and Acting cycle for intelligent task execution
- **Tool System**: Pluggable tools with trait-based design
- **LLM Abstraction**: Support for OpenAI, Azure, Bedrock, and local LLMs
- **Browser Automation**: Headless browser control (optional)
- **Sandboxed Execution**: Secure code execution with WASM or process isolation
- **Flow Orchestration**: Multi-agent planning and execution

## Installation

### Prerequisites

- Rust 1.72 or later
- Python 3.x (for Python tool execution)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/PhantomMatthew/OpenManusZero.git
cd OpenManusZero

# Build
cargo build --release

# Run tests
cargo test
```

### Using cargo

```bash
cargo install --path .
```

## Quick Start

### Run with a prompt

```bash
# Set your API key
export OPENMANUS_ZERO_LLM_API_KEY="your-api-key"

# Run with a prompt
openmanus-zero run --prompt "Search for Rust tutorials and summarize them"

# Or run interactively
openmanus-zero interactive
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `OPENMANUS_ZERO_LLM_BASE_URL` | LLM API base URL | `https://api.openai.com/v1` |
| `OPENMANUS_ZERO_LLM_API_KEY` | LLM API key | (required) |
| `OPENMANUS_ZERO_LLM_MODEL` | Model to use | `gpt-4` |

## Usage

```bash
# Single prompt mode
openmanus-zero run -p "Your task here"

# Interactive mode
openmanus-zero interactive

# Flow orchestration mode
openmanus-zero flow -t "Complex multi-step task"

# With debug logging
openmanus-zero --debug run -p "Debug this"

# Using mock LLM (for testing)
openmanus-zero --mock run -p "Test prompt"
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI (main.rs)                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    │
│  │   Manus     │    │   Flow      │    │   LLM       │    │
│  │   Agent     │───▶│ Orchestrator│◀──▶│   Client    │    │
│  └─────────────┘    └─────────────┘    └─────────────┘    │
│         │                                     │             │
│         ▼                                     ▼             │
│  ┌─────────────┐                      ┌─────────────┐     │
│  │  ToolCall   │                      │   Http /    │     │
│  │   Agent     │                      │   Mock      │     │
│  └─────────────┘                      └─────────────┘     │
│         │                                                  │
│         ▼                                                  │
│  ┌─────────────────────────────────────────────────────┐  │
│  │                   Tool Collection                    │  │
│  │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐      │  │
│  │  │ Bash │ │Python│ │ File │ │Browser│ │ ... │      │  │
│  │  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘      │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│                     Core Components                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Schema  │  │ Context  │  │  Memory  │  │  Error   │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Development

### Run Tests

```bash
# Run all tests
cargo test

# Run tests with coverage
cargo tarpaulin --out Html

# Run specific test
cargo test test_tool_collection

# Run integration tests
cargo test --test '*'
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy --all-targets --all-features -- -D warnings

# Run all checks
cargo fmt -- --check && cargo clippy -- -D warnings && cargo test
```

### Features

| Feature | Description |
|---------|-------------|
| `default` | browser, server, mcp |
| `browser` | Browser automation via chromiumoxide |
| `server` | HTTP server via axum |
| `mcp` | MCP protocol support |
| `wasm-sandbox` | WASM-based sandboxed execution |
| `computer-use` | Screen capture and desktop automation |

```bash
# Build without default features
cargo build --no-default-features

# Build with specific features
cargo build --features browser,wasm-sandbox
```

## Project Structure

```
.
├── Cargo.toml           # Project manifest
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library root
│   ├── agent/           # Agent implementations
│   │   ├── base.rs      # Base agent
│   │   ├── react.rs     # ReAct agent
│   │   ├── toolcall.rs  # Tool-calling agent
│   │   └── manus.rs     # Main Manus agent
│   ├── tool/            # Tool system
│   │   ├── base.rs      # Tool trait
│   │   ├── collection.rs# Tool collection
│   │   ├── bash.rs      # Bash tool
│   │   ├── python.rs    # Python tool
│   │   └── ...
│   ├── llm/             # LLM client
│   │   ├── client.rs    # Client implementations
│   │   └── types.rs     # Request/Response types
│   ├── context/         # Context and memory
│   ├── flow/            # Flow orchestration
│   ├── sandbox/         # Sandboxed execution
│   ├── schema.rs        # Data structures
│   └── error.rs         # Error types
├── tests/               # Integration tests
├── fuzz/                # Fuzzing targets
├── benches/             # Benchmarks
└── docs/                # Documentation
```

## API Example

```rust
use openmanus::agent::Manus;
use openmanus::llm::HttpLlmClient;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create LLM client
    let llm = Arc::new(HttpLlmClient::new(
        "https://api.openai.com/v1",
        "your-api-key",
        "gpt-4"
    ));

    // Create agent
    let mut agent = Manus::with_llm(llm);

    // Run task
    let result = agent.run("Search for Rust tutorials").await?;
    println!("Result: {}", result);

    // Cleanup
    agent.cleanup().await?;
    Ok(())
}
```

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please read our contributing guidelines and submit pull requests.

## Acknowledgments

This is a Rust rewrite of [OpenManus](https://github.com/FoundationAgents/OpenManus), inspired by:
- [anthropic-computer-use](https://github.com/anthropics/anthropic-quickstarts)
- [browser-use](https://github.com/browser-use/browser-use)
- [MetaGPT](https://github.com/geekan/MetaGPT)
