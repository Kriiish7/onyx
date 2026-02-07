# AGENTS.md

This file provides guidance for agentic coding agents working in the Onyx repository.

## Project Overview

Onyx is a Rust-native infrastructure engine for AI agents, combining semantic search, knowledge graphs, and temporal versioning in a graph-native vector memory system.

**Technology Stack:**
- Primary: Rust (edition 2021)
- Runtime: Tokio (async)
- Database: SurrealDB (in-memory), RocksDB (optional)
- HTTP: Axum web framework
- Authentication: JWT + bcrypt
- CLI: Clap
- Error Handling: thiserror + anyhow
- Logging: tracing + tracing-subscriber

## Development Commands

### Build Commands
```bash
cargo build                    # Standard build
cargo build --all-features     # Build with RocksDB
make build                     # Via Makefile
make build-all                 # All features via Makefile
```

### Test Commands
```bash
cargo test                     # Run all tests
cargo test --verbose           # Verbose output
cargo test test_name           # Run specific test by name
cargo test module::test_name   # Run specific test with module path
cargo test -- --nocapture      # Show test output (println!)
make test                      # Via Makefile
make test-all                  # All features
```

### Lint Commands
```bash
cargo fmt --all                # Format code
cargo fmt --all -- --check     # Check formatting
cargo clippy --all-targets --all-features -- -D warnings
make lint                      # Via Makefile
make fmt                       # Format via Makefile
make fmt-check                 # Check formatting via Makefile
```

### Full Quality Check
```bash
make check                     # Format + lint + test
```

## Code Style Guidelines

### Formatting
- Uses `rustfmt` with default configuration
- Enforced by CI and pre-commit hooks
- Always run `cargo fmt --all` before commits

### Naming Conventions
- Functions and variables: `snake_case`
- Types and traits: `CamelCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Use descriptive names with clear purpose

### Import Organization
```rust
// Standard library imports first
use std::collections::HashMap;
use std::sync::Arc;

// External crates second
use uuid::Uuid;
use serde::{Deserialize, Serialize};

// Internal crate imports third
use crate::model::version::VersionId;
use crate::error::{OnyxError, OnyxResult};
```

### Documentation Standards
- All public APIs must have doc comments with `///`
- Include `# Arguments`, `# Returns`, `# Errors`, `# Examples` sections
- Use comprehensive inline comments for complex logic
- Follow rustdoc conventions

### Type System Usage
- Strong typing with explicit types
- Result types for error handling (`OnyxResult<T>`)
- Option types for nullable values
- Newtype patterns for type safety

## Error Handling Patterns

### Centralized Error Type
```rust
#[derive(Error, Debug)]
pub enum OnyxError {
    #[error("Node not found: {0}")]
    NodeNotFound(uuid::Uuid),
    #[error("Embedding dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },
    // ... other variants
}

pub type OnyxResult<T> = Result<T, OnyxError>;
```

### Error Handling Philosophy
- Use `thiserror` for structured error types
- Chain errors with `?` operator
- Provide context with error messages
- Use `anyhow` for application-level error handling
- Async functions return `OnyxResult<T>`

## Async Patterns

### Async/Await Usage
- All store operations are async
- Uses `async-trait` for trait methods
- Tokio runtime for async execution
- Arc for shared state across async tasks

### Example Pattern
```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn insert(&self, id: Uuid, embedding: Vec<f32>) -> OnyxResult<()>;
    async fn search(&self, query: &[f32], k: usize) -> OnyxResult<Vec<(Uuid, f32)>>;
}
```

## Testing Guidelines

### Test Organization
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_unit_functionality() {
        // Test implementation
    }
    
    #[tokio::test]
    async fn test_async_functionality() {
        // Async test implementation
    }
}
```

### Testing Patterns
- Unit tests co-located with source code
- Integration tests in separate modules
- Property-based testing with proptest (production)
- Mock testing with mockall (production)
- Coverage target: 80%+ with tarpaulin

### Test Categories
- Model layer tests (embedding similarity, normalization)
- Store tests (vector, graph, history operations)
- Transaction tests (atomicity, rollback)
- Query engine tests (search, traversal, fusion)
- Ingestion tests (parsing, relationship detection)

## Configuration Management

### Runtime Configuration
- TOML-based configuration files
- Environment variable support
- Feature flags for optional components
- Separate configs for development/production

### Key Configuration Areas
- Server settings (host, port)
- Storage backend selection
- Vector search parameters
- Embedding model configuration
- Logging and monitoring

## Development Workflow

### Pre-commit Hooks
- Automatic formatting check
- Clippy linting with `-D warnings`
- Test execution before commits
- Fail-fast approach for code quality

### Branch Strategy
- `main` branch for stable releases
- `develop` branch for integration
- Feature branches for new work

## Security Considerations

- JWT-based authentication (planned)
- bcrypt password hashing
- Security auditing with cargo-audit
- Input validation in CLI and API
- Secure configuration management

## Performance Optimizations

### Release Profile
```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true
```

### Performance Features
- Zero-copy deserialization where possible
- Efficient vector operations
- Concurrent data structures (DashMap)
- Memory-efficient storage formats

## Project Structure

### Core Components
- Vector store: Semantic similarity search
- Graph store: Knowledge graph relationships
- History store: Temporal versioning
- Transaction manager: ACID compliance
- Query engine: Search and traversal
- Ingestion engine: Data processing

### Configuration Files
- `Cargo.toml`: Development/prototype configuration
- `Cargo.production.toml`: Production-ready dependencies
- `Cargo.prototype.toml`: Minimal prototype dependencies
- `config.toml.example`: Runtime configuration template

## Important Notes

- This is a Rust-only codebase (no TypeScript/JavaScript)
- Frontend planned for Phase 2+ but not implemented
- Always run tests before committing
- Use `make check` for comprehensive quality validation
- Follow existing patterns for new code
- Maintain comprehensive documentation