# StrReplaceEditor Tool Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a file editing tool with view, create, str_replace, insert, and undo_edit commands for the SWEAgent.

**Architecture:** A new tool `StrReplaceEditor` that wraps file operations with stateful history tracking for undo support. Uses async tokio::fs for all file I/O and maintains file history in an Arc<RwLock<HashMap>> for thread safety.

**Tech Stack:** Rust, tokio (async), serde (JSON parsing), tempfile (testing)

---

## Task 1: Create Tool Skeleton with Command Enum

**Files:**
- Create: `src/tool/str_replace_editor.rs`

**Step 1: Write the basic struct and Command enum**

```rust
//! StrReplaceEditor - file editing tool with undo support

use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
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
```

**Step 2: Run compilation to verify syntax**

Run: `cd openmanus-rs && cargo check 2>&1 | head -30`
Expected: Compilation errors about missing trait implementations

**Step 3: Add Tool trait implementation (stub)**

```rust
#[async_trait]
impl Tool for StrReplaceEditor {
    fn name(&self) -> &'static str {
        "str_replace_editor"
    }

    fn description(&self) -> &'static str {
        "Custom editing tool for viewing, creating and editing files. State is persistent across command calls."
    }

    fn parameters(&self) -> Option<ToolSchema> {
        // Will implement in next task
        None
    }

    async fn execute(&self, input: &str, ctx: &mut Context) -> Result<ToolResult, ToolError> {
        // Will implement in next task
        Ok(ToolResult::success("Not implemented"))
    }
}
```

**Step 4: Register in module**

Modify `src/tool/mod.rs`, add:
```rust
pub mod str_replace_editor;
pub use str_replace_editor::StrReplaceEditor;
```

**Step 5: Run tests to verify compilation**

Run: `cd openmanus-rs && cargo test --lib 2>&1 | tail -20`
Expected: Tests pass, no compilation errors

**Step 6: Commit**

```bash
cd openmanus-rs && git add src/tool/str_replace_editor.rs src/tool/mod.rs && git commit -m "feat(tool): add StrReplaceEditor skeleton with Command enum"
```

---

## Task 2: Implement Tool Schema

**Files:**
- Modify: `src/tool/str_replace_editor.rs`

**Step 1: Write failing test for schema**

Add to `str_replace_editor.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

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
```

**Step 2: Run test to verify it fails**

Run: `cd openmanus-rs && cargo test test_tool_has_schema -- --nocapture`
Expected: FAIL - assertion failed

**Step 3: Implement the schema**

Replace the `parameters()` method:
```rust
fn parameters(&self) -> Option<ToolSchema> {
    use std::collections::HashMap as StdHashMap;

    let mut props = StdHashMap::new();

    props.insert(
        "command".to_string(),
        ToolParameter {
            name: "command".to_string(),
            param_type: "string".to_string(),
            description: Some("The command to run: view, create, str_replace, insert, undo_edit".to_string()),
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

    props.insert(
        "path".to_string(),
        ToolParameter {
            name: "path".to_string(),
            param_type: "string".to_string(),
            description: Some("Absolute path to file or directory".to_string()),
            required: Some(true),
            default: None,
            enum_values: None,
        },
    );

    props.insert(
        "file_text".to_string(),
        ToolParameter {
            name: "file_text".to_string(),
            param_type: "string".to_string(),
            description: Some("Required for create command - content of the file to create".to_string()),
            required: Some(false),
            default: None,
            enum_values: None,
        },
    );

    props.insert(
        "old_str".to_string(),
        ToolParameter {
            name: "old_str".to_string(),
            param_type: "string".to_string(),
            description: Some("Required for str_replace - the exact string to replace".to_string()),
            required: Some(false),
            default: None,
            enum_values: None,
        },
    );

    props.insert(
        "new_str".to_string(),
        ToolParameter {
            name: "new_str".to_string(),
            param_type: "string".to_string(),
            description: Some("For str_replace: replacement string. For insert: text to insert".to_string()),
            required: Some(false),
            default: None,
            enum_values: None,
        },
    );

    props.insert(
        "insert_line".to_string(),
        ToolParameter {
            name: "insert_line".to_string(),
            param_type: "integer".to_string(),
            description: Some("Required for insert - line number to insert AFTER (0 = beginning)".to_string()),
            required: Some(false),
            default: None,
            enum_values: None,
        },
    );

    props.insert(
        "view_range".to_string(),
        ToolParameter {
            name: "view_range".to_string(),
            param_type: "array".to_string(),
            description: Some("Optional for view - [start, end] line range. Use -1 for end to show to EOF".to_string()),
            required: Some(false),
            default: None,
            enum_values: None,
        },
    );

    Some(ToolSchema {
        schema_type: "object".to_string(),
        properties: props,
        required: Some(vec!["command".to_string(), "path".to_string()]),
    })
}
```

