//! Content fetcher for web pages

use reqwest::Client;
use scraper::{Html, Selector};

/// Fetches and extracts text content from web pages
pub struct ContentFetcher {
    client: Client,
    max_content_size: usize,
}

impl ContentFetcher {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap_or_else(|_| Client::new()),
            max_content_size: 10000,
        }
    }

    /// Fetch content from a URL and extract text
    pub async fn fetch(&self, url: &str) -> Option<String> {
        let response = self.client.get(url).send().await.ok()?;

        if !response.status().is_success() {
            return None;
        }

        let html = response.text().await.ok()?;
        let content = self.extract_text(&html);

        if content.len() > self.max_content_size {
            Some(format!("{}...", &content[..self.max_content_size]))
        } else {
            Some(content)
        }
    }

    fn extract_text(&self, html: &str) -> String {
        let document = Html::parse_document(html);

        // Get text from body
        let body_selector = match Selector::parse("body") {
            Ok(s) => s,
            Err(_) => return String::new(),
        };

        let text = document
            .select(&body_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default();

        // Clean up whitespace
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

impl Default for ContentFetcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetcher_new() {
        let fetcher = ContentFetcher::new();
        assert_eq!(fetcher.max_content_size, 10000);
    }

    #[tokio::test]
    async fn test_fetch_example() {
        let fetcher = ContentFetcher::new();
        let content = fetcher.fetch("https://example.com").await;
        assert!(content.is_some());
    }
}
