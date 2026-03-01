//! Base types for search engines

use serde::{Deserialize, Serialize};

/// A single search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchItem {
    /// Title of the search result
    pub title: String,
    /// URL of the search result
    pub url: String,
    /// Description/snippet of the search result
    #[serde(default)]
    pub description: Option<String>,
}

impl std::fmt::Display for SearchItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.title, self.url)
    }
}

/// Trait for search engine implementations
#[async_trait::async_trait]
pub trait SearchEngine: Send + Sync {
    /// Get the name of the search engine
    fn name(&self) -> &str;

    /// Perform a search query
    async fn search(&self, query: &str, num_results: usize) -> crate::Result<Vec<SearchItem>>;

    /// Check if the search engine is available
    fn is_available(&self) -> bool;
}

/// Search result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Search engine used
    pub engine: String,
    /// Original query
    pub query: String,
    /// Search results
    pub items: Vec<SearchItem>,
    /// Total results found (if available)
    pub total: Option<usize>,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(engine: &str, query: &str, items: Vec<SearchItem>) -> Self {
        Self {
            engine: engine.to_string(),
            query: query.to_string(),
            items,
            total: None,
        }
    }

    /// Format as markdown
    pub fn to_markdown(&self) -> String {
        let mut output = format!("## Search Results ({})\n\n", self.engine);
        output.push_str(&format!("**Query:** {}\n\n", self.query));

        if self.items.is_empty() {
            output.push_str("No results found.\n");
        } else {
            for (i, item) in self.items.iter().enumerate() {
                output.push_str(&format!("{}. **[{}]({})**\n", i + 1, item.title, item.url));
                if let Some(ref desc) = item.description {
                    output.push_str(&format!("   {}\n\n", desc));
                } else {
                    output.push('\n');
                }
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_item_display() {
        let item = SearchItem {
            title: "Test Title".to_string(),
            url: "https://example.com".to_string(),
            description: Some("Test description".to_string()),
        };
        assert_eq!(format!("{}", item), "Test Title - https://example.com");
    }

    #[test]
    fn test_search_result_markdown() {
        let result = SearchResult::new(
            "test",
            "rust programming",
            vec![SearchItem {
                title: "Rust".to_string(),
                url: "https://rust-lang.org".to_string(),
                description: Some("Rust programming language".to_string()),
            }],
        );

        let md = result.to_markdown();
        assert!(md.contains("Rust"));
        assert!(md.contains("rust-lang.org"));
    }
}
