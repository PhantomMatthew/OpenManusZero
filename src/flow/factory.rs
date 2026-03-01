//! Flow Factory for creating different types of flows
//!
//! Provides a factory pattern for creating flow instances with support
//! for multiple agents and different flow types.

use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::Agent;
use crate::error::{Error, Result};
use crate::flow::{BaseFlow, PlanningFlow};
use crate::llm::LlmClient;

/// Flow type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlowType {
    /// Base flow - simple single agent execution
    Base,
    /// Planning flow - decomposes tasks and assigns to agents
    Planning,
}

impl std::fmt::Display for FlowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlowType::Base => write!(f, "base"),
            FlowType::Planning => write!(f, "planning"),
        }
    }
}

impl std::str::FromStr for FlowType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "base" | "default" => Ok(FlowType::Base),
            "planning" | "plan" => Ok(FlowType::Planning),
            _ => Err(Error::Flow(format!("Unknown flow type: {}", s))),
        }
    }
}

/// Enum to hold different flow types
pub enum FlowKind {
    /// Base flow
    Base(BaseFlow),
    /// Planning flow
    Planning(PlanningFlow),
}

impl std::fmt::Debug for FlowKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlowKind::Base(_) => f.debug_struct("FlowKind::Base").finish_non_exhaustive(),
            FlowKind::Planning(_) => f.debug_struct("FlowKind::Planning").finish_non_exhaustive(),
        }
    }
}

impl FlowKind {
    /// Execute the flow with given input
    pub async fn execute(&mut self, input: &str) -> Result<String> {
        match self {
            FlowKind::Base(flow) => flow.execute(input).await,
            FlowKind::Planning(flow) => flow.execute(input).await,
        }
    }

    /// Get all agent keys
    pub fn agent_keys(&self) -> Vec<&str> {
        match self {
            FlowKind::Base(flow) => flow.agent_keys(),
            FlowKind::Planning(flow) => flow.agent_keys(),
        }
    }

    /// Cleanup all agents
    pub async fn cleanup(&mut self) -> Result<()> {
        match self {
            FlowKind::Base(flow) => flow.cleanup().await,
            FlowKind::Planning(flow) => flow.cleanup().await,
        }
    }
}

/// Builder for creating flow configurations
pub struct FlowBuilder {
    /// Flow type
    flow_type: FlowType,
    /// Agents to include in the flow
    agents: HashMap<String, Box<dyn Agent>>,
    /// LLM client for planning flows
    llm: Option<Arc<dyn LlmClient>>,
    /// Maximum plan steps for planning flow
    max_plan_steps: usize,
    /// Primary agent key
    primary_agent_key: Option<String>,
}

impl FlowBuilder {
    /// Create a new flow builder
    pub fn new(flow_type: FlowType) -> Self {
        Self {
            flow_type,
            agents: HashMap::new(),
            llm: None,
            max_plan_steps: 10,
            primary_agent_key: None,
        }
    }

    /// Add an agent to the flow
    pub fn with_agent(mut self, key: impl Into<String>, agent: Box<dyn Agent>) -> Self {
        self.agents.insert(key.into(), agent);
        self
    }

    /// Add multiple agents to the flow
    pub fn with_agents(mut self, agents: HashMap<String, Box<dyn Agent>>) -> Self {
        self.agents.extend(agents);
        self
    }

    /// Set the LLM client (required for planning flow)
    pub fn with_llm(mut self, llm: Arc<dyn LlmClient>) -> Self {
        self.llm = Some(llm);
        self
    }

    /// Set maximum plan steps for planning flow
    pub fn with_max_plan_steps(mut self, steps: usize) -> Self {
        self.max_plan_steps = steps;
        self
    }

    /// Set the primary agent key
    pub fn with_primary(mut self, key: impl Into<String>) -> Self {
        self.primary_agent_key = Some(key.into());
        self
    }

    /// Build the flow
    pub fn build(self) -> Result<FlowKind> {
        if self.agents.is_empty() {
            return Err(Error::Flow("No agents provided".to_string()));
        }

        match self.flow_type {
            FlowType::Base => {
                let mut flow = BaseFlow::multiple(self.agents);
                if let Some(key) = self.primary_agent_key {
                    flow.set_primary(&key);
                }
                Ok(FlowKind::Base(flow))
            }
            FlowType::Planning => {
                let llm = self.llm.ok_or_else(|| {
                    Error::Flow("LLM client is required for planning flow".to_string())
                })?;
                let mut flow = PlanningFlow::new(self.agents, llm)
                    .with_max_plan_steps(self.max_plan_steps);
                if let Some(key) = self.primary_agent_key {
                    flow.set_primary(&key);
                }
                Ok(FlowKind::Planning(flow))
            }
        }
    }
}

/// Factory for creating different types of flows
pub struct FlowFactory;

impl FlowFactory {
    /// Create a base flow with a single agent
    pub fn base(agent: Box<dyn Agent>) -> BaseFlow {
        BaseFlow::single(agent)
    }

