# Symbion Kernel - Multi-stage Docker build
# Usage: docker build -f docker/kernel.Dockerfile -t symbion-kernel .

# ============================================================================
# Stage 1: Build
# ============================================================================
FROM rust:1.84-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY symbion-kernel/Cargo.toml symbion-kernel/Cargo.toml
COPY symbion-plugin-common/Cargo.toml symbion-plugin-common/Cargo.toml
COPY symbion-plugin-notes/Cargo.toml symbion-plugin-notes/Cargo.toml
COPY symbion-plugin-sensors/Cargo.toml symbion-plugin-sensors/Cargo.toml
COPY symbion-plugin-ssl/Cargo.toml symbion-plugin-ssl/Cargo.toml
COPY symbion-plugin-freebox/Cargo.toml symbion-plugin-freebox/Cargo.toml
COPY symbion-agent-host/Cargo.toml symbion-agent-host/Cargo.toml
COPY devkit/Cargo.toml devkit/Cargo.toml

# Create dummy src files for dependency caching
RUN mkdir -p symbion-kernel/src && echo "fn main() {}" > symbion-kernel/src/main.rs \
    && mkdir -p symbion-plugin-common/src && echo "" > symbion-plugin-common/src/lib.rs \
    && mkdir -p symbion-plugin-notes/src && echo "fn main() {}" > symbion-plugin-notes/src/main.rs \
    && mkdir -p symbion-plugin-sensors/src && echo "fn main() {}" > symbion-plugin-sensors/src/main.rs \
    && mkdir -p symbion-plugin-ssl/src && echo "fn main() {}" > symbion-plugin-ssl/src/main.rs \
    && mkdir -p symbion-plugin-freebox/src && echo "fn main() {}" > symbion-plugin-freebox/src/main.rs \
    && mkdir -p symbion-agent-host/src && echo "fn main() {}" > symbion-agent-host/src/main.rs \
    && mkdir -p devkit/src && echo "" > devkit/src/lib.rs

# Build dependencies only (cached layer)
RUN cargo build --release -p symbion-kernel 2>/dev/null || true

# Copy real source code
COPY symbion-kernel/src symbion-kernel/src
COPY symbion-plugin-common/src symbion-plugin-common/src

# Touch main.rs to invalidate the dummy binary
RUN touch symbion-kernel/src/main.rs

# Build the real kernel
RUN cargo build --release -p symbion-kernel

# Run tests
RUN cargo test -p symbion-kernel

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false symbion && mkdir -p /opt/symbion/data && chown symbion:symbion /opt/symbion/data

COPY --from=builder /build/target/release/symbion-kernel /opt/symbion/bin/symbion-kernel

WORKDIR /opt/symbion
USER symbion

# Default environment
ENV SYMBION_MQTT_BROKER=mosquitto:1883
ENV SYMBION_TIMEZONE=Europe/Paris

EXPOSE 8080 8443

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["/opt/symbion/bin/symbion-kernel"]
