# WebSearch Tool Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a multi-engine web search tool with DuckDuckGo, Google, and Bing support, fallback mechanism, and optional content fetching.

**Architecture:** Trait-based design with `SearchEngine` trait, multiple engine implementations, and a main `WebSearchTool` that handles fallback logic. Uses reqwest for HTTP and scraper for HTML parsing.

**Tech Stack:** Rust, tokio (async), reqwest (HTTP), scraper (HTML), serde_json

---

## Task 1: Add Dependencies and Create Module Structure

**Files:**
- Modify: `Cargo.toml`
- Create: `src/tool/web_search/mod.rs`
- Create: `src/tool/web_search/engine.rs`
- Modify: `src/tool/mod.rs`

**Step 1: Add scraper dependency to Cargo.toml**

Add under `[dependencies]`:
```toml
scraper = "0.19"
```

**Step 2: Create src/tool/web_search/mod.rs**

```rust
//! Web search tool with multiple search engines

mod engine;
mod duckduckgo;
mod fetcher;
mod tool;

pub use engine::{SearchEngine, SearchItem, SearchResult};
pub use duckduckgo::DuckDuckGoEngine;
pub use fetcher::ContentFetcher;
pub use tool::WebSearchTool;
```

**Step 3: Create src/tool/web_search/engine.rs**

```rust
//! Search engine trait and data types

use crate::error::ToolError;

/// A single search result item
#[derive(Debug, Clone)]
pub struct SearchItem {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
}

impl SearchItem {
    pub fn new(title: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            url: url.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// A search result with position and source info
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub position: usize,
    pub item: SearchItem,
    pub source: String,
    pub raw_content: Option<String>,
}

/// Trait for search engine implementations
#[async_trait::async_trait]
pub trait SearchEngine: Send + Sync {
    /// Get the engine name
    fn name(&self) -> &str;

    /// Check if the engine is available (has required API keys, etc.)
    fn is_available(&self) -> bool {
        true
    }

    /// Perform a search
    async fn search(&self, query: &str, num_results: usize) -> Result<Vec<SearchItem>, ToolError>;
}
```

**Step 4: Create stub files for other modules**

Create `src/tool/web_search/duckduckgo.rs`:
```rust
//! DuckDuckGo search engine

use crate::tool::web_search::{SearchEngine, SearchItem};
use crate::error::ToolError;
use async_trait::async_trait;

pub struct DuckDuckGoEngine;

impl DuckDuckGoEngine {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl SearchEngine for DuckDuckGoEngine {
    fn name(&self) -> &str {
        "duckduckgo"
    }

    async fn search(&self, query: &str, num_results: usize) -> Result<Vec<SearchItem>, ToolError> {
        // TODO: Implement
        Ok(vec![])
    }
}
```

Create `src/tool/web_search/fetcher.rs`:
```rust
//! Content fetcher for web pages

pub struct ContentFetcher;

impl ContentFetcher {
    pub fn new() -> Self {
        Self
    }

    pub async fn fetch(&self, url: &str) -> Option<String> {
        // TODO: Implement
        None
    }
}
```

Create `src/tool/web_search/tool.rs`:
```rust
//! Web search tool

use crate::tool::web_search::{SearchEngine, SearchResult};
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use crate::context::Context;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

pub struct WebSearchTool {
    engines: Vec<Arc<dyn SearchEngine>>,
}

impl WebSearchTool {
    pub fn new() -> Self {
        Self {
            engines: vec![],
        }
    }

    pub fn with_engine(mut self, engine: Arc<dyn SearchEngine>) -> Self {
        self.engines.push(engine);
        self
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn description(&self) -> &'static str {
        "Search the web for information using search engines"
    }

    fn parameters(&self) -> Option<ToolSchema> {
        // TODO: Implement
        None
    }

    async fn execute(&self, input: &str, ctx: &mut Context) -> Result<ToolResult, ToolError> {
        // TODO: Implement
        Ok(ToolResult::success("Not implemented"))
    }
}
```

