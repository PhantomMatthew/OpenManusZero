//! DuckDuckGo search engine

use crate::error::ToolError;
use crate::tool::web_search::{SearchEngine, SearchItem};
use async_trait::async_trait;

pub struct DuckDuckGoEngine;

impl DuckDuckGoEngine {
    pub fn new() -> Self {
        Self
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

    async fn search(&self, _query: &str, _num_results: usize) -> Result<Vec<SearchItem>, ToolError> {
        // TODO: Implement in next task
        Ok(vec![])
    }
}
