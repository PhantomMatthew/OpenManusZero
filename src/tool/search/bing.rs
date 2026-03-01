//! Bing search engine implementation

use super::base::{SearchEngine, SearchItem};
use crate::error::{Error, Result};
use reqwest::Client;
use scraper::{Html, Selector};

/// Bing search engine
pub struct BingSearch {
    client: Client,
    subscription_key: Option<String>,
}

impl BingSearch {
    /// Create a new Bing search instance
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap_or_default(),
            subscription_key: None,
        }
    }

    /// Create with Bing Search API subscription key
    pub fn with_api_key(subscription_key: String) -> Self {
        Self {
            client: Client::new(),
            subscription_key: Some(subscription_key),
        }
    }

    /// Search using Bing Search API
    async fn search_with_api(&self, query: &str, num_results: usize) -> Result<Vec<SearchItem>> {
        let key = self.subscription_key.as_ref().ok_or_else(|| {
            Error::Tool(crate::tool::ToolError::NotAvailable(
                "Bing API key not configured".to_string(),
            ))
        })?;

        let url = format!(
            "https://api.bing.microsoft.com/v7.0/search?q={}&count={}",
            urlencoding::encode(query),
            num_results
        );

        let response = self
            .client
            .get(&url)
            .header("Ocp-Apim-Subscription-Key", key)
            .send()
            .await
            .map_err(|e| Error::Http(format!("Bing API request failed: {}", e)))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::Http(format!("Failed to parse Bing API response: {}", e)))?;

        let items = json["webPages"]["value"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        Some(SearchItem {
                            title: item["name"].as_str()?.to_string(),
                            url: item["url"].as_str()?.to_string(),
                            description: item["snippet"].as_str().map(|s| s.to_string()),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(items)
    }

    /// Search by scraping (fallback when API not available)
    async fn search_by_scraping(&self, query: &str, num_results: usize) -> Result<Vec<SearchItem>> {
        let url = format!(
            "https://www.bing.com/search?q={}&count={}",
            urlencoding::encode(query),
            num_results
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Http(format!("Bing request failed: {}", e)))?;

        let html = response
            .text()
            .await
            .map_err(|e| Error::Http(format!("Failed to read Bing response: {}", e)))?;

        parse_bing_results(&html, num_results)
    }
}

impl Default for BingSearch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SearchEngine for BingSearch {
    fn name(&self) -> &str {
        "Bing"
    }

    async fn search(&self, query: &str, num_results: usize) -> Result<Vec<SearchItem>> {
        if self.subscription_key.is_some() {
            self.search_with_api(query, num_results).await
        } else {
            self.search_by_scraping(query, num_results).await
        }
    }

    fn is_available(&self) -> bool {
        true
    }
}

/// Parse Bing search results from HTML
fn parse_bing_results(html: &str, max_results: usize) -> Result<Vec<SearchItem>> {
    let document = Html::parse_document(html);
    let result_selector =
        Selector::parse("li.b_algo").map_err(|e| Error::Http(format!("Selector error: {:?}", e)))?;
    let title_selector =
        Selector::parse("h2 a").map_err(|e| Error::Http(format!("Selector error: {:?}", e)))?;
    let snippet_selector =
        Selector::parse("p, .b_caption p").map_err(|e| Error::Http(format!("Selector error: {:?}", e)))?;

    let mut items = Vec::new();

    for result in document.select(&result_selector) {
        if items.len() >= max_results {
            break;
        }

        let title = result
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default();

        let url = result
            .select(&title_selector)
            .next()
            .and_then(|el| el.value().attr("href"))
            .unwrap_or_default()
            .to_string();

        let description = result
            .select(&snippet_selector)
            .next()
            .map(|el| el.text().collect::<String>());

        if !title.is_empty() && !url.is_empty() {
            items.push(SearchItem {
                title,
                url,
                description,
            });
        }
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bing_search_new() {
        let search = BingSearch::new();
        assert_eq!(search.name(), "Bing");
        assert!(search.is_available());
    }

    #[test]
    fn test_bing_search_with_api_key() {
        let search = BingSearch::with_api_key("key".to_string());
        assert!(search.subscription_key.is_some());
    }
}