**Step 4: Run test to verify it passes**

Run: `cd openmanus-rs && cargo test test_tool_has_schema -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
cd openmanus-rs && git add src/tool/str_replace_editor.rs && git commit -m "feat(tool): add StrReplaceEditor parameter schema"
```

---

## Task 3: Implement Input Parsing

**Files:**
- Modify: `src/tool/str_replace_editor.rs`

**Step 1: Add input struct for parsing**

```rust
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
```

**Step 2: Write test for input parsing**

```rust
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
```

**Step 3: Run test**

Run: `cd openmanus-rs && cargo test test_parse_editor_input -- --nocapture`
Expected: PASS

**Step 4: Commit**

```bash
cd openmanus-rs && git add src/tool/str_replace_editor.rs && git commit -m "feat(tool): add EditorInput parsing for StrReplaceEditor"
```

---

## Task 4: Implement Helper Functions

**Files:**
- Modify: `src/tool/str_replace_editor.rs`

**Step 1: Add helper functions**

```rust
impl StrReplaceEditor {
    // ... existing new() ...

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
        // Create parent directories if needed
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
}
```

**Step 2: Write tests for helpers**

```rust
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
```

**Step 3: Run tests**

Run: `cd openmanus-rs && cargo test test_maybe_truncate test_make_output test_validate_absolute -- --nocapture`
Expected: All PASS

**Step 4: Commit**

```bash
cd openmanus-rs && git add src/tool/str_replace_editor.rs && git commit -m "feat(tool): add helper functions for StrReplaceEditor"
```

---

## Task 5: Implement View Command

**Files:**
- Modify: `src/tool/str_replace_editor.rs`

**Step 1: Add view_file and view_directory methods**

```rust
impl StrReplaceEditor {
    // ... existing methods ...

    /// View a directory (list contents)
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

    /// View a file with optional line range
    async fn view_file(
        &self,
        path: &PathBuf,
        view_range: Option<Vec<i32>>,
    ) -> Result<ToolResult, ToolError> {
        let content = Self::read_file(path).await?;
        let mut init_line = 1;

        let display_content = if let Some(range) = view_range {
            if range.len() != 2 {
                return Err(ToolError::InvalidInput(
                    "Invalid view_range. It should be a list of two integers.".to_string()
                ));
            }

            let lines: Vec<&str> = content.lines().collect();
            let n_lines = lines.len();
            let start = range[0] as usize;
            let end = range[1];

            if start < 1 || start > n_lines {
                return Err(ToolError::InvalidInput(format!(
                    "Invalid view_range: {:?}. Start should be within [1, {}]",
                    range, n_lines
                )));
            }

            let end_idx = if end == -1 {
                n_lines
            } else {
                if end as usize > n_lines {
                    return Err(ToolError::InvalidInput(format!(
                        "Invalid view_range: {:?}. End {} exceeds file length {}",
                        range, end, n_lines
                    )));
                }
                if end < start {
                    return Err(ToolError::InvalidInput(format!(
                        "Invalid view_range: {:?}. End should be >= start",
                        range
                    )));
                }
                end as usize
            };

            init_line = start;
            lines[start - 1..end_idx].join("\n")
        } else {
            content
        };

        Ok(ToolResult::success(Self::make_output(
            &display_content,
            &path.display().to_string(),
            init_line,
        )))
    }

    /// Handle view command
    async fn handle_view(
        &self,
        path: &PathBuf,
        view_range: Option<Vec<i32>>,
    ) -> Result<ToolResult, ToolError> {
        if !Self::path_exists(path).await {
            return Err(ToolError::InvalidInput(format!(
                "The path {} does not exist. Please provide a valid path.",
                path.display()
            )));
        }

        if Self::is_directory(path).await {
            if view_range.is_some() {
                return Err(ToolError::InvalidInput(
                    "The view_range parameter is not allowed when path points to a directory.".to_string()
                ));
            }
            self.view_directory(path).await
        } else {
            self.view_file(path, view_range).await
        }
    }
}
```

