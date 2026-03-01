//! Error types for OpenManus

use thiserror::Error;

/// Main error type for OpenManus operations
#[derive(Error, Debug)]
pub enum Error {
    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Context error: {0}")]
    Context(String),

    #[error("Sandbox error: {0}")]
    Sandbox(String),

    #[error("Flow error: {0}")]
    Flow(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Token limit exceeded: {0}")]
    TokenLimitExceeded(String),

    #[error("Agent is stuck in a loop")]
    StuckLoop,

    #[error("Max steps exceeded: {0}")]
    MaxStepsExceeded(usize),
}

/// Error type for tool operations
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Invalid tool input: {0}")]
    InvalidInput(String),

    #[error("Tool timeout: {0}")]
    Timeout(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Tool is not available: {0}")]
    NotAvailable(String),
}

/// Error type for LLM operations
#[derive(Error, Debug)]
pub enum LlmError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Context length exceeded: {0}")]
    ContextLengthExceeded(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Token limit exceeded: {0}")]
    TokenLimitExceeded(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Builder error: {0}")]
    BuilderError(String),
}

// Implement From<reqwest::Error> for LlmError
impl From<reqwest::Error> for LlmError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            LlmError::Timeout
        } else if err.is_status() {
            match err.status() {
                Some(status) => {
                    if status.as_u16() == 401 {
                        LlmError::AuthFailed("Invalid API key".to_string())
                    } else if status.as_u16() == 429 {
                        LlmError::RateLimitExceeded("Rate limit exceeded".to_string())
                    } else {
                        LlmError::ApiError(format!("HTTP {}: {}", status, err))
                    }
                }
                None => LlmError::ApiError(err.to_string()),
            }
        } else {
            LlmError::NetworkError(err.to_string())
        }
    }
}

/// Result type alias for OpenManus operations
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::Agent("test error".to_string());
        assert_eq!(err.to_string(), "Agent error: test error");
    }

    #[test]
    fn test_tool_error_display() {
        let err = ToolError::NotFound("bash".to_string());
        assert_eq!(err.to_string(), "Tool not found: bash");
    }

    #[test]
    fn test_llm_error_display() {
        let err = LlmError::ApiError("connection refused".to_string());
        assert_eq!(err.to_string(), "API error: connection refused");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn test_error_from_tool_error() {
        let tool_err = ToolError::ExecutionFailed("failed".to_string());
        let err: Error = tool_err.into();
        assert!(matches!(err, Error::Tool(_)));
    }
}
