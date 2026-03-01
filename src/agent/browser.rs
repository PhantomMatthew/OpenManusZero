//! Browser agent implementation for web automation
//!
//! This agent specializes in browser automation tasks, including:
//! - Web page navigation
//! - Form filling and interaction
//! - Content extraction
//! - Screenshot capture

use crate::agent::toolcall::ToolCallAgent;
use crate::error::{Error, Result};
use crate::llm::LlmClient;
use crate::schema::{AgentState, Message};
use crate::tool::{Tool, ToolCollection};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

/// System prompt for the browser agent
const SYSTEM_PROMPT: &str = r#"You are an AI agent designed to automate browser tasks. Your goal is to accomplish the ultimate task following the rules.

# Input Format
Task
Previous steps
Current URL
Open Tabs
Interactive Elements
[index]<type>text</type>
- index: Numeric identifier for interaction
- type: HTML element type (button, input, etc.)
- text: Element description
Example:
[33]<button>Submit Form</button>

- Only elements with numeric indexes in [] are interactive
- elements without [] provide only context

# Response Rules
1. RESPONSE FORMAT: You must ALWAYS respond with valid JSON in this exact format:
{"current_state": {"evaluation_previous_goal": "Success|Failed|Unknown - Analyze the current elements and the image to check if the previous goals/actions are successful like intended by the task. Mention if something unexpected happened. Shortly state why/why not",
"memory": "Description of what has been done and what you need to remember. Be very specific. Count here ALWAYS how many times you have done something and how many remain. E.g. 0 out of 10 websites analyzed. Continue with abc and xyz",
"next_goal": "What needs to be done with the next immediate action"},
"action":[{"one_action_name": {// action-specific parameter}}, // ... more actions in sequence]}

2. ACTIONS: You can specify multiple actions in the list to be executed in sequence. But always specify only one action name per item. Use maximum 5 actions per sequence.
Common action sequences:
- Form filling: [{"input_text": {"index": 1, "text": "username"}}, {"input_text": {"index": 2, "text": "password"}}, {"click_element": {"index": 3}}]
- Navigation and extraction: [{"go_to_url": {"url": "https://example.com"}}, {"extract_content": {"goal": "extract the names"}}]

3. ELEMENT INTERACTION:
- Only use indexes of the interactive elements
- Elements marked with "[]Non-interactive text" are non-interactive

4. NAVIGATION & ERROR HANDLING:
- If no suitable elements exist, use other functions to complete the task
- If stuck, try alternative approaches - like going back to a previous page, new search, new tab etc.
- Handle popups/cookies by accepting or closing them
- Use scroll to find elements you are looking for
- If captcha pops up, try to solve it - else try a different approach

5. TASK COMPLETION:
- Use the done action as the last action as soon as the ultimate task is complete
- Dont use "done" before you are done with everything the user asked you
- If you have to do something repeatedly, count inside "memory" how many times you have done it and how many remain.

6. VISUAL CONTEXT:
- When an image is provided, use it to understand the page layout
- Bounding boxes with labels on their top right corner correspond to element indexes

Your responses must be always JSON with the specified format."#;

/// Template for next step prompt
const NEXT_STEP_PROMPT_TEMPLATE: &str = r#"What should I do next to achieve my goal?

When you see [Current state starts here], focus on the following:
- Current URL and page title{url_info}
- Available tabs{tabs_info}
- Interactive elements and their indices
- Content above{content_above} or below{content_below} the viewport (if indicated)
- Any action results or errors{results_info}

For browser interactions:
- To navigate: browser with action="navigate", url="..."
- To click: browser with action="click", selector="..."
- To type: browser with action="type", selector="...", text="..."
- To extract: browser with action="extract", selector="..." (optional)
- To screenshot: browser with action="screenshot"

Consider both what's visible and what might be beyond the current viewport.
Be methodical - remember your progress and what you've learned so far.

If you want to stop the interaction at any point, use the `terminate` tool/function call."#;

/// Browser state information
#[derive(Debug, Clone, Default)]
pub struct BrowserState {
    /// Current page URL
    pub url: Option<String>,
    /// Current page title
    pub title: Option<String>,
    /// Number of open tabs
    pub tabs_count: usize,
    /// Pixels above current viewport
    pub pixels_above: u32,
    /// Pixels below current viewport
    pub pixels_below: u32,
    /// Current screenshot as base64
    pub screenshot: Option<String>,
}

/// Helper for managing browser context
pub struct BrowserContextHelper {
    /// Current browser state
    state: Arc<RwLock<BrowserState>>,
    /// Pending screenshot to be added to memory
    pending_screenshot: Arc<RwLock<Option<String>>>,
}

impl BrowserContextHelper {
    /// Create a new browser context helper
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(BrowserState::default())),
            pending_screenshot: Arc::new(RwLock::new(None)),
        }
    }

    /// Update browser state
    pub async fn update_state(&self, new_state: BrowserState) {
        let mut state = self.state.write().await;
        *state = new_state;
    }

    /// Get current browser state
    pub async fn get_state(&self) -> BrowserState {
        self.state.read().await.clone()
    }

    /// Set pending screenshot
    pub async fn set_pending_screenshot(&self, screenshot: String) {
        let mut pending = self.pending_screenshot.write().await;
        *pending = Some(screenshot);
    }

    /// Take and clear pending screenshot
    pub async fn take_pending_screenshot(&self) -> Option<String> {
        let mut pending = self.pending_screenshot.write().await;
        pending.take()
    }

    /// Format the next step prompt with current browser state
    pub async fn format_next_step_prompt(&self) -> String {
        let state = self.get_state().await;

        let url_info = match (&state.url, &state.title) {
            (Some(url), Some(title)) => format!("\n   URL: {}\n   Title: {}", url, title),
            (Some(url), None) => format!("\n   URL: {}", url),
            (None, Some(title)) => format!("\n   Title: {}", title),
            (None, None) => String::new(),
        };

        let tabs_info = if state.tabs_count > 0 {
            format!("\n   {} tab(s) available", state.tabs_count)
        } else {
            String::new()
        };

        let content_above = if state.pixels_above > 0 {
            format!(" ({} pixels)", state.pixels_above)
        } else {
            String::new()
        };

        let content_below = if state.pixels_below > 0 {
            format!(" ({} pixels)", state.pixels_below)
        } else {
            String::new()
        };

        let results_info = String::new(); // Can be extended to include action results

        NEXT_STEP_PROMPT_TEMPLATE
            .replace("{url_info}", &url_info)
            .replace("{tabs_info}", &tabs_info)
            .replace("{content_above}", &content_above)
            .replace("{content_below}", &content_below)
            .replace("{results_info}", &results_info)
    }
}

impl Default for BrowserContextHelper {
    fn default() -> Self {
        Self::new()
    }
}

/// Browser agent for web automation
pub struct BrowserAgent {
    /// Underlying tool call agent
    agent: ToolCallAgent,
    /// Browser context helper
    context_helper: Arc<BrowserContextHelper>,
}

impl BrowserAgent {
    /// Create a new browser agent with default configuration
    pub fn new(llm: Arc<dyn LlmClient>) -> Self {
        let tools = Self::create_default_tools();
        let context_helper = Arc::new(BrowserContextHelper::new());

        let agent = ToolCallAgent::new(
            "browser",
            "A browser agent that can control a browser to accomplish tasks",
            llm,
            tools,
        )
        .with_system_prompt(SYSTEM_PROMPT)
        .with_next_step_prompt(NEXT_STEP_PROMPT_TEMPLATE)
        .with_max_steps(20)
        .with_max_observe(10000)
        .with_special_tool_names(vec!["terminate".to_string()]);

        Self {
            agent,
            context_helper,
        }
    }

    /// Create browser agent with headless mode setting
    pub fn with_headless(llm: Arc<dyn LlmClient>, headless: bool) -> Self {
        let tools = Self::create_tools_with_headless(headless);
        let context_helper = Arc::new(BrowserContextHelper::new());

        let agent = ToolCallAgent::new(
            "browser",
            "A browser agent that can control a browser to accomplish tasks",
            llm,
            tools,
        )
        .with_system_prompt(SYSTEM_PROMPT)
        .with_next_step_prompt(NEXT_STEP_PROMPT_TEMPLATE)
        .with_max_steps(20)
        .with_max_observe(10000)
        .with_special_tool_names(vec!["terminate".to_string()]);

        Self {
            agent,
            context_helper,
        }
    }

    /// Create default tool collection
    fn create_default_tools() -> Arc<ToolCollection> {
        Self::create_tools_with_headless(true)
    }

    /// Create tools with headless setting
    fn create_tools_with_headless(_headless: bool) -> Arc<ToolCollection> {
        let mut collection = ToolCollection::new();

        // Add terminate tool
        collection.add_tool(Arc::new(crate::tool::terminate::TerminateTool::new()) as Arc<dyn Tool>);

        // Add browser tool if feature is enabled
        #[cfg(feature = "browser")]
        {
            collection.add_tool(Arc::new(crate::tool::browser::BrowserTool::new()) as Arc<dyn Tool>);
        }

        Arc::new(collection)
    }

    /// Get the browser context helper
    pub fn context_helper(&self) -> &BrowserContextHelper {
        &self.context_helper
    }

    /// Think step - decide next actions with browser state context
    pub async fn think(&mut self) -> Result<bool> {
        // Format next step prompt with current browser state
        let next_step = self.context_helper.format_next_step_prompt().await;

        // Update the agent's next step prompt
        self.agent.set_next_step_prompt(&next_step);

        // Check for pending screenshot
        if let Some(screenshot) = self.context_helper.take_pending_screenshot().await {
            // Add screenshot message to memory
            let msg = Message::user_with_image("Current browser screenshot:", screenshot);
            self.agent.base_mut().update_memory(msg);
        }

        // Call parent think
        self.agent.think().await
    }

    /// Act step - execute tool calls
    pub async fn act(&mut self) -> Result<String> {
        self.agent.act().await
    }

    /// Get reference to underlying agent
    pub fn inner(&self) -> &ToolCallAgent {
        &self.agent
    }

    /// Get mutable reference to underlying agent
    pub fn inner_mut(&mut self) -> &mut ToolCallAgent {
        &mut self.agent
    }
}

#[async_trait]
impl super::Agent for BrowserAgent {
    fn name(&self) -> &str {
        self.agent.name()
    }

    fn description(&self) -> &str {
        self.agent.description()
    }

    fn state(&self) -> AgentState {
        self.agent.state()
    }

    async fn run(&mut self, request: &str) -> Result<String> {
        if self.agent.state() != AgentState::Idle {
            return Err(Error::Agent(format!(
                "Cannot run agent from state: {:?}",
                self.agent.state()
            )));
        }

        self.agent.base_mut().update_memory(Message::user(request));
        self.agent.base_mut().set_state(AgentState::Running);
        let mut results = Vec::new();

        while self.agent.base().current_step() < self.agent.base().max_steps()
            && self.agent.state() != AgentState::Finished
        {
            self.agent.base_mut().increment_step();
            let step = self.agent.base().current_step();
            let max = self.agent.base().max_steps();
            tracing::info!("Browser agent executing step {}/{}", step, max);

            // Think with browser context
            let thinking = self.think().await?;
            if !thinking {
                tracing::info!("Browser agent finished thinking");
                break;
            }

            // Act
            let action_result = self.act().await?;
            results.push(format!("Step {}: {}", step, action_result));

            // Check for stuck state
            if self.agent.base().is_stuck() {
                self.agent.base_mut().handle_stuck_state();
            }
        }

        if self.agent.base().current_step() >= self.agent.base().max_steps() {
            results.push(format!(
                "Terminated: Reached max steps ({})",
                self.agent.base().max_steps()
            ));
        }

        self.agent.base_mut().reset_steps();
        self.agent.base_mut().set_state(AgentState::Idle);
        Ok(results.join("\n"))
    }

    async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Browser agent cleaning up");

        // Clear pending screenshot
        let mut pending = self.context_helper.pending_screenshot.write().await;
        *pending = None;

        // Clear browser state
        let mut state = self.context_helper.state.write().await;
        *state = BrowserState::default();

        // Cleanup agent
        self.agent.cleanup().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::llm::MockLlmClient;

    fn create_test_agent() -> BrowserAgent {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        BrowserAgent::new(llm)
    }

    #[test]
    fn test_browser_agent_new() {
        let agent = create_test_agent();
        assert_eq!(agent.name(), "browser");
        assert_eq!(
            agent.description(),
            "A browser agent that can control a browser to accomplish tasks"
        );
        assert_eq!(agent.state(), AgentState::Idle);
    }

    #[test]
    fn test_browser_agent_has_tools() {
        let agent = create_test_agent();
        assert!(!agent.inner().tools().is_empty());
        assert!(agent.inner().tools().has_tool("terminate"));
    }

    #[test]
    fn test_browser_context_helper_new() {
        let helper = BrowserContextHelper::new();
        let state = futures::executor::block_on(helper.get_state());
        assert!(state.url.is_none());
        assert!(state.title.is_none());
    }

    #[tokio::test]
    async fn test_browser_context_helper_update_state() {
        let helper = BrowserContextHelper::new();

        let new_state = BrowserState {
            url: Some("https://example.com".to_string()),
            title: Some("Example".to_string()),
            tabs_count: 2,
            pixels_above: 100,
            pixels_below: 200,
            screenshot: None,
        };

        helper.update_state(new_state).await;

        let state = helper.get_state().await;
        assert_eq!(state.url, Some("https://example.com".to_string()));
        assert_eq!(state.title, Some("Example".to_string()));
        assert_eq!(state.tabs_count, 2);
    }

    #[tokio::test]
    async fn test_browser_context_helper_format_prompt() {
        let helper = BrowserContextHelper::new();

        let state = BrowserState {
            url: Some("https://example.com".to_string()),
            title: Some("Example Domain".to_string()),
            tabs_count: 3,
            pixels_above: 50,
            pixels_below: 100,
            screenshot: None,
        };

        helper.update_state(state).await;

        let prompt = helper.format_next_step_prompt().await;

        assert!(prompt.contains("https://example.com"));
        assert!(prompt.contains("Example Domain"));
        assert!(prompt.contains("3 tab"));
        assert!(prompt.contains("50 pixels"));
        assert!(prompt.contains("100 pixels"));
    }

    #[tokio::test]
    async fn test_browser_context_helper_screenshot() {
        let helper = BrowserContextHelper::new();

        helper.set_pending_screenshot("base64data".to_string()).await;

        let screenshot = helper.take_pending_screenshot().await;
        assert_eq!(screenshot, Some("base64data".to_string()));

        // Should be cleared after take
        let screenshot2 = helper.take_pending_screenshot().await;
        assert!(screenshot2.is_none());
    }

    #[tokio::test]
    async fn test_browser_agent_cleanup() {
        let mut agent = create_test_agent();

        // Set some state
        agent
            .context_helper
            .update_state(BrowserState {
                url: Some("https://test.com".to_string()),
                title: Some("Test".to_string()),
                tabs_count: 1,
                pixels_above: 0,
                pixels_below: 0,
                screenshot: None,
            })
            .await;

        agent.cleanup().await.unwrap();

        let state = agent.context_helper.get_state().await;
        assert!(state.url.is_none());
    }

    #[tokio::test]
    async fn test_browser_agent_with_headless() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let agent = BrowserAgent::with_headless(llm, false);

        assert_eq!(agent.name(), "browser");
        assert_eq!(agent.state(), AgentState::Idle);
    }
}
