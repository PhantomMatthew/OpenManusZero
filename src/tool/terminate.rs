//! Terminate tool - ends agent execution

use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolResult};
use async_trait::async_trait;

/// Tool to terminate agent execution
pub struct TerminateTool;

impl TerminateTool {
    /// Create a new terminate tool
    pub fn new() -> Self {
        Self
    }
}

impl Default for TerminateTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TerminateTool {
    fn name(&self) -> &'static str {
        "terminate"
    }

    fn description(&self) -> &'static str {
        "End the agent execution. Use this when the task is complete."
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        let message = if input.is_empty() {
            "Execution terminated successfully.".to_string()
        } else {
            format!("Execution terminated: {}", input)
        };
        Ok(ToolResult::success(message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminate_tool_name() {
        let tool = TerminateTool::new();
        assert_eq!(tool.name(), "terminate");
    }

    #[test]
    fn test_terminate_tool_description() {
        let tool = TerminateTool::new();
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_terminate_tool_execute_empty() {
        let tool = TerminateTool::new();
        let mut ctx = Context::new();
        let result = tool.execute("", &mut ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(
            result.output,
            Some("Execution terminated successfully.".to_string())
        );
    }

    #[tokio::test]
    async fn test_terminate_tool_execute_with_message() {
        let tool = TerminateTool::new();
        let mut ctx = Context::new();
        let result = tool.execute("Task completed!", &mut ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(
            result.output,
            Some("Execution terminated: Task completed!".to_string())
        );
    }
}
