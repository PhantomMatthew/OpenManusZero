//! Planning flow implementation

use crate::agent::Agent;
use crate::error::{Error, Result};
use crate::flow::BaseFlow;
use crate::llm::{LlmClient, LlmRequest};
use crate::schema::Message;
use std::collections::HashMap;
use std::sync::Arc;

/// Planning flow that decomposes tasks and assigns them to agents
pub struct PlanningFlow {
    base: BaseFlow,
    llm: Arc<dyn LlmClient>,
    max_plan_steps: usize,
}

impl PlanningFlow {
    /// Create a new planning flow
    pub fn new(agents: HashMap<String, Box<dyn Agent>>, llm: Arc<dyn LlmClient>) -> Self {
        Self {
            base: BaseFlow::multiple(agents),
            llm,
            max_plan_steps: 10,
        }
    }

    /// Set maximum plan steps
    pub fn with_max_plan_steps(mut self, steps: usize) -> Self {
        self.max_plan_steps = steps;
        self
    }

    /// Set the primary agent key
    pub fn set_primary(&mut self, key: &str) -> bool {
        self.base.set_primary(key)
    }

    /// Get all agent keys
    pub fn agent_keys(&self) -> Vec<&str> {
        self.base.agent_keys()
    }

    /// Create a plan for the given task
    async fn create_plan(&self, task: &str) -> Result<Vec<PlanStep>> {
        let prompt = format!(
            r#"Given the following task, create a step-by-step plan to accomplish it.
Break down the task into specific, actionable steps.

Task: {}

Output the plan as a JSON array of objects with 'step', 'description', and 'agent' fields.
The 'agent' field should be one of: {:?}"#,
            task,
            self.base.agent_keys()
        );

        let request = LlmRequest::new(self.llm.model(), vec![Message::user(prompt)])
            .with_max_tokens(1000)
            .with_temperature(0.0);

        let response = self.llm.completion(&request).await.map_err(Error::Llm)?;
        let content = response.content().unwrap_or("");

        // Parse the plan
        self.parse_plan(content)
    }

    /// Parse plan from LLM response
    fn parse_plan(&self, content: &str) -> Result<Vec<PlanStep>> {
        // Try to extract JSON from the response
        let json_start = content.find('[').unwrap_or(0);
        let json_end = content.rfind(']').map(|i| i + 1).unwrap_or(content.len());
        let json_str = &content[json_start..json_end];

        // Parse JSON
        let steps: Vec<serde_json::Value> =
            serde_json::from_str(json_str).unwrap_or_else(|_| vec![]);

        let plan: Vec<PlanStep> = steps
            .iter()
            .enumerate()
            .map(|(i, v)| PlanStep {
                step: i + 1,
                description: v
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("Unknown step")
                    .to_string(),
                agent: v
                    .get("agent")
                    .and_then(|a| a.as_str())
                    .unwrap_or("default")
                    .to_string(),
                result: None,
            })
            .collect();

        if plan.is_empty() {
            // Create a default single-step plan
            Ok(vec![PlanStep {
                step: 1,
                description: "Execute the task".to_string(),
                agent: self
                    .base
                    .agent_keys()
                    .first()
                    .unwrap_or(&"default")
                    .to_string(),
                result: None,
            }])
        } else {
            Ok(plan)
        }
    }

    /// Execute the planning flow
    #[allow(clippy::await_holding_lock)]
    pub async fn execute(&mut self, task: &str) -> Result<String> {
        tracing::info!("Creating plan for task: {}", task);

        // Create a plan
        let mut plan = self.create_plan(task).await?;
        tracing::info!("Plan created with {} steps", plan.len());

        // Execute each step
        let mut results = Vec::new();
        for step in &mut plan {
            tracing::info!("Executing step {}: {}", step.step, step.description);

            // Get the agent for this step
            let agent_key = if self.base.agent_keys().contains(&step.agent.as_str()) {
                step.agent.clone()
            } else {
                self.base
                    .agent_keys()
                    .first()
                    .unwrap_or(&"default")
                    .to_string()
            };

            // Execute with the agent
            if let Some(agent) = self.base.get_agent(&agent_key) {
                let mut agent = agent.write();
                let result = agent.run(&step.description).await?;
                step.result = Some(result.clone());
                results.push(format!("Step {}: {}", step.step, result));
            } else {
                results.push(format!("Step {}: Agent {} not found", step.step, agent_key));
            }

            // Check if we've exceeded max steps
            if step.step >= self.max_plan_steps {
                results.push(format!(
                    "Reached maximum plan steps ({})",
                    self.max_plan_steps
                ));
                break;
            }
        }

        Ok(results.join("\n"))
    }

    /// Cleanup all agents
    pub async fn cleanup(&mut self) -> Result<()> {
        self.base.cleanup().await
    }
}

/// A step in the execution plan
#[derive(Debug, Clone)]
pub struct PlanStep {
    /// Step number
    pub step: usize,
    /// Description of the step
    pub description: String,
    /// Agent assigned to this step
    pub agent: String,
    /// Result of executing this step
    pub result: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLlmClient;

    #[tokio::test]
    async fn test_planning_flow_create() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let agents = HashMap::new();
        let flow = PlanningFlow::new(agents, llm);
        assert_eq!(flow.max_plan_steps, 10);
    }

    #[tokio::test]
    async fn test_planning_flow_parse_plan() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let agents = HashMap::new();
        let flow = PlanningFlow::new(agents, llm);

        let content = r#"[{"step": 1, "description": "Do task 1", "agent": "default"}]"#;
        let plan = flow.parse_plan(content).unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].description, "Do task 1");
    }

    #[tokio::test]
    async fn test_planning_flow_parse_plan_empty() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let agents = HashMap::new();
        let flow = PlanningFlow::new(agents, llm);

        let content = "No plan here";
        let plan = flow.parse_plan(content).unwrap();
        // Should create a default plan
        assert!(!plan.is_empty());
    }

    #[test]
    fn test_plan_step() {
        let step = PlanStep {
            step: 1,
            description: "Test step".to_string(),
            agent: "default".to_string(),
            result: None,
        };
        assert_eq!(step.step, 1);
        assert!(step.result.is_none());
    }
}
