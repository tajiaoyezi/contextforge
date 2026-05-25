# ContextForge daemon Docker image (task-10.6 / ADR-015 §D6).
# Multi-stage build: rust:1.93-slim-bookworm + golang:1.26-bookworm → debian:bookworm-slim runtime.
# Bundles `contextforge` binary; default CMD runs Console Contract v1
# REST surface (`console-api-serve`) on port 48181.
#
# v0.7.1 (vs v0.7.0):
#   - Rust 1.82-bullseye → 1.93-slim-bookworm（v0.7.0 transitive deps
#     darling@0.23 / tantivy@0.26 / time@0.3.47 要 rustc >= 1.88）
#   - Go 1.22-bullseye → 1.26-bookworm（go.mod 已要求 go 1.26；Go 1.26
#     dropped bullseye 支持）
#   - 加 ENV CONSOLE_API_FALLBACK_INMEM=1（single-image 部署默认 in-memory
#     MemStore 模式，/v1/health 返 200 让 docker healthcheck 过；用户起多
#     进程部署时可 `docker run -e CONSOLE_API_FALLBACK_INMEM=0` 覆盖）
#   - 加 .dockerignore（v0.7.0 build context 含 `target/` 9.3 GB cargo
#     cache 全 transfer，新版瘦身 ~50 MB）

# ---- Rust stage (Core data-plane) ----
FROM rust:1.93-slim-bookworm AS rust-build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY core/ ./core/
COPY proto/ ./proto/
RUN cargo build --release -p contextforge-core --bin contextforge-core

# ---- Go stage (Control-plane CLI + REST daemon) ----
FROM golang:1.26-bookworm AS go-build
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

# v0.7.1 (ADR-016 §D4): single-image 默认 in-memory MemStore 模式。
# 本镜像仅起 Go REST proxy（`console-api-serve`）单进程，无 Rust gRPC core
# (`contextforge-core`) 进程；不设此 env 则 daemon `/v1/health` 返 503
# (degraded) → docker healthcheck 永远不过。
# 真正 2 进程 + 持久化 storage 部署时，用户 `docker run -e CONSOLE_API_FALLBACK_INMEM=0`
# 关闭 fallback，同时另起 contextforge-core daemon（详 RELEASE_NOTES v0.7.1）。
ENV CONSOLE_API_FALLBACK_INMEM=1

EXPOSE 48181

# Default CMD: serve Console Contract v1 REST on 0.0.0.0:48181 so docker
# compose service mesh can reach the daemon (loopback would only be
# reachable from inside the container).
CMD ["contextforge", "console-api-serve", "--addr", "0.0.0.0:48181"]
