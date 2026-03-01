//! A2A Agent implementation that wraps OpenManus agents
//!
//! Provides integration between A2A protocol and the agent system

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::server::A2AAgent;
use super::types::*;
use crate::agent::Agent;
use crate::error::Result;
use crate::llm::LlmClient;

/// A2A wrapper for Manus agent
pub struct A2AManus {
    /// Agent card
    card: A2ACard,
    /// LLM client factory
    llm: Arc<dyn LlmClient>,
    /// Active tasks
    tasks: RwLock<HashMap<String, A2ATask>>,
    /// Maximum steps for agent execution
    max_steps: usize,
}

impl A2AManus {
    /// Create a new A2A Manus agent
    pub fn new(llm: Arc<dyn LlmClient>) -> Self {
        Self::with_config(llm, "Manus Agent", "http://localhost:10000", 20)
    }

    /// Create with custom configuration
    pub fn with_config(
        llm: Arc<dyn LlmClient>,
        name: &str,
        url: &str,
        max_steps: usize,
    ) -> Self {
        let card = A2ACard::new(name, "A versatile agent that can solve various tasks using multiple tools", url)
            .with_capabilities(A2ACapabilities {
                streaming: false,
                push_notifications: true,
            })
            .with_skills(vec![
                A2ASkill::new(
                    "python_execute",
                    "Python Execute Tool",
                    "Executes Python code string. Note: Only print outputs are visible, function return values are not captured. Use print statements to see results."
                )
                .with_tags(vec!["Execute Python Code".to_string()])
                .with_examples(vec!["Execute Python code: print('Hello World')".to_string()]),
                A2ASkill::new(
                    "bash",
                    "Bash Tool",
                    "Execute bash commands in a safe environment"
                )
                .with_tags(vec!["Execute Shell Commands".to_string()])
                .with_examples(vec!["Execute bash command: ls -la".to_string()]),
                A2ASkill::new(
                    "str_replace_editor",
                    "String Replace Editor",
                    "Custom editing tool for viewing, creating and editing files"
                )
                .with_tags(vec!["Operate Files".to_string()])
                .with_examples(vec!["Replace 'old' with 'new' in 'file.txt'".to_string()]),
                A2ASkill::new(
                    "ask_human",
                    "Ask Human Tool",
                    "Use this tool to ask human for help"
                )
                .with_tags(vec!["Ask human for help".to_string()])
                .with_examples(vec!["Ask human: 'What time is it?'".to_string()]),
                A2ASkill::new(
                    "terminate",
                    "Terminate Tool",
                    "Terminate the interaction when the request is met"
                )
                .with_tags(vec!["terminate task".to_string()])
                .with_examples(vec!["terminate".to_string()]),
            ]);

        Self {
            card,
            llm,
            tasks: RwLock::new(HashMap::new()),
            max_steps,
        }
    }

    /// Set agent card
    pub fn with_card(mut self, card: A2ACard) -> Self {
        self.card = card;
        self
    }

    /// Set maximum steps
    pub fn with_max_steps(mut self, steps: usize) -> Self {
        self.max_steps = steps;
        self
    }
}

#[async_trait]
impl A2AAgent for A2AManus {
    fn card(&self) -> A2ACard {
        self.card.clone()
    }

    async fn invoke(&self, query: &str, context_id: &str) -> Result<A2ATask> {
        use crate::agent::Manus;

        // Create a new Manus agent with the LLM
        let mut manus = Manus::with_llm(self.llm.clone());

        // Run the agent
        let result = manus.run(query).await?;

        // Create task with result
        let task = A2ATask::generate()
            .with_context(context_id)
            .with_history(A2AMessage::user(query))
            .complete(&result);

        // Store task
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task.id.clone(), task.clone());
        }

        Ok(task)
    }

    async fn get_task(&self, task_id: &str) -> Option<A2ATask> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    async fn cancel_task(&self, task_id: &str) -> Option<A2ATask> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.status = A2ATaskStatus::new(A2ATaskState::Canceled);
            Some(task.clone())
        } else {
            None
        }
    }

    fn supported_content_types(&self) -> Vec<String> {
        vec!["text".to_string(), "text/plain".to_string()]
    }
}

/// A2A wrapper for Browser Agent
pub struct A2ABrowser {
    /// Agent card
    card: A2ACard,
    /// LLM client
    llm: Arc<dyn LlmClient>,
    /// Active tasks
    tasks: RwLock<HashMap<String, A2ATask>>,
    /// Maximum steps
    max_steps: usize,
}

impl A2ABrowser {
    /// Create a new A2A Browser agent
    pub fn new(llm: Arc<dyn LlmClient>) -> Self {
        let card = A2ACard::new(
            "Browser Agent",
            "An agent that can browse the web and interact with web pages",
            "http://localhost:10001",
        )
        .with_capabilities(A2ACapabilities {
            streaming: false,
            push_notifications: true,
        })
        .with_skills(vec![
            A2ASkill::new(
                "browser",
                "Browser Tool",
                "A powerful browser automation tool for web interaction"
            )
            .with_tags(vec!["browser automation".to_string(), "web browsing".to_string()])
            .with_examples(vec!["Go to https://example.com".to_string()]),
        ]);

        Self {
            card,
            llm,
            tasks: RwLock::new(HashMap::new()),
            max_steps: 20,
        }
    }
}

#[async_trait]
impl A2AAgent for A2ABrowser {
    fn card(&self) -> A2ACard {
        self.card.clone()
    }

    async fn invoke(&self, query: &str, context_id: &str) -> Result<A2ATask> {
        use crate::agent::BrowserAgent;

        let mut agent = BrowserAgent::new(self.llm.clone());

        let result = agent.run(query).await?;

        let task = A2ATask::generate()
            .with_context(context_id)
            .with_history(A2AMessage::user(query))
            .complete(&result);

        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task.id.clone(), task.clone());
        }

        Ok(task)
    }

    async fn get_task(&self, task_id: &str) -> Option<A2ATask> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    async fn cancel_task(&self, task_id: &str) -> Option<A2ATask> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.status = A2ATaskStatus::new(A2ATaskState::Canceled);
            Some(task.clone())
        } else {
            None
        }
    }

    fn supported_content_types(&self) -> Vec<String> {
        vec!["text".to_string(), "text/plain".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLlmClient;

    #[test]
    fn test_a2a_manus_new() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let agent = A2AManus::new(llm);

        let card = agent.card();
        assert_eq!(card.name, "Manus Agent");
        assert!(!card.skills.is_empty());
    }

    #[test]
    fn test_a2a_manus_card() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let agent = A2AManus::new(llm);

        let card = agent.card();
        assert!(card.skills.iter().any(|s| s.id == "python_execute"));
        assert!(card.skills.iter().any(|s| s.id == "bash"));
    }

    #[test]
    fn test_a2a_browser_new() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let agent = A2ABrowser::new(llm);

        let card = agent.card();
        assert_eq!(card.name, "Browser Agent");
    }

    #[test]
    fn test_a2a_manus_supported_types() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let agent = A2AManus::new(llm);

        let types = agent.supported_content_types();
        assert!(types.contains(&"text".to_string()));
    }
}
