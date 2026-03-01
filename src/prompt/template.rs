//! Prompt template with variable substitution support

use std::collections::HashMap;
use std::fmt;

/// A prompt template that supports variable substitution
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// Template name
    name: String,
    /// Template content with {variable} placeholders
    template: String,
    /// Variable defaults
    defaults: HashMap<String, String>,
}

impl PromptTemplate {
    /// Create a new prompt template
    pub fn new(name: impl Into<String>, template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            template: template.into(),
            defaults: HashMap::new(),
        }
    }

    /// Get the template name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the raw template content
    pub fn template(&self) -> &str {
        &self.template
    }

    /// Add a default value for a variable
    pub fn with_default(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.defaults.insert(key.into(), value.into());
        self
    }

    /// Add multiple default values
    pub fn with_defaults(mut self, defaults: HashMap<String, String>) -> Self {
        self.defaults.extend(defaults);
        self
    }

    /// Render the template with provided variables
    pub fn render(&self, variables: &HashMap<String, String>) -> String {
        let mut result = self.template.clone();

        // Replace variables from provided map
        for (key, value) in variables {
            let placeholder = format!("{{{}}}", key);
            result = result.replace(&placeholder, value);
        }

        // Replace remaining variables with defaults
        for (key, value) in &self.defaults {
            let placeholder = format!("{{{}}}", key);
            if result.contains(&placeholder) {
                result = result.replace(&placeholder, value);
            }
        }

        result
    }

    /// Render with a single variable
    pub fn render_with(&self, key: &str, value: &str) -> String {
        let mut vars = HashMap::new();
        vars.insert(key.to_string(), value.to_string());
        self.render(&vars)
    }

    /// Extract variable names from the template
    pub fn variables(&self) -> Vec<String> {
        let mut vars = Vec::new();
        let mut in_brace = false;
        let mut current_var = String::new();

        for ch in self.template.chars() {
            match ch {
                '{' => {
                    in_brace = true;
                    current_var.clear();
                }
                '}' => {
                    if in_brace && !current_var.is_empty() {
                        if !vars.contains(&current_var) {
                            vars.push(current_var.clone());
                        }
                    }
                    in_brace = false;
                    current_var.clear();
                }
                _ => {
                    if in_brace {
                        current_var.push(ch);
                    }
                }
            }
        }

        vars
    }
}

impl fmt::Display for PromptTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PromptTemplate({})", self.name)
    }
}

impl From<&str> for PromptTemplate {
    fn from(template: &str) -> Self {
        Self::new("unnamed", template)
    }
}

impl From<String> for PromptTemplate {
    fn from(template: String) -> Self {
        Self::new("unnamed", template)
    }
}

/// Builder for creating prompt templates
pub struct PromptBuilder {
    name: String,
    system_prompt: Option<String>,
    next_step_prompt: Option<String>,
    error_prompt: Option<String>,
    variables: HashMap<String, String>,
}

impl PromptBuilder {
    /// Create a new prompt builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            system_prompt: None,
            next_step_prompt: None,
            error_prompt: None,
            variables: HashMap::new(),
        }
    }

    /// Set the system prompt
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the next step prompt
    pub fn next_step_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.next_step_prompt = Some(prompt.into());
        self
    }

    /// Set the error prompt
    pub fn error_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.error_prompt = Some(prompt.into());
        self
    }

    /// Add a variable default
    pub fn variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    /// Build the prompt set
    pub fn build(self) -> PromptSet {
        PromptSet {
            name: self.name,
            system_prompt: self.system_prompt.map(|t| {
                PromptTemplate::new("system", t).with_defaults(self.variables.clone())
            }),
            next_step_prompt: self.next_step_prompt.map(|t| {
                PromptTemplate::new("next_step", t).with_defaults(self.variables.clone())
            }),
            error_prompt: self.error_prompt.map(|t| {
                PromptTemplate::new("error", t).with_defaults(self.variables.clone())
            }),
        }
    }
}

/// A set of related prompts for an agent
#[derive(Debug, Clone)]
pub struct PromptSet {
    /// Name of the prompt set (usually agent name)
    pub name: String,
    /// System prompt template
    pub system_prompt: Option<PromptTemplate>,
    /// Next step prompt template
    pub next_step_prompt: Option<PromptTemplate>,
    /// Error handling prompt template
    pub error_prompt: Option<PromptTemplate>,
}

