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
}
