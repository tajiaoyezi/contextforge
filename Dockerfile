# ContextForge daemon Docker image (task-10.6 / ADR-015 §D6).
# Multi-stage build: rust:1.93-slim-bookworm + golang:1.26-bookworm → debian:bookworm-slim runtime.
# Bundles `contextforge` binary; default CMD runs Console Contract v1
# REST surface (`console-api-serve`) on port 48181.
#
# v0.7.2 (vs v0.7.1) — ADR-018 fallback-inmem-default-reversal:
#   - 删 `ENV CONSOLE_API_FALLBACK_INMEM=1` 行；daemon 默认 fallback deny
#     (binary default false 自然生效)，gRPC core 不可达时 `/v1/health` 返
#     503 → docker healthcheck 立即报 unhealthy（silent footgun fix）
#   - 用户显式 opt-in 保留 v0.7.1 行为：
#     `docker run -e CONSOLE_API_FALLBACK_INMEM=1 contextforge-daemon:v0.7.2`
#   - 多进程真持久化部署：另起 contextforge-core daemon + Go REST 配
#     `--grpc-addr` 指向 core
#
# v0.7.1 (vs v0.7.0):
#   - Rust 1.82-bullseye → 1.93-slim-bookworm（v0.7.0 transitive deps
#     darling@0.23 / tantivy@0.26 / time@0.3.47 要 rustc >= 1.88）
#   - Go 1.22-bullseye → 1.26-bookworm（go.mod 已要求 go 1.26；Go 1.26
#     dropped bullseye 支持）
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

# v0.7.2 (ADR-018): 删 ENV CONSOLE_API_FALLBACK_INMEM=1（v0.7.1 残留）。
# 现在默认 fallback deny — daemon binary default false 自然生效。
# 用户需 in-memory fallback 时显式 `docker run -e CONSOLE_API_FALLBACK_INMEM=1`
# opt-in（容器重启数据失，仅 dev/PoC 推荐）；真持久化部署另起 contextforge-core。

EXPOSE 48181

# Default CMD: serve Console Contract v1 REST on 0.0.0.0:48181 so docker
# compose service mesh can reach the daemon (loopback would only be
# reachable from inside the container).
CMD ["contextforge", "console-api-serve", "--addr", "0.0.0.0:48181"]
