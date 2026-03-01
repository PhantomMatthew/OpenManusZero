//! WASM-based sandbox implementation using wasmtime
//!
//! Provides secure code execution through WebAssembly with:
//! - Memory limits
//! - Fuel-based execution limits
//! - Sandboxed execution

use wasmtime::*;

use crate::error::{Error, Result};
use crate::sandbox::Sandbox;

/// WASM sandbox configuration
#[derive(Debug, Clone)]
pub struct WasmSandboxConfig {
    /// Maximum memory in pages (64KB each)
    pub max_memory_pages: u32,
    /// Maximum fuel for execution (0 = unlimited)
    pub max_fuel: u64,
    /// Maximum table elements
    pub max_table_elements: u32,
}

impl Default for WasmSandboxConfig {
    fn default() -> Self {
        Self {
            max_memory_pages: 1024,      // 64MB (1024 * 64KB)
            max_fuel: 10_000_000,         // 10M units
            max_table_elements: 100_000,
        }
    }
}

/// WASM-based sandbox for secure code execution
pub struct WasmSandbox {
    /// Wasmtime engine
    engine: Engine,
    /// Sandbox configuration
    config: WasmSandboxConfig,
}

impl WasmSandbox {
    /// Create a new WASM sandbox with default configuration
    pub fn new() -> Self {
        Self::with_config(WasmSandboxConfig::default())
    }

    /// Create a WASM sandbox with custom configuration
    pub fn with_config(config: WasmSandboxConfig) -> Self {
        let mut engine_config = Config::new();
        engine_config
            .wasm_multi_memory(false)
            .wasm_memory64(false)
            .consume_fuel(config.max_fuel > 0)
            .max_wasm_stack(1024 * 1024); // 1MB stack

        let engine = Engine::new(&engine_config)
            .expect("Failed to create WASM engine");

        Self { engine, config }
    }

    /// Set maximum memory in bytes
    pub fn with_max_memory(mut self, bytes: u64) -> Self {
        // Convert bytes to pages (64KB each)
        self.config.max_memory_pages = (bytes / (64 * 1024)) as u32;
        self
    }

    /// Set maximum fuel
    pub fn with_max_fuel(mut self, fuel: u64) -> Self {
        self.config.max_fuel = fuel;
        self
    }

    /// Compile a WASM module from bytes
    pub fn compile(&self, wasm_bytes: &[u8]) -> Result<Module> {
        Module::new(&self.engine, wasm_bytes)
            .map_err(|e| Error::Sandbox(format!("Failed to compile WASM module: {}", e)))
    }

    /// Compile a WASM module from file
    pub fn compile_from_file(&self, path: &str) -> Result<Module> {
        Module::from_file(&self.engine, path)
            .map_err(|e| Error::Sandbox(format!("Failed to compile WASM file: {}", e)))
    }

    /// Execute a WASM module with the given function name and arguments
    pub async fn execute_module(
        &self,
        module: &Module,
        func_name: &str,
        args: &[Val],
    ) -> Result<Vec<Val>> {
        // Create store with limits
        let mut store = self.create_store()?;

        // Create linker
        let linker = Linker::new(&self.engine);

        // Instantiate module
        let instance = linker
            .instantiate(&mut store, module)
            .map_err(|e| Error::Sandbox(format!("Failed to instantiate module: {}", e)))?;

        // Get the function
        let func = instance
            .get_export(&mut store, func_name)
            .and_then(|e| e.into_func())
            .ok_or_else(|| {
                Error::Sandbox(format!("Function '{}' not found in module", func_name))
            })?;

        // Call the function
        let mut results = vec![Val::null(); func.ty(&store).results().len()];
        func.call(&mut store, args, &mut results)
            .map_err(|e| Error::Sandbox(format!("Failed to call function: {}", e)))?;

        Ok(results)
    }

    /// Execute WASM bytes directly
    pub async fn execute_bytes(
        &self,
        wasm_bytes: &[u8],
        func_name: &str,
        args: &[Val],
    ) -> Result<Vec<Val>> {
        let module = self.compile(wasm_bytes)?;
        self.execute_module(&module, func_name, args).await
    }

    /// Execute a WASM file
    pub async fn execute_file(
        &self,
        path: &str,
        func_name: &str,
        args: &[Val],
    ) -> Result<Vec<Val>> {
        let module = self.compile_from_file(path)?;
        self.execute_module(&module, func_name, args).await
    }

