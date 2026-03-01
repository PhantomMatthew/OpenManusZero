//! Sandbox manager for managing multiple DockerSandbox instances
//!
//! Features:
//! - Lifecycle management (create, get, delete)
//! - Concurrent access control
//! - Automatic cleanup of idle sandboxes
//! - Resource limits

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;

#[cfg(feature = "docker-sandbox")]
use super::docker::{DockerSandbox, SandboxConfig, VolumeBinding};

/// Statistics about the sandbox manager
#[derive(Debug, Clone)]
pub struct ManagerStats {
    /// Total number of sandboxes
    pub total_sandboxes: usize,
    /// Number of active operations
    pub active_operations: usize,
    /// Maximum allowed sandboxes
    pub max_sandboxes: usize,
    /// Idle timeout in seconds
    pub idle_timeout_secs: u64,
    /// Whether manager is shutting down
    pub is_shutting_down: bool,
}

/// Sandbox manager configuration
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// Maximum number of sandboxes
    pub max_sandboxes: usize,
    /// Idle timeout in seconds
    pub idle_timeout_secs: u64,
    /// Cleanup interval in seconds
    pub cleanup_interval_secs: u64,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            max_sandboxes: 100,
            idle_timeout_secs: 3600,  // 1 hour
            cleanup_interval_secs: 300, // 5 minutes
        }
    }
}

/// Information about a managed sandbox
struct SandboxInfo {
    /// The sandbox instance
    #[cfg(feature = "docker-sandbox")]
    sandbox: Arc<DockerSandbox>,
    /// Last used timestamp
    last_used: Instant,
    /// Whether there are active operations
    has_active_ops: bool,
}

/// Manager for multiple sandbox instances
pub struct SandboxManager {
    /// Manager configuration
    config: ManagerConfig,
    /// Active sandboxes by ID
    #[cfg(feature = "docker-sandbox")]
    sandboxes: Arc<RwLock<HashMap<String, SandboxInfo>>>,
    /// Operation locks per sandbox
    locks: Arc<RwLock<HashMap<String, Arc<Mutex<()>>>>>,
    /// Cleanup task handle
    cleanup_task: Mutex<Option<JoinHandle<()>>>,
    /// Shutdown flag
    is_shutting_down: Arc<RwLock<bool>>,
}

