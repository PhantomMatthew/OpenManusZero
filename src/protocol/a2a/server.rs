//! A2A Server implementation
//!
//! Provides an HTTP server for A2A protocol

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::jsonrpc::*;
use super::types::*;
use crate::error::{Error, Result};

/// A2A Agent trait
#[async_trait]
pub trait A2AAgent: Send + Sync {
    /// Get the agent card
    fn card(&self) -> A2ACard;

    /// Invoke the agent with a message
    async fn invoke(&self, query: &str, context_id: &str) -> Result<A2ATask>;

    /// Get a task by ID
    async fn get_task(&self, task_id: &str) -> Option<A2ATask>;

    /// Cancel a task
    async fn cancel_task(&self, task_id: &str) -> Option<A2ATask>;

    /// Supported content types
    fn supported_content_types(&self) -> Vec<String> {
        vec!["text".to_string(), "text/plain".to_string()]
    }
}

/// Task store trait
#[async_trait]
pub trait TaskStore: Send + Sync {
    /// Save a task
    async fn save(&self, task: &A2ATask) -> Result<()>;

    /// Get a task by ID
    async fn get(&self, id: &str) -> Option<A2ATask>;

    /// Delete a task
    async fn delete(&self, id: &str) -> Result<()>;

    /// Update a task
    async fn update(&self, task: &A2ATask) -> Result<()>;
}

/// In-memory task store
pub struct InMemoryTaskStore {
    tasks: RwLock<HashMap<String, A2ATask>>,
}

impl InMemoryTaskStore {
    /// Create a new in-memory task store
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryTaskStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskStore for InMemoryTaskStore {
    async fn save(&self, task: &A2ATask) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id.clone(), task.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> Option<A2ATask> {
        let tasks = self.tasks.read().await;
        tasks.get(id).cloned()
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(id);
        Ok(())
    }

    async fn update(&self, task: &A2ATask) -> Result<()> {
        self.save(task).await
    }
}

/// A2A Server configuration
#[derive(Debug, Clone)]
pub struct A2AServerConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for A2AServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 10000,
            timeout_secs: 300,
        }
    }
}

/// A2A Server
pub struct A2AServer {
    /// Server configuration
    config: A2AServerConfig,
    /// Agent implementation
    agent: Arc<dyn A2AAgent>,
    /// Task store
    task_store: Arc<dyn TaskStore>,
    /// Agent card cache
    card_json: String,
}

impl A2AServer {
    /// Create a new A2A server
    pub fn new(agent: Arc<dyn A2AAgent>) -> Self {
        Self::with_config(agent, A2AServerConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(agent: Arc<dyn A2AAgent>, config: A2AServerConfig) -> Self {
        let card = agent.card();
        let card_json = serde_json::to_string(&card).unwrap_or_default();

        Self {
            config,
            agent,
            task_store: Arc::new(InMemoryTaskStore::new()),
            card_json,
        }
    }

    /// Set task store
    pub fn with_task_store(mut self, store: Arc<dyn TaskStore>) -> Self {
        self.task_store = store;
        self
    }

    /// Get the server URL
    pub fn url(&self) -> String {
        format!("http://{}:{}/", self.config.host, self.config.port)
    }

    /// Get agent card JSON
    pub fn card_json(&self) -> &str {
        &self.card_json
    }

    /// Handle JSON-RPC request
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            METHOD_MESSAGE_SEND => self.handle_message_send(request).await,
            METHOD_TASKS_GET => self.handle_tasks_get(request).await,
            METHOD_TASKS_CANCEL => self.handle_tasks_cancel(request).await,
            _ => JsonRpcResponse::error(request.id, JsonRpcError::method_not_found(&request.method)),
        }
    }

