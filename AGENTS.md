# AGENTS.md

This file provides guidance to agentic coding agents working in this repository.

## Repository Structure

This repository contains the Rust implementation of OpenManus — an AI agent framework.

```
.
├── src/                 # Library and binary source
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library root
│   ├── agent/           # Agent implementations
│   ├── tool/            # Tool implementations
│   ├── llm/             # LLM client
│   ├── flow/            # Flow orchestration
│   ├── sandbox/         # Sandboxed execution
│   ├── protocol/        # A2A / MCP protocol
│   └── ...
├── tests/               # Integration tests
├── benches/             # Benchmarks
├── fuzz/                # Fuzzing targets
├── config/              # Configuration files
├── docs/                # Documentation
└── examples/            # Example programs
```

---

## Rust

### Setup

```bash
# Toolchain is pinned in rust-toolchain.toml (1.72)
rustup show   # installs toolchain automatically
```

### Building and Running

```bash
cargo build                    # debug build
cargo build --release          # release build
cargo run -- --help            # run binary
cargo run --features mcp       # with optional features
```

### Testing

```bash
# Run all tests
cargo test

# Run a single test by name (substring match)
cargo test test_error_display

# Run tests in a specific file/module
cargo test --test property_tests
cargo test --lib error::tests

# Run with output shown
cargo test -- --nocapture

# Run ignored/slow tests
cargo test -- --include-ignored

# Benchmarks
cargo bench
```

### Linting and Formatting

```bash
# Format code
cargo fmt

# Lint (CI uses -D warnings — all warnings are errors)
cargo clippy -- -D warnings

# Clippy with all features
cargo clippy --all-features -- -D warnings

# Check without building
cargo check --all-features
```

### Rust Code Style

**Edition:** 2021. **MSRV:** 1.72.

**Module docs:** Every module file begins with a `//!` doc comment describing its purpose.

**Error handling:**
- Define domain errors with `thiserror` — use `#[derive(Error, Debug)]`.
- Use the crate-local `Result<T>` alias (`pub type Result<T> = std::result::Result<T, Error>`).
- Use `anyhow` only at binary entry points, not in library code.
- Prefer `?` propagation over `.unwrap()` / `.expect()` in library code.
- Reserve `.expect("reason")` for invariants that should never fail; document the reason.

```rust
// Correct
fn do_thing() -> Result<String> {
    let value = fallible_op()?;
    Ok(value)
}
```

**Async:**
- Use `tokio` as the async runtime.
- Trait methods that are async use `#[async_trait]` from the `async-trait` crate.
- Prefer `Arc<dyn Trait>` for shared ownership of async trait objects.

**Naming:**
- `snake_case` for functions, variables, modules, fields.
- `PascalCase` for types, traits, enums.
- `SCREAMING_SNAKE_CASE` for constants.
- Enum variants: `PascalCase`.

**Types and traits:**
- Derive `Debug`, `Clone`, `Serialize`, `Deserialize` on data structs where appropriate.
- Derive `Default` when a sensible default exists; mark the default variant with `#[default]`.
- Use `impl Into<String>` / `impl AsRef<str>` for flexible string parameters in constructors.

**Testing:**
- Unit tests live in a `#[cfg(test)] mod tests` block at the bottom of each source file.
- Integration tests live in `tests/`.
- Use `proptest` for property-based tests (`proptest!` macro).
- Use `wiremock` or `mockall` for mocking HTTP/trait dependencies.
- Use `tokio::test` (or `#[tokio::test]`) for async tests.
- All tests must pass without external network access (mock all HTTP calls).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::Agent("test".to_string());
        assert_eq!(err.to_string(), "Agent error: test");
    }

    #[tokio::test]
    async fn test_async_behaviour() {
        // ...
    }
}
```