impl SandboxManager {
    /// Create a new sandbox manager
    pub fn new(config: ManagerConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "docker-sandbox")]
            sandboxes: Arc::new(RwLock::new(HashMap::new())),
            locks: Arc::new(RwLock::new(HashMap::new())),
            cleanup_task: Mutex::new(None),
            is_shutting_down: Arc::new(RwLock::new(false)),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ManagerConfig::default())
    }

    /// Start the automatic cleanup task
    pub fn start_cleanup_task(&self) {
        #[cfg(feature = "docker-sandbox")]
        let sandboxes = self.sandboxes.clone();
        let is_shutting_down = self.is_shutting_down.clone();
        let idle_timeout = Duration::from_secs(self.config.idle_timeout_secs);
        let cleanup_interval = Duration::from_secs(self.config.cleanup_interval_secs);

        let handle = tokio::spawn(async move {
            loop {
                // Check shutdown flag
                if *is_shutting_down.read().await {
                    break;
                }

                // Sleep before cleanup
                tokio::time::sleep(cleanup_interval).await;

                // Perform cleanup
                #[cfg(feature = "docker-sandbox")]
                {
                    let mut sandboxes_guard = sandboxes.write().await;
                    let now = Instant::now();

                    // Find sandboxes to cleanup
                    let to_cleanup: Vec<String> = sandboxes_guard
                        .iter()
                        .filter(|(_, info)| {
                            !info.has_active_ops
                                && now.duration_since(info.last_used) > idle_timeout
                        })
                        .map(|(id, _)| id.clone())
                        .collect();

                    // Cleanup idle sandboxes
                    for id in to_cleanup {
                        if let Some(info) = sandboxes_guard.remove(&id) {
                            tracing::info!("Cleaning up idle sandbox: {}", id);
                            if let Err(e) = info.sandbox.cleanup().await {
                                tracing::warn!("Failed to cleanup sandbox {}: {}", id, e);
                            }
                        }
                    }
                }
            }
        });

        // Store the task handle
        let mut task = self.cleanup_task.blocking_lock();
        *task = Some(handle);
    }

    /// Create a new sandbox
    #[cfg(feature = "docker-sandbox")]
    pub async fn create_sandbox(
        &self,
        config: Option<SandboxConfig>,
        volume_bindings: Option<Vec<VolumeBinding>>,
    ) -> crate::Result<String> {
        // Check if shutting down
        if *self.is_shutting_down.read().await {
            return Err(crate::error::Error::Sandbox(
                "Manager is shutting down".to_string(),
            ));
        }

        // Check max limit
        {
            let sandboxes = self.sandboxes.read().await;
            if sandboxes.len() >= self.config.max_sandboxes {
                return Err(crate::error::Error::Sandbox(format!(
                    "Maximum number of sandboxes ({}) reached",
                    self.config.max_sandboxes
                )));
            }
        }

        // Create sandbox
        let sandbox_config = config.unwrap_or_default();
        let mut sandbox = DockerSandbox::new(sandbox_config);

        if let Some(bindings) = volume_bindings {
            sandbox = sandbox.with_volumes(bindings);
        }

        sandbox.create().await?;

        // Generate ID and store
        let id = uuid::Uuid::new_v4().to_string();
        let info = SandboxInfo {
            sandbox: Arc::new(sandbox),
            last_used: Instant::now(),
            has_active_ops: false,
        };

        {
            let mut sandboxes = self.sandboxes.write().await;
            sandboxes.insert(id.clone(), info);
        }

        // Create lock for this sandbox
        {
            let mut locks = self.locks.write().await;
            locks.insert(id.clone(), Arc::new(Mutex::new(())));
        }

        tracing::info!("Created sandbox: {}", id);
        Ok(id)
    }

    /// Get a sandbox by ID
    #[cfg(feature = "docker-sandbox")]
    pub async fn get_sandbox(&self, id: &str) -> crate::Result<Arc<DockerSandbox>> {
        let lock = {
            let locks = self.locks.read().await;
            locks
                .get(id)
                .ok_or_else(|| crate::error::Error::Sandbox(format!("Sandbox not found: {}", id)))?
                .clone()
        };

        // Acquire lock and update last used
        let _guard = lock.lock().await;

        let mut sandboxes = self.sandboxes.write().await;
        let info = sandboxes
            .get_mut(id)
            .ok_or_else(|| crate::error::Error::Sandbox(format!("Sandbox not found: {}", id)))?;

        info.last_used = Instant::now();
        Ok(info.sandbox.clone())
    }

    /// Delete a sandbox by ID
    #[cfg(feature = "docker-sandbox")]
    pub async fn delete_sandbox(&self, id: &str) -> crate::Result<()> {
        // Remove from map first
        let info = {
            let mut sandboxes = self.sandboxes.write().await;
            sandboxes.remove(id)
        };

        if let Some(info) = info {
            // Wait for active operations
            let max_wait = Duration::from_secs(10);
            let start = Instant::now();

            while info.has_active_ops && start.elapsed() < max_wait {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            // Cleanup sandbox
            if let Err(e) = info.sandbox.cleanup().await {
                tracing::warn!("Failed to cleanup sandbox {}: {}", id, e);
            }

            // Remove lock
            let mut locks = self.locks.write().await;
            locks.remove(id);

            tracing::info!("Deleted sandbox: {}", id);
        }

        Ok(())
    }

    /// Run an operation on a sandbox with automatic locking
    #[cfg(feature = "docker-sandbox")]
    pub async fn with_sandbox<F, T>(&self, id: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&DockerSandbox) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<T>> + Send>>
            + Send,
        T: Send,
    {
        let lock = {
            let locks = self.locks.read().await;
            locks
                .get(id)
                .ok_or_else(|| crate::error::Error::Sandbox(format!("Sandbox not found: {}", id)))?
                .clone()
        };

        let _guard = lock.lock().await;

        // Mark as active and update last used
        {
            let mut sandboxes = self.sandboxes.write().await;
            if let Some(info) = sandboxes.get_mut(id) {
                info.has_active_ops = true;
                info.last_used = Instant::now();
            }
        }

        // Get sandbox reference
        let sandbox = {
            let sandboxes = self.sandboxes.read().await;
            sandboxes
                .get(id)
                .ok_or_else(|| crate::error::Error::Sandbox(format!("Sandbox not found: {}", id)))?
                .sandbox
                .clone()
        };

        // Execute operation
        let result = f(&sandbox).await;

        // Mark as inactive
        {
            let mut sandboxes = self.sandboxes.write().await;
            if let Some(info) = sandboxes.get_mut(id) {
                info.has_active_ops = false;
            }
        }

        result
    }

    /// Get manager statistics
    pub async fn stats(&self) -> ManagerStats {
        #[cfg(feature = "docker-sandbox")]
        {
            let sandboxes = self.sandboxes.read().await;
            let active_ops = sandboxes.values().filter(|i| i.has_active_ops).count();

            ManagerStats {
                total_sandboxes: sandboxes.len(),
                active_operations: active_ops,
                max_sandboxes: self.config.max_sandboxes,
                idle_timeout_secs: self.config.idle_timeout_secs,
                is_shutting_down: *self.is_shutting_down.read().await,
            }
        }

        #[cfg(not(feature = "docker-sandbox"))]
        {
            ManagerStats {
                total_sandboxes: 0,
                active_operations: 0,
                max_sandboxes: self.config.max_sandboxes,
                idle_timeout_secs: self.config.idle_timeout_secs,
                is_shutting_down: *self.is_shutting_down.read().await,
            }
        }
    }

    /// Cleanup all resources
    pub async fn cleanup(&self) {
        tracing::info!("Starting sandbox manager cleanup...");

        // Set shutdown flag
        {
            let mut shutting_down = self.is_shutting_down.write().await;
            *shutting_down = true;
        }

        // Cancel cleanup task
        {
            let mut task = self.cleanup_task.lock().await;
            if let Some(handle) = task.take() {
                handle.abort();
            }
        }

        // Cleanup all sandboxes
        #[cfg(feature = "docker-sandbox")]
        {
            let ids: Vec<String> = {
                let sandboxes = self.sandboxes.read().await;
                sandboxes.keys().cloned().collect()
            };

            for id in ids {
                if let Err(e) = self.delete_sandbox(&id).await {
                    tracing::warn!("Failed to cleanup sandbox {}: {}", id, e);
                }
            }

            // Clear maps
            let mut sandboxes = self.sandboxes.write().await;
            sandboxes.clear();

            let mut locks = self.locks.write().await;
            locks.clear();
        }

        tracing::info!("Sandbox manager cleanup completed");
    }
}

