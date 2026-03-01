//! Sandbox module for secure code execution
//!
//! # Available Sandbox Types
//!
//! - [`LocalSandbox`] - Local process isolation (always available)
//! - [`DockerSandbox`] - Docker container isolation (requires `docker-sandbox` feature)
//! - [`WasmSandbox`] - WebAssembly isolation (requires `wasm-sandbox` feature)
//! - [`DaytonaSandbox`] - Remote cloud sandbox via Daytona.io (requires `daytona` feature)
//!
//! # Example
//!
//! ```rust,ignore
//! use openmanus::sandbox::{DockerSandbox, Sandbox, SandboxConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = SandboxConfig::default();
//!     let sandbox = DockerSandbox::new(config);
//!     sandbox.create().await?;
//!
//!     let result = sandbox.execute("print('Hello!')", 30).await?;
//!     println!("Output: {}", result);
//!
//!     sandbox.cleanup().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Daytona Example
//!
//! ```rust,ignore
//! use openmanus::sandbox::{DaytonaSandbox, DaytonaConfig, Sandbox};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Set environment variables:
//!     // DAYTONA_API_KEY, DAYTONA_SERVER_URL, DAYTONA_TARGET
//!     let config = DaytonaConfig::from_env()?;
//!     let sandbox = DaytonaSandbox::new(config);
//!
//!     let result = sandbox.execute("print('Hello from Daytona!')", 60).await?;
//!     println!("Output: {}", result);
//!
//!     sandbox.cleanup().await?;
//!     Ok(())
//! }
//! ```

mod local;
mod manager;

#[cfg(feature = "docker-sandbox")]
mod docker;

#[cfg(feature = "wasm-sandbox")]
mod wasm;

#[cfg(feature = "daytona")]
mod daytona;

pub use local::LocalSandbox;
pub use manager::{ManagerConfig, ManagerStats, SandboxManager};

#[cfg(feature = "docker-sandbox")]
pub use docker::{DockerSandbox, SandboxConfig, VolumeBinding};

#[cfg(feature = "wasm-sandbox")]
pub use wasm::{WasmSandbox, WasmSandboxConfig};

#[cfg(feature = "daytona")]
pub use daytona::{
    DaytonaConfig, DaytonaConfigBuilder, DaytonaSandbox, PreviewLink, Resources, SandboxInfo,
    SandboxState, SessionExecuteRequest,
};

/// Sandbox trait for secure code execution
#[async_trait::async_trait]
pub trait Sandbox: Send + Sync {
    /// Execute code in the sandbox
    async fn execute(&self, code: &str, timeout_secs: u64) -> crate::Result<String>;

    /// Check if the sandbox is available
    fn is_available(&self) -> bool;
}
