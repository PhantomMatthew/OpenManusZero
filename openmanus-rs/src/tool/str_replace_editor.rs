//! StrReplaceEditor - file editing tool with undo support

use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;

/// Number of context lines to show around edits
const SNIPPET_LINES: usize = 4;

/// Maximum response length before truncation
const MAX_RESPONSE_LEN: usize = 16000;

/// Truncation message
const TRUNCATED_MESSAGE: &str = "<response clipped><NOTE>To save on context only part of this file has been shown to you. You should retry this tool after you have searched inside the file with `grep -n` in order to find the line numbers of what you are looking for.</NOTE>";

/// Commands supported by StrReplaceEditor
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    View,
    Create,
    StrReplace,
    Insert,
    UndoEdit,
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::View => write!(f, "view"),
            Command::Create => write!(f, "create"),
            Command::StrReplace => write!(f, "str_replace"),
            Command::Insert => write!(f, "insert"),
            Command::UndoEdit => write!(f, "undo_edit"),
        }
    }
}

impl std::str::FromStr for Command {
    type Err = ToolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "view" => Ok(Command::View),
            "create" => Ok(Command::Create),
            "str_replace" => Ok(Command::StrReplace),
            "insert" => Ok(Command::Insert),
            "undo_edit" => Ok(Command::UndoEdit),
            _ => Err(ToolError::InvalidInput(format!(
                "Unknown command: {}. Allowed: view, create, str_replace, insert, undo_edit",
                s
            ))),
        }
    }
}

/// Parsed input for StrReplaceEditor
#[derive(Debug, Default)]
struct EditorInput {
    command: Option<Command>,
    path: Option<String>,
    file_text: Option<String>,
    old_str: Option<String>,
    new_str: Option<String>,
    insert_line: Option<i32>,
    view_range: Option<Vec<i32>>,
}

impl EditorInput {
    fn from_json(json: &serde_json::Value) -> Self {
        Self {
            command: json.get("command")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            path: json.get("path").and_then(|v| v.as_str()).map(String::from),
            file_text: json.get("file_text").and_then(|v| v.as_str()).map(String::from),
            old_str: json.get("old_str").and_then(|v| v.as_str()).map(String::from),
            new_str: json.get("new_str").and_then(|v| v.as_str()).map(String::from),
            insert_line: json.get("insert_line").and_then(|v| v.as_i64()).map(|i| i as i32),
            view_range: json.get("view_range")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_i64().map(|i| i as i32)).collect()),
        }
    }
}

/// File editing tool with undo support
pub struct StrReplaceEditor {
    /// File history for undo (path -> list of previous contents)
    file_history: Arc<AsyncRwLock<HashMap<PathBuf, Vec<String>>>>,
}

impl StrReplaceEditor {
    /// Create a new StrReplaceEditor
    pub fn new() -> Self {
        Self {
            file_history: Arc::new(AsyncRwLock::new(HashMap::new())),
        }
    }

    /// Truncate content if too long
    fn maybe_truncate(content: &str) -> String {
        if content.len() > MAX_RESPONSE_LEN {
            format!("{}{}", &content[..MAX_RESPONSE_LEN], TRUNCATED_MESSAGE)
        } else {
            content.to_string()
        }
    }

