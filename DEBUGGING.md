# Rust Debugging Guide

This guide covers local debugging techniques for the OpenManus Rust project.

## Table of Contents

- [Setup](#setup)
- [Building](#building)
- [Running](#running)
- [Logging](#logging)
- [Running Tests](#running-tests)
- [IDE Debugging](#ide-debugging)
- [Debugging Techniques](#debugging-techniques)
- [Profiling](#profiling)
- [Troubleshooting](#troubleshooting)

---

## Setup

### Prerequisites

- Rust toolchain (automatically installed via `rust-toolchain.toml`)
- For `computer-use` feature: system libraries for screen capture

### Install Toolchain

```bash
# The toolchain is pinned in rust-toolchain.toml (1.72)
rustup show   # Automatically installs the correct toolchain
```

### Configuration

```bash
# Copy example config
cp config/config.example.toml config/config.toml

# Edit with your API keys and settings
```

---

## Building

### Debug Build (Default)

```bash
# Fast compilation, includes debug symbols, no optimizations
cargo build
```

### Release Build

```bash
# Slower compilation, optimized, stripped symbols
cargo build --release
```

### Build with Features

```bash
# Single feature
cargo build --features computer-use

# Multiple features
cargo build --features "browser,mcp,computer-use"

# All default features are: browser, server, mcp
cargo build --all-features
```

### Check Without Building

```bash
# Fast syntax and type checking
cargo check

# With specific features
cargo check --features computer-use
```

---

## Running

### Basic Run

```bash
# Debug build
cargo run

# With arguments
cargo run -- --help

# Release build
cargo run --release
```

### Run with Features

```bash
cargo run --features computer-use
cargo run --features "browser,mcp,computer-use"
```

### Environment Variables

```bash
# Enable backtrace on panic
RUST_BACKTRACE=1 cargo run

# Full backtrace
RUST_BACKTRACE=full cargo run
```

---

## Logging

The project uses the `tracing` crate for logging. Control log levels via the `RUST_LOG` environment variable.

### Log Levels

From most to least verbose: `trace`, `debug`, `info`, `warn`, `error`

### Examples

```bash
# All debug logs
RUST_LOG=debug cargo run

# Only openmanus crate debug logs
RUST_LOG=openmanus=debug cargo run

# Trace level for specific module
RUST_LOG=openmanus::tool::computer_use=trace cargo run

# Multiple targets with different levels
RUST_LOG=openmanus=debug,reqwest=warn,tokio=info cargo run

# Combine with backtrace
RUST_LOG=debug RUST_BACKTRACE=1 cargo run
```

### Adding Log Statements

```rust
use tracing::{trace, debug, info, warn, error};

fn example() {
    trace!("Very detailed information");
    debug!("Debug information: {:?}", some_value);
    info!("General information");
    warn!("Warning: something might be wrong");
    error!("Error occurred: {}", error_message);
}
```

---

## Running Tests

### All Tests

```bash
cargo test
```

### Specific Test

```bash
# By name (substring match)
cargo test test_computer_use

# Exact match
cargo test tests::test_computer_use_tool_new -- --exact
```

### Tests in Module

```bash
# All tests in a module
cargo test tool::computer_use

# Integration tests
cargo test --test property_tests
```

### Test Options

```bash
# Show output (println!, dbg!, etc.)
cargo test -- --nocapture

# Run ignored tests
cargo test -- --include-ignored

# Run only ignored tests
cargo test -- --ignored

# List tests without running
cargo test -- --list

# Run tests in parallel (default) or sequentially
cargo test -- --test-threads=1
```

### Test with Features

```bash
cargo test --features computer-use
cargo test --features computer-use -- --nocapture
```

### Test with Logging

```bash
RUST_LOG=debug cargo test test_name -- --nocapture
```

---

## IDE Debugging

### VS Code with CodeLLDB

1. Install the **CodeLLDB** extension

2. Create `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Debug openmanus",
      "type": "lldb",
      "request": "launch",
      "program": "${workspaceFolder}/target/debug/openmanus",
      "args": [],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "debug",
        "RUST_BACKTRACE": "1"
      },
      "preLaunchTask": "cargo build"
    },
    {
      "name": "Debug with computer-use",
      "type": "lldb",
      "request": "launch",
      "program": "${workspaceFolder}/target/debug/openmanus",
      "args": [],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "openmanus=debug",
        "RUST_BACKTRACE": "1"
      },
      "preLaunchTask": "cargo build (computer-use)"
    },
    {
      "name": "Debug Unit Test",
      "type": "lldb",
      "request": "launch",
      "cargo": {
        "args": ["test", "--no-run", "--lib"],
        "filter": {
          "kind": "lib"
        }
      },
      "args": ["--test-threads=1"],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "debug",
        "RUST_BACKTRACE": "1"
      }
    }
  ]
}
```

3. Create `.vscode/tasks.json`:

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "cargo build",
      "type": "shell",
      "command": "cargo build",
      "group": "build",
      "problemMatcher": ["$rustc"]
    },
    {
      "label": "cargo build (computer-use)",
      "type": "shell",
      "command": "cargo build --features computer-use",
      "group": "build",
      "problemMatcher": ["$rustc"]
    },
    {
      "label": "cargo test",
      "type": "shell",
      "command": "cargo test",
      "group": "test",
      "problemMatcher": ["$rustc"]
    }
  ]
}
```

### IntelliJ IDEA / CLion

1. Install the **Rust** plugin
2. Open the project (it detects `Cargo.toml` automatically)
3. Click the gutter icon next to `fn main()` or any `#[test]` function
4. Select "Debug" to run with debugger

### Setting Breakpoints

- Click in the gutter (left margin) next to the line number
- Or use `F9` (VS Code) / `Cmd+F8` (IntelliJ) on the current line

---

## Debugging Techniques

### dbg! Macro

Quick debug printing that shows file, line, expression, and value:

```rust
let x = 42;
dbg!(x);  // Prints: [src/main.rs:2] x = 42

// Works with expressions
let result = dbg!(2 + 2);  // Prints: [src/main.rs:5] 2 + 2 = 4

// Chain multiple values
dbg!(x, result, x + result);
```

### println! for Quick Debugging

```rust
println!("Value: {:?}", some_value);      // Debug format
println!("Value: {:#?}", some_struct);    // Pretty-printed debug format
println!("Pointer: {:p}", &some_value);   // Pointer address
```

### assert! Macros

```rust
assert!(condition, "Condition failed!");
assert_eq!(left, right, "Values not equal: {} vs {}", left, right);
assert_ne!(a, b, "Values should differ");

// Debug-only assertions (removed in release builds)
debug_assert!(expensive_check());
```

### Conditional Compilation for Debug Code

```rust
#[cfg(debug_assertions)]
fn debug_only_function() {
    println!("This only runs in debug builds");
}

// Or inline
if cfg!(debug_assertions) {
    println!("Debug mode");
}
```

### Inspecting Panics

```rust
// Set a custom panic hook
std::panic::set_hook(Box::new(|panic_info| {
    eprintln!("Panic occurred: {:?}", panic_info);
    // Add custom logging here
}));
```

### Async Debugging

For async code, use `tokio-console` for runtime inspection:

```bash
# Install tokio-console
cargo install tokio-console

# Run your app with console support (requires adding tokio-console dependency)
RUSTFLAGS="--cfg tokio_unstable" cargo run
```

---

## Profiling

### Benchmarks

```bash
# Run all benchmarks
cargo bench

# Specific benchmark
cargo bench agent_bench
```

### CPU Profiling with flamegraph

```bash
# Install
cargo install flamegraph

# Generate flamegraph (requires root on Linux)
cargo flamegraph --bin openmanus

# On macOS, you may need:
sudo cargo flamegraph --bin openmanus
```

### Memory Profiling

```bash
# Using valgrind (Linux)
cargo build --release
valgrind --tool=massif ./target/release/openmanus

# Using heaptrack (Linux)
heaptrack ./target/release/openmanus
```

---

## Troubleshooting

### Common Issues

#### Build Fails with Missing System Libraries

For the `computer-use` feature on Linux:

```bash
# Ubuntu/Debian
sudo apt-get install libx11-dev libxcb1-dev libxkbcommon-dev

# Fedora
sudo dnf install libX11-devel libxcb-devel libxkbcommon-devel
```

#### Tests Fail Intermittently

Run tests sequentially to identify race conditions:

```bash
cargo test -- --test-threads=1
```

#### Stack Overflow in Debug Builds

Debug builds use more stack space. Increase stack size:

```bash
# Linux/macOS
RUST_MIN_STACK=8388608 cargo run

# Or in code
std::thread::Builder::new()
    .stack_size(8 * 1024 * 1024)
    .spawn(|| { /* ... */ });
```

#### Slow Compilation

```bash
# Use sccache for caching
cargo install sccache
export RUSTC_WRAPPER=sccache

# Or use mold linker (Linux)
sudo apt install mold
RUSTFLAGS="-C link-arg=-fuse-ld=mold" cargo build
```

### Getting Help

```bash
# Compiler error explanation
rustc --explain E0382

# Cargo help
cargo help build
cargo help test
```

---

## Quick Reference

| Task | Command |
|------|---------|
| Build (debug) | `cargo build` |
| Build (release) | `cargo build --release` |
| Run | `cargo run` |
| Test all | `cargo test` |
| Test one | `cargo test test_name` |
| Test with output | `cargo test -- --nocapture` |
| Check syntax | `cargo check` |
| Lint | `cargo clippy` |
| Format | `cargo fmt` |
| Debug logs | `RUST_LOG=debug cargo run` |
| Backtrace | `RUST_BACKTRACE=1 cargo run` |
| Benchmark | `cargo bench` |
