#!/bin/bash
# Release script for OpenManus Rust

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check for required tools
check_requirements() {
    log_info "Checking requirements..."
    
    command -v cargo >/dev/null 2>&1 || { log_error "cargo is required but not installed."; exit 1; }
    command -v git >/dev/null 2>&1 || { log_error "git is required but not installed."; exit 1; }
    
    log_info "All requirements met."
}

# Run tests
run_tests() {
    log_info "Running tests..."
    cargo test --all-features --all-targets
    if [ $? -ne 0 ]; then
        log_error "Tests failed!"
        exit 1
    fi
    log_info "All tests passed."
}

# Run clippy
run_clippy() {
    log_info "Running clippy..."
    cargo clippy --all-features --all-targets -- -D warnings
    if [ $? -ne 0 ]; then
        log_error "Clippy found issues!"
        exit 1
    fi
    log_info "Clippy checks passed."
}

# Check formatting
check_formatting() {
    log_info "Checking formatting..."
    cargo fmt --all -- --check
    if [ $? -ne 0 ]; then
        log_warn "Formatting issues found. Running cargo fmt..."
        cargo fmt --all
        log_warn "Please review and commit the formatting changes."
        exit 1
    fi
    log_info "Formatting checks passed."
}

# Run security audit
run_audit() {
    log_info "Running security audit..."
    cargo audit 2>/dev/null || {
        log_warn "cargo-audit not installed. Installing..."
        cargo install cargo-audit
        cargo audit
    }
    if [ $? -ne 0 ]; then
        log_error "Security audit found vulnerabilities!"
        exit 1
    fi
    log_info "Security audit passed."
}

# Build release
build_release() {
    log_info "Building release..."
    cargo build --release
    if [ $? -ne 0 ]; then
        log_error "Release build failed!"
        exit 1
    fi
    log_info "Release build completed."
}

# Generate documentation
build_docs() {
    log_info "Building documentation..."
    cargo doc --no-deps --all-features
    if [ $? -ne 0 ]; then
        log_error "Documentation build failed!"
        exit 1
    fi
    log_info "Documentation build completed."
}

# Create git tag
create_tag() {
    local version=$1
    
    if git tag | grep -q "^v${version}$"; then
        log_error "Tag v${version} already exists!"
        exit 1
    fi
    
    log_info "Creating tag v${version}..."
    git tag -a "v${version}" -m "Release v${version}"
    log_info "Tag created. Push with: git push origin v${version}"
}

# Main release process
main() {
    local version=$1
    local skip_tests=${2:-false}
    
    if [ -z "$version" ]; then
        log_error "Version is required. Usage: $0 <version> [--skip-tests]"
        exit 1
    fi
    
    # Validate version format
    if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
        log_error "Invalid version format. Expected: X.Y.Z or X.Y.Z-suffix"
        exit 1
    fi
    
    log_info "Starting release process for version $version..."
    
    check_requirements
    check_formatting
    
    if [ "$skip_tests" != "--skip-tests" ]; then
        run_tests
    else
        log_warn "Skipping tests as requested."
    fi
    
    run_clippy
    run_audit
    build_release
    build_docs
    create_tag "$version"
    
    log_info "Release preparation complete!"
    log_info "Next steps:"
    log_info "  1. Review the changes"
    log_info "  2. Push the tag: git push origin v${version}"
    log_info "  3. The CI will automatically build and publish the release"
}

main "$@"