**Step 2: Write tests for view**

```rust
#[tokio::test]
async fn test_view_file_full() {
    let tool = StrReplaceEditor::new();
    let temp = tempfile::NamedTempFile::new().unwrap();
    let content = "line1\nline2\nline3";
    tokio::fs::write(temp.path(), content).await.unwrap();

    let result = tool.handle_view(&temp.path().to_path_buf(), None).await.unwrap();
    assert!(result.output.unwrap().contains("line1"));
    assert!(result.output.unwrap().contains("line2"));
}

#[tokio::test]
async fn test_view_file_range() {
    let tool = StrReplaceEditor::new();
    let temp = tempfile::NamedTempFile::new().unwrap();
    let content = "line1\nline2\nline3\nline4\nline5";
    tokio::fs::write(temp.path(), content).await.unwrap();

    let result = tool.handle_view(&temp.path().to_path_buf(), Some(vec![2, 4])).await.unwrap();
    let output = result.output.unwrap();
    assert!(output.contains("line2"));
    assert!(output.contains("line3"));
    assert!(output.contains("line4"));
    assert!(!output.contains("line1"));
    assert!(!output.contains("line5"));
}

#[tokio::test]
async fn test_view_nonexistent_file() {
    let tool = StrReplaceEditor::new();
    let result = tool.handle_view(&PathBuf::from("/nonexistent/path"), None).await;
    assert!(result.is_err());
}
```

**Step 3: Run tests**

Run: `cd openmanus-rs && cargo test test_view -- --nocapture`
Expected: All PASS

**Step 4: Commit**

```bash
cd openmanus-rs && git add src/tool/str_replace_editor.rs && git commit -m "feat(tool): implement view command for StrReplaceEditor"
```

---

## Task 6: Implement Create Command

**Files:**
- Modify: `src/tool/str_replace_editor.rs`

**Step 1: Add handle_create method**

```rust
impl StrReplaceEditor {
    // ... existing methods ...

    /// Handle create command
    async fn handle_create(
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
```

**Step 2: Write tests for create**

```rust
#[tokio::test]
async fn test_create_file() {
    let tool = StrReplaceEditor::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("new_file.txt");

    let result = tool.handle_create(&path, "Hello, World!").await.unwrap();
    assert!(result.output.unwrap().contains("created successfully"));

    // Verify file exists and has content
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
```

**Step 3: Run tests**

Run: `cd openmanus-rs && cargo test test_create -- --nocapture`
Expected: All PASS

**Step 4: Commit**

```bash
cd openmanus-rs && git add src/tool/str_replace_editor.rs && git commit -m "feat(tool): implement create command for StrReplaceEditor"
```

---

## Task 7: Implement StrReplace Command

**Files:**
- Modify: `src/tool/str_replace_editor.rs`

**Step 1: Add handle_str_replace method**