impl PromptSet {
    /// Create a new prompt set
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            system_prompt: None,
            next_step_prompt: None,
            error_prompt: None,
        }
    }

    /// Create using builder pattern
    pub fn builder(name: impl Into<String>) -> PromptBuilder {
        PromptBuilder::new(name)
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(PromptTemplate::new("system", prompt));
        self
    }

    /// Set the next step prompt
    pub fn with_next_step_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.next_step_prompt = Some(PromptTemplate::new("next_step", prompt));
        self
    }

    /// Set the error prompt
    pub fn with_error_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.error_prompt = Some(PromptTemplate::new("error", prompt));
        self
    }

    /// Render system prompt with variables
    pub fn render_system(&self, variables: &HashMap<String, String>) -> Option<String> {
        self.system_prompt.as_ref().map(|t| t.render(variables))
    }

    /// Render next step prompt with variables
    pub fn render_next_step(&self, variables: &HashMap<String, String>) -> Option<String> {
        self.next_step_prompt.as_ref().map(|t| t.render(variables))
    }

    /// Render error prompt with variables
    pub fn render_error(&self, variables: &HashMap<String, String>) -> Option<String> {
        self.error_prompt.as_ref().map(|t| t.render(variables))
    }

    /// Get the raw system prompt template
    pub fn system(&self) -> Option<&str> {
        self.system_prompt.as_ref().map(|t| t.template())
    }

    /// Get the raw next step prompt template
    pub fn next_step(&self) -> Option<&str> {
        self.next_step_prompt.as_ref().map(|t| t.template())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_new() {
        let t = PromptTemplate::new("test", "Hello, {name}!");
        assert_eq!(t.name(), "test");
        assert_eq!(t.template(), "Hello, {name}!");
    }

    #[test]
    fn test_template_render() {
        let t = PromptTemplate::new("test", "Hello, {name}! You are in {city}.");
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("city".to_string(), "Tokyo".to_string());

        let result = t.render(&vars);
        assert_eq!(result, "Hello, Alice! You are in Tokyo.");
    }

    #[test]
    fn test_template_render_with_defaults() {
        let t = PromptTemplate::new("test", "Hello, {name}! Directory: {dir}")
            .with_default("dir", "/home/user");

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Bob".to_string());

        let result = t.render(&vars);
        assert_eq!(result, "Hello, Bob! Directory: /home/user");
    }

    #[test]
    fn test_template_render_with_single() {
        let t = PromptTemplate::new("test", "Path: {path}");
        let result = t.render_with("path", "/usr/local");
        assert_eq!(result, "Path: /usr/local");
    }

    #[test]
    fn test_template_extract_variables() {
        let t = PromptTemplate::new("test", "{greeting}, {name}! You are in {city}.");
        let vars = t.variables();
        assert_eq!(vars.len(), 3);
        assert!(vars.contains(&"greeting".to_string()));
        assert!(vars.contains(&"name".to_string()));
        assert!(vars.contains(&"city".to_string()));
    }

    #[test]
    fn test_template_missing_variable() {
        let t = PromptTemplate::new("test", "Hello, {name}!");
        let vars = HashMap::new();
        let result = t.render(&vars);
        assert_eq!(result, "Hello, {name}!"); // Unchanged if no value provided
    }

    #[test]
    fn test_prompt_set_new() {
        let set = PromptSet::new("manus");
        assert_eq!(set.name, "manus");
        assert!(set.system_prompt.is_none());
    }

    #[test]
    fn test_prompt_set_builder() {
        let set = PromptSet::builder("test")
            .system_prompt("You are a test agent.")
            .next_step_prompt("What's next?")
            .build();

        assert_eq!(set.name, "test");
        assert!(set.system_prompt.is_some());
        assert!(set.next_step_prompt.is_some());
    }

    #[test]
    fn test_prompt_set_with_prompts() {
        let set = PromptSet::new("test")
            .with_system_prompt("System prompt")
            .with_next_step_prompt("Next step prompt");

        assert_eq!(set.system(), Some("System prompt"));
        assert_eq!(set.next_step(), Some("Next step prompt"));
    }

    #[test]
    fn test_prompt_set_render() {
        let set = PromptSet::new("test")
            .with_system_prompt("Hello, {name}!")
            .with_next_step_prompt("Continue, {name}?");

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());

        let system = set.render_system(&vars);
        assert_eq!(system, Some("Hello, Alice!".to_string()));

        let next = set.render_next_step(&vars);
        assert_eq!(next, Some("Continue, Alice?".to_string()));
    }

    #[test]
    fn test_template_from_str() {
        let t: PromptTemplate = "Hello, world!".into();
        assert_eq!(t.template(), "Hello, world!");
    }

    #[test]
    fn test_template_display() {
        let t = PromptTemplate::new("my_template", "content");
        assert_eq!(format!("{}", t), "PromptTemplate(my_template)");
    }
}
