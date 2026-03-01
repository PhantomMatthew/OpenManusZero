//! Prompts for Manus agent

use crate::prompt::{PromptBuilder, PromptSet};

/// System prompt for Manus agent
const SYSTEM_PROMPT: &str = r#"You are OpenManus, an all-capable AI assistant, aimed at solving any task presented by the user. You have various tools at your disposal that you can call upon to efficiently complete complex requests. Whether it's programming, information retrieval, file processing, web browsing, or human interaction (only for extreme cases), you can handle it all.
The initial directory is: {directory}"#;

/// Next step prompt for Manus agent
const NEXT_STEP_PROMPT: &str = r#"Based on user needs, proactively select the most appropriate tool or combination of tools. For complex tasks, you can break down the problem and use different tools step by step to solve it. After using each tool, clearly explain the execution results and suggest the next steps.

If you want to stop the interaction at any point, use the `terminate` tool/function call."#;

/// Create the Manus prompt set
pub fn create_manus_prompts() -> PromptSet {
    PromptBuilder::new("manus")
        .system_prompt(SYSTEM_PROMPT)
        .next_step_prompt(NEXT_STEP_PROMPT)
        .variable("directory", ".")
        .build()
}

/// Get the raw system prompt
pub fn system_prompt() -> &'static str {
    SYSTEM_PROMPT
}

/// Get the raw next step prompt
pub fn next_step_prompt() -> &'static str {
    NEXT_STEP_PROMPT
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_create_manus_prompts() {
        let set = create_manus_prompts();
        assert_eq!(set.name, "manus");
        assert!(set.system_prompt.is_some());
        assert!(set.next_step_prompt.is_some());
    }

    #[test]
    fn test_manus_system_prompt_with_directory() {
        let set = create_manus_prompts();
        let mut vars = HashMap::new();
        vars.insert("directory".to_string(), "/home/user/workspace".to_string());

        let result = set.render_system(&vars).unwrap();
        assert!(result.contains("/home/user/workspace"));
    }

    #[test]
    fn test_manus_system_prompt_default_directory() {
        let set = create_manus_prompts();
        let vars = HashMap::new();

        let result = set.render_system(&vars).unwrap();
        assert!(result.contains("The initial directory is: ."));
    }

    #[test]
    fn test_raw_prompts() {
        assert!(system_prompt().contains("OpenManus"));
        assert!(next_step_prompt().contains("terminate"));
    }
}
