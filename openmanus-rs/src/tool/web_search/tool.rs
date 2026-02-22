//! Web search tool implementation

use crate::context::Context;
use crate::error::ToolError;
use crate::tool::web_search::{ContentFetcher, SearchEngine, SearchResult};
use crate::tool::{Tool, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Web search tool with multiple engines and fallback support
pub struct WebSearchTool {
    engines: Vec<Arc<dyn SearchEngine>>,
    fetcher: ContentFetcher,
    #[allow(dead_code)]
    max_retries: usize, // Reserved for future retry logic
}

impl WebSearchTool {
    pub fn new() -> Self {
        Self {
            engines: vec![],
            fetcher: ContentFetcher::new(),
            max_retries: 3,
        }
    }

    pub fn with_engine(mut self, engine: Arc<dyn SearchEngine>) -> Self {
        self.engines.push(engine);
        self
    }

    pub fn with_default_engines() -> Self {
        Self::new().with_engine(Arc::new(crate::tool::web_search::DuckDuckGoEngine::new()))
    }

    async fn search_with_fallback(
        &self,
        query: &str,
        num_results: usize,
        preferred_engine: Option<&str>,
    ) -> Result<Vec<SearchResult>, ToolError> {
        let mut tried_engines = Vec::new();

        // Sort engines: preferred first, then others
        let mut engine_order: Vec<_> = self.engines.iter().collect();
        if let Some(preferred) = preferred_engine {
            engine_order.sort_by_key(|e| if e.name() == preferred { 0 } else { 1 });
        }

        for engine in engine_order {
            if !engine.is_available() {
                continue;
            }

            tried_engines.push(engine.name().to_string());

            match engine.search(query, num_results).await {
                Ok(items) if !items.is_empty() => {
                    let results: Vec<SearchResult> = items
                        .into_iter()
                        .enumerate()
                        .map(|(i, item)| SearchResult {
                            position: i + 1,
                            item,
                            source: engine.name().to_string(),
                            raw_content: None,
                        })
                        .collect();

                    tracing::info!("Search successful with {} for query: {}", engine.name(), query);
                    return Ok(results);
                }
                Ok(_) => {
                    tracing::warn!("Engine {} returned no results", engine.name());
                }
                Err(e) => {
                    tracing::warn!("Engine {} failed: {}", engine.name(), e);
                }
            }
        }

        Err(ToolError::ExecutionFailed(format!(
            "All search engines failed. Tried: {}",
            tried_engines.join(", ")
        )))
    }

    async fn fetch_content_for_results(&self, results: &mut [SearchResult]) {
        for result in results.iter_mut() {
            if let Some(content) = self.fetcher.fetch(&result.item.url).await {
                result.raw_content = Some(content);
            }
        }
    }

    fn format_results(query: &str, results: &[SearchResult], fetch_content: bool) -> String {
        let mut output = vec![format!("Search results for '{}':", query)];

        for result in results {
            output.push(format!("\n{}. {}", result.position, result.item.title));
            output.push(format!("   URL: {}", result.item.url));

            if let Some(ref desc) = result.item.description {
                if !desc.is_empty() {
                    output.push(format!("   Description: {}", desc));
                }
            }

            if fetch_content {
                if let Some(ref content) = result.raw_content {
                    output.push(format!("   Content: {}", content));
                }
            }
        }

        output.push(format!("\nMetadata:"));
        output.push(format!("- Total results: {}", results.len()));
        if let Some(first) = results.first() {
            output.push(format!("- Engine: {}", first.source));
        }

        output.join("\n")
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::with_default_engines()
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn description(&self) -> &'static str {
        "Search the web for real-time information. Returns search results with titles, URLs, and descriptions."
    }

    fn parameters(&self) -> Option<ToolSchema> {
        let mut props = HashMap::new();

        props.insert(
            "query".to_string(),
            ToolParameter {
                name: "query".to_string(),
                param_type: "string".to_string(),
                description: Some("The search query to submit".to_string()),
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
                description: Some("Number of results to return (default: 5)".to_string()),
                required: Some(false),
                default: None,
                enum_values: None,
            },
        );

        props.insert(
            "engine".to_string(),
            ToolParameter {
                name: "engine".to_string(),
                param_type: "string".to_string(),
                description: Some("Preferred search engine (default: duckduckgo)".to_string()),
                required: Some(false),
                default: None,
                enum_values: Some(vec!["duckduckgo".to_string()]),
            },
        );

        props.insert(
            "fetch_content".to_string(),
            ToolParameter {
                name: "fetch_content".to_string(),
                param_type: "boolean".to_string(),
                description: Some("Whether to fetch full content from result pages (default: false)".to_string()),
                required: Some(false),
                default: None,
                enum_values: None,
            },
        );

        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: props,
            required: Some(vec!["query".to_string()]),
        })
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        let json: serde_json::Value = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid JSON: {}", e)))?;

        let query = json.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'query' parameter".to_string()))?;

        if query.trim().is_empty() {
            return Err(ToolError::InvalidInput("Query cannot be empty".to_string()));
        }

        let num_results = json.get("num_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;

        let preferred_engine = json.get("engine")
            .and_then(|v| v.as_str());

        let fetch_content = json.get("fetch_content")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut results = self.search_with_fallback(query, num_results, preferred_engine).await?;

        if fetch_content {
            self.fetch_content_for_results(&mut results).await;
        }

        let output = Self::format_results(query, &results, fetch_content);
        Ok(ToolResult::success(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::web_search::SearchItem;

    #[test]
    fn test_tool_name() {
        let tool = WebSearchTool::new();
        assert_eq!(tool.name(), "web_search");
    }

    #[test]
    fn test_tool_has_schema() {
        let tool = WebSearchTool::new();
        let schema = tool.parameters();
        assert!(schema.is_some());
        let schema = schema.unwrap();
        assert!(schema.properties.contains_key("query"));
    }

    #[test]
    fn test_tool_schema_required_params() {
        let tool = WebSearchTool::new();
        let schema = tool.parameters().unwrap();
        assert!(schema.required.is_some());
        let required = schema.required.unwrap();
        assert!(required.contains(&"query".to_string()));
    }

    #[test]
    fn test_tool_schema_all_params() {
        let tool = WebSearchTool::new();
        let schema = tool.parameters().unwrap();
        assert!(schema.properties.contains_key("query"));
        assert!(schema.properties.contains_key("num_results"));
        assert!(schema.properties.contains_key("engine"));
        assert!(schema.properties.contains_key("fetch_content"));
    }

    #[test]
    fn test_format_results() {
        let results = vec![
            SearchResult {
                position: 1,
                item: SearchItem::new("Test Title", "https://example.com")
                    .with_description("Test description"),
                source: "duckduckgo".to_string(),
                raw_content: None,
            },
        ];

        let output = WebSearchTool::format_results("test query", &results, false);
        assert!(output.contains("Search results for 'test query'"));
        assert!(output.contains("1. Test Title"));
        assert!(output.contains("https://example.com"));
        assert!(output.contains("Test description"));
        assert!(output.contains("Total results: 1"));
        assert!(output.contains("Engine: duckduckgo"));
    }

    #[test]
    fn test_format_results_with_content() {
        let results = vec![
            SearchResult {
                position: 1,
                item: SearchItem::new("Test Title", "https://example.com"),
                source: "duckduckgo".to_string(),
                raw_content: Some("Full page content here".to_string()),
            },
        ];

        let output = WebSearchTool::format_results("test", &results, true);
        assert!(output.contains("Content: Full page content here"));
    }

    #[test]
    fn test_default_engines() {
        let tool = WebSearchTool::with_default_engines();
        assert_eq!(tool.engines.len(), 1);
    }

    #[tokio::test]
    async fn test_empty_query_error() {
        let tool = WebSearchTool::new();
        let mut ctx = Context::new();

        let input = serde_json::json!({
            "query": ""
        })
        .to_string();

        let result = tool.execute(&input, &mut ctx).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_missing_query_error() {
        let tool = WebSearchTool::new();
        let mut ctx = Context::new();

        let input = serde_json::json!({
            "num_results": 5
        })
        .to_string();

        let result = tool.execute(&input, &mut ctx).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_invalid_json_error() {
        let tool = WebSearchTool::new();
        let mut ctx = Context::new();

        let input = "not valid json";

        let result = tool.execute(input, &mut ctx).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_no_engines_error() {
        let tool = WebSearchTool::new(); // No engines
        let mut ctx = Context::new();

        let input = serde_json::json!({
            "query": "test query"
        })
        .to_string();

        let result = tool.execute(&input, &mut ctx).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed(_)));
    }

    #[tokio::test]
    async fn test_format_results_in_output() {
        // Unit test for format_results which doesn't require network
        let results = vec![
            SearchResult {
                position: 1,
                item: crate::tool::web_search::SearchItem::new("Rust Programming Language", "https://rust-lang.org")
                    .with_description("Official Rust website"),
                source: "duckduckgo".to_string(),
                raw_content: None,
            },
            SearchResult {
                position: 2,
                item: crate::tool::web_search::SearchItem::new("Rust by Example", "https://doc.rust-lang.org/rust-by-example/")
                    .with_description("Learn Rust with examples"),
                source: "duckduckgo".to_string(),
                raw_content: None,
            },
        ];

        let output = WebSearchTool::format_results("Rust programming language", &results, false);
        assert!(output.contains("Search results for 'Rust programming language'"));
        assert!(output.contains("Rust Programming Language"));
        assert!(output.contains("https://rust-lang.org"));
        assert!(output.contains("Official Rust website"));
        assert!(output.contains("Rust by Example"));
        assert!(output.contains("Total results: 2"));
    }

    #[tokio::test]
    #[ignore = "Integration test - requires network access"]
    async fn test_web_search() {
        let tool = WebSearchTool::with_default_engines();
        let mut ctx = Context::new();

        let input = serde_json::json!({
            "query": "Rust programming language",
            "num_results": 3
        })
        .to_string();

        let result = tool.execute(&input, &mut ctx).await;
        assert!(result.is_ok());
        let tool_result = result.unwrap();
        assert!(tool_result.output.is_some());
        let output = tool_result.output.unwrap();
        assert!(output.contains("Search results for"));
    }
}
