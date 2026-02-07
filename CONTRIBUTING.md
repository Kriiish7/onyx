# Contributing to Onyx

Thank you for your interest in contributing to Onyx! This document provides guidelines and instructions for contributing.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Code Style](#code-style)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Community](#community)

## Code of Conduct

This project adheres to a Code of Conduct that all contributors are expected to follow. Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing.

## Getting Started

### Prerequisites

- **Rust 1.75+** - [Install Rust](https://rustup.rs/)
- **Git** - Version control
- **Docker** (optional but recommended) - For consistent development environment

### First-Time Setup

1. **Fork the repository** on GitHub
2. **Clone your fork**:
   ```bash
   git clone https://github.com/YOUR_USERNAME/onyx.git
   cd onyx
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/ORIGINAL_OWNER/onyx.git
   ```
4. **Install development tools**:
   ```bash
   make install-tools
   ```
5. **Install pre-commit hooks**:
   ```bash
   make install-hooks
   ```

## Development Setup

### Option 1: Docker (Recommended)

The easiest way to get started is with Docker:

```bash
# Start development environment
make docker-dev

# Inside the container:
cargo build
cargo test
```

### Option 2: Local Development

#### Linux/macOS

```bash
# Install system dependencies
# Ubuntu/Debian:
sudo apt-get install build-essential pkg-config libssl-dev clang cmake

# macOS:
brew install cmake

# Build and test
cargo build
cargo test
```

#### Windows

For Windows development, we recommend using Docker or WSL2 due to native dependency complexities. See [docs/BUILD_WINDOWS.md](docs/BUILD_WINDOWS.md) for detailed instructions.

## How to Contribute

### Types of Contributions

We welcome various types of contributions:

- 🐛 **Bug fixes** - Fix issues and improve stability
- ✨ **Features** - Implement new capabilities (discuss first!)
- 📝 **Documentation** - Improve docs, add examples
- 🧪 **Tests** - Add test coverage
- 🎨 **UI/UX** - Frontend improvements (Phase 2+)
- 🌍 **Translations** - i18n support (future)
- 🔧 **Tooling** - CI/CD, dev experience improvements

### Finding Issues to Work On

- Look for issues labeled [`good first issue`](https://github.com/OWNER/onyx/labels/good%20first%20issue)
- Check [`help wanted`](https://github.com/OWNER/onyx/labels/help%20wanted) for priorities
- Ask in Discord/Slack if you're unsure where to start

### Reporting Bugs

Before creating a bug report:

1. **Search existing issues** to avoid duplicates
2. **Verify it's a bug** - can you reproduce it consistently?
3. **Gather information**:
   - OS and version
   - Rust version (`rustc --version`)
   - Onyx version
   - Steps to reproduce
   - Expected vs actual behavior

Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md).

### Suggesting Features

For feature requests:

1. **Search existing issues** first
2. **Describe the use case** - why is this needed?
3. **Propose a solution** if you have one
4. **Consider alternatives** - are there other approaches?

Use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.md).

## Code Style

### Rust Code Style

We follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/):

- **Formatting**: Use `rustfmt` (enforced by CI)
  ```bash
  cargo fmt --all
  ```

- **Linting**: Use `clippy` with strict settings
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```

- **Naming**:
  - `snake_case` for functions and variables
  - `CamelCase` for types and traits
  - `SCREAMING_SNAKE_CASE` for constants

- **Documentation**: All public APIs must have doc comments
  ```rust
  /// Brief description of the function.
  ///
  /// # Arguments
  ///
  /// * `param` - Description
  ///
  /// # Returns
  ///
  /// Description of return value
  ///
  /// # Errors
  ///
  /// When does this error?
  ///
  /// # Examples
  ///
  /// ```
  /// use onyx::example;
  /// let result = example();
  /// ```
  pub fn example() -> Result<()> { ... }
  ```

### Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
type(scope): brief description

Optional longer description explaining the change in detail.

Fixes #123
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, no logic change)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Build process, tooling, dependencies

**Examples:**
```
feat(query): add support for cross-project queries

Implements the ability to query across multiple ingested projects
simultaneously. Includes new --all-projects flag for CLI.

Fixes #456
```

```
fix(storage): resolve race condition in RocksDB writes

The WAL commit was not properly synchronized, causing occasional
data corruption under high load.

Fixes #789
```

## Testing

### Running Tests

```bash
# Run all tests
make test

# Run tests with all features (including RocksDB)
make test-all

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Writing Tests

- **Unit tests**: Co-locate with code in `mod tests { ... }`
- **Integration tests**: Add to `tests/` directory
- **Property-based tests**: Use `proptest` for complex invariants

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new("test", "content");
        assert_eq!(node.name, "test");
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

### Test Coverage

We aim for **80%+ code coverage**:

```bash
# Generate coverage report
make coverage

# View report at: target/tarpaulin/index.html
```

## Pull Request Process

### Before Submitting

1. ✅ **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. ✅ **Make your changes** with clear, logical commits

3. ✅ **Run the full check suite**:
   ```bash
   make check
   ```

4. ✅ **Add tests** for new functionality

5. ✅ **Update documentation** if needed:
   - README.md
   - API docs (rustdoc)
   - CHANGELOG.md

6. ✅ **Sync with upstream**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

### Submitting a PR

1. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

2. **Create Pull Request** on GitHub

3. **Fill out the PR template** completely:
   - Description of changes
   - Related issues
   - Testing performed
   - Breaking changes (if any)

4. **Wait for review** - maintainers will review within 48 hours

### PR Review Process

- **CI must pass** - All checks must be green
- **Code review** - At least one maintainer approval required
- **Discussion** - Be responsive to feedback
- **Updates** - Make requested changes promptly

### After Merge

- 🎉 Your contribution is merged!
- 🏆 You'll be added to CONTRIBUTORS.md
- 📣 We may feature your contribution in release notes

## Community

### Communication Channels

- **GitHub Issues** - Bug reports, feature requests
- **GitHub Discussions** - Questions, ideas, show & tell
- **Discord** - Real-time chat (link coming soon)
- **Twitter/Mastodon** - Updates and announcements

### Getting Help

- 📖 Check the [documentation](docs/)
- 💬 Ask in GitHub Discussions
- 🐛 Report bugs in Issues
- 💡 Join our community chat

### Recognition

Contributors are recognized in:
- [CONTRIBUTORS.md](CONTRIBUTORS.md)
- Release notes
- Project README

We appreciate all contributions, big or small! 🙏

## License

By contributing to Onyx, you agree that your contributions will be licensed under the Apache 2.0 License (or MIT, depending on final choice).

---

**Thank you for contributing to Onyx!** 🚀
