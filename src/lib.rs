//! OpenManus - A versatile AI agent framework written in Rust
//!
//! This crate provides a complete implementation of an AI agent system
//! that can autonomously solve tasks using various tools including
//! browser automation, code execution, file editing, and more.
//!
//! # Architecture
//!
//! The framework is built around several core abstractions:
//! - [`agent`] - Agent implementations with ReAct-style reasoning
//! - [`tool`] - Pluggable tool system with trait-based design
//! - [`llm`] - LLM client abstraction supporting multiple backends
//! - [`context`] - Context and memory management
//! - [`sandbox`] - Sandboxed code execution
//! - [`mcp`] - MCP (Model Context Protocol) client and server support
//! - [`prompt`] - Prompt template system with variable substitution
//!
//! # Example
//!
//! ```rust,no_run
//! use openmanus::agent::{Manus, Agent};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut agent = Manus::new().await?;
//!     let result = agent.run("Search for Rust tutorials").await?;
//!     println!("Result: {}", result);
//!     Ok(())
//! }
//! ```

pub mod agent;
pub mod config;
pub mod context;
pub mod error;
pub mod flow;
pub mod llm;
pub mod protocol;
pub mod prompt;
pub mod sandbox;
pub mod schema;
pub mod tool;

#[cfg(feature = "mcp")]
pub mod mcp;

pub use error::{Error, Result};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::agent::{Agent, AgentState, BaseAgent, Manus};
    pub use crate::context::{Context, Memory};
    pub use crate::error::{Error, Result};
    pub use crate::llm::{LlmClient, LlmRequest, LlmResponse};
    pub use crate::prompt::{PromptLibrary, PromptSet, PromptTemplate};
    pub use crate::schema::{Message, Role, ToolCall};
    pub use crate::tool::{Tool, ToolCollection, ToolError, ToolResult};

    #[cfg(feature = "mcp")]
    pub use crate::mcp::{McpClient, McpClientConfig, McpServer, McpServerConfig, TransportType};
}

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
