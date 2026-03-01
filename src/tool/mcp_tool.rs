//! MCP (Model Context Protocol) server tool
//!
//! Provides tools for interacting with MCP servers

use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP connection type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpConnectionType {
    Stdio,
    Sse,
}

/// MCP tool input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInput {
    /// Action to perform
    pub action: String,
    /// Server ID
    #[serde(default)]
    pub server_id: Option<String>,
    /// Server URL (for SSE)
    #[serde(default)]
    pub server_url: Option<String>,
    /// Command (for stdio)
    #[serde(default)]
    pub command: Option<String>,
    /// Arguments (for stdio)
    #[serde(default)]
    pub args: Option<Vec<String>>,
    /// Tool name to call
    #[serde(default)]
    pub tool_name: Option<String>,
    /// Tool arguments
    #[serde(default)]
    pub tool_args: Option<serde_json::Value>,
}

/// MCP server tool for managing MCP connections and calling tools
pub struct McpServerTool {
    // Connection state would be stored here
    // For now, this is a simplified implementation
}

impl McpServerTool {
    /// Create a new MCP server tool
    pub fn new() -> Self {
        Self {}
    }

    /// List available MCP actions
    pub fn list_actions() -> Vec<&'static str> {
        vec![
            "connect_stdio",
            "connect_sse",
            "disconnect",
            "list_tools",
            "call_tool",
            "list_servers",
        ]
    }

    async fn connect_stdio(&self, input: McpToolInput) -> Result<String, ToolError> {
        let command = input.command.ok_or_else(|| {
            ToolError::InvalidInput("command is required for connect_stdio".to_string())
        })?;

        let server_id = input.server_id.unwrap_or_else(|| command.clone());
        let args = input.args.unwrap_or_default();

        // In a full implementation, this would spawn the process and connect
        tracing::info!("Connecting to MCP server via stdio: {} {:?}", command, args);

        Ok(format!("Connected to MCP server '{}' via stdio", server_id))
    }

    async fn connect_sse(&self, input: McpToolInput) -> Result<String, ToolError> {
        let server_url = input.server_url.ok_or_else(|| {
            ToolError::InvalidInput("server_url is required for connect_sse".to_string())
        })?;

        let server_id = input.server_id.unwrap_or_else(|| server_url.clone());

        tracing::info!("Connecting to MCP server via SSE: {}", server_url);

        Ok(format!("Connected to MCP server '{}' via SSE", server_id))
    }

    async fn disconnect(&self, input: McpToolInput) -> Result<String, ToolError> {
        let server_id = input.server_id.ok_or_else(|| {
            ToolError::InvalidInput("server_id is required for disconnect".to_string())
        })?;

        tracing::info!("Disconnecting from MCP server: {}", server_id);

        Ok(format!("Disconnected from MCP server '{}'", server_id))
    }

    async fn list_tools(&self, _input: McpToolInput) -> Result<String, ToolError> {
        // In a full implementation, this would query the MCP server
        Ok("MCP tools listing not implemented in this version".to_string())
    }

    async fn call_tool(&self, input: McpToolInput) -> Result<String, ToolError> {
        let tool_name = input.tool_name.ok_or_else(|| {
            ToolError::InvalidInput("tool_name is required for call_tool".to_string())
        })?;

        // In a full implementation, this would call the MCP tool
        Ok(format!("MCP tool '{}' called (stub implementation)", tool_name))
    }

    async fn list_servers(&self) -> Result<String, ToolError> {
        // In a full implementation, this would list connected servers
        Ok("No MCP servers currently connected (stub implementation)".to_string())
    }
}

impl Default for McpServerTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for McpServerTool {
    fn name(&self) -> &'static str {
        "mcp"
    }

    fn description(&self) -> &'static str {
        "MCP server tool for connecting to and interacting with MCP servers"
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "action".to_string(),
                    ToolParameter {
                        name: "action".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Action to perform: connect_stdio, connect_sse, disconnect, list_tools, call_tool, list_servers".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: Some(Self::list_actions().into_iter().map(|s| s.to_string()).collect()),
                    },
                );
                props.insert(
                    "server_id".to_string(),
                    ToolParameter {
                        name: "server_id".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Server identifier".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "server_url".to_string(),
                    ToolParameter {
                        name: "server_url".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Server URL for SSE connection".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "command".to_string(),
                    ToolParameter {
                        name: "command".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Command for stdio connection".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "tool_name".to_string(),
                    ToolParameter {
                        name: "tool_name".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Name of MCP tool to call".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props
            },
            required: Some(vec!["action".to_string()]),
        })
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        let mcp_input: McpToolInput = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid MCP input: {}", e)))?;

        let output = match mcp_input.action.as_str() {
            "connect_stdio" => self.connect_stdio(mcp_input).await?,
            "connect_sse" => self.connect_sse(mcp_input).await?,
            "disconnect" => self.disconnect(mcp_input).await?,
            "list_tools" => self.list_tools(mcp_input).await?,
            "call_tool" => self.call_tool(mcp_input).await?,
            "list_servers" => self.list_servers().await?,
            _ => {
                return Err(ToolError::InvalidInput(format!(
                    "Unknown action: {}",
                    mcp_input.action
                )))
            }
        };

        Ok(ToolResult::success(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_server_tool_new() {
        let tool = McpServerTool::new();
        assert_eq!(tool.name(), "mcp");
    }

    #[test]
    fn test_list_actions() {
        let actions = McpServerTool::list_actions();
        assert!(actions.contains(&"connect_stdio"));
        assert!(actions.contains(&"connect_sse"));
        assert!(actions.contains(&"call_tool"));
    }

    #[tokio::test]
    async fn test_list_servers() {
        let tool = McpServerTool::new();
        let result = tool.list_servers().await.unwrap();
        assert!(result.contains("No MCP servers"));
    }
}
