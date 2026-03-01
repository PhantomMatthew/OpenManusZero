//! Prompts for MCP (Model Context Protocol) agent

use crate::prompt::{PromptBuilder, PromptSet};

/// System prompt for MCP agent
const SYSTEM_PROMPT: &str = r#"You are an AI assistant with access to a Model Context Protocol (MCP) server.
You can use the tools provided by the MCP server to complete tasks.
The MCP server will dynamically expose tools that you can use - always check the available tools first.

When using an MCP tool:
1. Choose the appropriate tool based on your task requirements
2. Provide properly formatted arguments as required by the tool
3. Observe the results and use them to determine next steps
4. Tools may change during operation - new tools might appear or existing ones might disappear

Follow these guidelines:
- Call tools with valid parameters as documented in their schemas
- Handle errors gracefully by understanding what went wrong and trying again with corrected parameters
- For multimedia responses (like images), you'll receive a description of the content
- Complete user requests step by step, using the most appropriate tools
- If multiple tools need to be called in sequence, make one call at a time and wait for results

Remember to clearly explain your reasoning and actions to the user."#;

/// Next step prompt for MCP agent
const NEXT_STEP_PROMPT: &str = r#"Based on the current state and available tools, what should be done next?
Think step by step about the problem and identify which MCP tool would be most helpful for the current stage.
If you've already made progress, consider what additional information you need or what actions would move you closer to completing the task."#;

/// Tool error prompt
const TOOL_ERROR_PROMPT: &str = r#"You encountered an error with the tool '{tool_name}'.
Try to understand what went wrong and correct your approach.
Common issues include:
- Missing or incorrect parameters
- Invalid parameter formats
- Using a tool that's no longer available
- Attempting an operation that's not supported

Please check the tool specifications and try again with corrected parameters."#;

/// Multimedia response prompt
const MULTIMEDIA_RESPONSE_PROMPT: &str = r#"You've received a multimedia response (image, audio, etc.) from the tool '{tool_name}'.
This content has been processed and described for you.
Use this information to continue the task or provide insights to the user."#;

/// Create the MCP prompt set
pub fn create_mcp_prompts() -> PromptSet {
    PromptBuilder::new("mcp")
        .system_prompt(SYSTEM_PROMPT)
        .next_step_prompt(NEXT_STEP_PROMPT)
        .error_prompt(TOOL_ERROR_PROMPT)
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

/// Get the raw tool error prompt
pub fn tool_error_prompt() -> &'static str {
    TOOL_ERROR_PROMPT
}

/// Get the raw multimedia response prompt
pub fn multimedia_response_prompt() -> &'static str {
    MULTIMEDIA_RESPONSE_PROMPT
}

/// Render tool error prompt with tool name
pub fn render_tool_error(tool_name: &str) -> String {
    TOOL_ERROR_PROMPT.replace("{tool_name}", tool_name)
}

/// Render multimedia response prompt with tool name
pub fn render_multimedia_response(tool_name: &str) -> String {
    MULTIMEDIA_RESPONSE_PROMPT.replace("{tool_name}", tool_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_create_mcp_prompts() {
        let set = create_mcp_prompts();
        assert_eq!(set.name, "mcp");
        assert!(set.system_prompt.is_some());
        assert!(set.next_step_prompt.is_some());
        assert!(set.error_prompt.is_some());
    }

    #[test]
    fn test_mcp_error_prompt() {
        let set = create_mcp_prompts();
        let mut vars = HashMap::new();
        vars.insert("tool_name".to_string(), "file_read".to_string());

        let result = set.render_error(&vars);
        // The template uses {tool_name} but we need to check if it's in the template
        assert!(set.error_prompt.as_ref().unwrap().template().contains("{tool_name}"));
    }

    #[test]
    fn test_render_tool_error() {
        let result = render_tool_error("database_query");
        assert!(result.contains("database_query"));
        assert!(result.contains("Missing or incorrect parameters"));
    }

    #[test]
    fn test_render_multimedia_response() {
        let result = render_multimedia_response("image_analyzer");
        assert!(result.contains("image_analyzer"));
        assert!(result.contains("multimedia response"));
    }

    #[test]
    fn test_raw_prompts() {
        assert!(system_prompt().contains("Model Context Protocol"));
        assert!(next_step_prompt().contains("what should be done next"));
        assert!(tool_error_prompt().contains("{tool_name}"));
        assert!(multimedia_response_prompt().contains("multimedia response"));
    }
}
