//! LLM client abstraction for OpenManus
//!
//! This module provides the `LlmClient` trait and implementations for
//! interacting with LLM APIs, including OpenAI-compatible APIs and AWS Bedrock.
//!
//! # Example
//!
//! ```rust,no_run
//! use openmanus::llm::{HttpLlmClient, HttpLlmClientBuilder, LlmClient, LlmRequest};
//! use openmanus::schema::Message;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client from environment variables
//!     let client = HttpLlmClientBuilder::from_env()
//!         .model("gpt-4o")
//!         .build()?;
//!
//!     // Simple ask
//!     let answer = client.ask("What is Rust?").await?;
//!     println!("Answer: {}", answer);
//!
//!     Ok(())
//! }
//! ```
//!
//! # AWS Bedrock
//!
//! ```rust,no_run,ignore
//! use openmanus::llm::{BedrockClient, BedrockConfig, LlmClient};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Set environment variables:
//!     // AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION
//!     let client = BedrockClient::from_env()?;
//!
//!     let answer = client.ask("Hello!").await?;
//!     println!("Answer: {}", answer);
//!
//!     Ok(())
//! }
//! ```

mod client;
mod token;
mod types;

#[cfg(feature = "bedrock")]
mod bedrock;

pub use client::{HttpLlmClient, HttpLlmClientBuilder, LlmClient, LlmStream, MockLlmClient};
pub use token::TokenCounter;
pub use types::{
    FunctionDelta, LlmChunk, LlmChunkChoice, LlmChunkDelta, LlmRequest, LlmResponse,
    LlmResponseChoice, LlmResponseMessage, LlmUsage, ToolCallDelta, ToolChoice,
};

#[cfg(feature = "bedrock")]
pub use bedrock::{BedrockClient, BedrockConfig};

// Re-export LlmError from error module
pub use crate::error::LlmError;

/// Re-export common LLM types
pub mod prelude {
    pub use super::client::{HttpLlmClient, HttpLlmClientBuilder, LlmClient, MockLlmClient};
    pub use super::token::TokenCounter;
    pub use super::types::{LlmChunk, LlmRequest, LlmResponse, LlmUsage, ToolChoice};
    pub use crate::error::LlmError;

    #[cfg(feature = "bedrock")]
    pub use super::bedrock::{BedrockClient, BedrockConfig};
}
