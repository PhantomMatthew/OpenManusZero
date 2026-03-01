//! MCP (Model Context Protocol) support for OpenManus
//!
//! This module provides both client and server implementations for the
//! Model Context Protocol, allowing OpenManus to:
//! - Expose its tools to other MCP clients (like Claude Desktop)
//! - Connect to external MCP servers and use their tools
//!
//! # Architecture
//!
//! - [`types`] - MCP protocol types and configurations
//! - [`server`] - MCP server implementation for exposing tools
//! - [`client`] - MCP client implementation for connecting to servers
//! - [`tool_adapter`] - Conversion between OpenManus and MCP tools
//!
//! # Example: Running an MCP Server
//!
//! ```rust,no_run
//! use openmanus::mcp::{McpServer, McpServerConfig};
//! use openmanus::tool::ToolCollection;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = McpServerConfig::default();
//!     let server = McpServer::new(config);
//!     server.run_stdio().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Example: Connecting as an MCP Client
//!
//! ```rust,no_run
//! use openmanus::mcp::{McpClient, McpClientConfig, TransportType};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = McpClientConfig {
//!         transport: TransportType::Stdio {
//!             command: "mcp-server".to_string(),
//!             args: vec![],
//!         },
//!         ..Default::default()
//!     };
//!     let mut client = McpClient::new(config);
//!     client.connect().await?;
//!     let tools = client.list_tools().await?;
//!     println!("Available tools: {:?}", tools);
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod server;
pub mod tool_adapter;
pub mod types;

// Re-export from types module
pub use types::{
    McpClientConfig, McpConfigFile, McpContent, McpResource, McpServerConfig, McpToolInfo,
    McpToolResult, ServerTransportConfig, TransportType,
};

// Re-export server and client
pub use client::{McpClient, McpClientError, McpClientManager};
pub use server::{McpServer, McpServerError};
pub use tool_adapter::{McpToolAdapter, McpToolWrapper};
