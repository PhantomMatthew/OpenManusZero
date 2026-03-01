//! Base tool trait and types

use crate::context::Context;
use crate::error::ToolError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of a tool execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolResult {
    /// Output from the tool
    pub output: Option<String>,
    /// Error message if the tool failed
    pub error: Option<String>,
    /// Base64 encoded image if the tool produces an image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64_image: Option<String>,
    /// System message or metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

impl ToolResult {
    /// Create a successful tool result with output
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: Some(output.into()),
            error: None,
            base64_image: None,
            system: None,
        }
    }

    /// Create a failed tool result with error
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            output: None,
            error: Some(error.into()),
            base64_image: None,
            system: None,
        }
    }

    /// Create a tool result with an image
    pub fn with_image(output: impl Into<String>, image: impl Into<String>) -> Self {
        Self {
            output: Some(output.into()),
            error: None,
            base64_image: Some(image.into()),
            system: None,
        }
    }

    /// Check if the result has any content
    pub fn is_empty(&self) -> bool {
        self.output.is_none()
            && self.error.is_none()
            && self.base64_image.is_none()
            && self.system.is_none()
    }

    /// Check if the result is an error
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

impl std::fmt::Display for ToolResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref error) = self.error {
            write!(f, "Error: {}", error)
        } else if let Some(ref output) = self.output {
            write!(f, "{}", output)
        } else {
            write!(f, "")
        }
    }
}

/// Parameter definition for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Parameter name (may be empty when deserialized from a JSON schema properties map,
    /// where the name is the map key rather than a field value)
    #[serde(default)]
    pub name: String,
    /// Parameter type (string, number, boolean, object, array)
    #[serde(rename = "type")]
    pub param_type: String,
    /// Parameter description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the parameter is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    /// Default value for the parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Enum values if the parameter is an enum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

/// Schema definition for a tool's parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Parameter type (usually "object")
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Parameter definitions
    pub properties: HashMap<String, ToolParameter>,
    /// List of required parameter names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

impl Default for ToolSchema {
    fn default() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: HashMap::new(),
            required: None,
        }
    }
}

/// Tool definition in OpenAI function calling format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool type (always "function")
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function definition
    pub function: FunctionDefinition,
}

/// Function definition for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// Parameter schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<ToolSchema>,
}

/// Trait that all tools must implement
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name
    fn name(&self) -> &'static str;

    /// Get the tool description
    fn description(&self) -> &'static str;

    /// Get the tool parameter schema
    fn parameters(&self) -> Option<ToolSchema> {
        None
    }

    /// Convert to OpenAI function calling format
    fn to_definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: self.name().to_string(),
                description: self.description().to_string(),
                parameters: self.parameters(),
            },
        }
    }

    /// Execute the tool with given input
    async fn execute(&self, input: &str, ctx: &mut Context) -> Result<ToolResult, ToolError>;

    /// Cleanup resources (optional)
    async fn cleanup(&self) -> Result<(), ToolError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("Hello, world!");
        assert_eq!(result.output, Some("Hello, world!".to_string()));
        assert!(result.error.is_none());
        assert!(!result.is_error());
    }

    #[test]
    fn test_tool_result_failure() {
        let result = ToolResult::failure("Something went wrong");
        assert!(result.output.is_none());
        assert_eq!(result.error, Some("Something went wrong".to_string()));
        assert!(result.is_error());
    }

    #[test]
    fn test_tool_result_with_image() {
        let result = ToolResult::with_image("Screenshot", "base64imagedata");
        assert_eq!(result.output, Some("Screenshot".to_string()));
        assert_eq!(result.base64_image, Some("base64imagedata".to_string()));
    }

    #[test]
    fn test_tool_result_is_empty() {
        let result = ToolResult::default();
        assert!(result.is_empty());

        let result = ToolResult::success("data");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_tool_result_display() {
        let result = ToolResult::success("output");
        assert_eq!(format!("{}", result), "output");

        let result = ToolResult::failure("error");
        assert_eq!(format!("{}", result), "Error: error");
    }

    #[test]
    fn test_tool_schema_default() {
        let schema = ToolSchema::default();
        assert_eq!(schema.schema_type, "object");
        assert!(schema.properties.is_empty());
    }

    #[test]
    fn test_tool_parameter() {
        let param = ToolParameter {
            name: "command".to_string(),
            param_type: "string".to_string(),
            description: Some("The command to execute".to_string()),
            required: Some(true),
            default: None,
            enum_values: None,
        };
        assert_eq!(param.name, "command");
        assert_eq!(param.param_type, "string");
    }

    #[test]
    fn test_tool_definition_serialization() {
        let def = ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "bash".to_string(),
                description: "Execute bash commands".to_string(),
                parameters: None,
            },
        };
        let json = serde_json::to_string(&def).unwrap();
        assert!(json.contains("\"name\":\"bash\""));
        assert!(json.contains("\"type\":\"function\""));
    }
}
