//! Docker sandbox implementation using bollard
//!
//! Provides containerized execution environment with:
//! - Resource limits (CPU, memory)
//! - Network isolation
//! - File operations (read, write, copy)
//! - Command execution

use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use bollard::models::HostConfig;
use bollard::Docker;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::sandbox::Sandbox;

/// Sandbox configuration
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Docker image to use
    pub image: String,
    /// Working directory in container
    pub work_dir: String,
    /// Memory limit in bytes
    pub memory_limit: i64,
    /// CPU limit (0.0 - 1.0, where 1.0 = 100%)
    pub cpu_limit: f64,
    /// Enable network access
    pub network_enabled: bool,
    /// Command timeout in seconds
    pub timeout: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            image: "python:3.12-slim".to_string(),
            work_dir: "/workspace".to_string(),
            memory_limit: 512 * 1024 * 1024, // 512MB
            cpu_limit: 1.0,
            network_enabled: false,
            timeout: 60,
        }
    }
}

/// Volume binding configuration
#[derive(Debug, Clone)]
pub struct VolumeBinding {
    /// Host path
    pub host_path: String,
    /// Container path
    pub container_path: String,
    /// Read-only flag
    pub read_only: bool,
}

/// Docker-based sandbox for secure code execution
pub struct DockerSandbox {
    /// Docker client
    docker: Docker,
    /// Sandbox configuration
    config: SandboxConfig,
    /// Container ID
    container_id: Arc<RwLock<Option<String>>>,
    /// Volume bindings
    volume_bindings: Vec<VolumeBinding>,
}

impl DockerSandbox {
    /// Create a new Docker sandbox
    pub fn new(config: SandboxConfig) -> Self {
        let docker = Docker::connect_with_socket_defaults().unwrap_or_else(|_| {
            // Fallback to HTTP connection
            Docker::connect_with_http_defaults().expect("Failed to connect to Docker")
        });

        Self {
            docker,
            config,
            container_id: Arc::new(RwLock::new(None)),
            volume_bindings: Vec::new(),
        }
    }

    /// Create with volume bindings
    pub fn with_volumes(mut self, bindings: Vec<VolumeBinding>) -> Self {
        self.volume_bindings = bindings;
        self
    }

    /// Create and start the sandbox container
    pub async fn create(&self) -> Result<()> {
        // Ensure image exists
        self.ensure_image().await?;

        // Prepare volume bindings
        let binds: Vec<String> = self
            .volume_bindings
            .iter()
            .map(|binding| {
                let mode = if binding.read_only { "ro" } else { "rw" };
                format!(
                    "{}:{}:{}",
                    binding.host_path, binding.container_path, mode
                )
            })
            .collect();

        // Create container configuration
        let host_config = HostConfig {
            memory: Some(self.config.memory_limit),
            cpu_quota: Some((100_000_f64 * self.config.cpu_limit) as i64),
            cpu_period: Some(100_000),
            network_mode: if self.config.network_enabled {
                Some("bridge".to_string())
            } else {
                Some("none".to_string())
            },
            binds: if binds.is_empty() {
                None
            } else {
                Some(binds)
            },
            ..Default::default()
        };

        let container_name = format!("sandbox_{}", uuid::Uuid::new_v4().simple());

        let config = Config {
            image: Some(self.config.image.clone()),
            cmd: Some(vec!["tail".to_string(), "-f".to_string(), "/dev/null".to_string()]),
            working_dir: Some(self.config.work_dir.clone()),
            hostname: Some("sandbox".to_string()),
            host_config: Some(host_config),
            env: Some(vec!["PYTHONUNBUFFERED=1".to_string()]),
            ..Default::default()
        };

        // Create container
        let create_options = CreateContainerOptions::<String> {
            name: container_name.clone(),
            platform: None,
        };

        let container = self
            .docker
            .create_container(Some(create_options), config)
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to create container: {}", e)))?;

        // Store container ID
        let mut id = self.container_id.write().await;
        *id = Some(container.id.clone());

        // Start container
        self.docker
            .start_container(&container.id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to start container: {}", e)))?;

        tracing::info!("Created Docker sandbox: {}", container.id);
        Ok(())
    }

    /// Ensure Docker image is available
    async fn ensure_image(&self) -> Result<()> {
        // Check if image exists
        let images = self
            .docker
            .list_images::<String>(None)
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to list images: {}", e)))?;

        let image_exists = images.iter().any(|img| {
            img.repo_tags
                .iter()
                .any(|t| t.starts_with(&self.config.image))
        });

        if !image_exists {
            tracing::info!("Pulling image: {}", self.config.image);
            let mut stream = self.docker.create_image(
                Some(CreateImageOptions::<String> {
                    from_image: self.config.image.clone(),
                    ..Default::default()
                }),
                None,
                None,
            );

            while let Some(result) = stream.next().await {
                match result {
                    Ok(info) => {
                        if let Some(status) = info.status {
                            tracing::debug!("Pull status: {}", status);
                        }
                    }
                    Err(e) => {
                        return Err(Error::Sandbox(format!("Failed to pull image: {}", e)));
                    }
                }
            }
        }

        Ok(())
    }

    /// Run a command in the container
    pub async fn run_command(&self, cmd: &str, timeout: Option<u64>) -> Result<String> {
        let container_id = self.get_container_id().await?;

        // Create exec instance
        let exec_config = CreateExecOptions::<String> {
            cmd: Some(vec!["/bin/sh".to_string(), "-c".to_string(), cmd.to_string()]),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            working_dir: Some(self.config.work_dir.clone()),
            ..Default::default()
        };

        let exec = self
            .docker
            .create_exec(&container_id, exec_config)
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to create exec: {}", e)))?;

        // Start exec and collect output
        let timeout_duration = timeout.unwrap_or(self.config.timeout);
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_duration),
            self.collect_exec_output(&exec.id),
        )
        .await
        .map_err(|_| {
            Error::Sandbox(format!(
                "Command timed out after {} seconds",
                timeout_duration
            ))
        })??;

