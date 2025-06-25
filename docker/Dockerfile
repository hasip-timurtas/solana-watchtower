# Multi-stage build for Solana Watchtower
FROM rust:1.83-bullseye as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy manifests and build dependencies first (for better caching)
COPY Cargo.toml Cargo.lock ./
COPY crates/cli/Cargo.toml ./crates/cli/
COPY crates/dashboard/Cargo.toml ./crates/dashboard/
COPY crates/engine/Cargo.toml ./crates/engine/
COPY crates/notifier/Cargo.toml ./crates/notifier/
COPY crates/subscriber/Cargo.toml ./crates/subscriber/

# Create dummy source files to build dependencies
RUN mkdir -p crates/cli/src crates/dashboard/src crates/engine/src crates/notifier/src crates/subscriber/src
RUN echo "fn main() {}" > crates/cli/src/main.rs
RUN echo "" > crates/dashboard/src/lib.rs
RUN echo "" > crates/engine/src/lib.rs
RUN echo "" > crates/notifier/src/lib.rs
RUN echo "" > crates/subscriber/src/lib.rs

# Build dependencies
RUN cargo build --release --bins
RUN rm -rf crates/*/src

# Copy source code
COPY crates/ ./crates/
COPY configs/ ./configs/

# Build application
RUN cargo build --release --bins

# Runtime stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    libsqlite3-0 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false watchtower

# Set working directory
WORKDIR /app

# Copy binaries from builder
COPY --from=builder /app/target/release/watchtower-cli /usr/local/bin/
COPY --from=builder /app/target/release/watchtower-dashboard /usr/local/bin/

# Copy configuration and templates
COPY --from=builder /app/configs/ ./configs/
COPY --from=builder /app/crates/notifier/templates/ ./templates/
COPY --from=builder /app/crates/dashboard/templates/ ./dashboard/templates/
COPY --from=builder /app/crates/dashboard/static/ ./dashboard/static/

# Create directories for data and logs
RUN mkdir -p /app/data /app/logs && \
    chown -R watchtower:watchtower /app

# Copy entrypoint script
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Expose ports
EXPOSE 8080 9090

# Switch to app user
USER watchtower

# Set entrypoint
ENTRYPOINT ["/entrypoint.sh"] 