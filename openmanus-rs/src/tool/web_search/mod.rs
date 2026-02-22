//! Web search tool with multiple search engines

mod engine;
mod duckduckgo;
mod fetcher;
mod tool;

pub use engine::{SearchEngine, SearchItem, SearchResult};
pub use duckduckgo::DuckDuckGoEngine;
pub use fetcher::ContentFetcher;
pub use tool::WebSearchTool;
