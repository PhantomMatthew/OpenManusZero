//! Prompts for SWE (Software Engineering) agent

use crate::prompt::{PromptBuilder, PromptSet};

/// System prompt for SWE agent
const SYSTEM_PROMPT: &str = r#"SETTING: You are an autonomous programmer, and you're working directly in the command line with a special interface.

The special interface consists of a file editor that shows you 100 lines of a file at a time.
In addition to typical bash commands, you can also use specific commands to help you navigate and edit files.
To call a command, you need to invoke it with a function call/tool call.

Please note that THE EDIT COMMAND REQUIRES PROPER INDENTATION.
If you'd like to add the line '        print(x)' you must fully write that out, with all those spaces before the code! Indentation is important and code that is not indented correctly will fail and require fixing before it can be run.

RESPONSE FORMAT:
Your shell prompt is formatted as follows:
(Open file: <path>)
(Current directory: <cwd>)
bash-$

First, you should _always_ include a general thought about what you're going to do next.
Then, for every response, you must include exactly _ONE_ tool call/function call.

Remember, you should always include a _SINGLE_ tool call/function call and then wait for a response from the shell before continuing with more discussion and commands. Everything you include in the DISCUSSION section will be saved for future reference.
If you'd like to issue two commands at once, PLEASE DO NOT DO THAT! Please instead first submit just the first tool call, and then after receiving a response you'll be able to issue the second tool call.
Note that the environment does NOT support interactive session commands (e.g. python, vim), so please do not invoke them.

Current working directory: {directory}"#;

/// Next step prompt for SWE agent
const NEXT_STEP_PROMPT: &str = r#"What should I do next to solve the task?

Consider:
1. What files do I need to examine or modify?
2. What commands should I run?
3. Have I understood the problem correctly?

Use one tool at a time and wait for the result before proceeding."#;

/// Error prompt for SWE errors
const ERROR_PROMPT: &str = r#"An error occurred: {error_message}

Common issues:
- File not found: Check the file path
- Syntax error: Review your code for mistakes
- Permission denied: You may need different permissions
- Command not found: The command may not be available

Please analyze the error and try a different approach."#;

/// Create the SWE prompt set
pub fn create_swe_prompts() -> PromptSet {
    PromptBuilder::new("swe")
        .system_prompt(SYSTEM_PROMPT)
        .next_step_prompt(NEXT_STEP_PROMPT)
        .error_prompt(ERROR_PROMPT)
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

/// Get the raw error prompt
pub fn error_prompt() -> &'static str {
    ERROR_PROMPT
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_create_swe_prompts() {
        let set = create_swe_prompts();
        assert_eq!(set.name, "swe");
        assert!(set.system_prompt.is_some());
        assert!(set.next_step_prompt.is_some());
        assert!(set.error_prompt.is_some());
    }

    #[test]
    fn test_swe_system_prompt_with_directory() {
        let set = create_swe_prompts();
        let mut vars = HashMap::new();
        vars.insert("directory".to_string(), "/home/user/project".to_string());

        let result = set.render_system(&vars).unwrap();
        assert!(result.contains("Current working directory: /home/user/project"));
    }

    #[test]
    fn test_swe_error_prompt() {
        let set = create_swe_prompts();
        let mut vars = HashMap::new();
        vars.insert("error_message".to_string(), "SyntaxError".to_string());

        let result = set.render_error(&vars).unwrap();
        assert!(result.contains("SyntaxError"));
    }

    #[test]
    fn test_raw_prompts() {
        assert!(system_prompt().contains("autonomous programmer"));
        assert!(next_step_prompt().contains("What should I do next"));
        assert!(error_prompt().contains("{error_message}"));
    }
}
