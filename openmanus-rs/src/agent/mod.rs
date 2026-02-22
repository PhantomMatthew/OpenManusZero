//! Agent system for OpenManus

mod base;
mod manus;
mod react;
mod swe;
mod toolcall;

#[cfg(feature = "mcp")]
mod mcp;

pub use base::{Agent, BaseAgent};
pub use manus::Manus;
pub use react::ReActAgent;
pub use swe::SweAgent;
pub use toolcall::ToolCallAgent;

#[cfg(feature = "mcp")]
pub use mcp::McpAgent;

// Re-export AgentState from schema
pub use crate::schema::AgentState;

/// Re-export common agent types
pub mod prelude {
    pub use super::base::{Agent, BaseAgent};
    pub use super::manus::Manus;
    #[cfg(feature = "mcp")]
    pub use super::mcp::McpAgent;
    pub use super::react::ReActAgent;
    pub use super::swe::SweAgent;
    pub use super::toolcall::ToolCallAgent;
    pub use crate::schema::AgentState;
}