```rust
impl StrReplaceEditor {
    // ... existing methods ...

    /// Handle str_replace command
    async fn handle_str_replace(
        &self,
        path: &PathBuf,
        old_str: &str,
        new_str: Option<&str>,
    ) -> Result<ToolResult, ToolError> {
        let content = Self::read_file(path).await?;
        let content = content.replace('\t', "    ");
        let old_str = old_str.replace('\t', "    ");
        let new_str = new_str.unwrap_or("").replace('\t', "    ");

        // Count occurrences
        let occurrences = content.matches(&old_str).count();

        if occurrences == 0 {
            return Err(ToolError::InvalidInput(format!(
                "No replacement was performed, old_str `{}` did not appear verbatim in {}.",
                old_str,
                path.display()
            )));
        }

        if occurrences > 1 {
            let lines: Vec<usize> = content
                .lines()
                .enumerate()
                .filter(|(_, line)| line.contains(&old_str))
                .map(|(i, _)| i + 1)
                .collect();

            return Err(ToolError::InvalidInput(format!(
                "No replacement was performed. Multiple occurrences of old_str in lines {:?}. Please ensure it is unique",
                lines
            )));
        }

        // Perform replacement
        let new_content = content.replace(&old_str, &new_str);

        // Save history before writing
        self.save_history(path, content.clone()).await;

        // Write new content
        Self::write_file(path, &new_content).await?;

        // Create snippet preview
        let replacement_line = content.split(&old_str).next().unwrap_or("").matches('\n').count();
        let all_lines: Vec<&str> = new_content.lines().collect();
        let start = replacement_line.saturating_sub(SNIPPET_LINES);
        let end = (replacement_line + SNIPPET_LINES + new_str.matches('\n').count() + 1).min(all_lines.len());

        let snippet = all_lines[start..end].join("\n");

        Ok(ToolResult::success(format!(
            "The file {} has been edited. {}Review the changes and make sure they are as expected.",
            path.display(),
            Self::make_output(&snippet, &format!("a snippet of {}", path.display()), start + 1)
        )))
    }
}
```

**Step 2: Write tests for str_replace**

```rust
#[tokio::test]
async fn test_str_replace_unique() {
    let tool = StrReplaceEditor::new();
    let temp = tempfile::NamedTempFile::new().unwrap();
    tokio::fs::write(temp.path(), "Hello, World!\nGoodbye, World!").await.unwrap();

    let result = tool
        .handle_str_replace(&temp.path().to_path_buf(), "Hello", Some("Hi"))
        .await
        .unwrap();

    assert!(result.output.unwrap().contains("edited"));

    let content = tokio::fs::read_to_string(temp.path()).await.unwrap();
    assert_eq!(content, "Hi, World!\nGoodbye, World!");
}

#[tokio::test]
async fn test_str_replace_not_found() {
    let tool = StrReplaceEditor::new();
    let temp = tempfile::NamedTempFile::new().unwrap();
    tokio::fs::write(temp.path(), "Hello, World!").await.unwrap();

    let result = tool
        .handle_str_replace(&temp.path().to_path_buf(), "NotExist", Some("New"))
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("did not appear verbatim"));
}

#[tokio::test]
async fn test_str_replace_multiple_occurrences() {
    let tool = StrReplaceEditor::new();
    let temp = tempfile::NamedTempFile::new().unwrap();
    tokio::fs::write(temp.path(), "foo bar foo").await.unwrap();

    let result = tool
        .handle_str_replace(&temp.path().to_path_buf(), "foo", Some("baz"))
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Multiple occurrences"));
}
```

**Step 3: Run tests**

Run: `cd openmanus-rs && cargo test test_str_replace -- --nocapture`
Expected: All PASS

**Step 4: Commit**

```bash
cd openmanus-rs && git add src/tool/str_replace_editor.rs && git commit -m "feat(tool): implement str_replace command for StrReplaceEditor"
```

---

## Task 8: Implement Insert Command

**Files:**
- Modify: `src/tool/str_replace_editor.rs`

**Step 1: Add handle_insert method**

```rust
impl StrReplaceEditor {
    // ... existing methods ...

    /// Handle insert command
    async fn handle_insert(
        &self,
        path: &PathBuf,
        insert_line: i32,
        new_str: &str,
    ) -> Result<ToolResult, ToolError> {
        let content = Self::read_file(path).await?;
        let content = content.replace('\t', "    ");
        let new_str = new_str.replace('\t', "    ");

        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        let n_lines = lines.len();

        if insert_line < 0 || insert_line > n_lines as i32 {
            return Err(ToolError::InvalidInput(format!(
                "Invalid insert_line parameter: {}. It should be within [0, {}]",
                insert_line, n_lines
            )));
        }

        let insert_idx = insert_line as usize;
        let new_lines: Vec<String> = new_str.lines().map(String::from).collect();

        // Save history
        self.save_history(path, content.clone()).await;

        // Insert the new lines
        let mut new_content = Vec::with_capacity(lines.len() + new_lines.len());
        new_content.extend(lines[..insert_idx].to_vec());
        new_content.extend(new_lines.clone());
        new_content.extend(lines[insert_idx..].to_vec());

        let final_content = new_content.join("\n");
        Self::write_file(path, &final_content).await?;

        // Create snippet preview
        let snippet_start = insert_idx.saturating_sub(SNIPPET_LINES);
        let snippet_end = (insert_idx + new_lines.len() + SNIPPET_LINES).min(new_content.len());

        let snippet = new_content[snippet_start..snippet_end].join("\n");

        Ok(ToolResult::success(format!(
            "The file {} has been edited. {}Review the changes and make sure they are as expected (correct indentation, no duplicate lines, etc).",
            path.display(),
            Self::make_output(&snippet, "a snippet of the edited file", snippet_start + 1)
        )))
    }
}
```

