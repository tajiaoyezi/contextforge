# ContextForge daemon Docker image (task-10.6 / ADR-015 §D6).
# Multi-stage build: rust:1.82 + golang:1.22 → alpine runtime.
# Bundles `contextforge` binary; default CMD runs Console Contract v1
# REST surface (`console-api-serve`) on port 48181.

# ---- Rust stage (Core data-plane) ----
FROM rust:1.82-bullseye AS rust-build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY core/ ./core/
COPY proto/ ./proto/
RUN cargo build --release -p contextforge-core --bin contextforge-core

# ---- Go stage (Control-plane CLI + REST daemon) ----
FROM golang:1.22-bullseye AS go-build
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -o /out/contextforge ./cmd/contextforge

# ---- Runtime stage ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rust-build /src/target/release/contextforge-core /usr/local/bin/contextforge-core
COPY --from=go-build /out/contextforge /usr/local/bin/contextforge

ENV CONTEXTFORGE_DATA_DIR=/data
RUN mkdir -p /data

EXPOSE 48181

# Default CMD: serve Console Contract v1 REST on 0.0.0.0:48181 so docker
# compose service mesh can reach the daemon (loopback would only be
# reachable from inside the container).
CMD ["contextforge", "console-api-serve", "--addr", "0.0.0.0:48181"]
