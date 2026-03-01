//! Daytona sandbox implementation
//!
//! Provides remote sandbox environment via Daytona.io API with:
//! - Remote sandbox creation and management
//! - Code execution in isolated environments
//! - Browser automation support (VNC, Chrome)
//! - Auto-stop and auto-archive capabilities
//!
//! # Example
//!
//! ```rust,ignore
//! use openmanus::sandbox::{DaytonaSandbox, DaytonaConfig, Sandbox};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = DaytonaConfig::from_env()?;
//!     let sandbox = DaytonaSandbox::new(config);
//!
//!     let result = sandbox.execute("print('Hello!')", 30).await?;
//!     println!("Output: {}", result);
//!
//!     sandbox.cleanup().await?;
//!     Ok(())
//! }
//! ```

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::sandbox::Sandbox;

/// Daytona sandbox configuration
#[derive(Debug, Clone)]
pub struct DaytonaConfig {
    /// API key for Daytona
    pub api_key: String,
    /// Server URL (e.g., "https://app.daytona.io")
    pub server_url: String,
    /// Target environment (e.g., "us")
    pub target: String,
    /// Sandbox image name
    pub image: String,
    /// VNC password for browser sessions
    pub vnc_password: String,
    /// Auto-stop interval in minutes (0 = disabled)
    pub auto_stop_interval: i32,
    /// Auto-archive interval in minutes (0 = disabled)
    pub auto_archive_interval: i32,
    /// CPU resources (cores)
    pub cpu: i32,
    /// Memory resources (GB)
    pub memory: i32,
    /// Disk resources (GB)
    pub disk: i32,
}

impl Default for DaytonaConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            server_url: "https://app.daytona.io".to_string(),
            target: "us".to_string(),
            image: "openmanus/browser-use:latest".to_string(),
            vnc_password: "openmanus".to_string(),
            auto_stop_interval: 15,
            auto_archive_interval: 24 * 60,
            cpu: 2,
            memory: 4,
            disk: 5,
        }
    }
}

impl DaytonaConfig {
    /// Create config from environment variables
    pub fn from_env() -> Result<Self> {
        let config = Self {
            api_key: std::env::var("DAYTONA_API_KEY").unwrap_or_default(),
            server_url: std::env::var("DAYTONA_SERVER_URL")
                .unwrap_or_else(|_| "https://app.daytona.io".to_string()),
            target: std::env::var("DAYTONA_TARGET").unwrap_or_else(|_| "us".to_string()),
            image: std::env::var("DAYTONA_IMAGE")
                .unwrap_or_else(|_| "openmanus/browser-use:latest".to_string()),
            vnc_password: std::env::var("DAYTONA_VNC_PASSWORD")
                .unwrap_or_else(|_| "openmanus".to_string()),
            ..Default::default()
        };

        if config.api_key.is_empty() {
            return Err(Error::Sandbox(
                "DAYTONA_API_KEY environment variable not set".to_string(),
            ));
        }

        Ok(config)
    }

    /// Builder pattern for config
    pub fn builder() -> DaytonaConfigBuilder {
        DaytonaConfigBuilder::default()
    }
}

/// Builder for DaytonaConfig
#[derive(Default)]
pub struct DaytonaConfigBuilder {
    config: DaytonaConfig,
}

impl DaytonaConfigBuilder {
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.config.api_key = api_key.into();
        self
    }

    pub fn server_url(mut self, url: impl Into<String>) -> Self {
        self.config.server_url = url.into();
        self
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.config.target = target.into();
        self
    }

    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.config.image = image.into();
        self
    }

    pub fn vnc_password(mut self, password: impl Into<String>) -> Self {
        self.config.vnc_password = password.into();
        self
    }

    pub fn resources(mut self, cpu: i32, memory: i32, disk: i32) -> Self {
        self.config.cpu = cpu;
        self.config.memory = memory;
        self.config.disk = disk;
        self
    }

    pub fn auto_stop(mut self, minutes: i32) -> Self {
        self.config.auto_stop_interval = minutes;
        self
    }

    pub fn auto_archive(mut self, minutes: i32) -> Self {
        self.config.auto_archive_interval = minutes;
        self
    }

    pub fn build(self) -> Result<DaytonaConfig> {
        if self.config.api_key.is_empty() {
            return Err(Error::Sandbox("API key is required".to_string()));
        }
        Ok(self.config)
    }
}