**Step 2: Write tests for insert**

```rust
#[tokio::test]
async fn test_insert_at_beginning() {
    let tool = StrReplaceEditor::new();
    let temp = tempfile::NamedTempFile::new().unwrap();
    tokio::fs::write(temp.path(), "line2\nline3").await.unwrap();

    let result = tool
        .handle_insert(&temp.path().to_path_buf(), 0, "line1")
        .await
        .unwrap();

    assert!(result.output.unwrap().contains("edited"));

    let content = tokio::fs::read_to_string(temp.path()).await.unwrap();
    assert_eq!(content, "line1\nline2\nline3");
}

#[tokio::test]
async fn test_insert_in_middle() {
    let tool = StrReplaceEditor::new();
    let temp = tempfile::NamedTempFile::new().unwrap();
    tokio::fs::write(temp.path(), "line1\nline3").await.unwrap();

    let result = tool
        .handle_insert(&temp.path().to_path_buf(), 1, "line2")
        .await
        .unwrap();

    let content = tokio::fs::read_to_string(temp.path()).await.unwrap();
    assert_eq!(content, "line1\nline2\nline3");
}

#[tokio::test]
async fn test_insert_invalid_line() {
    let tool = StrReplaceEditor::new();
    let temp = tempfile::NamedTempFile::new().unwrap();
    tokio::fs::write(temp.path(), "line1").await.unwrap();

    let result = tool
        .handle_insert(&temp.path().to_path_buf(), 10, "new line")
        .await;

    assert!(result.is_err());
}
```

**Step 3: Run tests**

Run: `cd openmanus-rs && cargo test test_insert -- --nocapture`
Expected: All PASS

**Step 4: Commit**

```bash
cd openmanus-rs && git add src/tool/str_replace_editor.rs && git commit -m "feat(tool): implement insert command for StrReplaceEditor"
```

---

## Task 9: Implement UndoEdit Command

**Files:**
- Modify: `src/tool/str_replace_editor.rs`

**Step 1: Add handle_undo_edit method**

```rust
impl StrReplaceEditor {
    // ... existing methods ...

    /// Handle undo_edit command
    async fn handle_undo_edit(&self, path: &PathBuf) -> Result<ToolResult, ToolError> {
        let old_content = self.pop_history(path).await.ok_or_else(|| {
            ToolError::InvalidInput(format!("No edit history found for {}.", path.display()))
        })?;

        Self::write_file(path, &old_content).await?;

        Ok(ToolResult::success(format!(
            "Last edit to {} undone successfully. {}",
            path.display(),
            Self::make_output(&old_content, &path.display().to_string(), 1)
        )))
    }
}
```

**Step 2: Write tests for undo_edit**

```rust
#[tokio::test]
async fn test_undo_edit() {
    let tool = StrReplaceEditor::new();
    let temp = tempfile::NamedTempFile::new().unwrap();
    let original = "original content";
    tokio::fs::write(temp.path(), original).await.unwrap();

    // First do a replace
    tool.handle_str_replace(&temp.path().to_path_buf(), "original", Some("modified"))
        .await
        .unwrap();

    let content = tokio::fs::read_to_string(temp.path()).await.unwrap();
    assert_eq!(content, "modified content");

    // Now undo
    let result = tool.handle_undo_edit(&temp.path().to_path_buf()).await.unwrap();
    assert!(result.output.unwrap().contains("undone"));

    let content = tokio::fs::read_to_string(temp.path()).await.unwrap();
    assert_eq!(content, original);
}

#[tokio::test]
async fn test_undo_no_history() {
    let tool = StrReplaceEditor::new();
    let temp = tempfile::NamedTempFile::new().unwrap();

    let result = tool.handle_undo_edit(&temp.path().to_path_buf()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No edit history"));
}
```

