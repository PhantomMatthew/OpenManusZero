//! Example: Using MCPAgent
//!
//! This example demonstrates how to use MCPAgent to connect to
//! MCP servers and have an LLM agent use their tools.
//!
//! Usage:
//!   OPENAI_API_KEY=your-key cargo run --example mcp_agent --features mcp -- <command> [args...]
//!
//! Example:
//!   OPENAI_API_KEY=sk-... cargo run --example mcp_agent --features mcp -- npx -y @modelcontextprotocol/server-filesystem /tmp

#[cfg(feature = "mcp")]
use openmanus::agent::{Agent, McpAgent};
#[cfg(feature = "mcp")]
use openmanus::llm::HttpLlmClientBuilder;
#[cfg(feature = "mcp")]
use std::env;
#[cfg(feature = "mcp")]
use std::sync::Arc;

#[cfg(feature = "mcp")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <command> [args...]", args[0]);
        eprintln!();
        eprintln!("Set OPENAI_API_KEY environment variable for LLM access.");
        eprintln!();
        eprintln!("Example:");
        eprintln!(
            "  OPENAI_API_KEY=sk-... {} npx -y @modelcontextprotocol/server-filesystem /tmp",
            args[0]
        );
        std::process::exit(1);
    }

    // Create LLM client from environment
    let llm = Arc::new(
        HttpLlmClientBuilder::from_env()
            .model("gpt-4o")
            .build()
            .expect("Failed to create LLM client. Make sure OPENAI_API_KEY is set."),
    );

    // Create MCP agent
    let mut agent = McpAgent::new(llm).with_max_steps(20).with_system_prompt(
        "You are an AI assistant with access to MCP tools. \
             Use the available tools to help the user complete tasks. \
             Be thorough and explain what you're doing.",
    );

    println!("Connecting to MCP server...");
    println!("Command: {} {:?}", args[1], &args[2..]);

    // Connect to MCP server
    agent
        .connect_stdio(args[1].clone(), args[2..].to_vec(), None)
        .await?;

    println!("Connected to MCP server!");
    println!(
        "Available tools: {:?}",
        agent
            .tools()
            .tools()
            .iter()
            .map(|t| t.name())
            .collect::<Vec<_>>()
    );
    println!();

    // Run the agent with a task
    let task = env::var("TASK").unwrap_or_else(|_| {
        "List the files in the directory and tell me what you find.".to_string()
    });

    println!("Running agent with task: {}\n", task);

    let result = agent.run(&task).await?;

    println!("\n=== Agent Result ===\n{}", result);

    // Cleanup
    agent.cleanup().await?;

    Ok(())
}

#[cfg(not(feature = "mcp"))]
fn main() {
    eprintln!("This example requires the 'mcp' feature. Run with:");
    eprintln!("  cargo run --example mcp_agent --features mcp -- <command> [args...]");
}
