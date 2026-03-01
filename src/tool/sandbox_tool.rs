//! Sandbox tools for executing commands in sandboxed environments
//!
//! Provides tools for:
//! - Shell command execution
//! - File operations in sandbox
//! - Browser automation in sandbox
//! - Vision/screen capture in sandbox

use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sandbox tool action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SandboxAction {
    /// Execute shell command
    Shell { command: String },
    /// Read file from sandbox
    ReadFile { path: String },
    /// Write file to sandbox
    WriteFile { path: String, content: String },
    /// List directory
    ListDir { path: String },
    /// Remove file or directory
    Remove { path: String },
    /// Take screenshot (vision)
    Screenshot,
    /// Open browser
    OpenBrowser { url: String },
    /// Click at coordinates
    Click { x: i32, y: i32 },
    /// Type text
    Type { text: String },
}

/// Sandbox tool configuration
#[derive(Debug, Clone)]
pub struct SandboxToolConfig {
    /// Sandbox ID to use
    pub sandbox_id: Option<String>,
    /// Default working directory
    pub workdir: String,
    /// Timeout for commands in seconds
    pub timeout_secs: u64,
}

impl Default for SandboxToolConfig {
    fn default() -> Self {
        Self {
            sandbox_id: None,
            workdir: "/workspace".to_string(),
            timeout_secs: 60,
        }
    }
}

/// Sandbox tool for executing commands in isolated environments
pub struct SandboxTool {
    config: SandboxToolConfig,
}

