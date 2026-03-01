//! Configuration management for OpenManus
//!
//! Handles loading configuration from files and environment variables.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[cfg(feature = "mcp")]
use crate::mcp::types::{McpClientConfig, McpConfigFile, TransportType};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// LLM configuration
    #[serde(default)]
    pub llm: LlmConfig,
    /// Agent configuration
    #[serde(default)]
    pub agent: AgentConfig,
    /// Tool configuration
    #[serde(default)]
    pub tools: ToolConfig,
    /// MCP configuration (when mcp feature is enabled)
    #[cfg(feature = "mcp")]
    #[serde(default)]
    pub mcp: McpConfigSection,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LlmConfig::default(),
            agent: AgentConfig::default(),
            tools: ToolConfig::default(),
            #[cfg(feature = "mcp")]
            mcp: McpConfigSection::default(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content =
            std::fs::read_to_string(path.as_ref()).map_err(|e| ConfigError::Io(e.to_string()))?;
        toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// Load configuration from a JSON file
    pub fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content =
            std::fs::read_to_string(path.as_ref()).map_err(|e| ConfigError::Io(e.to_string()))?;
        serde_json::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// Save configuration to a TOML file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content =
            toml::to_string_pretty(self).map_err(|e| ConfigError::Serialize(e.to_string()))?;
        std::fs::write(path.as_ref(), content).map_err(|e| ConfigError::Io(e.to_string()))
    }

    /// Load MCP configuration from a JSON file
    #[cfg(feature = "mcp")]
    pub fn load_mcp_config<P: AsRef<Path>>(path: P) -> Result<McpConfigFile, ConfigError> {
        let content =
            std::fs::read_to_string(path.as_ref()).map_err(|e| ConfigError::Io(e.to_string()))?;
        serde_json::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))
    }
}

/// LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Default model to use
    #[serde(default = "default_model")]
    pub model: String,
    /// API key (can also be set via OPENAI_API_KEY env var)
    #[serde(default)]
    pub api_key: Option<String>,
    /// API base URL
    #[serde(default)]
    pub base_url: Option<String>,
    /// Maximum tokens for responses
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// Temperature for responses
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_model() -> String {
    "gpt-4".to_string()
}

fn default_max_tokens() -> usize {
    4096
}

fn default_temperature() -> f32 {
    0.7
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            api_key: None,
            base_url: None,
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
        }
    }
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Maximum steps before stopping
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,
    /// Maximum observation length
    #[serde(default = "default_max_observe")]
    pub max_observe: usize,
    /// Enable debug logging
    #[serde(default)]
    pub debug: bool,
}

fn default_max_steps() -> usize {
    30
}

fn default_max_observe() -> usize {
    10000
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: default_max_steps(),
            max_observe: default_max_observe(),
            debug: false,
        }
    }
}

/// Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Enable browser tool
    #[serde(default = "default_true")]
    pub browser_enabled: bool,
    /// Enable bash tool
    #[serde(default = "default_true")]
    pub bash_enabled: bool,
    /// Enable python tool
    #[serde(default = "default_true")]
    pub python_enabled: bool,
    /// Enable file operations
    #[serde(default = "default_true")]
    pub file_ops_enabled: bool,
    /// Browser headless mode
    #[serde(default = "default_true")]
    pub browser_headless: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            browser_enabled: true,
            bash_enabled: true,
            python_enabled: true,
            file_ops_enabled: true,
            browser_headless: true,
        }
    }
}

/// MCP configuration section
#[cfg(feature = "mcp")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfigSection {
    /// MCP servers to connect to
    #[serde(default)]
    pub servers: HashMap<String, McpServerEntry>,
    /// MCP server configuration (for exposing tools)
    #[serde(default)]
    pub server: McpServerEntryConfig,
}

#[cfg(feature = "mcp")]
impl Default for McpConfigSection {
    fn default() -> Self {
        Self {
            servers: HashMap::new(),
            server: McpServerEntryConfig::default(),
        }
    }
}

