//! Browser tool implementation using chromiumoxide

use crate::context::Context;
use crate::error::ToolError;
use crate::tool::{Tool, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::handler::viewport::Viewport as HandlerViewport;
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::Element;
use chromiumoxide::Page;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Browser tool for web automation using chromiumoxide
pub struct BrowserTool {
    /// Whether to run browser in headless mode
    headless: bool,
    /// Shared browser instance
    browser: Arc<RwLock<Option<Browser>>>,
    /// Current page
    current_page: Arc<RwLock<Option<Page>>>,
}

impl BrowserTool {
    /// Create a new browser tool
    pub fn new() -> Self {
        Self {
            headless: true,
            browser: Arc::new(RwLock::new(None)),
            current_page: Arc::new(RwLock::new(None)),
        }
    }

    /// Create with headless mode
    pub fn with_headless(headless: bool) -> Self {
        Self {
            headless,
            browser: Arc::new(RwLock::new(None)),
            current_page: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize browser connection
    async fn ensure_browser(&self) -> Result<(), ToolError> {
        let mut browser_guard = self.browser.write().await;

        if browser_guard.is_none() {
            let chrome_exe = which_chrome()?;

            let viewport = HandlerViewport {
                width: 1920,
                height: 1080,
                device_scale_factor: Some(1.0),
                emulating_mobile: false,
                is_landscape: true,
                has_touch: false,
            };

            let config = BrowserConfig::builder()
                .chrome_executable(chrome_exe)
                .window_size(1920, 1080)
                .viewport(viewport);

            let config = if self.headless {
                config.build().map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to build browser config: {}", e))
                })?
            } else {
                config.no_sandbox().build().map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to build browser config: {}", e))
                })?
            };

            let (browser, _handler) = Browser::launch(config).await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to launch browser: {}", e))
            })?;

            // Spawn handler task to process browser events
            // The handler needs to stay alive for the browser to work
            tokio::spawn(async move {
                let mut handler = _handler;
                while let Some(event) = futures::StreamExt::next(&mut handler).await {
                    if event.is_err() {
                        break;
                    }
                }
            });

            *browser_guard = Some(browser);
        }

        Ok(())
    }

    /// Get or create a page
    async fn get_page(&self) -> Result<Page, ToolError> {
        self.ensure_browser().await?;

        let mut page_guard = self.current_page.write().await;

        if page_guard.is_none() {
            let browser_guard = self.browser.read().await;
            if let Some(browser) = browser_guard.as_ref() {
                let page = browser.new_page("about:blank").await.map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to create page: {}", e))
                })?;
                *page_guard = Some(page);
            }
        }

        page_guard
            .clone()
            .ok_or_else(|| ToolError::ExecutionFailed("No page available".to_string()))
    }

    /// Navigate to a URL
    async fn navigate(&self, url: &str) -> Result<String, ToolError> {
        let page = self.get_page().await?;

        page.goto(url)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Navigation failed: {}", e)))?;

        // Wait for page to load
        page.wait_for_navigation().await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Wait for navigation failed: {}", e))
        })?;

        let title = page
            .get_title()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get title: {}", e)))?
            .unwrap_or_else(|| "Untitled".to_string());

        Ok(format!("Navigated to: {} (Title: {})", url, title))
    }

    /// Click an element by selector
    async fn click(&self, selector: &str) -> Result<String, ToolError> {
        let page = self.get_page().await?;

        let element = find_element(&page, selector).await?;

        // Scroll into view first
        element.scroll_into_view().await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to scroll element into view: {}", e))
        })?;

        element
            .click()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Click failed: {}", e)))?;

        Ok(format!("Clicked element: {}", selector))
    }

    /// Type text into an element
    async fn type_text(&self, selector: &str, text: &str) -> Result<String, ToolError> {
        let page = self.get_page().await?;

        let element = find_element(&page, selector).await?;

        // Click to focus
        element
            .click()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to focus element: {}", e)))?;

        // Type the text
        element
            .type_str(text)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Type failed: {}", e)))?;

        Ok(format!("Typed '{}' into: {}", text, selector))
    }

    /// Take a screenshot
    async fn screenshot(&self) -> Result<ToolResult, ToolError> {
        let page = self.get_page().await?;

        let params = ScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Png)
            .build();

        let screenshot_data = page
            .screenshot(params)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Screenshot failed: {}", e)))?;

        // Convert to base64 for result
        let base64 = base64_encode(&screenshot_data);

        Ok(ToolResult::with_image(
            "Screenshot captured".to_string(),
            base64,
        ))
    }

    /// Extract content from page
    async fn extract(&self, selector: Option<&str>) -> Result<String, ToolError> {
        let page = self.get_page().await?;

        let content = if let Some(sel) = selector {
            let element = find_element(&page, sel).await?;
            element
                .inner_text()
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to extract text: {}", e)))?
                .unwrap_or_default()
        } else {
            // Extract all text from body
            let result = page
                .evaluate("document.body.innerText")
                .await
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to extract page content: {}", e))
                })?;
            result.into_value::<String>().map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to parse page content: {}", e))
            })?
        };

        // Truncate if too long
        let truncated = if content.len() > 5000 {
            format!(
                "{}...\n[Content truncated, {} chars total]",
                &content[..5000],
                content.len()
            )
        } else {
            content
        };

        Ok(truncated)
    }
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &'static str {
        "browser"
    }

    fn description(&self) -> &'static str {
        "Browse the web and interact with pages. Can navigate, click, type, take screenshots, and extract content."
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "action".to_string(),
                    ToolParameter {
                        name: "action".to_string(),
                        param_type: "string".to_string(),
                        description: Some(
                            "Action to perform: navigate, click, type, screenshot, extract"
                                .to_string(),
                        ),
                        required: Some(true),
                        default: None,
                        enum_values: Some(vec![
                            "navigate".to_string(),
                            "click".to_string(),
                            "type".to_string(),
                            "screenshot".to_string(),
                            "extract".to_string(),
                        ]),
                    },
                );
                props.insert(
                    "url".to_string(),
                    ToolParameter {
                        name: "url".to_string(),
                        param_type: "string".to_string(),
                        description: Some("URL to navigate to (for navigate action)".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "selector".to_string(),
                    ToolParameter {
                        name: "selector".to_string(),
                        param_type: "string".to_string(),
                        description: Some(
                            "CSS selector for element (for click, type, extract actions)"
                                .to_string(),
                        ),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props.insert(
                    "text".to_string(),
                    ToolParameter {
                        name: "text".to_string(),
                        param_type: "string".to_string(),
                        description: Some("Text to type (for type action)".to_string()),
                        required: Some(false),
                        default: None,
                        enum_values: None,
                    },
                );
                props
            },
            required: Some(vec!["action".to_string()]),
        })
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        let parsed: serde_json::Value = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid JSON: {}", e)))?;

        let action = parsed
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'action' field".to_string()))?;

        let result = match action {
            "navigate" => {
                let url = parsed.get("url").and_then(|v| v.as_str()).ok_or_else(|| {
                    ToolError::InvalidInput("Missing 'url' for navigate".to_string())
                })?;
                self.navigate(url).await?
            }
            "click" => {
                let selector =
                    parsed
                        .get("selector")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ToolError::InvalidInput("Missing 'selector' for click".to_string())
                        })?;
                self.click(selector).await?
            }
            "type" => {
                let selector =
                    parsed
                        .get("selector")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ToolError::InvalidInput("Missing 'selector' for type".to_string())
                        })?;
                let text = parsed.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
                    ToolError::InvalidInput("Missing 'text' for type".to_string())
                })?;
                self.type_text(selector, text).await?
            }
            "screenshot" => {
                return self.screenshot().await;
            }
            "extract" => {
                let selector = parsed.get("selector").and_then(|v| v.as_str());
                self.extract(selector).await?
            }
            _ => {
                return Err(ToolError::InvalidInput(format!(
                    "Unknown action: {}",
                    action
                )))
            }
        };

        Ok(ToolResult::success(result))
    }

    async fn cleanup(&self) -> Result<(), ToolError> {
        let mut page_guard = self.current_page.write().await;
        *page_guard = None;

        let mut browser_guard = self.browser.write().await;
        if let Some(mut browser) = browser_guard.take() {
            browser.close().await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to close browser: {}", e))
            })?;
        }
        Ok(())
    }
}

