# Onyx - Docker Build Configuration
# Multi-stage build for minimal production image

# Stage 1: Builder
FROM rust:1.75-slim-bullseye AS builder

# Install build dependencies for native libraries
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    clang \
    cmake \
    git \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build for release with RocksDB feature
RUN cargo build --release --features rocksdb-storage

# Stage 2: Runtime
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 onyx

# Create data directory
RUN mkdir -p /data && chown onyx:onyx /data

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/onyx /usr/local/bin/onyx

# Copy configuration template
COPY config.toml.example /app/config.toml.example

# Set ownership
RUN chown -R onyx:onyx /app

# Switch to non-root user
USER onyx

# Expose ports
EXPOSE 3000

# Set data volume
VOLUME ["/data"]

# Environment variables
ENV RUST_LOG=info
ENV ONYX_DATA_DIR=/data

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/onyx", "status"] || exit 1

# Default command
CMD ["/usr/local/bin/onyx", "interactive"]
