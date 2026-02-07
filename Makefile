# Onyx Development Makefile
# Common development tasks

.PHONY: help build test lint fmt clean docker-build docker-dev install-hooks check coverage

# Default target
help:
	@echo "Onyx Development Commands:"
	@echo "  make build          - Build the project"
	@echo "  make test           - Run all tests"
	@echo "  make lint           - Run clippy linter"
	@echo "  make fmt            - Format code with rustfmt"
	@echo "  make check          - Run fmt, lint, and test"
	@echo "  make coverage       - Generate code coverage report"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make docker-build   - Build Docker image"
	@echo "  make docker-dev     - Start development environment"
	@echo "  make install-hooks  - Install git pre-commit hooks"

# Build the project (cross-platform)
build:
	@if [ -f "scripts/build.sh" ]; then \
		./scripts/build.sh build; \
	elif [ -f "scripts/build.ps1" ]; then \
		powershell -ExecutionPolicy Bypass -File scripts/build.ps1 build; \
	else \
		cargo build; \
	fi

# Build with all features (including RocksDB)
build-all:
	@if [ -f "scripts/build.sh" ]; then \
		./scripts/build.sh build; \
	elif [ -f "scripts/build.ps1" ]; then \
		powershell -ExecutionPolicy Bypass -File scripts/build.ps1 build; \
	else \
		cargo build --all-features; \
	fi

# Install platform-specific dependencies
deps:
	@if [ -f "scripts/build.sh" ]; then \
		./scripts/build.sh deps; \
	elif [ -f "scripts/build.ps1" ]; then \
		powershell -ExecutionPolicy Bypass -File scripts/build.ps1 deps; \
	else \
		echo "No build script found for this platform"; \
	fi

# Run tests (cross-platform)
test:
	@if [ -f "scripts/build.sh" ]; then \
		./scripts/build.sh test; \
	elif [ -f "scripts/build.ps1" ]; then \
		powershell -ExecutionPolicy Bypass -File scripts/build.ps1 test; \
	else \
		cargo test --verbose; \
	fi

# Run tests with all features
test-all:
	@if [ -f "scripts/build.sh" ]; then \
		./scripts/build.sh test; \
	elif [ -f "scripts/build.ps1" ]; then \
		powershell -ExecutionPolicy Bypass -File scripts/build.ps1 test; \
	else \
		cargo test --verbose --all-features; \
	fi

# Run clippy
lint:
	cargo clippy --all-targets --all-features -- -D warnings

# Format code
fmt:
	cargo fmt --all

# Check formatting
fmt-check:
	cargo fmt --all -- --check

# Full check (format, lint, test) - cross-platform
check:
	@if [ -f "scripts/build.sh" ]; then \
		./scripts/build.sh check; \
	elif [ -f "scripts/build.ps1" ]; then \
		powershell -ExecutionPolicy Bypass -File scripts/build.ps1 check; \
	else \
		cargo fmt --all -- --check; \
		cargo clippy --all-targets --all-features -- -D warnings; \
		cargo test --verbose; \
	fi

# Generate code coverage
coverage:
	cargo tarpaulin --verbose --all-features --workspace --timeout 300 --out Html
	@echo "Coverage report generated at: target/tarpaulin/index.html"

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/

# Build Docker image
docker-build:
	docker build -t onyx:latest .

# Start development environment with Docker
docker-dev:
	docker-compose -f docker-compose.dev.yml up

# Stop development environment
docker-dev-stop:
	docker-compose -f docker-compose.dev.yml down

# Install git pre-commit hooks
install-hooks:
	@echo "Installing pre-commit hooks..."
	@mkdir -p .git/hooks
	@cp scripts/pre-commit .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "Pre-commit hooks installed!"

# Run the CLI in interactive mode
run:
	cargo run -- interactive --demo

# Run the demo
demo:
	cargo run -- demo

# Install development tools
install-tools:
	cargo install cargo-watch cargo-edit cargo-tarpaulin cargo-audit

# Security audit
audit:
	cargo audit

# Update dependencies
update:
	cargo update

# Check for outdated dependencies
outdated:
	cargo outdated