/// Find the Chrome executable
fn which_chrome() -> Result<std::path::PathBuf, ToolError> {
    // Common Chrome/Chromium locations
    let candidates = if cfg!(target_os = "macos") {
        vec![
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
            "/snap/bin/chromium",
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files\Chromium\Application\chrome.exe",
        ]
    } else {
        vec![]
    };

    for candidate in candidates {
        let path = std::path::Path::new(candidate);
        if path.exists() {
            return Ok(path.to_path_buf());
        }
    }

    // Try to find in PATH
    which::which("chrome")
        .or_else(|_| which::which("google-chrome"))
        .or_else(|_| which::which("chromium"))
        .or_else(|_| which::which("chromium-browser"))
        .map_err(|_| {
            ToolError::ExecutionFailed(
                "Chrome/Chromium not found. Please install Chrome or Chromium.".to_string(),
            )
        })
}

/// Find an element by selector with retry
async fn find_element(page: &Page, selector: &str) -> Result<Element, ToolError> {
    // Try to find element with a short wait
    let mut attempts = 0;
    let max_attempts = 3;

    loop {
        match page.find_element(selector).await {
            Ok(element) => return Ok(element),
            Err(_) => {
                attempts += 1;
                if attempts >= max_attempts {
                    return Err(ToolError::ExecutionFailed(format!(
                        "Element not found: {}",
                        selector
                    )));
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
    }
}

/// Simple base64 encoding
fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_tool_name() {
        let tool = BrowserTool::new();
        assert_eq!(tool.name(), "browser");
    }

    #[test]
    fn test_browser_tool_has_parameters() {
        let tool = BrowserTool::new();
        assert!(tool.parameters().is_some());
    }

    #[test]
    fn test_browser_tool_default() {
        let tool = BrowserTool::default();
        assert_eq!(tool.name(), "browser");
    }

    #[test]
    fn test_browser_tool_with_headless() {
        let tool = BrowserTool::with_headless(false);
        assert!(!tool.headless);
    }

    #[tokio::test]
    async fn test_browser_tool_navigate_missing_url() {
        let tool = BrowserTool::new();
        let mut ctx = Context::new();
        let input = serde_json::json!({
            "action": "navigate"
        })
        .to_string();

        let result = tool.execute(&input, &mut ctx).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_browser_tool_click_missing_selector() {
        let tool = BrowserTool::new();
        let mut ctx = Context::new();
        let input = serde_json::json!({
            "action": "click"
        })
        .to_string();

        let result = tool.execute(&input, &mut ctx).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_browser_tool_invalid_action() {
        let tool = BrowserTool::new();
        let mut ctx = Context::new();
        let input = serde_json::json!({
            "action": "invalid"
        })
        .to_string();

        let result = tool.execute(&input, &mut ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_browser_tool_invalid_json() {
        let tool = BrowserTool::new();
        let mut ctx = Context::new();
        let input = "not valid json";

        let result = tool.execute(input, &mut ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_browser_tool_missing_action() {
        let tool = BrowserTool::new();
        let mut ctx = Context::new();
        let input = serde_json::json!({
            "url": "https://example.com"
        })
        .to_string();

        let result = tool.execute(&input, &mut ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_browser_tool_cleanup() {
        let tool = BrowserTool::new();
        let result = tool.cleanup().await;
        assert!(result.is_ok());
    }
}
