//! Google search engine implementation

use super::base::{SearchEngine, SearchItem};
use crate::error::{Error, Result};
use reqwest::Client;
use scraper::{Html, Selector};

/// Google search engine
pub struct GoogleSearch {
    client: Client,
    api_key: Option<String>,
    search_engine_id: Option<String>,
}

impl GoogleSearch {
    /// Create a new Google search instance
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap_or_default(),
            api_key: None,
            search_engine_id: None,
        }
    }

    /// Create with API key for Custom Search API
    pub fn with_api_key(api_key: String, search_engine_id: String) -> Self {
        Self {
            client: Client::new(),
            api_key: Some(api_key),
            search_engine_id: Some(search_engine_id),
        }
    }

    /// Search using Google Custom Search API
    async fn search_with_api(&self, query: &str, num_results: usize) -> Result<Vec<SearchItem>> {
        let api_key = self.api_key.as_ref().ok_or_else(|| {
            Error::Tool(crate::tool::ToolError::NotAvailable(
                "Google API key not configured".to_string(),
            ))
        })?;

        let cx = self.search_engine_id.as_ref().ok_or_else(|| {
            Error::Tool(crate::tool::ToolError::NotAvailable(
                "Google Search Engine ID not configured".to_string(),
            ))
        })?;

        let url = format!(
            "https://www.googleapis.com/customsearch/v1?key={}&cx={}&q={}&num={}",
            api_key,
            cx,
            urlencoding::encode(query),
            num_results
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Http(format!("Google API request failed: {}", e)))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::Http(format!("Failed to parse Google API response: {}", e)))?;

        let items = json["items"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        Some(SearchItem {
                            title: item["title"].as_str()?.to_string(),
                            url: item["link"].as_str()?.to_string(),
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
            "https://www.google.com/search?q={}&num={}",
            urlencoding::encode(query),
            num_results
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Http(format!("Google request failed: {}", e)))?;

        let html = response
            .text()
            .await
            .map_err(|e| Error::Http(format!("Failed to read Google response: {}", e)))?;

        parse_google_results(&html, num_results)
    }
}

impl Default for GoogleSearch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SearchEngine for GoogleSearch {
    fn name(&self) -> &str {
        "Google"
    }

    async fn search(&self, query: &str, num_results: usize) -> Result<Vec<SearchItem>> {
        // Prefer API if available, otherwise scrape
        if self.api_key.is_some() && self.search_engine_id.is_some() {
            self.search_with_api(query, num_results).await
        } else {
            self.search_by_scraping(query, num_results).await
        }
    }

    fn is_available(&self) -> bool {
        true // Always available via scraping fallback
    }
}

/// Parse Google search results from HTML
fn parse_google_results(html: &str, max_results: usize) -> Result<Vec<SearchItem>> {
    let document = Html::parse_document(html);
    let result_selector =
        Selector::parse("div.g").map_err(|e| Error::Http(format!("Selector error: {:?}", e)))?;
    let title_selector =
        Selector::parse("h3").map_err(|e| Error::Http(format!("Selector error: {:?}", e)))?;
    let link_selector =
        Selector::parse("a[href^='http']").map_err(|e| Error::Http(format!("Selector error: {:?}", e)))?;
    let snippet_selector =
        Selector::parse("div[data-sncf], div.VwiC3b").map_err(|e| Error::Http(format!("Selector error: {:?}", e)))?;

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
            .select(&link_selector)
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
    fn test_google_search_new() {
        let search = GoogleSearch::new();
        assert_eq!(search.name(), "Google");
        assert!(search.is_available());
    }

    #[test]
    fn test_google_search_with_api_key() {
        let search = GoogleSearch::with_api_key("key".to_string(), "cx".to_string());
        assert!(search.api_key.is_some());
        assert!(search.search_engine_id.is_some());
    }
}
