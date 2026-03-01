//! Prompt library for managing all agent prompts

use crate::prompt::{PromptSet, PromptTemplate};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Library of prompt templates organized by agent type
pub struct PromptLibrary {
    /// Prompt sets by agent type name
    prompts: HashMap<String, PromptSet>,
}

impl PromptLibrary {
    /// Create a new empty prompt library
    pub fn new() -> Self {
        Self {
            prompts: HashMap::new(),
        }
    }

    /// Create a prompt library with default prompts for all agents
    pub fn with_defaults() -> Self {
        let mut library = Self::new();

        // Register default prompts
        library.register(crate::prompt::manus::create_manus_prompts());
        library.register(crate::prompt::browser::create_browser_prompts());
        library.register(crate::prompt::swe::create_swe_prompts());
        library.register(crate::prompt::planning::create_planning_prompts());
        library.register(crate::prompt::mcp::create_mcp_prompts());
        library.register(crate::prompt::data_analysis::create_data_analysis_prompts());

        library
    }

    /// Register a prompt set
    pub fn register(&mut self, prompt_set: PromptSet) {
        self.prompts.insert(prompt_set.name.clone(), prompt_set);
    }

    /// Get a prompt set by agent name
    pub fn get(&self, agent_name: &str) -> Option<&PromptSet> {
        self.prompts.get(agent_name)
    }

    /// Check if a prompt set exists
    pub fn has(&self, agent_name: &str) -> bool {
        self.prompts.contains_key(agent_name)
    }

    /// Get system prompt for an agent
    pub fn get_system_prompt(&self, agent_name: &str) -> Option<&PromptTemplate> {
        self.prompts.get(agent_name).and_then(|p| p.system_prompt.as_ref())
    }

    /// Get next step prompt for an agent
    pub fn get_next_step_prompt(&self, agent_name: &str) -> Option<&PromptTemplate> {
        self.prompts.get(agent_name).and_then(|p| p.next_step_prompt.as_ref())
    }

    /// Render system prompt with variables
    pub fn render_system(&self, agent_name: &str, variables: &HashMap<String, String>) -> Option<String> {
        self.get(agent_name).and_then(|p| p.render_system(variables))
    }

    /// Render next step prompt with variables
    pub fn render_next_step(&self, agent_name: &str, variables: &HashMap<String, String>) -> Option<String> {
        self.get(agent_name).and_then(|p| p.render_next_step(variables))
    }

    /// List all registered agent names
    pub fn agent_names(&self) -> Vec<&String> {
        self.prompts.keys().collect()
    }

    /// Get the number of registered prompt sets
    pub fn len(&self) -> usize {
        self.prompts.len()
    }

    /// Check if the library is empty
    pub fn is_empty(&self) -> bool {
        self.prompts.is_empty()
    }
}

impl Default for PromptLibrary {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Global prompt library instance (thread-safe lazy initialization)
static GLOBAL_LIBRARY: OnceLock<PromptLibrary> = OnceLock::new();

/// Get the global prompt library (initialized with defaults on first call)
pub fn global_library() -> &'static PromptLibrary {
    GLOBAL_LIBRARY.get_or_init(PromptLibrary::with_defaults)
}

/// Convenience function to get system prompt
pub fn get_system(agent_name: &str) -> Option<&'static str> {
    global_library()
        .get(agent_name)
        .and_then(|p| p.system())
}

/// Convenience function to get next step prompt
pub fn get_next_step(agent_name: &str) -> Option<&'static str> {
    global_library()
        .get(agent_name)
        .and_then(|p| p.next_step())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_new() {
        let library = PromptLibrary::new();
        assert!(library.is_empty());
    }

    #[test]
    fn test_library_with_defaults() {
        let library = PromptLibrary::with_defaults();
        assert!(!library.is_empty());
        assert!(library.has("manus"));
        assert!(library.has("browser"));
        assert!(library.has("swe"));
        assert!(library.has("planning"));
        assert!(library.has("mcp"));
        assert!(library.has("data_analysis"));
    }

    #[test]
    fn test_library_register() {
        let mut library = PromptLibrary::new();
        library.register(crate::prompt::manus::create_manus_prompts());

        assert!(library.has("manus"));
        assert!(!library.has("browser"));
    }

    #[test]
    fn test_library_get() {
        let library = PromptLibrary::with_defaults();
        let set = library.get("manus");

        assert!(set.is_some());
        assert_eq!(set.unwrap().name, "manus");
    }

    #[test]
    fn test_library_get_system_prompt() {
        let library = PromptLibrary::with_defaults();
        let prompt = library.get_system_prompt("manus");

        assert!(prompt.is_some());
    }

    #[test]
    fn test_library_render_system() {
        let library = PromptLibrary::with_defaults();
        let mut vars = HashMap::new();
        vars.insert("directory".to_string(), "/test/path".to_string());

        let result = library.render_system("manus", &vars);
        assert!(result.is_some());
        assert!(result.unwrap().contains("/test/path"));
    }

    #[test]
    fn test_library_agent_names() {
        let library = PromptLibrary::with_defaults();
        let names = library.agent_names();

        assert!(names.contains(&&"manus".to_string()));
        assert!(names.contains(&&"browser".to_string()));
        assert!(names.contains(&&"data_analysis".to_string()));
        assert_eq!(names.len(), 6);
    }

    #[test]
    fn test_global_library() {
        let library = global_library();
        assert!(library.has("manus"));
        assert!(library.has("browser"));
    }

    #[test]
    fn test_convenience_functions() {
        let system = get_system("manus");
        assert!(system.is_some());

        let next_step = get_next_step("manus");
        assert!(next_step.is_some());
    }
}