impl Drop for SandboxManager {
    fn drop(&mut self) {
        // Set shutdown flag (non-blocking)
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let is_shutting_down = self.is_shutting_down.clone();
            // Use spawn instead of block_on to avoid runtime nesting
            let _ = handle.spawn(async move {
                let mut flag = is_shutting_down.write().await;
                *flag = true;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_config_default() {
        let config = ManagerConfig::default();
        assert_eq!(config.max_sandboxes, 100);
        assert_eq!(config.idle_timeout_secs, 3600);
    }

    #[test]
    fn test_manager_stats() {
        let manager = SandboxManager::with_defaults();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let stats = rt.block_on(manager.stats());

        assert_eq!(stats.total_sandboxes, 0);
        assert_eq!(stats.max_sandboxes, 100);
        assert!(!stats.is_shutting_down);
    }

    #[cfg(feature = "docker-sandbox")]
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_manager_create_and_delete() {
        let manager = SandboxManager::with_defaults();

        // Create sandbox
        let id = manager.create_sandbox(None, None).await.unwrap();
        assert!(!id.is_empty());

        // Check stats
        let stats = manager.stats().await;
        assert_eq!(stats.total_sandboxes, 1);

        // Delete sandbox
        manager.delete_sandbox(&id).await.unwrap();

        // Check stats
        let stats = manager.stats().await;
        assert_eq!(stats.total_sandboxes, 0);
    }

    #[cfg(feature = "docker-sandbox")]
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_manager_max_limit() {
        let config = ManagerConfig {
            max_sandboxes: 2,
            ..Default::default()
        };
        let manager = SandboxManager::new(config);

        // Create max sandboxes
        let id1 = manager.create_sandbox(None, None).await.unwrap();
        let id2 = manager.create_sandbox(None, None).await.unwrap();

        // Should fail to create more
        let result = manager.create_sandbox(None, None).await;
        assert!(result.is_err());

        // Cleanup
        manager.delete_sandbox(&id1).await.unwrap();
        manager.delete_sandbox(&id2).await.unwrap();
    }
}