/// Sandbox state enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SandboxState {
    Creating,
    Starting,
    Running,
    Stopping,
    Stopped,
    Archiving,
    Archived,
    Destroying,
    Error,
}

/// Resources configuration for sandbox creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resources {
    pub cpu: i32,
    pub memory: i32,
    pub disk: i32,
}

/// Parameters for creating a sandbox from image
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSandboxParams {
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Resources>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_stop_interval: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_interval: Option<i32>,
}

/// Sandbox response from API
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxInfo {
    pub id: String,
    pub state: SandboxState,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Preview link response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewLink {
    pub url: String,
    pub port: u16,
}

/// Session execute request
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionExecuteRequest {
    pub command: String,
    #[serde(rename = "async")]
    pub var_async: bool,
}

/// Execute response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteResponse {
    pub exit_code: i32,
    pub output: String,
}

/// Daytona sandbox client
pub struct DaytonaSandbox {
    /// HTTP client
    client: Client,
    /// Configuration
    config: DaytonaConfig,
    /// Current sandbox info
    sandbox_info: Arc<RwLock<Option<SandboxInfo>>>,
    /// Session ID for supervisord
    session_id: Arc<RwLock<Option<String>>>,
}

impl DaytonaSandbox {
    /// Create a new Daytona sandbox client
    pub fn new(config: DaytonaConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            config,
            sandbox_info: Arc::new(RwLock::new(None)),
            session_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Get API headers
    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.config.api_key).parse().unwrap(),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers.insert("X-Daytona-Target", self.config.target.parse().unwrap());
        headers
    }

    /// Get base URL for API
    fn api_url(&self, path: &str) -> String {
        format!("{}/api{}", self.config.server_url, path)
    }

    /// Create a new sandbox
    pub async fn create(&self) -> Result<SandboxInfo> {
        let env_vars = HashMap::from([
            ("CHROME_PERSISTENT_SESSION".to_string(), "true".to_string()),
            ("RESOLUTION".to_string(), "1024x768x24".to_string()),
            ("RESOLUTION_WIDTH".to_string(), "1024".to_string()),
            ("RESOLUTION_HEIGHT".to_string(), "768".to_string()),
            ("VNC_PASSWORD".to_string(), self.config.vnc_password.clone()),
            ("ANONYMIZED_TELEMETRY".to_string(), "false".to_string()),
            ("CHROME_DEBUGGING_PORT".to_string(), "9222".to_string()),
            ("CHROME_DEBUGGING_HOST".to_string(), "localhost".to_string()),
        ]);

        let params = CreateSandboxParams {
            image: self.config.image.clone(),
            public: Some(true),
            labels: None,
            env_vars: Some(env_vars),
            resources: Some(Resources {
                cpu: self.config.cpu,
                memory: self.config.memory,
                disk: self.config.disk,
            }),
            auto_stop_interval: Some(self.config.auto_stop_interval),
            auto_archive_interval: Some(self.config.auto_archive_interval),
        };

        let response = self
            .client
            .post(self.api_url("/sandbox"))
            .headers(self.headers())
            .json(&params)
            .send()
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to create sandbox: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Sandbox(format!(
                "Failed to create sandbox: {} - {}",
                status, body
            )));
        }

        let sandbox: SandboxInfo = response
            .json()
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to parse response: {}", e)))?;

        tracing::info!("Created Daytona sandbox: {}", sandbox.id);

        // Wait for sandbox to be ready
        self.wait_for_running(&sandbox.id).await?;

        // Start supervisord session
        self.start_supervisord_session(&sandbox.id).await?;

        // Store sandbox info
        let mut info = self.sandbox_info.write().await;
        *info = Some(sandbox.clone());

        Ok(sandbox)
    }

    /// Get sandbox by ID
    pub async fn get(&self, sandbox_id: &str) -> Result<SandboxInfo> {
        let response = self
            .client
            .get(self.api_url(&format!("/sandbox/{}", sandbox_id)))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to get sandbox: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Sandbox(format!(
                "Failed to get sandbox: {} - {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to parse response: {}", e)))
    }

    /// Start a stopped/archived sandbox
    pub async fn start(&self, sandbox_id: &str) -> Result<()> {
        let response = self
            .client
            .post(self.api_url(&format!("/sandbox/{}/start", sandbox_id)))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to start sandbox: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Sandbox(format!(
                "Failed to start sandbox: {} - {}",
                status, body
            )));
        }

        tracing::info!("Started Daytona sandbox: {}", sandbox_id);
        Ok(())
    }

    /// Stop a running sandbox
    pub async fn stop(&self, sandbox_id: &str) -> Result<()> {
        let response = self
            .client
            .post(self.api_url(&format!("/sandbox/{}/stop", sandbox_id)))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to stop sandbox: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Sandbox(format!(
                "Failed to stop sandbox: {} - {}",
                status, body
            )));
        }

        tracing::info!("Stopped Daytona sandbox: {}", sandbox_id);
        Ok(())
    }

    /// Delete a sandbox
    pub async fn delete(&self, sandbox_id: &str) -> Result<()> {
        let response = self
            .client
            .delete(self.api_url(&format!("/sandbox/{}", sandbox_id)))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to delete sandbox: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Sandbox(format!(
                "Failed to delete sandbox: {} - {}",
                status, body
            )));
        }

        tracing::info!("Deleted Daytona sandbox: {}", sandbox_id);
        Ok(())
    }

    /// Get or ensure sandbox is running
    pub async fn ensure_sandbox(&self) -> Result<SandboxInfo> {
        let info = self.sandbox_info.read().await.clone();

        match info {
            Some(sandbox) => {
                // Check current state
                let current = self.get(&sandbox.id).await?;

                match current.state {
                    SandboxState::Archived | SandboxState::Stopped => {
                        tracing::info!(
                            "Sandbox is in {:?} state. Starting...",
                            current.state
                        );
                        self.start(&sandbox.id).await?;
                        self.wait_for_running(&sandbox.id).await?;
                        self.start_supervisord_session(&sandbox.id).await?;
                        self.get(&sandbox.id).await
                    }
                    _ => Ok(current),
                }
            }
            None => self.create().await,
        }
    }

    /// Wait for sandbox to be in running state
    async fn wait_for_running(&self, sandbox_id: &str) -> Result<()> {
        let max_attempts = 60;
        let delay = Duration::from_secs(5);

        for _ in 0..max_attempts {
            let sandbox = self.get(sandbox_id).await?;

            match sandbox.state {
                SandboxState::Running => {
                    tracing::info!("Sandbox {} is now running", sandbox_id);
                    return Ok(());
                }
                SandboxState::Error => {
                    return Err(Error::Sandbox(format!(
                        "Sandbox {} entered error state",
                        sandbox_id
                    )));
                }
                _ => {
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(Error::Sandbox(format!(
            "Timeout waiting for sandbox {} to be running",
            sandbox_id
        )))
    }

    /// Start supervisord in a session
    async fn start_supervisord_session(&self, sandbox_id: &str) -> Result<()> {
        let session_id = "supervisord-session";

        // Create session
        let response = self
            .client
            .post(self.api_url(&format!(
                "/sandbox/{}/process/session/{}",
                sandbox_id, session_id
            )))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to create session: {}", e)))?;

        if !response.status().is_success() {
            tracing::warn!(
                "Failed to create session (may already exist): {}",
                response.status()
            );
        }

        // Execute supervisord
        let request = SessionExecuteRequest {
            command: "exec /usr/bin/supervisord -n -c /etc/supervisor/conf.d/supervisord.conf"
                .to_string(),
            var_async: true,
        };

        let response = self
            .client
            .post(self.api_url(&format!(
                "/sandbox/{}/process/session/{}/execute",
                sandbox_id, session_id
            )))
            .headers(self.headers())
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to start supervisord: {}", e)))?;

        if !response.status().is_success() {
            tracing::warn!(
                "Failed to start supervisord: {}",
                response.status()
            );
        } else {
            tracing::info!("Started supervisord in session {}", session_id);
        }

        // Wait for supervisord to initialize
        tokio::time::sleep(Duration::from_secs(25)).await;

        // Store session ID
        let mut sid = self.session_id.write().await;
        *sid = Some(session_id.to_string());

        Ok(())
    }

    /// Execute a command in the sandbox
    pub async fn execute_command(&self, command: &str, timeout_secs: u64) -> Result<String> {
        let sandbox = self.ensure_sandbox().await?;

        let request = SessionExecuteRequest {
            command: command.to_string(),
            var_async: false,
        };

        let url = if let Some(session_id) = self.session_id.read().await.as_ref() {
            self.api_url(&format!(
                "/sandbox/{}/process/session/{}/execute",
                sandbox.id, session_id
            ))
        } else {
            self.api_url(&format!("/sandbox/{}/process/execute", sandbox.id))
        };

        let result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            self.client
                .post(url)
                .headers(self.headers())
                .json(&request)
                .send(),
        )
        .await
        .map_err(|_| Error::Sandbox(format!("Command timed out after {} seconds", timeout_secs)))?
        .map_err(|e| Error::Sandbox(format!("Failed to execute command: {}", e)))?;

        if !result.status().is_success() {
            let status = result.status();
            let body = result.text().await.unwrap_or_default();
            return Err(Error::Sandbox(format!(
                "Command execution failed: {} - {}",
                status, body
            )));
        }

        let response: ExecuteResponse = result
            .json()
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to parse response: {}", e)))?;

        Ok(response.output)
    }

    /// Get preview link for a port
    pub async fn get_preview_link(&self, port: u16) -> Result<PreviewLink> {
        let sandbox = self.ensure_sandbox().await?;

        let response = self
            .client
            .get(self.api_url(&format!(
                "/sandbox/{}/preview/{}",
                sandbox.id, port
            )))
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to get preview link: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Sandbox(format!(
                "Failed to get preview link: {} - {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to parse response: {}", e)))
    }

    /// Read a file from the sandbox
    pub async fn read_file(&self, path: &str) -> Result<String> {
        let cmd = format!("cat {}", path);
        self.execute_command(&cmd, 60).await
    }

    /// Write content to a file in the sandbox
    pub async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        // Create parent directory
        let parent = std::path::Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        if !parent.is_empty() {
            let mkdir_cmd = format!("mkdir -p {}", parent);
            self.execute_command(&mkdir_cmd, 30).await?;
        }

        // Escape content for shell
        let escaped_content = content.replace("'", "'\\''");
        let cmd = format!("echo '{}' > {}", escaped_content, path);
        self.execute_command(&cmd, 60).await?;

        Ok(())
    }

    /// Get sandbox ID
    pub async fn sandbox_id(&self) -> Option<String> {
        self.sandbox_info.read().await.as_ref().map(|i| i.id.clone())
    }

    /// Get sandbox state
    pub async fn state(&self) -> Option<SandboxState> {
        self.sandbox_info.read().await.as_ref().map(|i| i.state.clone())
    }

    /// Cleanup sandbox resources
    pub async fn cleanup(&self) -> Result<()> {
        if let Some(sandbox) = self.sandbox_info.read().await.as_ref() {
            self.delete(&sandbox.id).await?;
            let mut info = self.sandbox_info.write().await;
            *info = None;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Sandbox for DaytonaSandbox {
    async fn execute(&self, code: &str, timeout_secs: u64) -> Result<String> {
        // Ensure sandbox is running
        self.ensure_sandbox().await?;

        // Write code to a temp file
        let script_path = "/tmp/execute_script.py";
        self.write_file(script_path, code).await?;

        // Execute the script
        let cmd = format!("python3 {}", script_path);
        let result = self.execute_command(&cmd, timeout_secs).await?;

        // Clean up script
        let _ = self.execute_command(&format!("rm {}", script_path), 10).await;

        Ok(result)
    }

    fn is_available(&self) -> bool {
        // Check if API key is configured
        !self.config.api_key.is_empty()
    }
}

impl Drop for DaytonaSandbox {
    fn drop(&mut self) {
        // Best effort cleanup on drop
        if let Some(rt) = tokio::runtime::Handle::try_current().ok() {
            let sandbox_info = self.sandbox_info.clone();
            let client = self.client.clone();
            let headers = self.headers();
            let api_url_fn = |path: &str| self.api_url(path);

            if let Some(info) = rt.block_on(async { sandbox_info.read().await.clone() }) {
                let sandbox_id = info.id;
                let url = api_url_fn(&format!("/sandbox/{}", sandbox_id));

                let _ = rt.block_on(async {
                    client
                        .delete(&url)
                        .headers(headers)
                        .send()
                        .await
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = DaytonaConfig::default();
        assert_eq!(config.server_url, "https://app.daytona.io");
        assert_eq!(config.target, "us");
        assert_eq!(config.cpu, 2);
        assert_eq!(config.memory, 4);
        assert_eq!(config.disk, 5);
    }

    #[test]
    fn test_config_builder() {
        let config = DaytonaConfig::builder()
            .api_key("test-key")
            .server_url("https://custom.daytona.io")
            .target("eu")
            .image("custom/image:latest")
            .resources(4, 8, 10)
            .auto_stop(30)
            .auto_archive(48 * 60)
            .build()
            .unwrap();

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.server_url, "https://custom.daytona.io");
        assert_eq!(config.target, "eu");
        assert_eq!(config.image, "custom/image:latest");
        assert_eq!(config.cpu, 4);
        assert_eq!(config.memory, 8);
        assert_eq!(config.disk, 10);
        assert_eq!(config.auto_stop_interval, 30);
        assert_eq!(config.auto_archive_interval, 48 * 60);
    }

    #[test]
    fn test_config_builder_missing_api_key() {
        let result = DaytonaConfig::builder().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_state_serde() {
        assert_eq!(
            serde_json::to_string(&SandboxState::Running).unwrap(),
            "\"RUNNING\""
        );
        assert_eq!(
            serde_json::from_str::<SandboxState>("\"STOPPED\"").unwrap(),
            SandboxState::Stopped
        );
    }

    #[test]
    fn test_resources() {
        let resources = Resources {
            cpu: 2,
            memory: 4,
            disk: 5,
        };
        let json = serde_json::to_string(&resources).unwrap();
        assert!(json.contains("\"cpu\":2"));
        assert!(json.contains("\"memory\":4"));
        assert!(json.contains("\"disk\":5"));
    }

    #[test]
    fn test_create_sandbox_params() {
        let params = CreateSandboxParams {
            image: "test:latest".to_string(),
            public: Some(true),
            labels: None,
            env_vars: Some(HashMap::from([("KEY".to_string(), "VALUE".to_string())])),
            resources: Some(Resources {
                cpu: 2,
                memory: 4,
                disk: 5,
            }),
            auto_stop_interval: Some(15),
            auto_archive_interval: Some(1440),
        };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"image\":\"test:latest\""));
        assert!(json.contains("\"public\":true"));
        assert!(json.contains("\"autoStopInterval\":15"));
    }

    #[test]
    fn test_session_execute_request() {
        let request = SessionExecuteRequest {
            command: "echo hello".to_string(),
            var_async: true,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"command\":\"echo hello\""));
        assert!(json.contains("\"async\":true"));
    }

    #[test]
    fn test_daytona_sandbox_new() {
        let config = DaytonaConfig {
            api_key: "test-key".to_string(),
            ..Default::default()
        };
        let sandbox = DaytonaSandbox::new(config);
        assert!(sandbox.is_available());
    }

    #[test]
    fn test_daytona_sandbox_not_available_without_key() {
        let config = DaytonaConfig {
            api_key: String::new(),
            ..Default::default()
        };
        let sandbox = DaytonaSandbox::new(config);
        assert!(!sandbox.is_available());
    }

    #[test]
    fn test_api_url() {
        let config = DaytonaConfig {
            api_key: "test-key".to_string(),
            server_url: "https://app.daytona.io".to_string(),
            ..Default::default()
        };
        let sandbox = DaytonaSandbox::new(config);
        assert_eq!(sandbox.api_url("/sandbox"), "https://app.daytona.io/api/sandbox");
        assert_eq!(sandbox.api_url("/sandbox/123"), "https://app.daytona.io/api/sandbox/123");
    }

    // Integration tests require actual Daytona API access
    #[tokio::test]
    #[ignore = "Requires Daytona API key and network access"]
    async fn test_daytona_sandbox_create() {
        let config = DaytonaConfig::from_env().unwrap();
        let sandbox = DaytonaSandbox::new(config);

        let result = sandbox.create().await;
        assert!(result.is_ok());

        sandbox.cleanup().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "Requires Daytona API key and network access"]
    async fn test_daytona_sandbox_execute() {
        let config = DaytonaConfig::from_env().unwrap();
        let sandbox = DaytonaSandbox::new(config);

        let code = r#"print("Hello from Daytona!")"#;
        let result = sandbox.execute(code, 60).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Hello from Daytona!"));

        sandbox.cleanup().await.unwrap();
    }
}
