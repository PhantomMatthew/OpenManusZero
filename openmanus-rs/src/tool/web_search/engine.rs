//! Search engine trait and data types

use crate::error::ToolError;
use async_trait::async_trait;

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
#[async_trait]
pub trait SearchEngine: Send + Sync {
    /// Get the engine name
    fn name(&self) -> &str;

    /// Check if the engine is available
    fn is_available(&self) -> bool {
        true
    }

    /// Perform a search
    async fn search(&self, query: &str, num_results: usize) -> Result<Vec<SearchItem>, ToolError>;
}
