# StrReplaceEditor Tool Design

## Overview

StrReplaceEditor is a file editing tool that provides precise, line-aware file manipulation capabilities. It is a key component for the SWEAgent, enabling autonomous code editing with undo support.

## Core Structure

```rust
pub struct StrReplaceEditor {
    /// File history for undo functionality (path -> list of previous contents)
    file_history: Arc<RwLock<HashMap<PathBuf, Vec<String>>>>,
}

pub enum Command {
    View,        // View file or directory
    Create,      // Create new file
    StrReplace,  // Replace unique string
    Insert,      // Insert text at line
    UndoEdit,    // Revert last edit
}
```

## Commands

### view
Display file or directory contents.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| path | string | yes | Absolute path to file or directory |
| view_range | [int, int] | no | Line range to display (e.g., [10, 20], [-1] for EOF) |

### create
Create a new file with content.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| path | string | yes | Absolute path for new file |
| file_text | string | yes | Content to write |

### str_replace
Replace a unique string in a file.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| path | string | yes | Absolute path to file |
| old_str | string | yes | Exact string to replace (must be unique) |
| new_str | string | no | Replacement string (defaults to empty) |

### insert
Insert text at a specific line.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| path | string | yes | Absolute path to file |
| insert_line | int | yes | Line number to insert AFTER (0 = beginning) |
| new_str | string | yes | Text to insert |

### undo_edit
Revert the last edit to a file.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| path | string | yes | Absolute path to file |

## Output Formatting

### Line Numbers
All file output uses `cat -n` style formatting:
```
     1  fn main() {
     2      println!("hello");
     3  }
```

### Snippet Preview
After edits, shows context (4 lines before/after the change):
```
The file /path/to/file.rs has been edited. Here's the result of running `cat -n` on a snippet of /path/to/file.rs:
     5  // before
     6  fn new_code() {
     7      // edited line
     8  }
     9  // after
```

### Truncation
Long outputs (>16000 chars) are truncated with a guidance message:
```
<response clipped><NOTE>To save on context only part of this file has been shown to you. You should retry this tool after you have searched inside the file with `grep -n` in order to find the line numbers of what you are looking for.</NOTE>
```

### Directory View
Uses `find {path} -maxdepth 2 -not -path '*/\.*'` to show directory structure.

## Error Handling

| Scenario | Error Message |
|----------|---------------|
| Non-absolute path | `The path {path} is not an absolute path` |
| File not found | `The path {path} does not exist. Please provide a valid path.` |
| Create on existing file | `File already exists at: {path}. Cannot overwrite files using command create.` |
| `old_str` not found | `No replacement was performed, old_str did not appear verbatim in {path}.` |
| `old_str` not unique | `No replacement was performed. Multiple occurrences of old_str in lines {lines}. Please ensure it is unique` |
| Invalid `view_range` | `Invalid view_range: {range}...` |
| Invalid `insert_line` | `Invalid insert_line parameter: {line}...` |
| No history for undo | `No edit history found for {path}.` |
| Directory for non-view | `The path {path} is a directory and only the view command can be used on directories` |

## Constants

```rust
const SNIPPET_LINES: usize = 4;        // Lines of context around edits
const MAX_RESPONSE_LEN: usize = 16000; // Max output before truncation
```

## Implementation Notes

1. **Tab handling**: Expand tabs to spaces consistently using `expandtabs()` equivalent
2. **Thread safety**: File history uses `Arc<RwLock<>>` for concurrent access
3. **Async**: All file operations are async using `tokio::fs`

## Test Strategy

### Unit Tests
- Each command with valid input
- Each error case
- Edge cases: empty files, single line, long lines
- Undo history tracking
- Truncation behavior

### Integration Tests
- Multi-step edit workflow
- Create → edit → undo cycle

## Dependencies

This tool is a prerequisite for implementing `SweAgent`, which uses:
- `Bash` (existing)
- `StrReplaceEditor` (this tool)
- `Terminate` (existing)
