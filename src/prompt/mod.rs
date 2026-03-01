//! Prompt system for OpenManus agents
//!
//! This module provides a flexible prompt template system with variable substitution
//! support, organized by agent type.
//!
//! # Example
//!
//! ```
//! use openmanus::prompt::{PromptTemplate, PromptSet, PromptLibrary};
//! use std::collections::HashMap;
//!
//! // Create a simple template
//! let template = PromptTemplate::new("greeting", "Hello, {name}!");
//!
//! // Render with variables
//! let mut vars = HashMap::new();
//! vars.insert("name".to_string(), "Alice".to_string());
//! let result = template.render(&vars);
//! assert_eq!(result, "Hello, Alice!");
//!
//! // Use the prompt library
//! let library = PromptLibrary::with_defaults();
//! let system = library.render_system("manus", &vars);
//! ```

mod template;
mod library;

// Agent-specific prompts
pub mod manus;
pub mod browser;
pub mod swe;
pub mod planning;
pub mod mcp;
pub mod data_analysis;

// Re-export main types
pub use template::{PromptTemplate, PromptSet, PromptBuilder};
pub use library::{PromptLibrary, global_library, get_system, get_next_step};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_prompt_template_basic() {
        let t = PromptTemplate::new("test", "Value: {value}");
        let mut vars = HashMap::new();
        vars.insert("value".to_string(), "123".to_string());
        assert_eq!(t.render(&vars), "Value: 123");
    }

    #[test]
    fn test_prompt_set_builder() {
        let set = PromptSet::builder("custom")
            .system_prompt("System: {mode}")
            .next_step_prompt("Next: {action}")
            .variable("mode", "production")
            .build();

        let vars = HashMap::new();
        let system = set.render_system(&vars).unwrap();
        assert!(system.contains("production"));
    }

    #[test]
    fn test_library_integration() {
        let library = PromptLibrary::with_defaults();

        // Test all agent types have prompts
        for agent in &["manus", "browser", "swe", "planning", "mcp", "data_analysis"] {
            assert!(library.has(agent), "Missing prompts for {}", agent);
        }
    }

    #[test]
    fn test_manus_prompts() {
        let set = manus::create_manus_prompts();
        assert!(set.system_prompt.is_some());
        assert!(set.next_step_prompt.is_some());
    }

    #[test]
    fn test_browser_prompts() {
        let set = browser::create_browser_prompts();
        assert!(set.system_prompt.is_some());
        assert!(set.next_step_prompt.is_some());
        assert!(set.error_prompt.is_some());
    }

    #[test]
    fn test_swe_prompts() {
        let set = swe::create_swe_prompts();
        assert!(set.system_prompt.is_some());
        assert!(set.next_step_prompt.is_some());
    }

    #[test]
    fn test_planning_prompts() {
        let set = planning::create_planning_prompts();
        assert!(set.system_prompt.is_some());
        assert!(set.next_step_prompt.is_some());
    }

    #[test]
    fn test_mcp_prompts() {
        let set = mcp::create_mcp_prompts();
        assert!(set.system_prompt.is_some());
        assert!(set.next_step_prompt.is_some());
    }

    #[test]
    fn test_data_analysis_prompts() {
        let set = data_analysis::create_data_analysis_prompts();
        assert!(set.system_prompt.is_some());
        assert!(set.next_step_prompt.is_some());
        assert!(set.error_prompt.is_some());
    }

    #[test]
    fn test_global_library_access() {
        let system = get_system("manus");
        assert!(system.is_some());

        let next_step = get_next_step("manus");
        assert!(next_step.is_some());
    }
}
