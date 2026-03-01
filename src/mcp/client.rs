//! MCP Client implementation for OpenManus
//!
//! Connects to external MCP servers and uses their tools.

use crate::error::ToolError;
use crate::mcp::tool_adapter::{mcp_result_to_tool_result, McpToolWrapper};
use crate::mcp::types::{McpClientConfig, McpToolInfo, TransportType};
use crate::tool::{ToolCollection, ToolResult};
use rmcp::{model::*, service::ServiceExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tracing::info;

/// Type-erased MCP client handle
type McpClientHandle = Arc<tokio::sync::RwLock<Option<McpClientInner>>>;

/// Inner client type - we use a trait object approach
struct McpClientInner {
    call_tool: Box<
        dyn Fn(
                String,
                serde_json::Map<String, serde_json::Value>,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Result<CallToolResult, rmcp::ServiceError>>
                        + Send,
                >,
            > + Send
            + Sync,
    >,
    list_tools: Box<
        dyn Fn() -> std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Result<ListToolsResult, rmcp::ServiceError>>
                        + Send,
                >,
            > + Send
            + Sync,
    >,
}

/// MCP Client that connects to external MCP servers
pub struct McpClient {
    config: McpClientConfig,
    client: McpClientHandle,
    tools: HashMap<String, McpToolInfo>,
    connected: bool,
}

impl McpClient {
    /// Create a new MCP client with the given configuration
    pub fn new(config: McpClientConfig) -> Self {
        let server_id = if config.server_id.is_empty() {
            match &config.transport {
                TransportType::Stdio { command, .. } => command.clone(),
                TransportType::Sse { url, .. } => url.clone(),
                TransportType::WebSocket { url, .. } => url.clone(),
            }
        } else {
            config.server_id.clone()
        };

        Self {
            config: McpClientConfig {
                server_id,
                ..config
            },
            client: Arc::new(tokio::sync::RwLock::new(None)),
            tools: HashMap::new(),
            connected: false,
        }
    }

    /// Connect to the MCP server
    pub async fn connect(&mut self) -> Result<(), McpClientError> {
        info!(
            "Connecting to MCP server '{}' via {:?}",
            self.config.server_id, self.config.transport
        );

        let client = match &self.config.transport {
            TransportType::Stdio { command, args } => {
                self.connect_stdio(command, args.clone()).await?
            }
            TransportType::Sse { url, .. } => self.connect_sse(url).await?,
            TransportType::WebSocket { url, .. } => self.connect_websocket(url).await?,
        };

        // Store client reference
        *self.client.write().await = Some(client);
        self.connected = true;

        // Initialize and fetch tools
        self.refresh_tools().await?;

        info!(
            "Connected to MCP server '{}' with {} tools",
            self.config.server_id,
            self.tools.len()
        );

        Ok(())
    }

    /// Connect via stdio transport
    async fn connect_stdio(
        &self,
        command: &str,
        args: Vec<String>,
    ) -> Result<McpClientInner, McpClientError> {
        use rmcp::transport::TokioChildProcess;

        let mut cmd = Command::new(command);
        cmd.args(&args);

        let transport = TokioChildProcess::new(cmd)
            .map_err(|e| McpClientError::Connection(format!("Failed to spawn process: {}", e)))?;

        let client = ()
            .serve(transport)
            .await
            .map_err(|e| McpClientError::Connection(format!("Failed to connect: {}", e)))?;

        // Create closure that captures the client
        let client_arc = Arc::new(tokio::sync::RwLock::new(client));

        let client_clone = client_arc.clone();
        let call_tool = Box::new(
            move |name: String, args: serde_json::Map<String, serde_json::Value>| {
                let client = client_clone.clone();
                Box::pin(async move {
                    let client = client.read().await;
                    client
                        .call_tool(CallToolRequestParams {
                            name: name.into(),
                            arguments: Some(args),
                            meta: None,
                            task: None,
                        })
                        .await
                })
                    as std::pin::Pin<
                        Box<
                            dyn std::future::Future<
                                    Output = Result<CallToolResult, rmcp::ServiceError>,
                                > + Send,
                        >,
                    >
            },
        );

        let client_clone = client_arc.clone();
        let list_tools = Box::new(move || {
            let client = client_clone.clone();
            Box::pin(async move {
                let client = client.read().await;
                client.list_tools(Default::default()).await
            })
                as std::pin::Pin<
                    Box<
                        dyn std::future::Future<
                                Output = Result<ListToolsResult, rmcp::ServiceError>,
                            > + Send,
                    >,
                >
        });

        Ok(McpClientInner {
            call_tool,
            list_tools,
        })
    }

    /// Connect via SSE transport
    async fn connect_sse(&self, _url: &str) -> Result<McpClientInner, McpClientError> {
        // SSE transport - simplify by just returning error for now
        // The rmcp SSE transport API is complex and requires specific client types
        Err(McpClientError::Connection(
            "SSE transport is not yet supported in this version. Use stdio transport instead."
                .to_string(),
        ))
    }

    /// Connect via WebSocket transport
    async fn connect_websocket(&self, _url: &str) -> Result<McpClientInner, McpClientError> {
        // WebSocket transport is not yet fully supported
        // TODO: Implement proper WebSocket transport when rmcp adds better support
        Err(McpClientError::Connection(
            "WebSocket transport is not yet supported. Use stdio or SSE instead.".to_string(),
        ))
    }

    /// Refresh tools from the server
    async fn refresh_tools(&mut self) -> Result<(), McpClientError> {
        let client_guard = self.client.read().await;
        let client = client_guard.as_ref().ok_or(McpClientError::NotConnected)?;

        let tools_result = (client.list_tools)()
            .await
            .map_err(|e| McpClientError::Protocol(format!("List tools failed: {}", e)))?;

        self.tools.clear();
        for tool in tools_result.tools {
            let info = McpToolInfo {
                name: tool.name.to_string(),
                description: tool.description.map(|d| d.to_string()).unwrap_or_default(),
                input_schema: serde_json::Value::Object(tool.input_schema.as_ref().clone()),
            };
            self.tools.insert(info.name.clone(), info);
        }

        Ok(())
    }

    /// List all available tools
    pub async fn list_tools(&self) -> Result<Vec<McpToolInfo>, McpClientError> {
        Ok(self.tools.values().cloned().collect())
    }

    /// Get tool information by name
    pub fn get_tool(&self, name: &str) -> Option<&McpToolInfo> {
        self.tools.get(name)
    }

    /// Call a tool on the MCP server
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, McpClientError> {
        let client_guard = self.client.read().await;
        let client = client_guard.as_ref().ok_or(McpClientError::NotConnected)?;

        // Convert Value to Map
        let args_map = if let serde_json::Value::Object(map) = arguments {
            map
        } else {
            serde_json::Map::new()
        };

        (client.call_tool)(name.to_string(), args_map)
            .await
            .map_err(|e| McpClientError::Protocol(format!("Tool call failed: {}", e)))
    }

    /// Get tools as a ToolCollection (wrapped as OpenManus tools)
    pub fn to_tool_collection(&self) -> ToolCollection {
        let mut collection = ToolCollection::new();
        let server_id = self.config.server_id.clone();
        let client = self.client.clone();

        for (name, info) in &self.tools {
            let info_clone = info.clone();
            let server_id_clone = server_id.clone();
            let client_clone = client.clone();
            let name_clone = name.clone();

            let call_fn = move |_tool_name: String, args: serde_json::Value| {
                let client = client_clone.clone();
                let name = name_clone.clone();
                Box::pin(async move {
                    let client_guard = client.read().await;
                    if let Some(client) = client_guard.as_ref() {
                        // Convert Value to Map
                        let args_map = if let serde_json::Value::Object(map) = args {
                            map
                        } else {
                            serde_json::Map::new()
                        };
                        let result =
                            (client.call_tool)(name.clone(), args_map)
                                .await
                                .map_err(|e| {
                                    ToolError::ExecutionFailed(format!("MCP call failed: {}", e))
                                })?;
                        Ok(mcp_result_to_tool_result(result))
                    } else {
                        Err(ToolError::ExecutionFailed(
                            "MCP client not connected".to_string(),
                        ))
                    }
                })
                    as std::pin::Pin<
                        Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>,
                    >
            };

            let wrapper = McpToolWrapper::new(info_clone, server_id_clone, call_fn);
            collection.add_tool(Arc::new(wrapper));
        }

        collection
    }

    /// Get the server ID
    pub fn server_id(&self) -> &str {
        &self.config.server_id
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Disconnect from the server
    pub async fn disconnect(&mut self) -> Result<(), McpClientError> {
        if self.connected {
            // Just clear the client - it will be dropped automatically
            let mut client_guard = self.client.write().await;
            *client_guard = None;
            self.connected = false;
            self.tools.clear();
            info!("Disconnected from MCP server '{}'", self.config.server_id);
        }
        Ok(())
    }
}

/// MCP Client errors
#[derive(Debug, thiserror::Error)]
pub enum McpClientError {
    #[error("Not connected to MCP server")]
    NotConnected,

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),
}

/// Manager for multiple MCP client connections
pub struct McpClientManager {
    clients: HashMap<String, McpClient>,
}

impl McpClientManager {
    /// Create a new client manager
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    /// Connect to a new MCP server
    pub async fn connect(&mut self, config: McpClientConfig) -> Result<(), McpClientError> {
        let server_id = if config.server_id.is_empty() {
            match &config.transport {
                TransportType::Stdio { command, .. } => command.clone(),
                TransportType::Sse { url, .. } => url.clone(),
                TransportType::WebSocket { url, .. } => url.clone(),
            }
        } else {
            config.server_id.clone()
        };

        // Disconnect existing connection if any
        if let Some(mut client) = self.clients.remove(&server_id) {
            let _ = client.disconnect().await;
        }

        let mut client = McpClient::new(McpClientConfig {
            server_id: server_id.clone(),
            ..config
        });
        client.connect().await?;
        self.clients.insert(server_id, client);

        Ok(())
    }

    /// Disconnect from a specific server
    pub async fn disconnect(&mut self, server_id: &str) -> Result<(), McpClientError> {
        if let Some(mut client) = self.clients.remove(server_id) {
            client.disconnect().await?;
        }
        Ok(())
    }

    /// Disconnect from all servers
    pub async fn disconnect_all(&mut self) {
        let server_ids: Vec<String> = self.clients.keys().cloned().collect();
        for server_id in server_ids {
            if let Some(mut client) = self.clients.remove(&server_id) {
                let _ = client.disconnect().await;
            }
        }
    }

    /// Get a client by server ID
    pub fn get_client(&self, server_id: &str) -> Option<&McpClient> {
        self.clients.get(server_id)
    }

    /// Get all connected clients
    pub fn clients(&self) -> &HashMap<String, McpClient> {
        &self.clients
    }

    /// Get all tools from all connected servers as a ToolCollection
    pub fn to_tool_collection(&self) -> ToolCollection {
        let mut collection = ToolCollection::new();

        for client in self.clients.values() {
            let client_collection = client.to_tool_collection();
            for tool in client_collection.tools() {
                collection.add_tool(tool);
            }
        }

        collection
    }

    /// List all available tools from all servers
    pub async fn list_all_tools(
        &self,
    ) -> Result<HashMap<String, Vec<McpToolInfo>>, McpClientError> {
        let mut all_tools = HashMap::new();

        for (server_id, client) in &self.clients {
            let tools = client.list_tools().await?;
            all_tools.insert(server_id.clone(), tools);
        }

        Ok(all_tools)
    }
}

impl Default for McpClientManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config() {
        let config = McpClientConfig {
            server_id: "test_server".to_string(),
            transport: TransportType::Stdio {
                command: "test-command".to_string(),
                args: vec![],
            },
            ..Default::default()
        };
        let client = McpClient::new(config);
        assert_eq!(client.config.server_id, "test_server");
        assert!(!client.is_connected());
    }

    #[test]
    fn test_client_manager() {
        let manager = McpClientManager::new();
        assert!(manager.clients.is_empty());
    }
}
