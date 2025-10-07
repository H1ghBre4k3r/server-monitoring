# syntax=docker/dockerfile:1

ARG RUST_VERSION=nightly
ARG ALPINE_VERSION=3.22

FROM rustlang/rust:${RUST_VERSION}-alpine AS builder

WORKDIR /app

# Install build dependencies
RUN apk add --no-cache \
    clang \
    lld \
    musl-dev \
    git \
    pkgconfig \
    openssl-dev \
    openssl-libs-static \
    curl

# Configure environment for static linking
ENV OPENSSL_DIR=/usr \
    OPENSSL_STATIC=1 \
    PKG_CONFIG_ALLOW_CROSS=1 \
    RUSTFLAGS="-C target-feature=-crt-static"

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code and migrations
COPY src ./src
COPY migrations ./migrations

# Build only the hub binary with optimizations
# Strip debug symbols and enable LTO for smaller binary
RUN cargo build --bin guardia-hub --locked --release && \
    strip target/release/guardia-hub && \
    mv target/release/guardia-hub /guardia-hub

################################################################################
# Runtime stage - minimal Alpine image
################################################################################
FROM alpine:${ALPINE_VERSION}

# Install runtime dependencies only
RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    && adduser \
    --disabled-password \
    --gecos "" \
    --home "/app" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "10001" \
    guardia

# Create directories for config and data
RUN mkdir -p /app/config /app/data && \
    chown -R guardia:guardia /app

WORKDIR /app

# Copy binary from builder
COPY --from=builder --chown=guardia:guardia /guardia-hub /usr/local/bin/guardia-hub

# Switch to non-root user
USER guardia

# API server port
EXPOSE 8080

# Volume for persistent data (metrics.db, config)
VOLUME ["/app/data", "/app/config"]

# Default command - expects config at /app/config/config.json
CMD ["guardia-hub", "-f", "/app/config/config.json"]
