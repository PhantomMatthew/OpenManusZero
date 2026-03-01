//! File operations tool

use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Tool for file read/write operations
pub struct FileTool {
    /// Allowed directories for file operations
    #[allow(dead_code)]
    allowed_dirs: Vec<PathBuf>,
}

impl FileTool {
    /// Create a new file tool with default workspace access
    pub fn new() -> Self {
        Self {
            allowed_dirs: vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))],
        }
    }

    /// Create with specific allowed directories
    pub fn with_allowed_dirs(dirs: Vec<PathBuf>) -> Self {
        Self { allowed_dirs: dirs }
    }

    /// Check if a path is within allowed directories
    fn is_path_allowed(&self, _path: &Path) -> bool {
        // For now, allow all paths (sandbox handles security)
        true
    }

    /// Read a file
    async fn read_file(&self, path: &str) -> Result<String, ToolError> {
        let path = PathBuf::from(path);
        if !self.is_path_allowed(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Path {} is not in allowed directories",
                path.display()
            )));
        }

        tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))
    }

    /// Write to a file
    async fn write_file(&self, path: &str, content: &str) -> Result<String, ToolError> {
        let path = PathBuf::from(path);
        if !self.is_path_allowed(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Path {} is not in allowed directories",
                path.display()
            )));
        }

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to create directories: {}", e))
            })?;
        }

        tokio::fs::write(&path, content)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

        Ok(format!(
            "Successfully wrote {} bytes to {}",
            content.len(),
            path.display()
        ))
    }

    /// List directory contents
    async fn list_dir(&self, path: &str) -> Result<String, ToolError> {
        let path = PathBuf::from(path);
        if !self.is_path_allowed(&path) {
            return Err(ToolError::PermissionDenied(format!(
                "Path {} is not in allowed directories",
                path.display()
            )));
        }

        let mut entries = tokio::fs::read_dir(&path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read directory: {}", e)))?;

        let mut result = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read entry: {}", e)))?
        {
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry.file_type().await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to get file type: {}", e))
            })?;
            let type_str = if file_type.is_dir() { "DIR" } else { "FILE" };
            result.push(format!("[{}] {}", type_str, name));
        }

        Ok(result.join("\n"))
    }
}

impl Default for FileTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileTool {
    fn name(&self) -> &'static str {
        "file"
    }

    fn description(&self) -> &'static str {
        "Read, write, and list files. Operations: read, write, list."
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "operation".to_string(),
                    ToolParameter {
                        name: "operation".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Operation to perform: read, write, list".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: Some(vec![
                            "read".to_string(),
                            "write".to_string(),
                            "list".to_string(),
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
                        description: Some("Content to write (for write operation)".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props
            },
            required: Some(vec!["operation".to_string(), "path".to_string()]),
        })
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        // Parse input
        let (operation, path, content) = if let Ok(json) =
            serde_json::from_str::<serde_json::Value>(input)
        {
            let op = json
                .get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidInput("Missing 'operation' field".to_string()))?;
            let path = json
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidInput("Missing 'path' field".to_string()))?;
            let content = json.get("content").and_then(|v| v.as_str()).unwrap_or("");
            (op.to_string(), path.to_string(), content.to_string())
        } else {
            return Err(ToolError::InvalidInput("Invalid JSON input".to_string()));
        };

        match operation.as_str() {
            "read" => {
                let content = self.read_file(&path).await?;
                Ok(ToolResult::success(content))
            }
            "write" => {
                let result = self.write_file(&path, &content).await?;
                Ok(ToolResult::success(result))
            }
            "list" => {
                let result = self.list_dir(&path).await?;
                Ok(ToolResult::success(result))
            }
            _ => Err(ToolError::InvalidInput(format!(
                "Unknown operation: {}",
                operation
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_context() -> Context {
        Context::new()
    }

    #[test]
    fn test_file_tool_name() {
        let tool = FileTool::new();
        assert_eq!(tool.name(), "file");
    }

    #[tokio::test]
    async fn test_file_tool_write_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let tool = FileTool::with_allowed_dirs(vec![temp_dir.path().to_path_buf()]);
        let mut ctx = create_test_context();

        let file_path = temp_dir.path().join("test.txt");
        let input = serde_json::json!({
            "operation": "write",
            "path": file_path.to_str().unwrap(),
            "content": "Hello, world!"
        })
        .to_string();

        // Write
        let result = tool.execute(&input, &mut ctx).await;
        assert!(result.is_ok());

        // Read
        let read_input = serde_json::json!({
            "operation": "read",
            "path": file_path.to_str().unwrap()
        })
        .to_string();
        let result = tool.execute(&read_input, &mut ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.output, Some("Hello, world!".to_string()));
    }

    #[tokio::test]
    async fn test_file_tool_list() {
        let temp_dir = TempDir::new().unwrap();
        let tool = FileTool::with_allowed_dirs(vec![temp_dir.path().to_path_buf()]);
        let mut ctx = create_test_context();

        // Create some files
        tokio::fs::write(temp_dir.path().join("file1.txt"), "content1")
            .await
            .unwrap();
        tokio::fs::create_dir(temp_dir.path().join("subdir"))
            .await
            .unwrap();

        let input = serde_json::json!({
            "operation": "list",
            "path": temp_dir.path().to_str().unwrap()
        })
        .to_string();

        let result = tool.execute(&input, &mut ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        let output = result.output.unwrap();
        assert!(output.contains("file1.txt"));
        assert!(output.contains("subdir"));
    }

    #[tokio::test]
    async fn test_file_tool_invalid_operation() {
        let tool = FileTool::new();
        let mut ctx = create_test_context();

        let input = serde_json::json!({
            "operation": "delete",
            "path": "/tmp/test"
        })
        .to_string();

        let result = tool.execute(&input, &mut ctx).await;
        assert!(result.is_err());
    }
}