**Step 5: Update src/tool/mod.rs**

Add:
```rust
pub mod web_search;
pub use web_search::WebSearchTool;
```

**Step 6: Verify compilation**

Run: `cargo check`
Expected: Compiles with warnings about unused code

**Step 7: Commit**

```bash
git add Cargo.toml src/tool/web_search/ src/tool/mod.rs && git commit -m "feat(tool): add WebSearch module structure"
```

---

## Task 2: Implement DuckDuckGo Engine

**Files:**
- Modify: `src/tool/web_search/duckduckgo.rs`

**Step 1: Implement DuckDuckGo search**

```rust
//! DuckDuckGo search engine implementation

use crate::error::ToolError;
use crate::tool::web_search::{SearchEngine, SearchItem};
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};

/// DuckDuckGo HTML search engine
pub struct DuckDuckGoEngine {
    client: Client,
}

impl DuckDuckGoEngine {
    const SEARCH_URL: &'static str = "https://html.duckduckgo.com/html/";

    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }
}

impl Default for DuckDuckGoEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SearchEngine for DuckDuckGoEngine {
    fn name(&self) -> &str {
        "duckduckgo"
    }

    async fn search(&self, query: &str, num_results: usize) -> Result<Vec<SearchItem>, ToolError> {
        let params = [
            ("q", query),
            ("kl", "us-en"),
        ];

        let response = self.client
            .get(Self::SEARCH_URL)
            .query(&params)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("DuckDuckGo request failed: {}", e)))?;

        let html = response.text().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read response: {}", e)))?;

        let results = self.parse_results(&html, num_results);
        Ok(results)
    }
}

impl DuckDuckGoEngine {
    fn parse_results(&self, html: &str, max_results: usize) -> Vec<SearchItem> {
        let document = Html::parse_document(html);
        let result_selector = Selector::parse(".result").ok()?;
        let title_selector = Selector::parse(".result__a").ok()?;
        let url_selector = Selector::parse(".result__url").ok()?;
        let snippet_selector = Selector::parse(".result__snippet").ok()?;

        let mut results = Vec::new();

        for result in document.select(&result_selector).take(max_results) {
            let title = result
                .select(&title_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default()
                .trim()
                .to_string();

            let url = result
                .select(&url_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default()
                .trim()
                .to_string();

            let description = result
                .select(&snippet_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .map(|s| s.trim().to_string());

            if !title.is_empty() && !url.is_empty() {
                results.push(SearchItem {
                    title,
                    url,
                    description,
                });
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_name() {
        let engine = DuckDuckGoEngine::new();
        assert_eq!(engine.name(), "duckduckgo");
    }

    #[tokio::test]
    async fn test_duckduckgo_search() {
        let engine = DuckDuckGoEngine::new();
        let results = engine.search("Rust programming", 3).await;
        assert!(results.is_ok());
        // Note: This test makes a real network request
    }
}
```

**Step 2: Run tests**

Run: `cargo test duckduckgo -- --nocapture`
Expected: Tests pass (may be slow due to network)

**Step 3: Commit**

```bash
git add src/tool/web_search/duckduckgo.rs && git commit -m "feat(tool): implement DuckDuckGo search engine"
```

---

## Task 3: Implement Content Fetcher

**Files:**
- Modify: `src/tool/web_search/fetcher.rs`

**Step 1: Implement content fetcher**

