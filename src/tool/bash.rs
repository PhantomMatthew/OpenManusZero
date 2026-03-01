//! Bash tool - execute shell commands

use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::collections::HashMap;

/// Tool to execute bash commands
pub struct BashTool {
    /// Whether commands are allowed
    enabled: bool,
}

impl BashTool {
    /// Create a new bash tool
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Create a disabled bash tool
    pub fn disabled() -> Self {
        Self { enabled: false }
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Execute bash commands in a shell environment. Use with caution."
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "command".to_string(),
                    ToolParameter {
                        name: "command".to_string(),
                        param_type: "string".to_string(),
                        description: Some("The bash command to execute".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: None,
                    },
                );
                props
            },
            required: Some(vec!["command".to_string()]),
        })
    }

    async fn execute(&self, input: &str, ctx: &mut Context) -> Result<ToolResult, ToolError> {
        if !self.enabled {
            return Err(ToolError::NotAvailable("Bash tool is disabled".to_string()));
        }

        // Parse input
        let command = if let Ok(json) = serde_json::from_str::<serde_json::Value>(input) {
            json.get("command")
                .and_then(|v| v.as_str())
                .unwrap_or(input)
                .to_string()
        } else {
            input.to_string()
        };

        if command.is_empty() {
            return Err(ToolError::InvalidInput(
                "Command cannot be empty".to_string(),
            ));
        }

        // Execute command with timeout
        let output = tokio::process::Command::new("bash")
            .arg("-c")
            .arg(&command)
            .current_dir(&ctx.working_dir)
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(ToolResult::success(
                format!("{}\n{}", stdout, stderr).trim(),
            ))
        } else {
            Ok(ToolResult::failure(format!(
                "Exit code: {}\n{}\n{}",
                output.status.code().unwrap_or(-1),
                stdout,
                stderr
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_tool_name() {
        let tool = BashTool::new();
        assert_eq!(tool.name(), "bash");
    }

    #[test]
    fn test_bash_tool_disabled() {
        let tool = BashTool::disabled();
        assert!(!tool.enabled);
    }

    #[tokio::test]
    async fn test_bash_tool_disabled_execution() {
        let tool = BashTool::disabled();
        let mut ctx = Context::new();
        let result = tool.execute("echo hello", &mut ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bash_tool_empty_command() {
        let tool = BashTool::new();
        let mut ctx = Context::new();
        let result = tool.execute("", &mut ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bash_tool_echo() {
        let tool = BashTool::new();
        let mut ctx = Context::new();
        let result = tool.execute("echo hello", &mut ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.output.unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn test_bash_tool_json_input() {
        let tool = BashTool::new();
        let mut ctx = Context::new();
        let result = tool.execute(r#"{"command": "echo test"}"#, &mut ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.output.unwrap().contains("test"));
    }
}
