//! Python execution tool

use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::collections::HashMap;

/// Tool to execute Python code
pub struct PythonTool {
    /// Whether execution is enabled
    enabled: bool,
    /// Timeout in seconds
    timeout_secs: u64,
}

impl PythonTool {
    /// Create a new python tool
    pub fn new() -> Self {
        Self {
            enabled: true,
            timeout_secs: 30,
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self {
            enabled: true,
            timeout_secs,
        }
    }

    /// Create a disabled python tool
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            timeout_secs: 30,
        }
    }
}

impl Default for PythonTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PythonTool {
    fn name(&self) -> &'static str {
        "python"
    }

    fn description(&self) -> &'static str {
        "Execute Python code and return the output. Use for calculations, data processing, etc."
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "code".to_string(),
                    ToolParameter {
                        name: "code".to_string(),
                        param_type: "string".to_string(),
                        description: Some("The Python code to execute".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: None,
                    },
                );
                props
            },
            required: Some(vec!["code".to_string()]),
        })
    }

    async fn execute(&self, input: &str, ctx: &mut Context) -> Result<ToolResult, ToolError> {
        if !self.enabled {
            return Err(ToolError::NotAvailable(
                "Python tool is disabled".to_string(),
            ));
        }

        // Parse input
        let code = if let Ok(json) = serde_json::from_str::<serde_json::Value>(input) {
            json.get("code")
                .and_then(|v| v.as_str())
                .unwrap_or(input)
                .to_string()
        } else {
            input.to_string()
        };

        if code.is_empty() {
            return Err(ToolError::InvalidInput("Code cannot be empty".to_string()));
        }

        // Execute Python code
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            tokio::process::Command::new("python3")
                .arg("-c")
                .arg(&code)
                .current_dir(&ctx.working_dir)
                .output(),
        )
        .await
        .map_err(|_| ToolError::Timeout("Python execution timed out".to_string()))?
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);

        if result.status.success() {
            Ok(ToolResult::success(stdout.trim().to_string()))
        } else {
            Ok(ToolResult::failure(format!(
                "Python error:\n{}\n{}",
                stdout, stderr
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_tool_name() {
        let tool = PythonTool::new();
        assert_eq!(tool.name(), "python");
    }

    #[test]
    fn test_python_tool_disabled() {
        let tool = PythonTool::disabled();
        assert!(!tool.enabled);
    }

    #[tokio::test]
    async fn test_python_tool_disabled_execution() {
        let tool = PythonTool::disabled();
        let mut ctx = Context::new();
        let result = tool.execute("print('hello')", &mut ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_python_tool_empty_code() {
        let tool = PythonTool::new();
        let mut ctx = Context::new();
        let result = tool.execute("", &mut ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_python_tool_print() {
        let tool = PythonTool::new();
        let mut ctx = Context::new();
        let result = tool.execute("print('hello world')", &mut ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.output, Some("hello world".to_string()));
    }

    #[tokio::test]
    async fn test_python_tool_calculation() {
        let tool = PythonTool::new();
        let mut ctx = Context::new();
        let result = tool.execute("print(2 + 2)", &mut ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.output, Some("4".to_string()));
    }
}
