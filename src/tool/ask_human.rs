//! AskHuman tool - requests user input

use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::collections::HashMap;

/// Type alias for the input callback
type InputCallback = Box<dyn Fn(&str) -> String + Send + Sync>;

/// Tool to ask the human for input
#[allow(clippy::type_complexity)]
pub struct AskHumanTool {
    /// Optional callback to get user input
    input_callback: Option<InputCallback>,
}

impl AskHumanTool {
    /// Create a new ask human tool
    pub fn new() -> Self {
        Self {
            input_callback: None,
        }
    }

    /// Create with a custom input callback (for testing or custom input)
    pub fn with_callback<F>(callback: F) -> Self
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        Self {
            input_callback: Some(Box::new(callback)),
        }
    }

    /// Get user input (default implementation using stdin)
    fn get_input(&self, prompt: &str) -> String {
        if let Some(ref callback) = self.input_callback {
            callback(prompt)
        } else {
            // Default: read from stdin
            use std::io::{self, Write};
            print!("{}", prompt);
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            input.trim().to_string()
        }
    }
}

impl Default for AskHumanTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AskHumanTool {
    fn name(&self) -> &'static str {
        "ask_human"
    }

    fn description(&self) -> &'static str {
        "Ask the human user for input or clarification. Use this when you need more information to complete the task."
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "question".to_string(),
                    ToolParameter {
                        name: "question".to_string(),
                        param_type: "string".to_string(),
                        description: Some("The question to ask the user".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: None,
                    },
                );
                props
            },
            required: Some(vec!["question".to_string()]),
        })
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        // Parse input as JSON to get question, or use input directly
        let question = if let Ok(json) = serde_json::from_str::<serde_json::Value>(input) {
            json.get("question")
                .and_then(|v| v.as_str())
                .unwrap_or(input)
                .to_string()
        } else {
            input.to_string()
        };

        if question.is_empty() {
            return Err(ToolError::InvalidInput(
                "Question cannot be empty".to_string(),
            ));
        }

        // Get user input
        let prompt = format!("{}: ", question);
        let response = self.get_input(&prompt);

        Ok(ToolResult::success(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ask_human_tool_name() {
        let tool = AskHumanTool::new();
        assert_eq!(tool.name(), "ask_human");
    }

    #[test]
    fn test_ask_human_tool_description() {
        let tool = AskHumanTool::new();
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_ask_human_tool_parameters() {
        let tool = AskHumanTool::new();
        let schema = tool.parameters();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert!(schema.properties.contains_key("question"));
    }

    #[tokio::test]
    async fn test_ask_human_tool_with_callback() {
        let tool = AskHumanTool::with_callback(|_| "test response".to_string());
        let mut ctx = Context::new();
        let result = tool.execute("What is your name?", &mut ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.output, Some("test response".to_string()));
    }

    #[tokio::test]
    async fn test_ask_human_tool_json_input() {
        let tool = AskHumanTool::with_callback(|_| "42".to_string());
        let mut ctx = Context::new();
        let result = tool
            .execute(r#"{"question": "What is 6 * 7?"}"#, &mut ctx)
            .await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.output, Some("42".to_string()));
    }

    #[tokio::test]
    async fn test_ask_human_tool_empty_input() {
        let tool = AskHumanTool::with_callback(|_| "".to_string());
        let mut ctx = Context::new();
        let result = tool.execute("", &mut ctx).await;
        assert!(result.is_err());
    }
}
