//! Search tools module
//!
//! Provides web search capabilities through multiple search engines:
//! - Google Search
//! - Bing Search
//! - Baidu Search

mod base;
mod google;
mod bing;
mod baidu;
mod tool;

pub use base::{SearchEngine, SearchItem};
pub use google::GoogleSearch;
pub use bing::BingSearch;
pub use baidu::BaiduSearch;
pub use tool::SearchTool;