**Step 3: Run tests**

Run: `cd openmanus-rs && cargo test test_undo -- --nocapture`
Expected: All PASS

**Step 4: Commit**

```bash
cd openmanus-rs && git add src/tool/str_replace_editor.rs && git commit -m "feat(tool): implement undo_edit command for StrReplaceEditor"
```

---

## Task 10: Wire Up Execute Method

**Files:**
- Modify: `src/tool/str_replace_editor.rs`

**Step 1: Implement the execute method**

```rust
async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
    // Parse JSON input
    let json: serde_json::Value = serde_json::from_str(input)
        .map_err(|e| ToolError::InvalidInput(format!("Invalid JSON: {}", e)))?;

    let parsed = EditorInput::from_json(&json);

    // Get command
    let command = parsed.command.ok_or_else(|| {
        ToolError::InvalidInput("Missing required parameter: command".to_string())
    })?;

    // Get and validate path
    let path_str = parsed.path.ok_or_else(|| {
        ToolError::InvalidInput("Missing required parameter: path".to_string())
    })?;
    let path = Self::validate_absolute_path(&path_str)?;

    // Dispatch to appropriate handler
    match command {
        Command::View => {
            // View can work on non-existent paths (will be caught in handle_view)
            self.handle_view(&path, parsed.view_range).await
        }
        Command::Create => {
            let file_text = parsed.file_text.ok_or_else(|| {
                ToolError::InvalidInput("Parameter file_text is required for command: create".to_string())
            })?;
            self.handle_create(&path, &file_text).await
        }
        Command::StrReplace => {
            if !Self::path_exists(&path).await {
                return Err(ToolError::InvalidInput(format!(
                    "The path {} does not exist.",
                    path.display()
                )));
            }
            if Self::is_directory(&path).await {
                return Err(ToolError::InvalidInput(format!(
                    "The path {} is a directory and only the view command can be used on directories",
                    path.display()
                )));
            }
            let old_str = parsed.old_str.ok_or_else(|| {
                ToolError::InvalidInput("Parameter old_str is required for command: str_replace".to_string())
            })?;
            self.handle_str_replace(&path, &old_str, parsed.new_str.as_deref()).await
        }
        Command::Insert => {
            if !Self::path_exists(&path).await {
                return Err(ToolError::InvalidInput(format!(
                    "The path {} does not exist.",
                    path.display()
                )));
            }
            if Self::is_directory(&path).await {
                return Err(ToolError::InvalidInput(format!(
                    "The path {} is a directory and only the view command can be used on directories",
                    path.display()
                )));
            }
            let insert_line = parsed.insert_line.ok_or_else(|| {
                ToolError::InvalidInput("Parameter insert_line is required for command: insert".to_string())
            })?;
            let new_str = parsed.new_str.ok_or_else(|| {
                ToolError::InvalidInput("Parameter new_str is required for command: insert".to_string())
            })?;
            self.handle_insert(&path, insert_line, &new_str).await
        }
        Command::UndoEdit => {
            self.handle_undo_edit(&path).await
        }
    }
}
```

**Step 2: Write integration tests**

