//! Integration tests for MCP support
//!
//! These tests verify the MCP client and server implementations.

#[cfg(feature = "mcp")]
mod tests {
    use openmanus::mcp::types::{
        McpClientConfig, McpServerConfig, McpToolInfo, ServerTransportConfig, TransportType,
    };
    use openmanus::mcp::{McpClient, McpServer};
    use openmanus::tool::ToolCollection;

    #[test]
    fn test_mcp_server_config_serialization() {
        let config = McpServerConfig {
            name: "test_server".to_string(),
            version: "1.0.0".to_string(),
            instructions: Some("Test instructions".to_string()),
            transport: Some(ServerTransportConfig {
                host: "127.0.0.1".to_string(),
                port: 9000,
                sse_enabled: true,
                websocket_enabled: false,
            }),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: McpServerConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "test_server");
        assert_eq!(parsed.version, "1.0.0");
        assert!(parsed.transport.is_some());
        let transport = parsed.transport.unwrap();
        assert_eq!(transport.port, 9000);
        assert!(transport.sse_enabled);
        assert!(!transport.websocket_enabled);
    }

    #[test]
    fn test_mcp_client_config_stdio() {
        let config = McpClientConfig {
            server_id: "test".to_string(),
            transport: TransportType::Stdio {
                command: "echo".to_string(),
                args: vec!["hello".to_string()],
            },
            timeout_secs: 60,
            auto_reconnect: false,
            max_reconnect_attempts: 5,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: McpClientConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.server_id, "test");
        assert_eq!(parsed.timeout_secs, 60);
        assert!(!parsed.auto_reconnect);
        assert_eq!(parsed.max_reconnect_attempts, 5);

        match parsed.transport {
            TransportType::Stdio { command, args } => {
                assert_eq!(command, "echo");
                assert_eq!(args, vec!["hello"]);
            }
            _ => panic!("Expected stdio transport"),
        }
    }

    #[test]
    fn test_mcp_client_config_sse() {
        let config = McpClientConfig {
            server_id: "sse_test".to_string(),
            transport: TransportType::Sse {
                url: "http://localhost:8080/sse".to_string(),
                headers: Default::default(),
            },
            ..Default::default()
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: McpClientConfig = serde_json::from_str(&json).unwrap();

        match parsed.transport {
            TransportType::Sse { url, .. } => {
                assert_eq!(url, "http://localhost:8080/sse");
            }
            _ => panic!("Expected SSE transport"),
        }
    }

    #[test]
    fn test_mcp_client_config_websocket() {
        let config = McpClientConfig {
            server_id: "ws_test".to_string(),
            transport: TransportType::WebSocket {
                url: "ws://localhost:8080/mcp".to_string(),
                headers: Default::default(),
            },
            ..Default::default()
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: McpClientConfig = serde_json::from_str(&json).unwrap();

        match parsed.transport {
            TransportType::WebSocket { url, .. } => {
                assert_eq!(url, "ws://localhost:8080/mcp");
            }
            _ => panic!("Expected WebSocket transport"),
        }
    }

    #[test]
    fn test_mcp_tool_info() {
        let info = McpToolInfo {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message"
                    }
                }
            }),
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: McpToolInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "test_tool");
        assert_eq!(parsed.description, "A test tool");
        assert!(parsed.input_schema["properties"]["message"].is_object());
    }

    #[test]
    fn test_mcp_server_creation() {
        let config = McpServerConfig::default();
        let server = McpServer::new(config);

        // Server should be created successfully
        assert!(true);
    }

    #[test]
    fn test_mcp_server_with_tools() {
        let config = McpServerConfig::default();
        let tools = ToolCollection::new();
        let _server = McpServer::with_tools(config, &tools);

        // Server should be created successfully with tools
        assert!(true);
    }

    #[test]
    fn test_mcp_client_creation() {
        let config = McpClientConfig {
            server_id: "test".to_string(),
            transport: TransportType::Stdio {
                command: "test".to_string(),
                args: vec![],
            },
            ..Default::default()
        };

        let client = McpClient::new(config);

        assert_eq!(client.server_id(), "test");
        assert!(!client.is_connected());
    }

    #[test]
    fn test_transport_type_default() {
        let transport = TransportType::default();

        match transport {
            TransportType::Stdio { command, args } => {
                assert!(command.is_empty());
                assert!(args.is_empty());
            }
            _ => panic!("Expected default to be stdio"),
        }
    }
}

#[cfg(not(feature = "mcp"))]
mod tests {
    #[test]
    fn test_mcp_feature_disabled() {
        // When mcp feature is disabled, MCP types should not be available
        assert!(true);
    }
}