    /// Format file content with line numbers (cat -n style)
    fn make_output(content: &str, descriptor: &str, init_line: usize) -> String {
        let content = Self::maybe_truncate(content);
        let content = content.replace('\t', "    "); // expandtabs

        let numbered = content
            .lines()
            .enumerate()
            .map(|(i, line)| format!("{:6}\t{}", i + init_line, line))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "Here's the result of running `cat -n` on {}:\n{}\n",
            descriptor, numbered
        )
    }

    /// Validate that path is absolute
    fn validate_absolute_path(path: &str) -> Result<PathBuf, ToolError> {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err(ToolError::InvalidInput(format!(
                "The path {} is not an absolute path",
                path.display()
            )));
        }
        Ok(path)
    }

    /// Check if path exists
    async fn path_exists(path: &PathBuf) -> bool {
        tokio::fs::try_exists(path).await.unwrap_or(false)
    }

    /// Check if path is a directory
    async fn is_directory(path: &PathBuf) -> bool {
        tokio::fs::metadata(path)
            .await
            .map(|m| m.is_dir())
            .unwrap_or(false)
    }

    /// Read file content
    async fn read_file(path: &PathBuf) -> Result<String, ToolError> {
        tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))
    }

    /// Write file content
    async fn write_file(path: &PathBuf, content: &str) -> Result<(), ToolError> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create directories: {}", e)))?;
        }
        tokio::fs::write(path, content)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))
    }

    /// Save file to history for undo
    async fn save_history(&self, path: &PathBuf, content: String) {
        let mut history = self.file_history.write().await;
        history.entry(path.clone()).or_default().push(content);
    }

    /// Pop last history for undo
    async fn pop_history(&self, path: &PathBuf) -> Option<String> {
        let mut history = self.file_history.write().await;
        history.get_mut(path).and_then(|h| h.pop())
    }

    /// View directory contents using find command
    async fn view_directory(&self, path: &PathBuf) -> Result<ToolResult, ToolError> {
        let output = tokio::process::Command::new("find")
            .arg(path)
            .arg("-maxdepth")
            .arg("2")
            .arg("-not")
            .arg("-path")
            .arg("*/\\.*")
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list directory: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stderr.is_empty() {
            return Ok(ToolResult::failure(stderr.to_string()));
        }

        Ok(ToolResult::success(format!(
            "Here's the files and directories up to 2 levels deep in {}, excluding hidden items:\n{}\n",
            path.display(),
            stdout
        )))
    }

    /// View file with optional line range
    async fn view_file(
        &self,
        path: &PathBuf,
        view_range: Option<Vec<i32>>,
    ) -> Result<ToolResult, ToolError> {
        // Check if file exists
        if !Self::path_exists(path).await {
            return Err(ToolError::ExecutionFailed(format!(
                "File does not exist: {}",
                path.display()
            )));
        }

        // Read file content
        let content = Self::read_file(path).await?;

        // Handle view range
        if let Some(range) = view_range {
            if range.len() != 2 {
                return Err(ToolError::InvalidInput(
                    "view_range must have exactly 2 elements [start, end]".to_string(),
                ));
            }

            let start = range[0];
            let end = range[1];

            if start < 1 {
                return Err(ToolError::InvalidInput(
                    "view_range start must be >= 1".to_string(),
                ));
            }

            let lines: Vec<&str> = content.lines().collect();
            let total_lines = lines.len() as i32;

            // Calculate end line (handle -1 for end of file)
            let end_line = if end == -1 { total_lines } else { end };

            if end_line > total_lines {
                return Err(ToolError::InvalidInput(format!(
                    "view_range end ({}) exceeds file length ({})",
                    end_line, total_lines
                )));
            }

            if start > end_line {
                return Err(ToolError::InvalidInput(format!(
                    "view_range start ({}) > end ({})",
                    start, end_line
                )));
            }

            // Extract lines (convert to 0-indexed)
            let start_idx = (start - 1) as usize;
            let end_idx = end_line as usize;
            let selected_lines: Vec<&str> = lines[start_idx..end_idx].to_vec();
            let selected_content = selected_lines.join("\n");

            Ok(ToolResult::success(Self::make_output(
                &selected_content,
                &format!("{}: [{}-{}]", path.display(), start, end_line),
                start as usize,
            )))
        } else {
            // Show full file
            Ok(ToolResult::success(Self::make_output(
                &content,
                &path.display().to_string(),
                1,
            )))
        }
    }

    /// Handle view command - dispatch based on path type
    pub async fn handle_view(
        &self,
        path: &PathBuf,
        view_range: Option<Vec<i32>>,
    ) -> Result<ToolResult, ToolError> {
        // Check if path exists
        if !Self::path_exists(path).await {
            return Err(ToolError::ExecutionFailed(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }

        // Check if path is a directory or file
        if Self::is_directory(path).await {
            self.view_directory(path).await
        } else {
            self.view_file(path, view_range).await
        }
    }

    /// Handle create command - create a new file
    pub async fn handle_create(
        &self,
        path: &PathBuf,
        file_text: &str,
    ) -> Result<ToolResult, ToolError> {
        // Check if file already exists
        if Self::path_exists(path).await {
            return Err(ToolError::InvalidInput(format!(
                "File already exists at: {}. Cannot overwrite files using command create.",
                path.display()
            )));
        }

        // Write the file
        Self::write_file(path, file_text).await?;

        // Save to history
        self.save_history(path, file_text.to_string()).await;

        Ok(ToolResult::success(format!(
            "File created successfully at: {}",
            path.display()
        )))
    }
}

impl Default for StrReplaceEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for StrReplaceEditor {
    fn name(&self) -> &'static str {
        "str_replace_editor"
    }

    fn description(&self) -> &'static str {
        "Custom editing tool for viewing, creating and editing files. State is persistent across command calls."
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();

                // command - required, enum
                props.insert(
                    "command".to_string(),
                    ToolParameter {
                        name: "command".to_string(),
                        param_type: "string".to_string(),
                        description: Some(
                            "The command to execute: view, create, str_replace, insert, undo_edit"
                                .to_string(),
                        ),
                        required: Some(true),
                        default: None,
                        enum_values: Some(vec![
                            "view".to_string(),
                            "create".to_string(),
                            "str_replace".to_string(),
                            "insert".to_string(),
                            "undo_edit".to_string(),
                        ]),
                    },
                );

                // path - required
                props.insert(
                    "path".to_string(),
                    ToolParameter {
                        name: "path".to_string(),
                        param_type: "string".to_string(),
                        description: Some(
                            "Absolute path to file or directory, e.g., /repo/file.py".to_string(),
                        ),
                        required: Some(true),
                        default: None,
                        enum_values: None,
                    },
                );

                // file_text - optional (for create)
                props.insert(
                    "file_text".to_string(),
                    ToolParameter {
                        name: "file_text".to_string(),
                        param_type: "string".to_string(),
                        description: Some(
                            "Required parameter of 'create' command, with the content of the file to be created."
                                .to_string(),
                        ),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );

                // old_str - optional (for str_replace)
                props.insert(
                    "old_str".to_string(),
                    ToolParameter {
                        name: "old_str".to_string(),
                        param_type: "string".to_string(),
                        description: Some(
                            "Required parameter of 'str_replace' command containing the string in 'path' to replace."
                                .to_string(),
                        ),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );

                // new_str - optional (for str_replace and insert)
                props.insert(
                    "new_str".to_string(),
                    ToolParameter {
                        name: "new_str".to_string(),
                        param_type: "string".to_string(),
                        description: Some(
                            "Optional parameter of 'str_replace' command containing the new string (if not given, no string will be added). Required parameter of 'insert' command containing the string to insert."
                                .to_string(),
                        ),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );

                // insert_line - optional (for insert)
                props.insert(
                    "insert_line".to_string(),
                    ToolParameter {
                        name: "insert_line".to_string(),
                        param_type: "integer".to_string(),
                        description: Some(
                            "Required parameter of 'insert' command. The 'new_str' will be inserted AFTER the line 'insert_line' of 'path'."
                                .to_string(),
                        ),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );

                // view_range - optional (for view)
                props.insert(
                    "view_range".to_string(),
                    ToolParameter {
                        name: "view_range".to_string(),
                        param_type: "array".to_string(),
                        description: Some(
                            "Optional parameter of 'view' command when 'path' points to a file. If none is given, the full file is shown. If provided, the file will be shown in the indicated line number range, e.g., [11, 12] will show lines 11 and 12. Index starts at 1. Setting [start, -1] shows all lines from 'start' to end of file."
                                .to_string(),
                        ),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );

                props
            },
            required: Some(vec!["command".to_string(), "path".to_string()]),
        })
    }

    async fn execute(&self, _input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        // Will implement in next task
        Ok(ToolResult::success("Not implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_display() {
        assert_eq!(Command::View.to_string(), "view");
        assert_eq!(Command::Create.to_string(), "create");
        assert_eq!(Command::StrReplace.to_string(), "str_replace");
        assert_eq!(Command::Insert.to_string(), "insert");
        assert_eq!(Command::UndoEdit.to_string(), "undo_edit");
    }

    #[test]
    fn test_command_from_str() {
        assert_eq!("view".parse::<Command>().unwrap(), Command::View);
        assert_eq!("create".parse::<Command>().unwrap(), Command::Create);
        assert_eq!("str_replace".parse::<Command>().unwrap(), Command::StrReplace);
        assert_eq!("insert".parse::<Command>().unwrap(), Command::Insert);
        assert_eq!("undo_edit".parse::<Command>().unwrap(), Command::UndoEdit);
    }

    #[test]
    fn test_command_from_str_invalid() {
        let result = "invalid".parse::<Command>();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unknown command"));
    }

    #[test]
    fn test_str_replace_editor_new() {
        let editor = StrReplaceEditor::new();
        assert_eq!(editor.name(), "str_replace_editor");
    }

    #[test]
    fn test_str_replace_editor_default() {
        let editor = StrReplaceEditor::default();
        assert_eq!(editor.name(), "str_replace_editor");
    }

    #[test]
    fn test_constants() {
        assert_eq!(SNIPPET_LINES, 4);
        assert_eq!(MAX_RESPONSE_LEN, 16000);
        assert!(TRUNCATED_MESSAGE.contains("response clipped"));
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = StrReplaceEditor::new();
        let schema = tool.parameters();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert!(schema.properties.contains_key("command"));
        assert!(schema.properties.contains_key("path"));
    }

    #[test]
    fn test_parse_editor_input() {
        let json = serde_json::json!({
            "command": "view",
            "path": "/tmp/test.txt",
            "view_range": [1, 10]
        });
        let input = EditorInput::from_json(&json);
        assert_eq!(input.command, Some(Command::View));
        assert_eq!(input.path, Some("/tmp/test.txt".to_string()));
        assert_eq!(input.view_range, Some(vec![1, 10]));
    }

    #[test]
    fn test_maybe_truncate_short() {
        let content = "short content";
        let result = StrReplaceEditor::maybe_truncate(content);
        assert_eq!(result, content);
    }

    #[test]
    fn test_make_output() {
        let content = "line1\nline2";
        let result = StrReplaceEditor::make_output(content, "test.txt", 1);
        assert!(result.contains("cat -n"));
        assert!(result.contains("     1"));
        assert!(result.contains("     2"));
    }

    #[tokio::test]
    async fn test_validate_absolute_path() {
        let result = StrReplaceEditor::validate_absolute_path("/tmp/test.txt");
        assert!(result.is_ok());

        let result = StrReplaceEditor::validate_absolute_path("relative/path");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_view_file_full() {
        let tool = StrReplaceEditor::new();
        let temp = tempfile::NamedTempFile::new().unwrap();
        let content = "line1\nline2\nline3";
        tokio::fs::write(temp.path(), content).await.unwrap();

        let result = tool
            .handle_view(&temp.path().to_path_buf(), None)
            .await
            .unwrap();
        assert!(result.output.unwrap().contains("line1"));
    }

    #[tokio::test]
    async fn test_view_file_range() {
        let tool = StrReplaceEditor::new();
        let temp = tempfile::NamedTempFile::new().unwrap();
        let content = "line1\nline2\nline3\nline4\nline5";
        tokio::fs::write(temp.path(), content).await.unwrap();

        let result = tool
            .handle_view(&temp.path().to_path_buf(), Some(vec![2, 4]))
            .await
            .unwrap();
        let output = result.output.unwrap();
        assert!(output.contains("line2"));
        assert!(output.contains("line3"));
        assert!(!output.contains("line1"));
    }

    #[tokio::test]
    async fn test_view_nonexistent_file() {
        let tool = StrReplaceEditor::new();
        let result = tool
            .handle_view(&PathBuf::from("/nonexistent/path"), None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_file() {
        let tool = StrReplaceEditor::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("new_file.txt");

        let result = tool.handle_create(&path, "Hello, World!").await.unwrap();
        assert!(result.output.unwrap().contains("created successfully"));

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_create_existing_file() {
        let tool = StrReplaceEditor::new();
        let temp = tempfile::NamedTempFile::new().unwrap();

        let result = tool.handle_create(&temp.path().to_path_buf(), "content").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }
}
