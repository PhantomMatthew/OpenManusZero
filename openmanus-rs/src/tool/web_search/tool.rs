//! Web search tool

use crate::context::Context;
use crate::error::ToolError;
use crate::tool::web_search::{ContentFetcher, SearchEngine};
use crate::tool::{Tool, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::sync::Arc;

pub struct WebSearchTool {
    engines: Vec<Arc<dyn SearchEngine>>,
    fetcher: ContentFetcher,
}

impl WebSearchTool {
    pub fn new() -> Self {
        Self {
            engines: vec![],
            fetcher: ContentFetcher::new(),
        }
    }

    pub fn with_engine(mut self, engine: Arc<dyn SearchEngine>) -> Self {
        self.engines.push(engine);
        self
    }

    pub fn with_default_engines() -> Self {
        use crate::tool::web_search::DuckDuckGoEngine;
        Self::new().with_engine(Arc::new(DuckDuckGoEngine::new()))
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
        "Search the web for real-time information"
    }

    fn parameters(&self) -> Option<ToolSchema> {
        None // TODO: Implement in later task
    }

    async fn execute(&self, _input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success("Not implemented"))
    }
}
