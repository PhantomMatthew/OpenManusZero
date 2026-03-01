# MCP (Model Context Protocol) Support for OpenManus Rust

This document describes the MCP implementation for the Rust version of OpenManus.

## Overview

MCP (Model Context Protocol) is an open standard developed by Anthropic that defines how LLM applications and agents integrate with external data sources and tools. This implementation provides both client and server capabilities, allowing OpenManus to:

- **Server Mode**: Expose OpenManus tools to other MCP clients (like Claude Desktop)
- **Client Mode**: Connect to external MCP servers and use their tools

## Architecture

```
.
├── src/
│   ├── protocol/                 # MCP module
│   │   ├── mod.rs                # Module exports
│   │   ├── types.rs              # Protocol types (TransportType, Configs, etc.)
│   │   ├── server.rs             # MCP Server implementation
│   │   ├── client.rs             # MCP Client implementation
│   │   └── tool_adapter.rs       # OpenManus ↔ MCP tool conversion
│   ├── agent/
│   │   └── mcp.rs                # MCPAgent implementation
│   └── config.rs                 # Configuration with MCP support
├── config/
│   └── mcp.example.json          # Example MCP configuration
└── examples/
    ├── mcp_server.rs             # Run as MCP server
    ├── mcp_client.rs             # Connect to MCP servers
    └── mcp_agent.rs              # Use MCPAgent with LLM
```

## Dependencies

- `rmcp = "0.16"` - Official Rust MCP SDK
- `tokio-tungstenite` - WebSocket transport support (optional)

## Features

### 1. MCP Server

The MCP server exposes OpenManus tools to external MCP clients via stdio transport.

```rust
use openmanus::mcp::{McpServer, McpServerConfig};
use openmanus::tool::ToolCollection;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = McpServerConfig::default();
    let tools = ToolCollection::with_defaults();
    let server = McpServer::with_tools(config, &tools);
    server.run_stdio().await?;
    Ok(())
}
```

### 2. MCP Client

The MCP client connects to external MCP servers and wraps their tools as OpenManus tools.

```rust
use openmanus::mcp::{McpClient, McpClientConfig, TransportType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = McpClientConfig {
        server_id: "filesystem".to_string(),
        transport: TransportType::Stdio {
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string(), "/tmp".to_string()],
        },
        ..Default::default()
    };

    let mut client = McpClient::new(config);
    client.connect().await?;

    let tools = client.list_tools().await?;
    println!("Available tools: {:?}", tools);

    client.disconnect().await?;
    Ok(())
}
```

### 3. MCPAgent

MCPAgent is an agent that connects to MCP servers and uses their tools with LLM reasoning.

```rust
use openmanus::agent::McpAgent;
use openmanus::llm::OpenAIClient;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm = Arc::new(OpenAIClient::new(api_key)?);

    let mut agent = McpAgent::new(llm)
        .with_max_steps(20)
        .with_system_prompt("You are an AI assistant with MCP tools.");

    // Connect to MCP server
    agent.connect_stdio("npx", vec!["-y", "@modelcontextprotocol/server-filesystem", "/tmp"], None).await?;

    // Run the agent
    let result = agent.run("List files in the directory").await?;
    println!("Result: {}", result);

    agent.cleanup().await?;
    Ok(())
}
```

### 4. McpClientManager

Manages multiple MCP client connections.

```rust
use openmanus::mcp::{McpClientManager, McpClientConfig, TransportType};

let mut manager = McpClientManager::new();

// Connect to multiple servers
manager.connect(McpClientConfig {
    server_id: "filesystem".to_string(),
    transport: TransportType::Stdio { command: "mcp-filesystem".to_string(), args: vec![] },
    ..Default::default()
}).await?;

// Get all tools from all servers
let collection = manager.to_tool_collection();

// Disconnect from all
manager.disconnect_all().await;
```

## Configuration

### MCP Server Configuration (`config/mcp.json`)

```json
{
  "servers": {
    "filesystem": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/dir"]
    },
    "github": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"]
    }
  },
  "server": {
    "enabled": true,
    "name": "openmanus",
    "host": "127.0.0.1",
    "port": 8080
  }
}
```

### Transport Types

| Type | Description | Status |
|------|-------------|--------|
| `stdio` | Standard I/O transport | Implemented |
| `sse` | Server-Sent Events (HTTP) | TODO |
| `websocket` | WebSocket transport | TODO |

## Usage Examples

### Running as an MCP Server

```bash
# Build with MCP support
cargo build --features mcp

# Run the MCP server
cargo run --example mcp_server --features mcp

# Connect from Claude Desktop (add to claude_desktop_config.json)
{
  "mcpServers": {
    "openmanus": {
      "command": "cargo",
      "args": ["run", "--example", "mcp_server", "--features", "mcp"]
    }
  }
}
```

### Connecting to an MCP Server

```bash
# Connect to a filesystem MCP server
cargo run --example mcp_client --features mcp -- npx -y @modelcontextprotocol/server-filesystem /tmp

# Connect to a git MCP server
cargo run --example mcp_client --features mcp -- uvx mcp-server-git
```

### Using MCPAgent

```bash
# Set API key and run
OPENAI_API_KEY=sk-... cargo run --example mcp_agent --features mcp -- npx -y @modelcontextprotocol/server-filesystem /tmp
```

## Building

```bash
# Build with MCP support (default)
cargo build

# Build without MCP support
cargo build --no-default-features --features browser,server

# Build only MCP support
cargo build --no-default-features --features mcp
```

## Known Limitations

1. **SSE Transport**: Not yet fully implemented - returns error message
2. **WebSocket Transport**: Requires additional rmcp transport implementation
3. **Cancel/Disconnect**: Client disconnect is simplified (drops connection rather than graceful shutdown)

## Testing

```bash
# Run MCP integration tests
cargo test --features mcp mcp_integration

# Run all tests
cargo test --features mcp
```

## API Reference

### Types

- `TransportType` - Transport protocol (Stdio, Sse, WebSocket)
- `McpServerConfig` - Server configuration
- `McpClientConfig` - Client configuration
- `McpToolInfo` - Tool information from MCP server
- `McpToolResult` - Tool execution result
- `McpContent` - Content type (Text, Image, Resource)

### Server

- `McpServer::new(config)` - Create new server
- `McpServer::with_tools(config, collection)` - Create with tools
- `server.run_stdio()` - Run via stdio transport

### Client

- `McpClient::new(config)` - Create new client
- `client.connect()` - Connect to server
- `client.list_tools()` - List available tools
- `client.call_tool(name, args)` - Execute a tool
- `client.to_tool_collection()` - Get tools as ToolCollection
- `client.disconnect()` - Disconnect from server

### Agent

- `McpAgent::new(llm)` - Create new agent
- `agent.connect_stdio(cmd, args, id)` - Connect via stdio
- `agent.run(task)` - Execute task using MCP tools
- `agent.cleanup()` - Cleanup resources

## References

- [MCP Specification](https://modelcontextprotocol.io/)
- [Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [Anthropic MCP Documentation](https://docs.anthropic.com/claude/docs/mcp)