```rust
#[tokio::test]
async fn test_execute_full_workflow() {
    let tool = StrReplaceEditor::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let mut ctx = Context::new();

    // Create file
    let create_input = serde_json::json!({
        "command": "create",
        "path": file_path.to_str().unwrap(),
        "file_text": "Hello\nWorld"
    })
    .to_string();
    let result = tool.execute(&create_input, &mut ctx).await.unwrap();
    assert!(result.output.unwrap().contains("created"));

    // View file
    let view_input = serde_json::json!({
        "command": "view",
        "path": file_path.to_str().unwrap()
    })
    .to_string();
    let result = tool.execute(&view_input, &mut ctx).await.unwrap();
    assert!(result.output.unwrap().contains("Hello"));

    // Replace
    let replace_input = serde_json::json!({
        "command": "str_replace",
        "path": file_path.to_str().unwrap(),
        "old_str": "Hello",
        "new_str": "Hi"
    })
    .to_string();
    let result = tool.execute(&replace_input, &mut ctx).await.unwrap();
    assert!(result.output.unwrap().contains("edited"));

    // Undo
    let undo_input = serde_json::json!({
        "command": "undo_edit",
        "path": file_path.to_str().unwrap()
    })
    .to_string();
    let result = tool.execute(&undo_input, &mut ctx).await.unwrap();
    assert!(result.output.unwrap().contains("undone"));
}

#[tokio::test]
async fn test_execute_invalid_command() {
    let tool = StrReplaceEditor::new();
    let mut ctx = Context::new();

    let input = serde_json::json!({
        "command": "invalid",
        "path": "/tmp/test"
    })
    .to_string();
    let result = tool.execute(&input, &mut ctx).await;
    assert!(result.is_err());
}
```

**Step 3: Run all tests**

Run: `cd openmanus-rs && cargo test str_replace_editor -- --nocapture`
Expected: All PASS

**Step 4: Commit**

```bash
cd openmanus-rs && git add src/tool/str_replace_editor.rs && git commit -m "feat(tool): wire up execute method for StrReplaceEditor"
```

---

## Task 11: Update ToolCollection Defaults

**Files:**
- Modify: `src/tool/collection.rs`

**Step 1: Add StrReplaceEditor to default tools**

Modify `with_defaults()`:
```rust
pub fn with_defaults() -> Self {
    use crate::tool::{BashTool, TerminateTool, FileTool, StrReplaceEditor};

    let mut collection = Self::new();
    collection.add_tool(Arc::new(BashTool::new()));
    collection.add_tool(Arc::new(TerminateTool::new()));
    collection.add_tool(Arc::new(FileTool::new()));
    collection.add_tool(Arc::new(StrReplaceEditor::new()));
    collection
}
```

**Step 2: Run full test suite**

Run: `cd openmanus-rs && cargo test --lib`
Expected: All tests pass

**Step 3: Commit**

```bash
cd openmanus-rs && git add src/tool/collection.rs && git commit -m "feat(tool): add StrReplaceEditor to default tool collection"
```

---

## Task 12: Final Verification and Documentation

**Step 1: Run full test suite**

Run: `cd openmanus-rs && cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cd openmanus-rs && cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Run formatter**

Run: `cd openmanus-rs && cargo fmt --check`
Expected: No changes needed (or run `cargo fmt` to fix)

**Step 4: Update pending_tasks.md**

Mark StrReplaceEditor as complete in `/Users/matthew/SourceCode/github/PhantomMatthew/OpenManusZero/openmanus-rs/pending_tasks.md`:

Change:
```markdown
| StrReplaceEditor | `app/tool/str_replace_editor.py` | 字符串替换编辑器 | 高 |
```
To:
```markdown
| StrReplaceEditor | `app/tool/str_replace_editor.py` | 字符串替换编辑器 | ✅ |
```

**Step 5: Final commit**

```bash
cd openmanus-rs && git add . && git commit -m "feat(tool): complete StrReplaceEditor implementation with all commands

- view: display files/directories with optional line range
- create: create new files
- str_replace: unique string replacement with preview
- insert: insert text at specific line
- undo_edit: revert last edit using history

Includes comprehensive test coverage for all commands."
```

---

## Summary

| Task | Description | Tests |
|------|-------------|-------|
| 1 | Tool skeleton + Command enum | 1 |
| 2 | Parameter schema | 1 |
| 3 | Input parsing | 1 |
| 4 | Helper functions | 3 |
| 5 | View command | 3 |
| 6 | Create command | 2 |
| 7 | StrReplace command | 3 |
| 8 | Insert command | 3 |
| 9 | UndoEdit command | 2 |
| 10 | Execute wiring | 2 |
| 11 | ToolCollection integration | 0 |
| 12 | Final verification | 0 |

**Total: ~21 tests**
