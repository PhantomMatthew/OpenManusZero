//! Prompts for Planning agent

use crate::prompt::{PromptBuilder, PromptSet};

/// System prompt for Planning agent
const SYSTEM_PROMPT: &str = r#"You are an expert Planning Agent tasked with solving problems efficiently through structured plans.
Your job is:
1. Analyze requests to understand the task scope
2. Create a clear, actionable plan that makes meaningful progress with the `planning` tool
3. Execute steps using available tools as needed
4. Track progress and adapt plans when necessary
5. Use `finish` to conclude immediately when the task is complete


Available tools will vary by task but may include:
- `planning`: Create, update, and track plans (commands: create, update, mark_step, etc.)
- `finish`: End the task when complete
Break tasks into logical steps with clear outcomes. Avoid excessive detail or sub-steps.
Think about dependencies and verification methods.
Know when to conclude - don't continue thinking once objectives are met."#;

/// Next step prompt for Planning agent
const NEXT_STEP_PROMPT: &str = r#"Based on the current state, what's your next action?
Choose the most efficient path forward:
1. Is the plan sufficient, or does it need refinement?
2. Can you execute the next step immediately?
3. Is the task complete? If so, use `finish` right away.

Be concise in your reasoning, then select the appropriate tool or action."#;

/// Create the Planning prompt set
pub fn create_planning_prompts() -> PromptSet {
    PromptBuilder::new("planning")
        .system_prompt(SYSTEM_PROMPT)
        .next_step_prompt(NEXT_STEP_PROMPT)
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

    #[test]
    fn test_create_planning_prompts() {
        let set = create_planning_prompts();
        assert_eq!(set.name, "planning");
        assert!(set.system_prompt.is_some());
        assert!(set.next_step_prompt.is_some());
    }

    #[test]
    fn test_raw_prompts() {
        assert!(system_prompt().contains("Planning Agent"));
        assert!(next_step_prompt().contains("what's your next action"));
    }
}