impl SandboxTool {
    /// Create a new sandbox tool
    pub fn new() -> Self {
        Self {
            config: SandboxToolConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: SandboxToolConfig) -> Self {
        Self { config }
    }

    /// Execute a sandbox action
    async fn execute_action(&self, action: SandboxAction) -> Result<String, ToolError> {
        match action {
            SandboxAction::Shell { command } => self.execute_shell(&command).await,
            SandboxAction::ReadFile { path } => self.read_file(&path).await,
            SandboxAction::WriteFile { path, content } => self.write_file(&path, &content).await,
            SandboxAction::ListDir { path } => self.list_dir(&path).await,
            SandboxAction::Remove { path } => self.remove(&path).await,
            SandboxAction::Screenshot => self.take_screenshot().await,
            SandboxAction::OpenBrowser { url } => self.open_browser(&url).await,
            SandboxAction::Click { x, y } => self.click(x, y).await,
            SandboxAction::Type { text } => self.type_text(&text).await,
        }
    }

    async fn execute_shell(&self, command: &str) -> Result<String, ToolError> {
        tracing::info!("Executing shell command in sandbox: {}", command);

        // In a real implementation, this would execute in the sandbox
        // For now, return a stub response
        Ok(format!(
            "[Sandbox {}] Executed: {}\n(Output would appear here)",
            self.config.sandbox_id.as_deref().unwrap_or("default"),
            command
        ))
    }

    async fn read_file(&self, path: &str) -> Result<String, ToolError> {
        tracing::info!("Reading file from sandbox: {}", path);

        Ok(format!(
            "[Sandbox] Content of {} would be returned here",
            path
        ))
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<String, ToolError> {
        tracing::info!("Writing file to sandbox: {} ({} bytes)", path, content.len());

        Ok(format!(
            "[Sandbox] Wrote {} bytes to {}",
            content.len(),
            path
        ))
    }

    async fn list_dir(&self, path: &str) -> Result<String, ToolError> {
        tracing::info!("Listing directory in sandbox: {}", path);

        Ok(format!(
            "[Sandbox] Directory listing for {}:\n- file1.txt\n- file2.rs\n- subdir/",
            path
        ))
    }

    async fn remove(&self, path: &str) -> Result<String, ToolError> {
        tracing::info!("Removing from sandbox: {}", path);

        Ok(format!("[Sandbox] Removed: {}", path))
    }

    async fn take_screenshot(&self) -> Result<String, ToolError> {
        tracing::info!("Taking screenshot in sandbox");

        Ok("[Sandbox] Screenshot captured (base64 data would be here)".to_string())
    }

    async fn open_browser(&self, url: &str) -> Result<String, ToolError> {
        tracing::info!("Opening browser in sandbox: {}", url);

        Ok(format!("[Sandbox] Browser opened to: {}", url))
    }

    async fn click(&self, x: i32, y: i32) -> Result<String, ToolError> {
        tracing::info!("Clicking in sandbox at ({}, {})", x, y);

        Ok(format!("[Sandbox] Clicked at ({}, {})", x, y))
    }

    async fn type_text(&self, text: &str) -> Result<String, ToolError> {
        tracing::info!("Typing in sandbox: {} chars", text.len());

        Ok(format!("[Sandbox] Typed: {}", text))
    }
}

impl Default for SandboxTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SandboxTool {
    fn name(&self) -> &'static str {
        "sandbox"
    }

    fn description(&self) -> &'static str {
        "Sandbox tool for executing commands in isolated environments"
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "action".to_string(),
                    ToolParameter {
                        name: "action".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Action to perform: shell, read_file, write_file, list_dir, remove, screenshot, open_browser, click, type".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: Some(vec![
                            "shell".to_string(),
                            "read_file".to_string(),
                            "write_file".to_string(),
                            "list_dir".to_string(),
                            "remove".to_string(),
                            "screenshot".to_string(),
                            "open_browser".to_string(),
                            "click".to_string(),
                            "type".to_string(),
                        ]),
                    },
                );
                props.insert(
                    "command".to_string(),
                    ToolParameter {
                        name: "command".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Shell command to execute".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "path".to_string(),
                    ToolParameter {
                        name: "path".to_string(),
                        param_type: "string".to_string(),
                        description: Some("File or directory path".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "content".to_string(),
                    ToolParameter {
                        name: "content".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Content to write to file".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "url".to_string(),
                    ToolParameter {
                        name: "url".to_string(),
                        param_type: "string".to_string(),
                        description: Some("URL to open in browser".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "x".to_string(),
                    ToolParameter {
                        name: "x".to_string(),
                        param_type: "integer".to_string(),
                        description: Some("X coordinate for click".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "y".to_string(),
                    ToolParameter {
                        name: "y".to_string(),
                        param_type: "integer".to_string(),
                        description: Some("Y coordinate for click".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "text".to_string(),
                    ToolParameter {
                        name: "text".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Text to type".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props
            },
            required: Some(vec!["action".to_string()]),
        })
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        let action: SandboxAction = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid sandbox action: {}", e)))?;

        let output = self.execute_action(action).await?;
        Ok(ToolResult::success(output))
    }
}

/// Shell sandbox tool (specialized for shell commands)
pub struct SandboxShellTool {
    sandbox_tool: SandboxTool,
}

impl SandboxShellTool {
    /// Create a new shell sandbox tool
    pub fn new() -> Self {
        Self {
            sandbox_tool: SandboxTool::new(),
        }
    }
}

impl Default for SandboxShellTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SandboxShellTool {
    fn name(&self) -> &'static str {
        "sandbox_shell"
    }

    fn description(&self) -> &'static str {
        "Execute shell commands in a sandboxed environment"
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
                        description: Some("Shell command to execute".to_string()),
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

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct ShellInput {
            command: String,
        }

        let cmd: ShellInput = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid shell input: {}", e)))?;

        let action = SandboxAction::Shell { command: cmd.command };
        let output = self.sandbox_tool.execute_action(action).await?;
        Ok(ToolResult::success(output))
    }
}

/// Vision sandbox tool (specialized for screenshots)
pub struct SandboxVisionTool {
    sandbox_tool: SandboxTool,
}

impl SandboxVisionTool {
    /// Create a new vision sandbox tool
    pub fn new() -> Self {
        Self {
            sandbox_tool: SandboxTool::new(),
        }
    }
}

impl Default for SandboxVisionTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SandboxVisionTool {
    fn name(&self) -> &'static str {
        "sandbox_vision"
    }

    fn description(&self) -> &'static str {
        "Capture screenshots in a sandboxed environment"
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: HashMap::new(),
            required: None,
        })
    }

    async fn execute(&self, _input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        let action = SandboxAction::Screenshot;
        let output = self.sandbox_tool.execute_action(action).await?;
        Ok(ToolResult::success(output))
    }
}

/// Browser sandbox tool (specialized for browser automation)
pub struct SandboxBrowserTool {
    sandbox_tool: SandboxTool,
}

impl SandboxBrowserTool {
    /// Create a new browser sandbox tool
    pub fn new() -> Self {
        Self {
            sandbox_tool: SandboxTool::new(),
        }
    }
}

impl Default for SandboxBrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SandboxBrowserTool {
    fn name(&self) -> &'static str {
        "sandbox_browser"
    }

    fn description(&self) -> &'static str {
        "Browser automation in a sandboxed environment"
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "action".to_string(),
                    ToolParameter {
                        name: "action".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Browser action: open, click, type".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: Some(vec![
                            "open".to_string(),
                            "click".to_string(),
                            "type".to_string(),
                        ]),
                    },
                );
                props.insert(
                    "url".to_string(),
                    ToolParameter {
                        name: "url".to_string(),
                        param_type: "string".to_string(),
                        description: Some("URL to open".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "x".to_string(),
                    ToolParameter {
                        name: "x".to_string(),
                        param_type: "integer".to_string(),
                        description: Some("X coordinate for click".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "y".to_string(),
                    ToolParameter {
                        name: "y".to_string(),
                        param_type: "integer".to_string(),
                        description: Some("Y coordinate for click".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "text".to_string(),
                    ToolParameter {
                        name: "text".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Text to type".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props
            },
            required: Some(vec!["action".to_string()]),
        })
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct BrowserInput {
            action: String,
            url: Option<String>,
            x: Option<i32>,
            y: Option<i32>,
            text: Option<String>,
        }

        let browser_input: BrowserInput = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid browser input: {}", e)))?;

        let action = match browser_input.action.as_str() {
            "open" => SandboxAction::OpenBrowser {
                url: browser_input.url.unwrap_or_default(),
            },
            "click" => SandboxAction::Click {
                x: browser_input.x.unwrap_or(0),
                y: browser_input.y.unwrap_or(0),
            },
            "type" => SandboxAction::Type {
                text: browser_input.text.unwrap_or_default(),
            },
            _ => {
                return Err(ToolError::InvalidInput(format!(
                    "Unknown browser action: {}",
                    browser_input.action
                )))
            }
        };

        let output = self.sandbox_tool.execute_action(action).await?;
        Ok(ToolResult::success(output))
    }
}

/// File sandbox tool (specialized for file operations)
pub struct SandboxFileTool {
    sandbox_tool: SandboxTool,
}

impl SandboxFileTool {
    /// Create a new file sandbox tool
    pub fn new() -> Self {
        Self {
            sandbox_tool: SandboxTool::new(),
        }
    }
}

impl Default for SandboxFileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SandboxFileTool {
    fn name(&self) -> &'static str {
        "sandbox_file"
    }

    fn description(&self) -> &'static str {
        "File operations in a sandboxed environment"
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "action".to_string(),
                    ToolParameter {
                        name: "action".to_string(),
                        param_type: "string".to_string(),
                        description: Some("File action: read, write, list, remove".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: Some(vec![
                            "read".to_string(),
                            "write".to_string(),
                            "list".to_string(),
                            "remove".to_string(),
                        ]),
                    },
                );
                props.insert(
                    "path".to_string(),
                    ToolParameter {
                        name: "path".to_string(),
                        param_type: "string".to_string(),
                        description: Some("File or directory path".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "content".to_string(),
                    ToolParameter {
                        name: "content".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Content to write (for write action)".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props
            },
            required: Some(vec!["action".to_string(), "path".to_string()]),
        })
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct FileInput {
            action: String,
            path: String,
            content: Option<String>,
        }

        let file_input: FileInput = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid file input: {}", e)))?;

        let action = match file_input.action.as_str() {
            "read" => SandboxAction::ReadFile { path: file_input.path },
            "write" => SandboxAction::WriteFile {
                path: file_input.path,
                content: file_input.content.unwrap_or_default(),
            },
            "list" => SandboxAction::ListDir { path: file_input.path },
            "remove" => SandboxAction::Remove { path: file_input.path },
            _ => {
                return Err(ToolError::InvalidInput(format!(
                    "Unknown file action: {}",
                    file_input.action
                )))
            }
        };

        let output = self.sandbox_tool.execute_action(action).await?;
        Ok(ToolResult::success(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_tool_new() {
        let tool = SandboxTool::new();
        assert_eq!(tool.name(), "sandbox");
    }

    #[test]
    fn test_sandbox_shell_tool_new() {
        let tool = SandboxShellTool::new();
        assert_eq!(tool.name(), "sandbox_shell");
    }

    #[test]
    fn test_sandbox_vision_tool_new() {
        let tool = SandboxVisionTool::new();
        assert_eq!(tool.name(), "sandbox_vision");
    }

    #[test]
    fn test_sandbox_browser_tool_new() {
        let tool = SandboxBrowserTool::new();
        assert_eq!(tool.name(), "sandbox_browser");
    }

    #[test]
    fn test_sandbox_file_tool_new() {
        let tool = SandboxFileTool::new();
        assert_eq!(tool.name(), "sandbox_file");
    }

    #[tokio::test]
    async fn test_execute_shell() {
        let tool = SandboxTool::new();
        let action = SandboxAction::Shell {
            command: "ls -la".to_string(),
        };
        let result = tool.execute_action(action).await.unwrap();
        assert!(result.contains("ls -la"));
    }

    #[tokio::test]
    async fn test_read_file() {
        let tool = SandboxTool::new();
        let action = SandboxAction::ReadFile {
            path: "/workspace/test.txt".to_string(),
        };
        let result = tool.execute_action(action).await.unwrap();
        assert!(result.contains("/workspace/test.txt"));
    }

    #[test]
    fn test_sandbox_action_deserialize() {
        let json = r#"{"action": "shell", "command": "echo hello"}"#;
        let action: SandboxAction = serde_json::from_str(json).unwrap();
        match action {
            SandboxAction::Shell { command } => {
                assert_eq!(command, "echo hello");
            }
            _ => panic!("Expected Shell action"),
        }
    }
}
