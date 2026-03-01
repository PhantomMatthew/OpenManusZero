//! Agent system for OpenManus

mod base;
mod browser;
mod data_analysis;
mod manus;
mod react;
mod sandbox_agent;
mod swe;
mod toolcall;

#[cfg(feature = "mcp")]
mod mcp;

pub use base::{Agent, BaseAgent};
pub use browser::{BrowserAgent, BrowserContextHelper, BrowserState};
pub use data_analysis::DataAnalysisAgent;
pub use manus::Manus;
pub use react::ReActAgent;
pub use sandbox_agent::SandboxAgent;
pub use swe::SweAgent;
pub use toolcall::ToolCallAgent;

#[cfg(feature = "mcp")]
pub use mcp::McpAgent;

// Re-export AgentState from schema
pub use crate::schema::AgentState;

/// Re-export common agent types
pub mod prelude {
    pub use super::base::{Agent, BaseAgent};
    pub use super::browser::{BrowserAgent, BrowserContextHelper, BrowserState};
    pub use super::data_analysis::DataAnalysisAgent;
    #[cfg(feature = "mcp")]
    pub use super::mcp::McpAgent;
    pub use super::manus::Manus;
    pub use super::react::ReActAgent;
    pub use super::sandbox_agent::SandboxAgent;
    pub use super::swe::SweAgent;
    pub use super::toolcall::ToolCallAgent;
    pub use crate::schema::AgentState;
}
