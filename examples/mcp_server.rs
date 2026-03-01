//! Example: Running an MCP Server
//!
//! This example demonstrates how to run an MCP server that exposes
//! OpenManus tools to MCP clients like Claude Desktop.
//!
//! Usage:
//!   cargo run --example mcp_server --features mcp
//!
//! To connect from Claude Desktop, add to claude_desktop_config.json:
//! ```json
//! {
//!   "mcpServers": {
//!     "openmanus": {
//!       "command": "cargo",
//!       "args": ["run", "--example", "mcp_server", "--features", "mcp"]
//!     }
//!   }
//! }
//! ```

#[cfg(feature = "mcp")]
use openmanus::mcp::{McpServer, McpServerConfig};
#[cfg(feature = "mcp")]
use openmanus::tool::ToolCollection;

#[cfg(feature = "mcp")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Starting OpenManus MCP Server...");
    println!("This server exposes OpenManus tools via the Model Context Protocol.");
    println!("Connect using an MCP client (like Claude Desktop) to use the tools.\n");

    // Create server configuration
    let config = McpServerConfig {
        name: "openmanus".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        instructions: Some(
            "OpenManus AI Agent Framework\n\
             \n\
             Available tools:\n\
             - bash: Execute shell commands\n\
             - python: Execute Python code\n\
             - file_read: Read file contents\n\
             - file_write: Write to files\n\
             - terminate: End the session"
                .to_string(),
        ),
        transport: None, // Use stdio by default
    };

    // Create tool collection with standard tools
    let tools = ToolCollection::with_defaults();

    // Create and run server
    let server = McpServer::with_tools(config, &tools);

    println!("Server ready. Waiting for MCP client connection via stdio...\n");

    server.run_stdio().await?;

    println!("Server shut down.");
    Ok(())
}

#[cfg(not(feature = "mcp"))]
fn main() {
    eprintln!("This example requires the 'mcp' feature. Run with:");
    eprintln!("  cargo run --example mcp_server --features mcp");
}
