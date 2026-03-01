//! Unified search tool that supports multiple search engines

use super::base::{SearchEngine, SearchItem, SearchResult};
use super::{BaiduSearch, BingSearch, GoogleSearch};
use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Available search engines
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SearchEngineType {
    Google,
    Bing,
    Baidu,
    DuckDuckGo,
}

impl Default for SearchEngineType {
    fn default() -> Self {
        Self::Google
    }
}

/// Search tool input parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchInput {
    /// Search query
    pub query: String,
    /// Number of results to return
    #[serde(default = "default_num_results")]
    pub num_results: usize,
    /// Search engine to use
    #[serde(default)]
    pub engine: SearchEngineType,
}

fn default_num_results() -> usize {
    10
}

/// Unified search tool supporting multiple search engines
pub struct SearchTool {
    google: GoogleSearch,
    bing: BingSearch,
    baidu: BaiduSearch,
}

impl SearchTool {
    /// Create a new search tool
    pub fn new() -> Self {
        Self {
            google: GoogleSearch::new(),
            bing: BingSearch::new(),
            baidu: BaiduSearch::new(),
        }
    }

    /// Get the appropriate search engine
    fn get_engine(&self, engine_type: &SearchEngineType) -> &dyn SearchEngine {
        match engine_type {
            SearchEngineType::Google => &self.google,
            SearchEngineType::Bing => &self.bing,
            SearchEngineType::Baidu => &self.baidu,
            SearchEngineType::DuckDuckGo => &self.google, // Fallback to Google
        }
    }

    /// Perform a search
    pub async fn search(
        &self,
        query: &str,
        num_results: usize,
        engine: &SearchEngineType,
    ) -> Result<SearchResult, ToolError> {
        let search_engine = self.get_engine(engine);
        let items = search_engine.search(query, num_results).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(SearchResult::new(search_engine.name(), query, items))
    }
}

impl Default for SearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &'static str {
        "search"
    }

    fn description(&self) -> &'static str {
        "Search the web using various search engines (Google, Bing, Baidu)"
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "query".to_string(),
                    ToolParameter {
                        name: "query".to_string(),
                        param_type: "string".to_string(),
                        description: Some("The search query".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "num_results".to_string(),
                    ToolParameter {
                        name: "num_results".to_string(),
                        param_type: "integer".to_string(),
                        description: Some("Number of results to return (default: 10)".to_string()),
                        required: Some(false),
                        default: Some(serde_json::json!(10)),
                        enum_values: None,
                    },
                );
                props.insert(
                    "engine".to_string(),
                    ToolParameter {
                        name: "engine".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Search engine to use: google, bing, baidu".to_string()),
                        required: Some(false),
                        default: Some(serde_json::json!("google")),
                        enum_values: Some(vec![
                            "google".to_string(),
                            "bing".to_string(),
                            "baidu".to_string(),
                        ]),
                    },
                );
                props
            },
            required: Some(vec!["query".to_string()]),
        })
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        let search_input: SearchInput = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid search input: {}", e)))?;

        let result = self
            .search(
                &search_input.query,
                search_input.num_results,
                &search_input.engine,
            )
            .await?;

        Ok(ToolResult::success(result.to_markdown()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_tool_new() {
        let tool = SearchTool::new();
        assert_eq!(tool.name(), "search");
    }

    #[test]
    fn test_search_input_deserialize() {
        let json = r#"{"query": "rust programming", "num_results": 5, "engine": "google"}"#;
        let input: SearchInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.query, "rust programming");
        assert_eq!(input.num_results, 5);
    }

    #[test]
    fn test_search_input_default_engine() {
        let json = r#"{"query": "test"}"#;
        let input: SearchInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.engine, SearchEngineType::Google);
        assert_eq!(input.num_results, 10);
    }
}
