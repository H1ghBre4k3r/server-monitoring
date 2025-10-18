# syntax=docker/dockerfile:1

ARG RUST_VERSION=nightly
ARG ALPINE_VERSION=3.22
ARG NODE_VERSION=20

################################################################################
# Web Dashboard Builder - Build React/TypeScript frontend
################################################################################
FROM node:${NODE_VERSION}-alpine AS web-builder

WORKDIR /app/web-dashboard

# Copy package files for dependency installation
COPY web-dashboard/package.json web-dashboard/package-lock.json ./

# Install dependencies including devDependencies (needed for build)
# Cache npm downloads for faster builds
RUN --mount=type=cache,target=/root/.npm \
    npm ci

# Copy web dashboard source code
COPY web-dashboard/ ./

# Build production bundle (outputs to dist/)
RUN npm run build

################################################################################
# Rust Chef Installer - Install cargo-chef once and cache
################################################################################
FROM rustlang/rust:${RUST_VERSION}-alpine AS chef-installer

# Install build dependencies and cargo-chef
# This layer is cached and only rebuilds when RUST_VERSION changes
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    apk add --no-cache musl-dev && \
    cargo install cargo-chef --locked

################################################################################
# Rust Chef Base - Reuse pre-built cargo-chef
################################################################################
FROM rustlang/rust:${RUST_VERSION}-alpine AS chef

WORKDIR /app

# Copy pre-built cargo-chef binary from installer stage
COPY --from=chef-installer /usr/local/cargo/bin/cargo-chef /usr/local/cargo/bin/cargo-chef

# Install musl-dev needed for builds
RUN apk add --no-cache musl-dev

################################################################################
# Planner - Generate dependency recipe
################################################################################
FROM chef AS planner

# Copy manifests and source to analyze dependencies
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

# Generate recipe.json with all dependencies
RUN cargo chef prepare --recipe-path recipe.json

################################################################################
# Rust Builder - Build hub binary with cached dependencies
################################################################################
FROM chef AS builder

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

# Build dependencies only (cached layer)
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json

# Copy source code and migrations
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

# Build only the hub binary (dependencies already built)
# Strip debug symbols for smaller binary
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --bin guardia-hub --locked --release && \
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

# Create directories for config, data, and web dashboard
RUN mkdir -p /app/config /app/data /app/web-dashboard && \
    chown -R guardia:guardia /app

WORKDIR /app

# Copy binary from builder
COPY --from=builder --chown=guardia:guardia /guardia-hub /usr/local/bin/guardia-hub

# Copy web dashboard static files from web-builder
COPY --from=web-builder --chown=guardia:guardia /app/web-dashboard/dist /app/web-dashboard/dist

# Switch to non-root user
USER guardia

# API server port
EXPOSE 8080

# Volume for persistent data (metrics.db, config)
VOLUME ["/app/data", "/app/config"]

# Default command - expects config at /app/config/config.json
CMD ["guardia-hub", "-f", "/app/config/config.json"]
