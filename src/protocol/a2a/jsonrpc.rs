//! A2A JSON-RPC Types
//!
//! JSON-RPC 2.0 types for A2A protocol

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::types::{A2AMessage, A2ATask};

/// JSON-RPC Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<JsonRpcId>,
    /// Method name
    pub method: String,
    /// Method parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC ID type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    Number(i64),
    String(String),
}

/// JSON-RPC Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<JsonRpcId>,
    /// Result (on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error (on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// Parse error
    pub fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "Parse error".to_string(),
            data: None,
        }
    }

    /// Invalid request
    pub fn invalid_request() -> Self {
        Self {
            code: -32600,
            message: "Invalid request".to_string(),
            data: None,
        }
    }

    /// Method not found
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }

    /// Invalid params
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
            data: None,
        }
    }

    /// Internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
            data: None,
        }
    }
}

impl JsonRpcRequest {
    /// Create a new request
    pub fn new(id: impl Into<JsonRpcId>, method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id.into()),
            method: method.into(),
            params: None,
        }
    }

    /// Parse from JSON
    pub fn from_json(json: &str) -> Result<Self, JsonRpcError> {
        serde_json::from_str(json).map_err(|_| JsonRpcError::parse_error())
    }

    /// Create a notification (no ID)
    pub fn notification(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: method.into(),
            params: None,
        }
    }

    /// Add parameters
    pub fn with_params(mut self, params: serde_json::Value) -> Self {
        self.params = Some(params);
        self
    }
}

impl JsonRpcResponse {
    /// Create a success response
    pub fn success(id: Option<JsonRpcId>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: Option<JsonRpcId>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"Internal error"}}"#
                .to_string()
        })
    }
}

/// A2A method: message/send params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSendParams {
    /// The message to send
    pub message: A2AMessage,
    /// Configuration options
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub configuration: HashMap<String, serde_json::Value>,
}

/// A2A method: tasks/get params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksGetParams {
    /// Task ID
    pub id: String,
    /// History length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_length: Option<usize>,
}

/// A2A method: tasks/cancel params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksCancelParams {
    /// Task ID
    pub id: String,
}

/// A2A method: tasks/pushNotification/set params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNotificationParams {
    /// Task ID
    pub id: String,
    /// Push notification config
    pub push_notification_config: PushNotificationConfig,
}

/// Push notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNotificationConfig {
    /// Notification URL
    pub url: String,
    /// Authentication token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// A2A methods
pub const METHOD_MESSAGE_SEND: &str = "message/send";
pub const METHOD_MESSAGE_STREAM: &str = "message/stream";
pub const METHOD_TASKS_GET: &str = "tasks/get";
pub const METHOD_TASKS_CANCEL: &str = "tasks/cancel";
pub const METHOD_TASKS_PUSH_NOTIFICATION_SET: &str = "tasks/pushNotification/set";

impl From<i64> for JsonRpcId {
    fn from(id: i64) -> Self {
        Self::Number(id)
    }
}

impl From<String> for JsonRpcId {
    fn from(id: String) -> Self {
        Self::String(id)
    }
}

impl From<&str> for JsonRpcId {
    fn from(id: &str) -> Self {
        Self::String(id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request_new() {
        let req = JsonRpcRequest::new(1, "test/method");
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "test/method");
    }

    #[test]
    fn test_jsonrpc_request_json() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{"key":"value"}}"#;
        let req = JsonRpcRequest::from_json(json).unwrap();
        assert_eq!(req.method, "test");
        assert!(req.params.is_some());
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let resp = JsonRpcResponse::success(
            Some(JsonRpcId::Number(1)),
            serde_json::json!({"result": "ok"}),
        );
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let resp = JsonRpcResponse::error(
            Some(JsonRpcId::Number(1)),
            JsonRpcError::method_not_found("unknown"),
        );
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
    }

    #[test]
    fn test_jsonrpc_error_codes() {
        let err = JsonRpcError::parse_error();
        assert_eq!(err.code, -32700);

        let err = JsonRpcError::invalid_request();
        assert_eq!(err.code, -32600);

        let err = JsonRpcError::method_not_found("test");
        assert_eq!(err.code, -32601);

        let err = JsonRpcError::invalid_params("bad params");
        assert_eq!(err.code, -32602);

        let err = JsonRpcError::internal_error("oops");
        assert_eq!(err.code, -32603);
    }

    #[test]
    fn test_message_send_params() {
        let json = r#"{"message":{"role":"user","parts":[{"kind":"text","text":"Hello"}]}}"#;
        let params: MessageSendParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.message.role, super::super::types::A2ARole::User);
    }
}
