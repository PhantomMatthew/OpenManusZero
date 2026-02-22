//! Content fetcher for web pages

pub struct ContentFetcher;

impl ContentFetcher {
    pub fn new() -> Self {
        Self
    }

    pub async fn fetch(&self, _url: &str) -> Option<String> {
        // TODO: Implement in next task
        None
    }
}

impl Default for ContentFetcher {
    fn default() -> Self {
        Self::new()
    }
}
