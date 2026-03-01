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
pub mod chart;
pub mod chat_completion;
pub mod computer_use;
pub mod crawl;
pub mod file_ops;
pub mod mcp_tool;
pub mod planning;
pub mod python;
pub mod sandbox_tool;
pub mod search;
pub mod str_replace_editor;
pub mod terminate;
pub mod web_search;

// Re-export specific tool types
pub use ask_human::AskHumanTool;
pub use bash::BashTool;
pub use chart::ChartTool;
pub use chat_completion::ChatCompletionTool;
pub use computer_use::ComputerUseTool;
pub use crawl::CrawlTool;
pub use file_ops::FileTool;
pub use mcp_tool::McpServerTool;
pub use planning::PlanningTool;
pub use python::PythonTool;
pub use sandbox_tool::{
    SandboxBrowserTool, SandboxFileTool, SandboxShellTool, SandboxTool, SandboxVisionTool,
};
pub use search::SearchTool;
pub use str_replace_editor::StrReplaceEditor;
pub use terminate::TerminateTool;
pub use web_search::WebSearchTool;

/// Re-export common tool types
pub mod prelude {
    pub use super::base::{Tool, ToolDefinition, ToolParameter, ToolResult, ToolSchema};
    pub use super::collection::ToolCollection;
    pub use crate::error::ToolError;
}
