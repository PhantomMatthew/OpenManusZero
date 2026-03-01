//! Web crawler tool for scraping web pages

use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Crawler configuration
#[derive(Debug, Clone)]
pub struct CrawlerConfig {
    /// Maximum pages to crawl
    pub max_pages: usize,
    /// Maximum depth
    pub max_depth: usize,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Follow links
    pub follow_links: bool,
    /// User agent
    pub user_agent: String,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            max_pages: 10,
            max_depth: 2,
            timeout_secs: 30,
            follow_links: false,
            user_agent: "OpenManus-Crawler/1.0".to_string(),
        }
    }
}

/// Crawl result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResult {
    /// URL crawled
    pub url: String,
    /// Page title
    pub title: Option<String>,
    /// Page text content
    pub text: String,
    /// Links found
    pub links: Vec<String>,
    /// Images found
    pub images: Vec<String>,
    /// Metadata
    pub metadata: PageMetadata,
}

/// Page metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageMetadata {
    /// Description meta tag
    pub description: Option<String>,
    /// Keywords meta tag
    pub keywords: Option<String>,
    /// Author meta tag
    pub author: Option<String>,
}

/// Crawl tool input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlInput {
    /// URL to crawl
    pub url: String,
    /// Maximum pages to crawl
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,
    /// Maximum depth
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    /// Extract specific selector
    #[serde(default)]
    pub selector: Option<String>,
    /// Follow links
    #[serde(default)]
    pub follow_links: bool,
}

fn default_max_pages() -> usize { 10 }
fn default_max_depth() -> usize { 2 }

/// Web crawler tool
pub struct CrawlTool {
    client: Client,
    config: CrawlerConfig,
}

impl CrawlTool {
    /// Create a new crawl tool
    pub fn new() -> Self {
        let config = CrawlerConfig::default();
        let client = Client::builder()
            .user_agent(&config.user_agent)
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_default();

        Self { client, config }
    }

    /// Create with custom configuration
    pub fn with_config(config: CrawlerConfig) -> Self {
        let client = Client::builder()
            .user_agent(&config.user_agent)
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_default();

        Self { client, config }
    }

    /// Crawl a single page
    pub async fn crawl_page(&self, url: &str, selector: Option<&str>) -> Result<CrawlResult, ToolError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to fetch {}: {}", url, e)))?;

        let html = response
            .text()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read response: {}", e)))?;

        self.parse_page(url, &html, selector)
    }

    /// Parse HTML page
    fn parse_page(&self, url: &str, html: &str, selector: Option<&str>) -> Result<CrawlResult, ToolError> {
        let document = Html::parse_document(html);

        // Simple URL join without url crate
        let base_url = url.to_string();

        // Extract title
        let title = document
            .select(&Selector::parse("title").unwrap())
            .next()
            .map(|el| el.text().collect::<String>());

        // Extract text
        let text = if let Some(sel) = selector {
            document
                .select(&Selector::parse(sel).map_err(|e| {
                    ToolError::InvalidInput(format!("Invalid selector: {:?}", e))
                })?)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default()
        } else {
            // Extract body text
            document
                .select(&Selector::parse("body").unwrap())
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default()
        };

        // Extract links (simplified - just get href values)
        let links: Vec<String> = document
            .select(&Selector::parse("a[href]").unwrap())
            .filter_map(|el| {
                let href = el.value().attr("href")?;
                if href.starts_with("javascript:") || href.starts_with("#") {
                    return None;
                }
                // Simple URL resolution
                if href.starts_with("http://") || href.starts_with("https://") {
                    Some(href.to_string())
                } else if href.starts_with("/") {
                    // Assuming base URL ends without trailing slash
                    let base = base_url.split('?').next().unwrap_or(&base_url);
                    let base = base.split('#').next().unwrap_or(base);
                    // Find the protocol:// part and get the base
                    let scheme_end = base.find("://").map(|p| p + 3).unwrap_or(0);
                    let path_start = base[scheme_end..].find('/').map(|p| scheme_end + p);
                    if let Some(pos) = path_start {
                        Some(format!("{}{}", &base[..pos], href))
                    } else {
                        Some(format!("{}{}", base, href))
                    }
                } else {
                    None
                }
            })
            .collect();

        // Extract images
        let images: Vec<String> = document
            .select(&Selector::parse("img[src]").unwrap())
            .filter_map(|el| {
                let src = el.value().attr("src")?;
                if src.starts_with("http://") || src.starts_with("https://") {
                    Some(src.to_string())
                } else {
                    None
                }
            })
            .collect();

        // Extract metadata
        let metadata = PageMetadata {
            description: document
                .select(&Selector::parse("meta[name='description']").unwrap())
                .next()
                .and_then(|el| el.value().attr("content").map(|s| s.to_string())),
            keywords: document
                .select(&Selector::parse("meta[name='keywords']").unwrap())
                .next()
                .and_then(|el| el.value().attr("content").map(|s| s.to_string())),
            author: document
                .select(&Selector::parse("meta[name='author']").unwrap())
                .next()
                .and_then(|el| el.value().attr("content").map(|s| s.to_string())),
        };

        Ok(CrawlResult {
            url: url.to_string(),
            title,
            text: text.trim().to_string(),
            links,
            images,
            metadata,
        })
    }

    /// Format crawl result as markdown
    fn format_result(result: &CrawlResult) -> String {
        let mut output = format!("## Crawl Result: {}\n\n", result.url);

        if let Some(ref title) = result.title {
            output.push_str(&format!("**Title:** {}\n\n", title));
        }

        if let Some(ref desc) = result.metadata.description {
            output.push_str(&format!("**Description:** {}\n\n", desc));
        }

        output.push_str("**Content:**\n");
        output.push_str(&format!("{}\n\n", &result.text[..result.text.len().min(2000)]));

        if !result.links.is_empty() {
            output.push_str(&format!("**Links ({}):**\n", result.links.len()));
            for link in result.links.iter().take(10) {
                output.push_str(&format!("- {}\n", link));
            }
            if result.links.len() > 10 {
                output.push_str(&format!("  ... and {} more\n", result.links.len() - 10));
            }
        }

        output
    }
}

