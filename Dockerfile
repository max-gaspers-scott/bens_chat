
# -----------------------------------------------------------------------------
#  Multi-stage Dockerfile for "testing" Rust / Axum API (production)
# -----------------------------------------------------------------------------
# 1. Builder image: compile the binary in release mode
# -----------------------------------------------------------------------------
FROM rust:1.98-slim AS builder

# Install build dependencies that some crates (e.g. sqlx / openssl) may need
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# Create app directory inside the container
WORKDIR /app

# Copy workspace manifests and lock file (for all members)
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml ./backend/
COPY shared/Cargo.toml ./shared/
COPY chat-cli/Cargo.toml ./chat-cli/

# Copy the actual source tree and build the real binary
COPY backend/src ./backend/src
COPY shared/src ./shared/src
COPY chat-cli/src ./chat-cli/src
COPY backend/migrations ./backend/migrations
RUN cargo build -j 6 --release -p bens_chat2


# -----------------------------------------------------------------------------
# Frontend build stage
# -----------------------------------------------------------------------------
FROM node:18-alpine AS frontend-builder

WORKDIR /app/frontend

COPY frontend/package.json frontend/package-lock.json ./
RUN npm install

COPY frontend/ ./
RUN npm run build


# -----------------------------------------------------------------------------
# 2. Runtime image: copy the binary into a minimal base image
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install certificates (TLS), curl, & clean apt caches
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 curl && rm -rf /var/lib/apt/lists/*

# Create an unprivileged user to run the app
RUN useradd -m -u 10001 appuser

WORKDIR /app

# Copy compiled binary & any runtime assets (e.g. migrations)
COPY --from=builder /app/target/release/bens_chat2 ./
COPY --from=builder /app/backend/migrations ./migrations
COPY --from=frontend-builder /app/frontend/build ./frontend/build

# Ensure the binary is executable
RUN chown -R appuser:appuser /app && chmod +x /app/bens_chat2

USER appuser

# The application listens on port 8081 – expose it to the host
EXPOSE 8081

# Start the server
ENV STATIC_DIR=/app/frontend/build
CMD ["/app/bens_chat2"]

