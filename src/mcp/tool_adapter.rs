//! Tool adapter for converting between OpenManus and MCP tools
//!
//! This module provides bidirectional conversion:
//! - OpenManus `Tool` trait → MCP tool definitions
//! - MCP tool definitions → OpenManus tool wrappers

use crate::context::Context;
use crate::error::ToolError;
use crate::mcp::types::{McpContent, McpToolInfo, McpToolResult};
use crate::tool::{Tool, ToolDefinition, ToolResult};
use async_trait::async_trait;
use rmcp::model::{CallToolResult, RawContent};
use std::sync::Arc;

/// Adapter for converting OpenManus tools to MCP format
pub struct McpToolAdapter {
    tools: Vec<Arc<dyn Tool>>,
}

impl McpToolAdapter {
    /// Create a new adapter with the given tools
    pub fn new(tools: Vec<Arc<dyn Tool>>) -> Self {
        Self { tools }
    }

    /// Get all tool names
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name()).collect()
    }

    /// Execute a tool by name with the given arguments
    pub async fn execute_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
        ctx: &mut crate::context::Context,
    ) -> Result<McpToolResult, ToolError> {
        let tool = self
            .tools
            .iter()
            .find(|t| t.name() == name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        let input_str = serde_json::to_string(&arguments).unwrap_or_default();
        let result = tool.execute(&input_str, ctx).await?;

        Ok(tool_result_to_mcp(result))
    }
}

/// Convert an OpenManus ToolResult to an MCP ToolResult
pub fn tool_result_to_mcp(result: ToolResult) -> McpToolResult {
    let mut content = Vec::new();
    let is_error = result.is_error();

    if let Some(output) = result.output {
        content.push(McpContent::Text { text: output });
    }

    if let Some(error) = result.error {
        content.push(McpContent::Text {
            text: format!("Error: {}", error),
        });
    }

    if let Some(image) = result.base64_image {
        content.push(McpContent::Image {
            data: image,
            mime_type: "image/png".to_string(),
        });
    }

    McpToolResult { content, is_error }
}

/// Convert an MCP CallToolResult to OpenManus format
pub fn mcp_result_to_tool_result(result: CallToolResult) -> ToolResult {
    let mut output_parts: Vec<String> = Vec::new();
    let mut base64_image = None;

    for content in result.content {
        match content.raw {
            RawContent::Text(text_content) => {
                output_parts.push(text_content.text);
            }
            RawContent::Image(image_data) => {
                base64_image = Some(image_data.data);
            }
            RawContent::Resource(_resource) => {
                // Resource content - format for display
                output_parts.push(format!("[Resource content]"));
            }
            RawContent::Audio(_) => {
                // Audio content not supported
                output_parts.push("[Audio content]".to_string());
            }
            RawContent::ResourceLink(_) => {
                // Resource link - format for display
                output_parts.push("[Resource link]".to_string());
            }
        }
    }

    let is_error = result.is_error.unwrap_or(false);

    if is_error {
        ToolResult::failure(output_parts.join("\n"))
    } else if !output_parts.is_empty() {
        if let Some(image) = base64_image {
            ToolResult::with_image(output_parts.join("\n"), image)
        } else {
            ToolResult::success(output_parts.join("\n"))
        }
    } else {
        ToolResult::default()
    }
}

/// Wrapper for MCP tools to be used as OpenManus tools
pub struct McpToolWrapper {
    info: McpToolInfo,
    server_id: String,
    /// Store the client as a type-erased function caller
    call_fn: Arc<
        dyn Fn(
                String,
                serde_json::Value,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>,
            > + Send
            + Sync,
    >,
}

impl McpToolWrapper {
    /// Create a new MCP tool wrapper with a call function
    pub fn new<F>(info: McpToolInfo, server_id: String, call_fn: F) -> Self
    where
        F: Fn(
                String,
                serde_json::Value,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>,
            > + Send
            + Sync
            + 'static,
    {
        Self {
            info,
            server_id,
            call_fn: Arc::new(call_fn),
        }
    }

    /// Get the tool name (namespaced with server ID)
    pub fn namespaced_name(&self) -> String {
        format!("mcp_{}_{}", self.server_id, self.info.name)
    }

    /// Get the original tool name
    pub fn original_name(&self) -> &str {
        &self.info.name
    }

    /// Get the server ID
    pub fn server_id(&self) -> &str {
        &self.server_id
    }
}

#[async_trait]
impl Tool for McpToolWrapper {
    fn name(&self) -> &'static str {
        Box::leak(self.namespaced_name().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        Box::leak(self.info.description.clone().into_boxed_str())
    }

    fn parameters(&self) -> Option<crate::tool::ToolSchema> {
        serde_json::from_value(self.info.input_schema.clone()).ok()
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        // Parse input as JSON, or empty object if parsing fails
        let arguments: serde_json::Value = if input.is_empty() || input == "{}" {
            serde_json::Value::Object(Default::default())
        } else {
            serde_json::from_str(input).unwrap_or(serde_json::Value::Object(Default::default()))
        };

        (self.call_fn)(self.info.name.clone(), arguments).await
    }
}

/// Convert MCP tool info to OpenManus ToolDefinition
pub fn mcp_info_to_definition(info: &McpToolInfo, server_id: &str) -> ToolDefinition {
    let parameters: Option<crate::tool::ToolSchema> =
        serde_json::from_value(info.input_schema.clone()).ok();

    ToolDefinition {
        tool_type: "function".to_string(),
        function: crate::tool::FunctionDefinition {
            name: format!("mcp_{}_{}", server_id, info.name),
            description: info.description.clone(),
            parameters,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_to_mcp_success() {
        let result = ToolResult::success("Hello, world!");
        let mcp_result = tool_result_to_mcp(result);

        assert!(!mcp_result.is_error);
        assert_eq!(mcp_result.content.len(), 1);
        match &mcp_result.content[0] {
            McpContent::Text { text } => assert_eq!(text, "Hello, world!"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_tool_result_to_mcp_error() {
        let result = ToolResult::failure("Something went wrong");
        let mcp_result = tool_result_to_mcp(result);

        assert!(mcp_result.is_error);
    }

    #[test]
    fn test_tool_result_to_mcp_with_image() {
        let result = ToolResult::with_image("Screenshot", "base64imagedata");
        let mcp_result = tool_result_to_mcp(result);

        assert!(!mcp_result.is_error);
        assert_eq!(mcp_result.content.len(), 2);
    }

    #[test]
    fn test_mcp_info_to_definition() {
        let info = McpToolInfo {
            name: "echo".to_string(),
            description: "Echo a message".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message to echo"
                    }
                },
                "required": ["message"]
            }),
        };

        let def = mcp_info_to_definition(&info, "test_server");
        assert_eq!(def.function.name, "mcp_test_server_echo");
        assert_eq!(def.function.description, "Echo a message");
        assert!(def.function.parameters.is_some());
    }
}