    /// Create a base flow with multiple agents
    pub fn base_multiple(agents: HashMap<String, Box<dyn Agent>>) -> BaseFlow {
        BaseFlow::multiple(agents)
    }

    /// Create a planning flow with agents and LLM
    pub fn planning(
        agents: HashMap<String, Box<dyn Agent>>,
        llm: Arc<dyn LlmClient>,
    ) -> PlanningFlow {
        PlanningFlow::new(agents, llm)
    }

    /// Create a planning flow with custom max steps
    pub fn planning_with_steps(
        agents: HashMap<String, Box<dyn Agent>>,
        llm: Arc<dyn LlmClient>,
        max_steps: usize,
    ) -> PlanningFlow {
        PlanningFlow::new(agents, llm).with_max_plan_steps(max_steps)
    }

    /// Create a flow using FlowKind enum
    pub fn create(
        flow_type: FlowType,
        agents: HashMap<String, Box<dyn Agent>>,
        llm: Option<Arc<dyn LlmClient>>,
    ) -> Result<FlowKind> {
        if agents.is_empty() {
            return Err(Error::Flow("No agents provided".to_string()));
        }

        match flow_type {
            FlowType::Base => Ok(FlowKind::Base(BaseFlow::multiple(agents))),
            FlowType::Planning => {
                let llm = llm.ok_or_else(|| {
                    Error::Flow("LLM client is required for planning flow".to_string())
                })?;
                Ok(FlowKind::Planning(PlanningFlow::new(agents, llm)))
            }
        }
    }

    /// Create a builder for more complex flow configurations
    pub fn builder(flow_type: FlowType) -> FlowBuilder {
        FlowBuilder::new(flow_type)
    }

    /// Get available flow types
    pub fn available_types() -> Vec<FlowType> {
        vec![FlowType::Base, FlowType::Planning]
    }

    /// Check if a flow type requires an LLM client
    pub fn requires_llm(flow_type: FlowType) -> bool {
        matches!(flow_type, FlowType::Planning)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Manus;
    use crate::llm::MockLlmClient;
    use std::str::FromStr;

    #[test]
    fn test_flow_type_display() {
        assert_eq!(FlowType::Base.to_string(), "base");
        assert_eq!(FlowType::Planning.to_string(), "planning");
    }

    #[test]
    fn test_flow_type_from_str() {
        assert_eq!(FlowType::from_str("base").unwrap(), FlowType::Base);
        assert_eq!(FlowType::from_str("planning").unwrap(), FlowType::Planning);
        assert_eq!(FlowType::from_str("PLAN").unwrap(), FlowType::Planning);
        assert!(FlowType::from_str("unknown").is_err());
    }

    #[test]
    fn test_flow_factory_available_types() {
        let types = FlowFactory::available_types();
        assert!(types.contains(&FlowType::Base));
        assert!(types.contains(&FlowType::Planning));
    }

    #[test]
    fn test_flow_factory_requires_llm() {
        assert!(!FlowFactory::requires_llm(FlowType::Base));
        assert!(FlowFactory::requires_llm(FlowType::Planning));
    }

    #[tokio::test]
    async fn test_flow_factory_base() {
        let manus = Manus::new().await.unwrap();
        let flow = FlowFactory::base(Box::new(manus));
        assert!(!flow.agent_keys().is_empty());
    }

    #[tokio::test]
    async fn test_flow_builder_base() {
        let manus = Manus::new().await.unwrap();
        let flow = FlowBuilder::new(FlowType::Base)
            .with_agent("main", Box::new(manus))
            .build()
            .unwrap();

        assert!(!flow.agent_keys().is_empty());
    }

    #[test]
    fn test_flow_builder_no_agents_error() {
        let result = FlowBuilder::new(FlowType::Base).build();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_flow_builder_planning_no_llm_error() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let manus = Manus::with_llm(llm);

        let result = FlowBuilder::new(FlowType::Planning)
            .with_agent("main", Box::new(manus))
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("LLM client"));
    }

    #[test]
    fn test_flow_factory_create_empty_error() {
        let agents = HashMap::new();
        let result = FlowFactory::create(FlowType::Base, agents, None);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_flow_kind_base() {
        let manus = Manus::new().await.unwrap();
        let flow = FlowFactory::base(Box::new(manus));
        let mut flow_kind = FlowKind::Base(flow);
        assert!(!flow_kind.agent_keys().is_empty());
    }

    #[tokio::test]
    async fn test_flow_kind_planning() {
        let llm = Arc::new(MockLlmClient::new("gpt-4"));
        let mut agents: HashMap<String, Box<dyn Agent>> = HashMap::new();
        let manus = Manus::with_llm(llm.clone());
        agents.insert("main".to_string(), Box::new(manus));

        let flow = PlanningFlow::new(agents, llm);
        let flow_kind = FlowKind::Planning(flow);
        assert!(!flow_kind.agent_keys().is_empty());
    }
}
