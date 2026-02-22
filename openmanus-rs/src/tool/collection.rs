//! Tool collection for managing multiple tools

use crate::context::Context;
use crate::error::ToolError;
use crate::tool::{Tool, ToolDefinition, ToolResult};
use std::collections::HashMap;
use std::sync::Arc;

/// A collection of tools that can be looked up by name
#[derive(Clone)]
pub struct ToolCollection {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolCollection {
    /// Create a new empty tool collection
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Create a tool collection with initial tools
    pub fn with_tools(tools: Vec<Arc<dyn Tool>>) -> Self {
        let mut collection = Self::new();
        for tool in tools {
            collection.add_tool(tool);
        }
        collection
    }

    /// Add a tool to the collection
    pub fn add_tool(&mut self, tool: Arc<dyn Tool>) -> &mut Self {
        let name = tool.name().to_string();
        if self.tools.contains_key(&name) {
            tracing::warn!("Tool {} already exists in collection, replacing", name);
        }
        self.tools.insert(name, tool);
        self
    }

    /// Remove a tool from the collection
    pub fn remove_tool(&mut self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.remove(name)
    }

    /// Get a tool by name
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Check if a tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all tool names
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Convert all tools to OpenAI function definitions
    pub fn to_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.to_definition()).collect()
    }

    /// Execute a tool by name
    pub async fn execute(
        &self,
        name: &str,
        input: &str,
        ctx: &mut Context,
    ) -> Result<ToolResult, ToolError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;
        tool.execute(input, ctx).await
    }

    /// Cleanup all tools
    pub async fn cleanup(&self) -> Result<(), ToolError> {
        for (name, tool) in &self.tools {
            if let Err(e) = tool.cleanup().await {
                tracing::error!("Failed to cleanup tool {}: {}", name, e);
            }
        }
        Ok(())
    }

    /// Iterate over all tools
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Tool>> {
        self.tools.values()
    }

    /// Get all tools as a slice
    pub fn tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values().cloned().collect()
    }

    /// Create a tool collection with default tools (bash, terminate, file, str_replace_editor, web_search)
    pub fn with_defaults() -> Self {
        use crate::tool::{BashTool, FileTool, StrReplaceEditor, TerminateTool, WebSearchTool};

        let mut collection = Self::new();
        collection.add_tool(Arc::new(BashTool::new()));
        collection.add_tool(Arc::new(TerminateTool::new()));
        collection.add_tool(Arc::new(FileTool::new()));
        collection.add_tool(Arc::new(StrReplaceEditor::new()));
        collection.add_tool(Arc::new(WebSearchTool::with_default_engines()));
        collection
    }
}

impl Default for ToolCollection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    // Mock tool for testing
    struct MockTool {
        name: &'static str,
        description: &'static str,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &'static str {
            self.name
        }

        fn description(&self) -> &'static str {
            self.description
        }

        async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success(format!(
                "Executed {} with: {}",
                self.name, input
            )))
        }
    }

    fn create_test_context() -> Context {
        Context::new()
    }

    #[test]
    fn test_tool_collection_new() {
        let collection = ToolCollection::new();
        assert!(collection.is_empty());
        assert_eq!(collection.len(), 0);
    }

    #[test]
    fn test_tool_collection_add_tool() {
        let mut collection = ToolCollection::new();
        let tool = Arc::new(MockTool {
            name: "test",
            description: "A test tool",
        });
        collection.add_tool(tool);
        assert!(!collection.is_empty());
        assert_eq!(collection.len(), 1);
        assert!(collection.has_tool("test"));
    }

    #[test]
    fn test_tool_collection_remove_tool() {
        let mut collection = ToolCollection::new();
        let tool = Arc::new(MockTool {
            name: "test",
            description: "A test tool",
        });
        collection.add_tool(tool);
        assert!(collection.has_tool("test"));

        let removed = collection.remove_tool("test");
        assert!(removed.is_some());
        assert!(!collection.has_tool("test"));
        assert!(collection.is_empty());
    }

    #[test]
    fn test_tool_collection_get_tool() {
        let mut collection = ToolCollection::new();
        let tool = Arc::new(MockTool {
            name: "test",
            description: "A test tool",
        });
        collection.add_tool(tool);

        let retrieved = collection.get_tool("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test");

        let missing = collection.get_tool("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_tool_collection_tool_names() {
        let mut collection = ToolCollection::new();
        collection.add_tool(Arc::new(MockTool {
            name: "tool1",
            description: "Tool 1",
        }));
        collection.add_tool(Arc::new(MockTool {
            name: "tool2",
            description: "Tool 2",
        }));

        let names = collection.tool_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"tool1"));
        assert!(names.contains(&"tool2"));
    }

    #[test]
    fn test_tool_collection_to_definitions() {
        let mut collection = ToolCollection::new();
        collection.add_tool(Arc::new(MockTool {
            name: "bash",
            description: "Execute bash commands",
        }));

        let definitions = collection.to_definitions();
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].function.name, "bash");
        assert_eq!(definitions[0].function.description, "Execute bash commands");
    }

    #[tokio::test]
    async fn test_tool_collection_execute() {
        let mut collection = ToolCollection::new();
        collection.add_tool(Arc::new(MockTool {
            name: "test",
            description: "A test tool",
        }));

        let mut ctx = create_test_context();
        let result = collection.execute("test", "hello", &mut ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.output.unwrap().contains("Executed test with: hello"));
    }

    #[tokio::test]
    async fn test_tool_collection_execute_not_found() {
        let collection = ToolCollection::new();
        let mut ctx = create_test_context();
        let result = collection.execute("nonexistent", "input", &mut ctx).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::NotFound(_)));
    }

    #[test]
    fn test_tool_collection_with_tools() {
        let tools = vec![
            Arc::new(MockTool {
                name: "tool1",
                description: "Tool 1",
            }) as Arc<dyn Tool>,
            Arc::new(MockTool {
                name: "tool2",
                description: "Tool 2",
            }) as Arc<dyn Tool>,
        ];
        let collection = ToolCollection::with_tools(tools);
        assert_eq!(collection.len(), 2);
        assert!(collection.has_tool("tool1"));
        assert!(collection.has_tool("tool2"));
    }
}
