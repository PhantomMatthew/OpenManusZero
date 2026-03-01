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
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
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

#[async_trait]
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

        // Check for successful response
        let status = response.status();
        if !status.is_success() {
            return Err(ToolError::ExecutionFailed(format!(
                "DuckDuckGo returned status: {}",
                status
            )));
        }

        let html = response.text().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read response: {}", e)))?;

        let results = self.parse_results(&html, num_results);
        Ok(results)
    }
}

impl DuckDuckGoEngine {
    fn parse_results(&self, html: &str, max_results: usize) -> Vec<SearchItem> {
        // Parse selectors - if any fail, return empty results
        let result_selector = match Selector::parse(".result") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let title_selector = match Selector::parse(".result__a") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let url_selector = match Selector::parse(".result__url") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let snippet_selector = match Selector::parse(".result__snippet") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let document = Html::parse_document(html);
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

    #[test]
    fn test_parse_results() {
        let engine = DuckDuckGoEngine::new();
        let html = r#"
        <html>
        <body>
            <div class="result">
                <a class="result__a" href="https://example.com">Example Title</a>
                <span class="result__url">example.com</span>
                <a class="result__snippet">This is a description</a>
            </div>
            <div class="result">
                <a class="result__a" href="https://rust-lang.org">Rust Programming</a>
                <span class="result__url">rust-lang.org</span>
                <a class="result__snippet">A language empowering everyone</a>
            </div>
        </body>
        </html>
        "#;

        let results = engine.parse_results(html, 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Example Title");
        assert_eq!(results[0].url, "example.com");
        assert_eq!(results[0].description, Some("This is a description".to_string()));
        assert_eq!(results[1].title, "Rust Programming");
    }

    #[test]
    fn test_parse_results_empty() {
        let engine = DuckDuckGoEngine::new();
        let html = "<html><body></body></html>";
        let results = engine.parse_results(html, 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_results_max_limit() {
        let engine = DuckDuckGoEngine::new();
        let html = r#"
        <html>
        <body>
            <div class="result">
                <a class="result__a">Title 1</a>
                <span class="result__url">url1.com</span>
            </div>
            <div class="result">
                <a class="result__a">Title 2</a>
                <span class="result__url">url2.com</span>
            </div>
            <div class="result">
                <a class="result__a">Title 3</a>
                <span class="result__url">url3.com</span>
            </div>
        </body>
        </html>
        "#;

        let results = engine.parse_results(html, 2);
        assert_eq!(results.len(), 2);
    }

    // Integration test - marked with #[ignore] to avoid running in CI
    // Run with: cargo test duckduckgo_search_integration -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn test_duckduckgo_search_integration() {
        let engine = DuckDuckGoEngine::new();
        let results = engine.search("Rust programming language", 5).await;

        // The search should succeed (no network error)
        assert!(results.is_ok(), "Search failed: {:?}", results.err());

        let items = results.unwrap();
        // We should get some results
        assert!(!items.is_empty(), "Expected at least one search result");

        // Each result should have a title and URL
        for item in &items {
            assert!(!item.title.is_empty(), "Title should not be empty");
            assert!(!item.url.is_empty(), "URL should not be empty");
        }
    }
}
