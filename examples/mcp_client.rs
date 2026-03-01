//! Example: Using an MCP Client
//!
//! This example demonstrates how to connect to an MCP server
//! and use its tools.
//!
//! Usage:
//!   cargo run --example mcp_client --features mcp -- <command> [args...]
//!
//! Example:
//!   cargo run --example mcp_client --features mcp -- npx -y @modelcontextprotocol/server-filesystem /tmp

#[cfg(feature = "mcp")]
use openmanus::mcp::{McpClient, McpClientConfig, TransportType};
#[cfg(feature = "mcp")]
use std::env;

#[cfg(feature = "mcp")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <command> [args...]", args[0]);
        eprintln!("       {} --sse <url>", args[0]);
        eprintln!("       {} --ws <url>", args[0]);
        eprintln!();
        eprintln!("Examples:");
        eprintln!(
            "  {} npx -y @modelcontextprotocol/server-filesystem /tmp",
            args[0]
        );
        eprintln!("  {} --sse http://localhost:8080/sse", args[0]);
        eprintln!("  {} --ws ws://localhost:8080/mcp", args[0]);
        std::process::exit(1);
    }

    let config = if args[1] == "--sse" {
        if args.len() < 3 {
            eprintln!("Error: --sse requires a URL");
            std::process::exit(1);
        }
        McpClientConfig {
            server_id: "sse_server".to_string(),
            transport: TransportType::Sse {
                url: args[2].clone(),
                headers: Default::default(),
            },
            ..Default::default()
        }
    } else if args[1] == "--ws" || args[1] == "--websocket" {
        if args.len() < 3 {
            eprintln!("Error: --ws requires a URL");
            std::process::exit(1);
        }
        McpClientConfig {
            server_id: "ws_server".to_string(),
            transport: TransportType::WebSocket {
                url: args[2].clone(),
                headers: Default::default(),
            },
            ..Default::default()
        }
    } else {
        // stdio transport
        let command = args[1].clone();
        let cmd_args = args[2..].to_vec();
        McpClientConfig {
            server_id: command.clone(),
            transport: TransportType::Stdio {
                command,
                args: cmd_args,
            },
            ..Default::default()
        }
    };

    println!("Connecting to MCP server...");
    println!("Transport: {:?}", config.transport);

    let mut client = McpClient::new(config);
    client.connect().await?;

    println!("Connected!\n");

    // List available tools
    let tools = client.list_tools().await?;
    println!("Available tools ({}):", tools.len());
    for tool in &tools {
        println!("  - {}:", tool.name);
        println!("    {}", tool.description);
    }
    println!();

    // Interactive mode - call tools
    if !tools.is_empty() {
        println!("Calling first tool '{}' as a test...", tools[0].name);
        let result = client
            .call_tool(&tools[0].name, serde_json::json!({}))
            .await?;
        println!("Result: {:?}\n", result);
    }

    // Disconnect
    client.disconnect().await?;
    println!("Disconnected.");

    Ok(())
}

#[cfg(not(feature = "mcp"))]
fn main() {
    eprintln!("This example requires the 'mcp' feature. Run with:");
    eprintln!("  cargo run --example mcp_client --features mcp -- <command> [args...]");
}
