# WebSearch Tool Design

## Overview

Multi-engine web search tool with fallback support, allowing agents to search the web for real-time information.

## Architecture

```rust
pub trait SearchEngine: Send + Sync {
    fn name(&self) -> &str;
    fn search(&self, query: &str, num_results: usize) -> Result<Vec<SearchItem>>;
}

pub struct WebSearchTool {
    engines: Vec<Box<dyn SearchEngine>>,
    max_retries: usize,
    retry_delay_secs: u64,
}
```

## SearchEngine Implementations

### DuckDuckGo (no API key)
- HTTP client to query DuckDuckGo HTML
- Parse results from HTML response
- Works without authentication

### Google Custom Search (requires API key)
- Google Custom Search JSON API
- Environment variables: `GOOGLE_API_KEY`, `GOOGLE_CX`

### Bing Search (requires API key)
- Bing Web Search API v7
- Environment variable: `BING_API_KEY`

## Data Types

```rust
pub struct SearchItem {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
}

pub struct SearchResult {
    pub position: usize,
    pub item: SearchItem,
    pub source: String,
    pub raw_content: Option<String>,
}
```

## Tool Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| query | string | yes | - | Search query |
| num_results | integer | no | 5 | Number of results (1-20) |
| engine | string | no | "duckduckgo" | Preferred engine |
| fetch_content | boolean | no | false | Fetch page content |

## Output Format

```
Search results for 'query':
1. Title
   URL: https://...
   Description: ...

2. Title
   URL: https://...
   Description: ...

Metadata:
- Total results: 5
- Engine: duckduckgo
```

## Fallback Logic

1. Try preferred engine first
2. If fails, try configured fallback engines
3. If all fail, try remaining engines
4. If all engines fail, retry with exponential backoff (up to 3 times)

## Content Fetcher

- HTTP GET with user-agent header
- Parse HTML, extract text content
- Remove scripts, styles, nav, header, footer
- Limit to 10KB per page

## File Structure

```
src/tool/web_search/
├── mod.rs           # WebSearchTool, exports
├── engine.rs        # SearchEngine trait, SearchItem, SearchResult
├── duckduckgo.rs    # DuckDuckGoEngine
├── google.rs        # GoogleEngine (optional, requires feature flag)
├── bing.rs          # BingEngine (optional, requires feature flag)
└── fetcher.rs       # ContentFetcher
```

## Error Handling

- Empty query → InvalidInput error
- All engines fail → Return error with tried engines list
- API key missing → Log warning, skip engine
- Network timeout → Retry with exponential backoff

## Dependencies

- `reqwest` - HTTP client
- `scraper` - HTML parsing
- `tokio` - Async runtime
- `serde_json` - JSON parsing

## Tests

- Unit tests for each component
- Integration test with DuckDuckGo (no API key needed)
- Error handling tests
- Fallback behavior tests