/// MCP server entry in configuration
#[cfg(feature = "mcp")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    /// Transport type
    #[serde(rename = "type")]
    pub transport_type: String,
    /// Command (for stdio)
    #[serde(default)]
    pub command: Option<String>,
    /// Arguments (for stdio)
    #[serde(default)]
    pub args: Option<Vec<String>>,
    /// URL (for SSE/WebSocket)
    #[serde(default)]
    pub url: Option<String>,
    /// Headers (for SSE/WebSocket)
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
}

#[cfg(feature = "mcp")]
impl McpServerEntry {
    /// Convert to McpClientConfig
    pub fn to_client_config(&self, server_id: String) -> Result<McpClientConfig, ConfigError> {
        let transport = match self.transport_type.as_str() {
            "stdio" => {
                let command = self.command.clone().ok_or_else(|| {
                    ConfigError::Validation("command is required for stdio transport".to_string())
                })?;
                TransportType::Stdio {
                    command,
                    args: self.args.clone().unwrap_or_default(),
                }
            }
            "sse" => {
                let url = self.url.clone().ok_or_else(|| {
                    ConfigError::Validation("url is required for sse transport".to_string())
                })?;
                TransportType::Sse {
                    url,
                    headers: self.headers.clone().unwrap_or_default(),
                }
            }
            "websocket" | "ws" => {
                let url = self.url.clone().ok_or_else(|| {
                    ConfigError::Validation("url is required for websocket transport".to_string())
                })?;
                TransportType::WebSocket {
                    url,
                    headers: self.headers.clone().unwrap_or_default(),
                }
            }
            _ => {
                return Err(ConfigError::Validation(format!(
                    "Unknown transport type: {}",
                    self.transport_type
                )));
            }
        };

        Ok(McpClientConfig {
            server_id,
            transport,
            ..Default::default()
        })
    }
}

/// MCP server configuration for exposing tools
#[cfg(feature = "mcp")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntryConfig {
    /// Enable MCP server
    #[serde(default)]
    pub enabled: bool,
    /// Server name
    #[serde(default = "default_mcp_server_name")]
    pub name: String,
    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,
    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,
}

#[cfg(feature = "mcp")]
fn default_mcp_server_name() -> String {
    "openmanus".to_string()
}

#[cfg(feature = "mcp")]
fn default_host() -> String {
    "127.0.0.1".to_string()
}

#[cfg(feature = "mcp")]
fn default_port() -> u16 {
    8080
}

#[cfg(feature = "mcp")]
impl Default for McpServerEntryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            name: default_mcp_server_name(),
            host: default_host(),
            port: default_port(),
        }
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Serialize error: {0}")]
    Serialize(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.llm.model, "gpt-4");
        assert_eq!(config.agent.max_steps, 30);
    }

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.max_tokens, 4096);
        assert!((config.temperature - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.max_steps, 30);
        assert_eq!(config.max_observe, 10000);
        assert!(!config.debug);
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn test_mcp_server_entry_stdio() {
        let entry = McpServerEntry {
            transport_type: "stdio".to_string(),
            command: Some("mcp-server".to_string()),
            args: Some(vec!["--port".to_string(), "8080".to_string()]),
            url: None,
            headers: None,
        };

        let config = entry.to_client_config("test".to_string()).unwrap();
        assert_eq!(config.server_id, "test");
        match config.transport {
            TransportType::Stdio { command, args } => {
                assert_eq!(command, "mcp-server");
                assert_eq!(args, vec!["--port", "8080"]);
            }
            _ => panic!("Expected stdio transport"),
        }
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn test_mcp_server_entry_sse() {
        let entry = McpServerEntry {
            transport_type: "sse".to_string(),
            command: None,
            args: None,
            url: Some("http://localhost:8080/sse".to_string()),
            headers: None,
        };

        let config = entry.to_client_config("test".to_string()).unwrap();
        match config.transport {
            TransportType::Sse { url, .. } => {
                assert_eq!(url, "http://localhost:8080/sse");
            }
            _ => panic!("Expected sse transport"),
        }
    }

    #[test]
    fn test_config_serialize() {
        let config = Config::default();
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("model = \"gpt-4\""));
    }
}