impl Default for CrawlTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CrawlTool {
    fn name(&self) -> &'static str {
        "crawl"
    }

    fn description(&self) -> &'static str {
        "Web crawler tool for scraping web pages and extracting content"
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "url".to_string(),
                    ToolParameter {
                        name: "url".to_string(),
                        param_type: "string".to_string(),
                        description: Some("URL to crawl".to_string()),
                        required: Some(true),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "max_pages".to_string(),
                    ToolParameter {
                        name: "max_pages".to_string(),
                        param_type: "integer".to_string(),
                        description: Some("Maximum pages to crawl (default: 10)".to_string()),
                        required: Some(false),
                        default: Some(serde_json::json!(10)),
                        enum_values: None,
                    },
                );
                props.insert(
                    "max_depth".to_string(),
                    ToolParameter {
                        name: "max_depth".to_string(),
                        param_type: "integer".to_string(),
                        description: Some("Maximum crawl depth (default: 2)".to_string()),
                        required: Some(false),
                        default: Some(serde_json::json!(2)),
                        enum_values: None,
                    },
                );
                props.insert(
                    "selector".to_string(),
                    ToolParameter {
                        name: "selector".to_string(),
                        param_type: "string".to_string(),
                        description: Some("CSS selector to extract specific content".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "follow_links".to_string(),
                    ToolParameter {
                        name: "follow_links".to_string(),
                        param_type: "boolean".to_string(),
                        description: Some("Whether to follow links (default: false)".to_string()),
                        required: Some(false),
                        default: Some(serde_json::json!(false)),
                        enum_values: None,
                    },
                );
                props
            },
            required: Some(vec!["url".to_string()]),
        })
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        let crawl_input: CrawlInput = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid crawl input: {}", e)))?;

        let result = self
            .crawl_page(&crawl_input.url, crawl_input.selector.as_deref())
            .await?;

        Ok(ToolResult::success(Self::format_result(&result)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crawl_tool_new() {
        let tool = CrawlTool::new();
        assert_eq!(tool.name(), "crawl");
    }

    #[test]
    fn test_crawl_config_default() {
        let config = CrawlerConfig::default();
        assert_eq!(config.max_pages, 10);
        assert_eq!(config.max_depth, 2);
    }

    #[test]
    fn test_crawl_result_format() {
        let result = CrawlResult {
            url: "https://example.com".to_string(),
            title: Some("Example".to_string()),
            text: "Hello world".to_string(),
            links: vec!["https://example.com/page1".to_string()],
            images: vec![],
            metadata: PageMetadata::default(),
        };

        let formatted = CrawlTool::format_result(&result);
        assert!(formatted.contains("Example"));
        assert!(formatted.contains("Hello world"));
    }

    #[test]
    fn test_crawl_input_deserialize() {
        let json = r#"{"url": "https://example.com", "max_pages": 5}"#;
        let input: CrawlInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.url, "https://example.com");
        assert_eq!(input.max_pages, 5);
    }
}
