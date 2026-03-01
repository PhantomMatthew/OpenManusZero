//! Baidu search engine implementation

use super::base::{SearchEngine, SearchItem};
use crate::error::{Error, Result};
use reqwest::Client;
use scraper::{Html, Selector};

/// Baidu search engine
pub struct BaiduSearch {
    client: Client,
}

impl BaiduSearch {
    /// Create a new Baidu search instance
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap_or_default(),
        }
    }

    /// Search Baidu
    async fn search_baidu(&self, query: &str, num_results: usize) -> Result<Vec<SearchItem>> {
        let url = format!(
            "https://www.baidu.com/s?wd={}&rn={}",
            urlencoding::encode(query),
            num_results
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Http(format!("Baidu request failed: {}", e)))?;

        let html = response
            .text()
            .await
            .map_err(|e| Error::Http(format!("Failed to read Baidu response: {}", e)))?;

        parse_baidu_results(&html, num_results)
    }
}

impl Default for BaiduSearch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SearchEngine for BaiduSearch {
    fn name(&self) -> &str {
        "Baidu"
    }

    async fn search(&self, query: &str, num_results: usize) -> Result<Vec<SearchItem>> {
        self.search_baidu(query, num_results).await
    }

    fn is_available(&self) -> bool {
        true
    }
}

/// Parse Baidu search results from HTML
fn parse_baidu_results(html: &str, max_results: usize) -> Result<Vec<SearchItem>> {
    let document = Html::parse_document(html);
    let result_selector =
        Selector::parse("div.result, div.c-container").map_err(|e| Error::Http(format!("Selector error: {:?}", e)))?;
    let title_selector =
        Selector::parse("h3 a, h3.t a").map_err(|e| Error::Http(format!("Selector error: {:?}", e)))?;
    let snippet_selector =
        Selector::parse("div.c-abstract, div.c-span9, p").map_err(|e| Error::Http(format!("Selector error: {:?}", e)))?;

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
                title: title.trim().to_string(),
                url,
                description: description.map(|d| d.trim().to_string()),
            });
        }
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baidu_search_new() {
        let search = BaiduSearch::new();
        assert_eq!(search.name(), "Baidu");
        assert!(search.is_available());
    }
}
