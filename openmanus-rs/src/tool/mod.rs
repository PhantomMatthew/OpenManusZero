//! Tool system for OpenManus
//!
//! This module provides the trait-based tool system that allows
//! agents to interact with various systems and services.

mod base;
mod collection;

pub use base::{FunctionDefinition, Tool, ToolDefinition, ToolParameter, ToolResult, ToolSchema};
pub use collection::ToolCollection;

// Re-export ToolError from error module
pub use crate::error::ToolError;

#[cfg(feature = "browser")]
pub mod browser;

pub mod ask_human;
pub mod bash;
pub mod file_ops;
pub mod python;
pub mod str_replace_editor;
pub mod terminate;

// Re-export specific tool types
pub use ask_human::AskHumanTool;
pub use bash::BashTool;
pub use file_ops::FileTool;
pub use python::PythonTool;
pub use str_replace_editor::StrReplaceEditor;
pub use terminate::TerminateTool;

/// Re-export common tool types
pub mod prelude {
    pub use super::base::{Tool, ToolDefinition, ToolParameter, ToolResult, ToolSchema};
    pub use super::collection::ToolCollection;
    pub use crate::error::ToolError;
}