    /// Handle message/send method
    async fn handle_message_send(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params = match &request.params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::invalid_params("Missing params"),
                );
            }
        };

        let msg_params: MessageSendParams = match serde_json::from_value(params.clone()) {
            Ok(p) => p,
            Err(e) => {
                return JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::invalid_params(format!("Invalid params: {}", e)),
                );
            }
        };

        // Get context ID or generate new one
        let context_id = msg_params
            .message
            .context_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Get text from message
        let query = msg_params.message.get_text();

        // Invoke agent
        match self.agent.invoke(&query, &context_id).await {
            Ok(task) => {
                // Save task
                let _ = self.task_store.save(&task).await;

                JsonRpcResponse::success(request.id, serde_json::to_value(&task).unwrap())
            }
            Err(e) => JsonRpcResponse::error(
                request.id,
                JsonRpcError::internal_error(e.to_string()),
            ),
        }
    }

    /// Handle tasks/get method
    async fn handle_tasks_get(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params = match &request.params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::invalid_params("Missing params"),
                );
            }
        };

        let get_params: TasksGetParams = match serde_json::from_value(params.clone()) {
            Ok(p) => p,
            Err(e) => {
                return JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::invalid_params(format!("Invalid params: {}", e)),
                );
            }
        };

        match self.task_store.get(&get_params.id).await {
            Some(task) => JsonRpcResponse::success(request.id, serde_json::to_value(&task).unwrap()),
            None => JsonRpcResponse::error(
                request.id,
                JsonRpcError::invalid_params("Task not found"),
            ),
        }
    }

    /// Handle tasks/cancel method
    async fn handle_tasks_cancel(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params = match &request.params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::invalid_params("Missing params"),
                );
            }
        };

        let cancel_params: TasksCancelParams = match serde_json::from_value(params.clone()) {
            Ok(p) => p,
            Err(e) => {
                return JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::invalid_params(format!("Invalid params: {}", e)),
                );
            }
        };

        match self.agent.cancel_task(&cancel_params.id).await {
            Some(task) => {
                let _ = self.task_store.update(&task).await;
                JsonRpcResponse::success(request.id, serde_json::to_value(&task).unwrap())
            }
            None => JsonRpcResponse::error(
                request.id,
                JsonRpcError::invalid_params("Task not found"),
            ),
        }
    }

    /// Handle agent card request (GET /.well-known/agent.json)
    pub fn handle_agent_card(&self) -> &str {
        &self.card_json
    }

    /// Get server configuration
    pub fn config(&self) -> &A2AServerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockAgent;

    #[async_trait]
    impl A2AAgent for MockAgent {
        fn card(&self) -> A2ACard {
            A2ACard::new("Mock Agent", "A mock agent for testing", "http://localhost:10000")
        }

        async fn invoke(&self, query: &str, context_id: &str) -> Result<A2ATask> {
            let task = A2ATask::generate()
                .with_context(context_id)
                .complete(format!("Response to: {}", query));
            Ok(task)
        }

        async fn get_task(&self, _task_id: &str) -> Option<A2ATask> {
            None
        }

        async fn cancel_task(&self, task_id: &str) -> Option<A2ATask> {
            let task = A2ATask::new(task_id)
                .with_status(A2ATaskStatus::new(A2ATaskState::Canceled));
            Some(task)
        }
    }

    #[test]
    fn test_a2a_server_new() {
        let agent = Arc::new(MockAgent);
        let server = A2AServer::new(agent);
        assert_eq!(server.url(), "http://127.0.0.1:10000/");
    }

    #[test]
    fn test_a2a_server_card_json() {
        let agent = Arc::new(MockAgent);
        let server = A2AServer::new(agent);
        let card = server.handle_agent_card();
        assert!(card.contains("Mock Agent"));
    }

    #[tokio::test]
    async fn test_in_memory_task_store() {
        let store = InMemoryTaskStore::new();
        let task = A2ATask::generate().complete("Done");

        store.save(&task).await.unwrap();
        let retrieved = store.get(&task.id).await;
        assert!(retrieved.is_some());

        store.delete(&task.id).await.unwrap();
        let deleted = store.get(&task.id).await;
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_handle_message_send() {
        let agent = Arc::new(MockAgent);
        let server = A2AServer::new(agent);

        let request = JsonRpcRequest::new(1, METHOD_MESSAGE_SEND).with_params(json!({
            "message": {
                "role": "user",
                "parts": [{"kind": "text", "text": "Hello"}]
            }
        }));

        let response = server.handle_request(request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_unknown_method() {
        let agent = Arc::new(MockAgent);
        let server = A2AServer::new(agent);

        let request = JsonRpcRequest::new(1, "unknown/method");
        let response = server.handle_request(request).await;

        assert!(response.result.is_none());
        assert!(response.error.is_some());
    }
}
