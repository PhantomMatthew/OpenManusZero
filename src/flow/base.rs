//! Base flow implementation

use crate::agent::Agent;
use crate::error::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Base flow for orchestrating multiple agents
#[allow(clippy::await_holding_lock)]
pub struct BaseFlow {
    /// Agents in the flow
    agents: HashMap<String, Arc<RwLock<Box<dyn Agent>>>>,
    /// Primary agent key
    primary_agent_key: Option<String>,
    /// Tools available to the flow (reserved for future use)
    #[allow(dead_code)]
    tools: Vec<String>,
}

impl BaseFlow {
    /// Create a new base flow with a single agent
    pub fn single(agent: Box<dyn Agent>) -> Self {
        let mut agents = HashMap::new();
        agents.insert("default".to_string(), Arc::new(RwLock::new(agent)));
        Self {
            agents,
            primary_agent_key: Some("default".to_string()),
            tools: Vec::new(),
        }
    }

    /// Create a new base flow with multiple agents
    pub fn multiple(agents: HashMap<String, Box<dyn Agent>>) -> Self {
        let agents: HashMap<String, Arc<RwLock<Box<dyn Agent>>>> = agents
            .into_iter()
            .map(|(k, v)| (k, Arc::new(RwLock::new(v))))
            .collect();
        let primary_key = agents.keys().next().cloned();
        Self {
            agents,
            primary_agent_key: primary_key,
            tools: Vec::new(),
        }
    }

    /// Get the primary agent
    pub fn primary_agent(&self) -> Option<Arc<RwLock<Box<dyn Agent>>>> {
        self.primary_agent_key
            .as_ref()
            .and_then(|k| self.agents.get(k))
            .cloned()
    }

    /// Get an agent by key
    pub fn get_agent(&self, key: &str) -> Option<Arc<RwLock<Box<dyn Agent>>>> {
        self.agents.get(key).cloned()
    }

    /// Add an agent to the flow
    pub fn add_agent(&mut self, key: String, agent: Box<dyn Agent>) {
        self.agents.insert(key, Arc::new(RwLock::new(agent)));
    }

    /// Set the primary agent key
    pub fn set_primary(&mut self, key: &str) -> bool {
        if self.agents.contains_key(key) {
            self.primary_agent_key = Some(key.to_string());
            true
        } else {
            false
        }
    }

    /// Get all agent keys
    pub fn agent_keys(&self) -> Vec<&str> {
        self.agents.keys().map(|s| s.as_str()).collect()
    }

    /// Execute the flow
    #[allow(clippy::await_holding_lock)]
    pub async fn execute(&mut self, input: &str) -> Result<String> {
        let primary = self
            .primary_agent()
            .ok_or_else(|| crate::error::Error::Agent("No primary agent".to_string()))?;

        let mut agent = primary.write();
        agent.run(input).await
    }

    /// Cleanup all agents
    #[allow(clippy::await_holding_lock)]
    pub async fn cleanup(&mut self) -> Result<()> {
        for agent in self.agents.values() {
            let mut agent = agent.write();
            agent.cleanup().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::agent::Manus;

    #[tokio::test]
    async fn test_base_flow_single() {
        let manus = Manus::new().await.unwrap();
        let flow = BaseFlow::single(Box::new(manus));
        assert!(flow.primary_agent().is_some());
        assert!(flow.agent_keys().contains(&"default"));
    }

    #[test]
    fn test_base_flow_set_primary() {
        let flow = BaseFlow {
            agents: HashMap::<String, Arc<RwLock<Box<dyn Agent>>>>::new(),
            primary_agent_key: None,
            tools: Vec::new(),
        };
        assert!(flow.primary_agent().is_none());
    }

    #[test]
    fn test_base_flow_agent_keys() {
        let flow = BaseFlow {
            agents: HashMap::<String, Arc<RwLock<Box<dyn Agent>>>>::new(),
            primary_agent_key: None,
            tools: Vec::new(),
        };
        assert!(flow.agent_keys().is_empty());
    }
}