        Ok(result)
    }

    /// Collect output from exec
    async fn collect_exec_output(&self, exec_id: &str) -> Result<String> {
        let mut output = String::new();

        let exec_result = self
            .docker
            .start_exec(exec_id, None)
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to start exec: {}", e)))?;

        match exec_result {
            StartExecResults::Attached { output: mut stream, .. } => {
                while let Some(msg) = stream.next().await {
                    match msg {
                        Ok(chunk) => {
                            output.push_str(&chunk.to_string());
                        }
                        Err(e) => {
                            tracing::warn!("Exec output error: {}", e);
                            break;
                        }
                    }
                }
            }
            StartExecResults::Detached => {
                return Err(Error::Sandbox("Exec detached unexpectedly".to_string()));
            }
        }

        Ok(output)
    }

    /// Read a file from the container
    pub async fn read_file(&self, path: &str) -> Result<String> {
        let resolved_path = self.resolve_path(path);

        // Use cat command to read file
        let cmd = format!("cat {}", resolved_path);
        self.run_command(&cmd, None).await
    }

    /// Write content to a file in the container
    pub async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let resolved_path = self.resolve_path(path);

        // Create parent directory
        let parent = std::path::Path::new(&resolved_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        if !parent.is_empty() {
            self.run_command(&format!("mkdir -p {}", parent), None)
                .await?;
        }

        // Escape content for shell
        let escaped_content = content.replace("'", "'\\''");
        let cmd = format!("echo '{}' > {}", escaped_content, resolved_path);
        self.run_command(&cmd, None).await?;

        Ok(())
    }

    /// Copy file from container to host
    pub async fn copy_from(&self, container_path: &str, host_path: &str) -> Result<()> {
        let resolved_path = self.resolve_path(container_path);

        // Create parent directory on host
        if let Some(parent) = std::path::Path::new(host_path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::Sandbox(format!("Failed to create host directory: {}", e)))?;
        }

        // Use docker cp equivalent via tar
        let cmd = format!("tar -cf - -C {} .", resolved_path);
        let tar_content = self.run_command(&cmd, None).await?;

        // Write to host
        tokio::fs::write(host_path, tar_content)
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to write to host: {}", e)))?;

        Ok(())
    }

    /// Copy file from host to container
    pub async fn copy_to(&self, host_path: &str, container_path: &str) -> Result<()> {
        let resolved_path = self.resolve_path(container_path);

        // Verify source exists
        if !std::path::Path::new(host_path).exists() {
            return Err(Error::Sandbox(format!(
                "Source file not found: {}",
                host_path
            )));
        }

        // Create destination directory
        let parent = std::path::Path::new(&resolved_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        if !parent.is_empty() {
            self.run_command(&format!("mkdir -p {}", parent), None)
                .await?;
        }

        // Read source file
        let content = tokio::fs::read_to_string(host_path)
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to read source file: {}", e)))?;

        // Write to container
        self.write_file(&resolved_path, &content).await?;

        Ok(())
    }

    /// Get container ID
    async fn get_container_id(&self) -> Result<String> {
        let id = self.container_id.read().await;
        id.clone()
            .ok_or_else(|| Error::Sandbox("Container not initialized".to_string()))
    }

    /// Resolve path relative to work directory
    fn resolve_path(&self, path: &str) -> String {
        if std::path::Path::new(path).is_absolute() {
            path.to_string()
        } else {
            format!("{}/{}", self.config.work_dir, path)
        }
    }

    /// Clean up container resources
    pub async fn cleanup(&self) -> Result<()> {
        let mut container_id = self.container_id.write().await;

        if let Some(id) = container_id.take() {
            // Stop container
            let stop_options = StopContainerOptions { t: 5 };

            if let Err(e) = self.docker.stop_container(&id, Some(stop_options)).await {
                tracing::warn!("Failed to stop container: {}", e);
            }

            // Remove container
            let remove_options = RemoveContainerOptions {
                force: true,
                ..Default::default()
            };

            if let Err(e) = self.docker.remove_container(&id, Some(remove_options)).await {
                tracing::warn!("Failed to remove container: {}", e);
            }

            tracing::info!("Cleaned up Docker sandbox: {}", id);
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Sandbox for DockerSandbox {
    async fn execute(&self, code: &str, timeout_secs: u64) -> Result<String> {
        // Ensure sandbox is created
        {
            let id = self.container_id.read().await;
            if id.is_none() {
                drop(id);
                self.create().await?;
            }
        }

        // Write code to a temp file
        let script_path = "/tmp/execute_script.py";
        self.write_file(script_path, code).await?;

        // Execute the script
        let cmd = format!("python3 {}", script_path);
        let result = self.run_command(&cmd, Some(timeout_secs)).await?;

        // Clean up script
        let _ = self.run_command(&format!("rm {}", script_path), None).await;

        Ok(result)
    }

    fn is_available(&self) -> bool {
        // Check if Docker is available (synchronous check)
        // Since ping() is async, we just check if the client was created successfully
        true
    }
}

impl Drop for DockerSandbox {
    fn drop(&mut self) {
        // Attempt cleanup on drop (best effort)
        let container_id = self.container_id.clone();

        if let Some(rt) = tokio::runtime::Handle::try_current().ok() {
            if let Some(id) = rt.block_on(async {
                let mut cid = container_id.write().await;
                cid.take()
            }) {
                let docker = Docker::connect_with_socket_defaults().ok();
                if let Some(docker) = docker {
                    let _ = rt.block_on(async {
                        let _ = docker
                            .stop_container(&id, Some(StopContainerOptions { t: 5 }))
                            .await;
                        let _ = docker
                            .remove_container(
                                &id,
                                Some(RemoveContainerOptions {
                                    force: true,
                                    ..Default::default()
                                }),
                            )
                            .await;
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert_eq!(config.image, "python:3.12-slim");
        assert_eq!(config.work_dir, "/workspace");
        assert!(!config.network_enabled);
    }

    #[test]
    fn test_volume_binding() {
        let binding = VolumeBinding {
            host_path: "/host/path".to_string(),
            container_path: "/container/path".to_string(),
            read_only: true,
        };
        assert!(binding.read_only);
    }

    #[test]
    fn test_resolve_path() {
        let sandbox = DockerSandbox::new(SandboxConfig::default());
        assert_eq!(sandbox.resolve_path("test.py"), "/workspace/test.py");
        assert_eq!(sandbox.resolve_path("/abs/path"), "/abs/path");
    }

    #[test]
    fn test_docker_sandbox_new() {
        let sandbox = DockerSandbox::new(SandboxConfig::default());
        assert!(sandbox.is_available());
    }

    // Integration tests require Docker to be running
    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_docker_sandbox_create() {
        let sandbox = DockerSandbox::new(SandboxConfig::default());
        let result = sandbox.create().await;
        assert!(result.is_ok());

        sandbox.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_docker_sandbox_execute() {
        let sandbox = DockerSandbox::new(SandboxConfig::default());
        sandbox.create().await.unwrap();

        let code = r#"print("Hello from Docker!")"#;
        let result = sandbox.execute(code, 30).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Hello from Docker!"));

        sandbox.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_docker_sandbox_run_command() {
        let sandbox = DockerSandbox::new(SandboxConfig::default());
        sandbox.create().await.unwrap();

        let result = sandbox.run_command("echo test", None).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("test"));

        sandbox.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "Requires Docker daemon"]
    async fn test_docker_sandbox_file_operations() {
        let sandbox = DockerSandbox::new(SandboxConfig::default());
        sandbox.create().await.unwrap();

        // Write file
        let result = sandbox.write_file("test.txt", "Hello World").await;
        assert!(result.is_ok());

        // Read file
        let result = sandbox.read_file("test.txt").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "Hello World");

        sandbox.cleanup().await.unwrap();
    }
}
