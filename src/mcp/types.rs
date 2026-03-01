//! MCP protocol types and configurations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transport type for MCP connections
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TransportType {
    /// Standard I/O transport (stdin/stdout)
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
    /// Server-Sent Events transport (HTTP)
    Sse {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    },
    /// WebSocket transport
    WebSocket {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

impl Default for TransportType {
    fn default() -> Self {
        TransportType::Stdio {
            command: String::new(),
            args: vec![],
        }
    }
}

/// Configuration for MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name (shown to clients)
    pub name: String,
    /// Server version
    #[serde(default = "default_version")]
    pub version: String,
    /// Server instructions for clients
    #[serde(default)]
    pub instructions: Option<String>,
    /// Transport configuration (for SSE/WebSocket modes)
    #[serde(default)]
    pub transport: Option<ServerTransportConfig>,
}

fn default_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: "openmanus".to_string(),
            version: default_version(),
            instructions: Some("OpenManus AI Agent Framework - Exposes tools for task automation, browser control, and code execution".to_string()),
            transport: None,
        }
    }
}

/// Server transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerTransportConfig {
    /// Host to bind to (for SSE/WebSocket)
    #[serde(default = "default_host")]
    pub host: String,
    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,
    /// Enable SSE endpoint
    #[serde(default = "default_true")]
    pub sse_enabled: bool,
    /// Enable WebSocket endpoint
    #[serde(default = "default_true")]
    pub websocket_enabled: bool,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_true() -> bool {
    true
}

impl Default for ServerTransportConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            sse_enabled: true,
            websocket_enabled: true,
        }
    }
}

/// Configuration for MCP client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientConfig {
    /// Server identifier for logging and tool namespacing
    #[serde(default)]
    pub server_id: String,
    /// Transport type
    pub transport: TransportType,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Auto-reconnect on disconnect
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
    /// Maximum reconnection attempts
    #[serde(default = "default_max_reconnect")]
    pub max_reconnect_attempts: u32,
}

fn default_timeout() -> u64 {
    30
}

fn default_max_reconnect() -> u32 {
    3
}

impl Default for McpClientConfig {
    fn default() -> Self {
        Self {
            server_id: String::new(),
            transport: TransportType::default(),
            timeout_secs: default_timeout(),
            auto_reconnect: true,
            max_reconnect_attempts: default_max_reconnect(),
        }
    }
}

/// MCP tool information received from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    /// Tool name
    pub name: String,
    /// Tool description
    #[serde(default)]
    pub description: String,
    /// Input schema (JSON Schema format)
    #[serde(default)]
    pub input_schema: serde_json::Value,
}

/// MCP tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    /// Tool output content
    pub content: Vec<McpContent>,
    /// Whether the tool execution resulted in an error
    #[serde(default)]
    pub is_error: bool,
}

/// Content type in MCP messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { resource: McpResource },
}

/// Resource reference in MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
}

/// MCP server configuration file format (mcp.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfigFile {
    /// MCP servers to connect to
    #[serde(default)]
    pub servers: HashMap<String, McpClientConfig>,
}

impl Default for McpConfigFile {
    fn default() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_type_stdio() {
        let transport = TransportType::Stdio {
            command: "mcp-server".to_string(),
            args: vec!["--port".to_string(), "8080".to_string()],
        };
        let json = serde_json::to_string(&transport).unwrap();
        assert!(json.contains("\"type\":\"stdio\""));
        assert!(json.contains("\"command\":\"mcp-server\""));
    }

    #[test]
    fn test_transport_type_sse() {
        let transport = TransportType::Sse {
            url: "http://localhost:8080/sse".to_string(),
            headers: HashMap::new(),
        };
        let json = serde_json::to_string(&transport).unwrap();
        assert!(json.contains("\"type\":\"sse\""));
    }

    #[test]
    fn test_server_config_default() {
        let config = McpServerConfig::default();
        assert_eq!(config.name, "openmanus");
        assert!(config.instructions.is_some());
    }

    #[test]
    fn test_client_config_default() {
        let config = McpClientConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert!(config.auto_reconnect);
    }

    #[test]
    fn test_mcp_content_serialization() {
        let content = McpContent::Text {
            text: "Hello, MCP!".to_string(),
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello, MCP!\""));
    }
}
