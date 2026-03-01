//! MCP Server implementation for OpenManus
//!
//! Exposes OpenManus tools to MCP clients (like Claude Desktop)
//! via stdio transport.

use crate::context::Context;
use crate::error::ToolError;
use crate::mcp::types::McpServerConfig;
use crate::tool::{Tool, ToolCollection};
use rmcp::{
    model::*,
    service::{RequestContext, RoleServer, ServiceExt},
    ServerHandler,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// MCP Server that exposes OpenManus tools
pub struct McpServer {
    config: McpServerConfig,
    tools: Vec<Arc<dyn Tool>>,
    context: Arc<RwLock<Context>>,
}

impl McpServer {
    /// Create a new MCP server with default configuration
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            tools: Vec::new(),
            context: Arc::new(RwLock::new(Context::default())),
        }
    }

    /// Create a new MCP server with tools from a collection
    pub fn with_tools(config: McpServerConfig, collection: &ToolCollection) -> Self {
        Self {
            config,
            tools: collection.tools(),
            context: Arc::new(RwLock::new(Context::default())),
        }
    }

    /// Add a tool to the server
    pub fn add_tool(&mut self, tool: Arc<dyn Tool>) {
        self.tools.push(tool);
    }

    /// Add multiple tools to the server
    pub fn add_tools(&mut self, tools: Vec<Arc<dyn Tool>>) {
        self.tools.extend(tools);
    }

    /// Set the execution context
    pub fn set_context(&mut self, context: Context) {
        self.context = Arc::new(RwLock::new(context));
    }

    /// Run the server using stdio transport
    pub async fn run_stdio(self) -> Result<(), McpServerError> {
        info!("Starting MCP server '{}' on stdio", self.config.name);

        let handler = OpenManusServerHandler::new(
            self.tools,
            self.context,
            self.config.name,
            self.config.version,
            self.config.instructions,
        );

        let service = handler
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|e| McpServerError::Transport(e.to_string()))?;

        service
            .waiting()
            .await
            .map_err(|e| McpServerError::Transport(e.to_string()))?;

        Ok(())
    }

    /// Run the server (auto-select transport based on config)
    pub async fn run(self) -> Result<(), McpServerError> {
        // For now, always use stdio
        self.run_stdio().await
    }
}

/// Internal server handler for MCP protocol
#[derive(Clone)]
pub struct OpenManusServerHandler {
    tools: Arc<Vec<Arc<dyn Tool>>>,
    context: Arc<RwLock<Context>>,
    server_name: String,
    server_version: String,
    instructions: Option<String>,
}

impl OpenManusServerHandler {
    fn new(
        tools: Vec<Arc<dyn Tool>>,
        context: Arc<RwLock<Context>>,
        server_name: String,
        server_version: String,
        instructions: Option<String>,
    ) -> Self {
        Self {
            tools: Arc::new(tools),
            context,
            server_name,
            server_version,
            instructions,
        }
    }
}

impl ServerHandler for OpenManusServerHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: self.server_name.clone(),
                version: self.server_version.clone(),
                ..Default::default()
            },
            instructions: self.instructions.clone().map(Into::into),
        }
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let tools: Vec<rmcp::model::Tool> = self
            .tools
            .iter()
            .map(|tool| {
                let def = tool.to_definition();
                let schema: serde_json::Map<String, serde_json::Value> = def
                    .function
                    .parameters
                    .and_then(|p| {
                        if let serde_json::Value::Object(map) =
                            serde_json::to_value(p).unwrap_or_default()
                        {
                            Some(map)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                rmcp::model::Tool::new(
                    def.function.name.clone(),
                    def.function.description.clone(),
                    Arc::new(schema),
                )
            })
            .collect();

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let tool_name = request.name.to_string();
        let arguments = request.arguments.unwrap_or_default();

        info!(
            "Executing MCP tool: {} with args: {:?}",
            tool_name, arguments
        );

        // Find the tool
        let tool = self
            .tools
            .iter()
            .find(|t| t.name() == tool_name)
            .ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::METHOD_NOT_FOUND,
                    format!("Tool not found: {}", tool_name),
                    None,
                )
            })?;

        // Execute the tool
        let input_str = serde_json::to_string(&arguments).unwrap_or_default();
        let mut ctx = self.context.write().await;

        match tool.execute(&input_str, &mut ctx).await {
            Ok(result) => {
                let mut content = Vec::new();

                if let Some(output) = result.output {
                    content.push(Annotated::new(RawContent::text(output), None));
                }

                if let Some(error) = result.error {
                    content.push(Annotated::new(
                        RawContent::text(format!("Error: {}", error)),
                        None,
                    ));
                    return Ok(CallToolResult {
                        content,
                        is_error: Some(true),
                        meta: None,
                        structured_content: None,
                    });
                }

                if let Some(image) = result.base64_image {
                    content.push(Annotated::new(RawContent::image(image, "image/png"), None));
                }

                Ok(CallToolResult {
                    content,
                    is_error: Some(false),
                    meta: None,
                    structured_content: None,
                })
            }
            Err(e) => {
                error!("Tool execution failed: {}", e);
                Ok(CallToolResult {
                    content: vec![Annotated::new(
                        RawContent::text(format!("Error: {}", e)),
                        None,
                    )],
                    is_error: Some(true),
                    meta: None,
                    structured_content: None,
                })
            }
        }
    }
}

/// MCP Server errors
#[derive(Debug, thiserror::Error)]
pub enum McpServerError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config() {
        let config = McpServerConfig::default();
        let server = McpServer::new(config);
        assert_eq!(server.config.name, "openmanus");
    }

    #[test]
    fn test_server_with_tools() {
        let config = McpServerConfig::default();
        let collection = ToolCollection::new();
        let server = McpServer::with_tools(config, &collection);
        assert!(server.tools.is_empty());
    }
}
