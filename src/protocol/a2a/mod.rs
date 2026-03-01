//! A2A Protocol Implementation
//!
//! Based on Google's A2A (Agent-to-Agent) Protocol
//! https://google.github.io/A2A/#/documentation
//!
//! # Overview
//!
//! The A2A protocol enables communication between AI agents using JSON-RPC 2.0
//! over HTTP. It provides:
//!
//! - **Agent Card**: Metadata describing agent capabilities and skills
//! - **Tasks**: Units of work that agents perform
//! - **Messages**: Communication between users and agents
//! - **Artifacts**: Outputs produced by agents
//!
//! # Example
//!
//! ```rust,ignore
//! use openmanus::protocol::a2a::*;
//! use std::sync::Arc;
//!
//! // Create an agent
//! let agent = Arc::new(MyAgent::new());
//!
//! // Create server
//! let server = A2AServer::new(agent);
//!
//! // Handle JSON-RPC request
//! let request = JsonRpcRequest::new(1, "message/send")
//!     .with_params(json!({
//!         "message": {
//!             "role": "user",
//!             "parts": [{"kind": "text", "text": "Hello"}]
//!         }
//!     }));
//!
//! let response = server.handle_request(request).await;
//! ```
//!
//! # Protocol Methods
//!
//! - `message/send` - Send a message to the agent
//! - `message/stream` - Stream responses (not yet supported)
//! - `tasks/get` - Get task status
//! - `tasks/cancel` - Cancel a running task
//! - `tasks/pushNotification/set` - Configure push notifications

mod types;
mod jsonrpc;
mod server;
mod agent_adapter;

// Re-export main types
pub use types::{
    A2AArtifact, A2ACapabilities, A2ACard, A2AFilePart, A2AMessage, A2APart, A2ARole,
    A2ASkill, A2ATask, A2ATaskState, A2ATaskStatus,
};

pub use jsonrpc::{
    JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse,
    MessageSendParams, PushNotificationConfig, PushNotificationParams,
    TasksCancelParams, TasksGetParams,
    METHOD_MESSAGE_SEND, METHOD_MESSAGE_STREAM, METHOD_TASKS_CANCEL,
    METHOD_TASKS_GET, METHOD_TASKS_PUSH_NOTIFICATION_SET,
};

pub use server::{
    A2AAgent, A2AServer, A2AServerConfig, InMemoryTaskStore, TaskStore,
};

pub use agent_adapter::{A2ABrowser, A2AManus};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that all types are exported correctly
        let _caps = A2ACapabilities::default();
        let _skill = A2ASkill::new("test", "Test", "A test skill");
        let _card = A2ACard::new("Agent", "Test agent", "http://localhost");
        let _part = A2APart::text("Hello");
        let _msg = A2AMessage::user("Hello");
        let _artifact = A2AArtifact::text("test", "content");
        let _task = A2ATask::generate();
    }
}