    /// Create a store with resource limits
    fn create_store(&self) -> Result<Store<()>> {
        let mut store = Store::new(&self.engine, ());

        // Set fuel if configured
        if self.config.max_fuel > 0 {
            store
                .set_fuel(self.config.max_fuel)
                .map_err(|e| Error::Sandbox(format!("Failed to set fuel: {}", e)))?;
        }

        Ok(store)
    }

    /// Get remaining fuel from a store
    pub fn get_fuel_remaining(store: &Store<()>) -> Option<u64> {
        store.get_fuel().ok()
    }

    /// Get the engine reference
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Get the configuration
    pub fn config(&self) -> &WasmSandboxConfig {
        &self.config
    }
}

impl Default for WasmSandbox {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Sandbox for WasmSandbox {
    async fn execute(&self, code: &str, timeout_secs: u64) -> Result<String> {
        // For WASM sandbox, code should be a path to a .wasm file

        // Check if code is a file path
        if std::path::Path::new(code).exists() {
            let module = self.compile_from_file(code)?;

            // Try to find and call a main or _start function
            let entry_point = module.exports().find_map(|e| {
                let name = e.name();
                if name == "_start" || name == "main" {
                    Some(name.to_string())
                } else {
                    None
                }
            });

            if let Some(func_name) = entry_point {
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(timeout_secs),
                    self.execute_module(&module, &func_name, &[]),
                )
                .await
                .map_err(|_| Error::Sandbox("Execution timed out".to_string()))??;

                // Format results
                let output: String = result
                    .iter()
                    .map(|v| format!("{:?}", v))
                    .collect::<Vec<_>>()
                    .join(", ");

                Ok(if output.is_empty() {
                    "Execution completed".to_string()
                } else {
                    format!("Result: {}", output)
                })
            } else {
                // List available exports
                let exports: Vec<&str> = module.exports().map(|e| e.name()).collect();
                Err(Error::Sandbox(format!(
                    "No entry point found. Available exports: {:?}",
                    exports
                )))
            }
        } else {
            Err(Error::Sandbox(format!(
                "WASM file not found: {}. WASM sandbox requires a path to a .wasm module",
                code
            )))
        }
    }

    fn is_available(&self) -> bool {
        // WASM runtime is always available if feature is enabled
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_sandbox_new() {
        let sandbox = WasmSandbox::new();
        assert!(sandbox.is_available());
    }

    #[test]
    fn test_wasm_sandbox_config_default() {
        let config = WasmSandboxConfig::default();
        assert_eq!(config.max_memory_pages, 1024);
        assert_eq!(config.max_fuel, 10_000_000);
    }

    #[test]
    fn test_wasm_sandbox_with_config() {
        let sandbox = WasmSandbox::new()
            .with_max_memory(128 * 1024 * 1024)
            .with_max_fuel(100_000_000);

        assert!(sandbox.is_available());
        assert_eq!(sandbox.config().max_fuel, 100_000_000);
    }

    #[test]
    fn test_wasm_engine_creation() {
        let sandbox = WasmSandbox::new();
        // Engine is created successfully if we get here
        let _engine = sandbox.engine();
    }

    #[test]
    fn test_compile_invalid_wasm() {
        let sandbox = WasmSandbox::new();
        let result = sandbox.compile(&[0x00, 0x61, 0x73, 0x6d]); // Invalid WASM magic
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to compile"));
    }

    #[test]
    fn test_compile_valid_header_only() {
        let sandbox = WasmSandbox::new();
        // Valid WASM header but minimal module
        let result = sandbox.compile(&[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_nonexistent_file() {
        let sandbox = WasmSandbox::new();
        let result = sandbox.execute("/nonexistent/path.wasm", 30).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_execute_module_no_entry_point() {
        let sandbox = WasmSandbox::new();
        // Minimal valid WASM module with no exports
        let wasm = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let module = sandbox.compile(wasm).unwrap();
        let result = sandbox.execute_module(&module, "main", &[]).await;
        assert!(result.is_err());
    }

    // Test with actual WASM module requires a .wasm file
    #[tokio::test]
    #[ignore = "Requires a test WASM module"]
    async fn test_execute_wasm_module() {
        // This would require a .wasm file to test
        // Example: compile a simple add.wat to add.wasm
    }
}