```rust
//! Content fetcher for web pages

use reqwest::Client;
use scraper::{Html, Selector};

/// Fetches and extracts text content from web pages
pub struct ContentFetcher {
    client: Client,
    max_content_size: usize,
}

impl ContentFetcher {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap_or_else(|_| Client::new()),
            max_content_size: 10000,
        }
    }

    /// Fetch content from a URL and extract text
    pub async fn fetch(&self, url: &str) -> Option<String> {
        let response = self.client.get(url).send().await.ok()?;

        if !response.status().is_success() {
            return None;
        }

        let html = response.text().await.ok()?;
        let content = self.extract_text(&html);

        if content.len() > self.max_content_size {
            Some(format!("{}...", &content[..self.max_content_size]))
        } else {
            Some(content)
        }
    }

    fn extract_text(&self, html: &str) -> String {
        let document = Html::parse_document(html);

        // Remove unwanted elements
        let remove_selector = Selector::parse("script, style, nav, header, footer").ok();
        if let Some(selector) = remove_selector {
            // Note: scraper doesn't support removal, so we just extract from body
        }

        // Get text from body
        let body_selector = Selector::parse("body").ok();
        let text = if let Some(selector) = body_selector {
            document
                .select(&selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default()
        } else {
            document.root_element().text().collect::<String>()
        };

        // Clean up whitespace
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

impl Default for ContentFetcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetcher_new() {
        let fetcher = ContentFetcher::new();
        assert_eq!(fetcher.max_content_size, 10000);
    }

    #[tokio::test]
    async fn test_fetch_example() {
        let fetcher = ContentFetcher::new();
        let content = fetcher.fetch("https://example.com").await;
        assert!(content.is_some());
    }
}
```

**Step 2: Run tests**

Run: `cargo test fetcher -- --nocapture`
Expected: Tests pass

**Step 3: Commit**

```bash
git add src/tool/web_search/fetcher.rs && git commit -m "feat(tool): implement content fetcher"
```

---

## Task 4: Implement WebSearchTool

**Files:**
- Modify: `src/tool/web_search/tool.rs`

**Step 1: Implement full WebSearchTool**

```rust
//! Web search tool implementation

use crate::context::Context;
use crate::error::ToolError;
use crate::tool::web_search::{ContentFetcher, SearchEngine, SearchItem, SearchResult};
use crate::tool::{Tool, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Web search tool with multiple engines and fallback support
pub struct WebSearchTool {
    engines: Vec<Arc<dyn SearchEngine>>,
    fetcher: ContentFetcher,
    max_retries: usize,
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

    #[tokio::test]
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
    }
}
```

**Step 2: Run tests**

Run: `cargo test web_search -- --nocapture`
Expected: Tests pass

**Step 3: Commit**

```bash
git add src/tool/web_search/tool.rs && git commit -m "feat(tool): implement WebSearchTool with fallback support"
```

---

## Task 5: Update ToolCollection and Documentation

**Files:**
- Modify: `src/tool/collection.rs`
- Modify: `pending_tasks.md`

**Step 1: Add WebSearchTool to default collection**

In `src/tool/collection.rs`, update `with_defaults()`:
```rust
pub fn with_defaults() -> Self {
    use crate::tool::{BashTool, TerminateTool, FileTool, StrReplaceEditor, WebSearchTool};

    let mut collection = Self::new();
    collection.add_tool(Arc::new(BashTool::new()));
    collection.add_tool(Arc::new(TerminateTool::new()));
    collection.add_tool(Arc::new(FileTool::new()));
    collection.add_tool(Arc::new(StrReplaceEditor::new()));
    collection.add_tool(Arc::new(WebSearchTool::with_default_engines()));
    collection
}
```

**Step 2: Update pending_tasks.md**

Change:
```
| WebSearch | `app/tool/web_search.py` | 网页搜索工具 | 高 |
```
to:
```
| WebSearch | `app/tool/web_search.py` | 网页搜索工具 | ✅ |
```

**Step 3: Run full test suite**

Run: `cargo test --lib`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/tool/collection.rs pending_tasks.md && git commit -m "feat(tool): add WebSearchTool to default collection"
```

---

## Summary

| Task | Description | Tests |
|------|-------------|-------|
| 1 | Module structure | 0 |
| 2 | DuckDuckGo engine | 2 |
| 3 | Content fetcher | 2 |
| 4 | WebSearchTool | 3 |
| 5 | Integration | 0 |

**Total: 7 tests**
