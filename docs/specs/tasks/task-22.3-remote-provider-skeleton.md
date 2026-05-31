# Task `22.3`: `remote-provider-skeleton — core/src/embedding/remote_provider.rs RemoteEmbeddingProvider（OpenAI/Cohere 风格 HTTP，embedding-remote feature-gated）+ 契约级确定性测试（请求构造/响应解析/错误路径，不打真实网络）`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 22 (embedding-provider-completion)
**Dependencies**: task-22.1（`select_provider` 工厂 — 落地 `"remote"` 分支）/ task-19.1（`EmbeddingProvider` trait + fastembed rustls 口径范例）/ ADR-004（local-first：远程显式 opt-in + 默认 0 网络 dep）/ ADR-008（core-library + embedding crate Amendment — rustls 而非 OpenSSL）/ ADR-027（embedding-provider-abstraction D4 远程骨架 + D5 本地优先红线）/ ADR-013（禁伪造凭据 — 不打真实网络、真实联调如实 defer）/ ADR-014 D1-D5（第十三次激活）

## 1. Background

Phase 19 的真实 embedding 只有本地 `FastEmbedProvider`（ONNX，`embedding-fastembed` feature）。`docs/roadmap.md` §3.3 把 `[SPEC-DEFER:phase-future.embedding-provider-remote]`（adr-008:56 + phase-19 §2）排入本 phase：为 OpenAI / Cohere 等远程 embedding 提供通路。

约束硬底（ADR-004 / ADR-027 D5）：本地优先是产品红线 —— 远程 provider 必须 feature-gated（默认构建 0 网络 dep）+ 运行时显式 opt-in（承既有 `RemoteProviderConfig.Enabled`）+ 密钥不入库不入日志。ADR-013 禁伪造：CI / 无人值守环境**没有**远程密钥 / 网络，真实可达性 / 真实召回质量**不能**在无网络下伪造。

本 task 落 `RemoteEmbeddingProvider` HTTP **骨架** + **契约级确定性测试**：请求体构造 / 响应 JSON 解析 / HTTP 与解析错误路径用固定 fixture 断言，**不打真实网络**；真实联调 + 密钥 + 真实召回质量如实 defer（记 stop-condition）。

## 2. Goal

新增 `core/src/embedding/remote_provider.rs::RemoteEmbeddingProvider`（feature `embedding-remote`，默认构建不编入）：实现 `EmbeddingProvider` trait，覆盖 OpenAI / Cohere 风格 HTTP embedding 的（a）请求体构造（`model` / `input` / 可选 `dimensions`，序列化为 JSON）；（b）响应 JSON 解析（`{"data":[{"embedding":[...]}]}` 风格，提取每条 `embedding` 向量）；（c）错误路径映射（HTTP 非 2xx / body 解析失败 / 空 data → `EmbeddingError::Backend` / `EmbeddingError::Other`）。把请求构造 / 响应解析拆成**纯函数**（`build_request_body(model, texts, dim) -> serde_json::Value` / `parse_response(json: &str) -> Result<Vec<Vec<f32>>, EmbeddingError>`），使契约级测试无需网络即可断言。HTTP client（如 `reqwest` rustls，承 fastembed 既有 rustls 口径避 OpenSSL）作 optional dep（`embedding-remote` feature 下编入）。task-22.1 工厂 `"remote"` 分支在本 feature 下返回 `RemoteEmbeddingProvider`、否则维持明确 feature-未启用错误。≥3 Rust 测试全 PASS（全 fixture / 纯函数，无网络）；默认构建 `cargo test --workspace` 不退化（0 网络 dep）；D2 lint 触及行 0 未标注命中。真实网络联调 + 密钥 + 真实召回质量如实 defer（ADR-013，stop-condition 见 §8 R1）。

## 3. Scope

### In Scope

- **新增 `core/src/embedding/remote_provider.rs`**（`core/src/embedding/mod.rs` `#[cfg(feature="embedding-remote")] pub mod remote_provider;` + feature-gated re-export）：
  - `pub struct RemoteEmbeddingProvider { endpoint: String, model: String, dim: usize, api_key: Option<String>, client: <HttpClient> }`（`api_key` 从环境 / 配置读，不硬编码、不入日志）
  - 纯函数 `fn build_request_body(model: &str, texts: &[String], dim: usize) -> serde_json::Value`（OpenAI / Cohere 风格请求体）
  - 纯函数 `fn parse_response(body: &str) -> Result<Vec<Vec<f32>>, EmbeddingError>`（解析 `data[].embedding`；空 data / 缺字段 → `EmbeddingError`）
  - `impl EmbeddingProvider`：`embed` 用 client POST endpoint（请求由 `build_request_body` 构造、响应由 `parse_response` 解析）；`dim()` → 配置 dim；`name()` → `"remote-<provider>"` provenance
  - HTTP / 解析错误映射到 `EmbeddingError::Backend{source}` / `EmbeddingError::Other`
- **修改 `core/Cargo.toml`**：加 `embedding-remote` feature + HTTP client optional dep（rustls TLS，承 fastembed `hf-hub-rustls-tls` 口径避 OpenSSL/pkg-config）；`[features]` 加 `embedding-remote = ["dep:<client>", ...]`；默认构建不拉（R7：dep 变更走主 agent lockfile，§10 disclosed）
- **修改 `core/src/embedding/factory.rs`**（task-22.1 留的 `"remote"` 分支）：`#[cfg(feature="embedding-remote")]` 下返回 `RemoteEmbeddingProvider`（从配置 / 环境读 endpoint / model / api_key）；未编入 feature 时维持明确 feature-未启用 `EmbeddingError`
- **契约级测试（`remote_provider.rs` 内 `#[cfg(all(test, feature="embedding-remote"))]`）**：≥3 测试——（a）`build_request_body` 对给定 model / texts / dim 构造期望 JSON（断言 `model` / `input` / `dimensions` 字段）；（b）`parse_response` 对固定 fixture 响应 JSON 解析出期望向量（顺序 / 维度）；（c）错误路径——malformed JSON / 空 data / 缺 `embedding` 字段 → 期望 `EmbeddingError` 变体。**不打真实网络**。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **远程 provider 真实网络联调 + 真实 API 密钥** [SPEC-DEFER:phase-future.embedding-provider-remote]：需 OpenAI / Cohere 密钥 + 网络，CI 无凭据；本 task 不打真实网络（ADR-013）。真实联调受阻 → stop-condition（§8 R1），如实 defer 不伪造。
- **远程 provider 真实召回质量数值** [SPEC-DEFER:phase-future.embedding-provider-remote]：真实 embedding 召回需密钥 + 真实 eval；本 task 是骨架 + 契约测试，不产出召回数值（ADR-013）。
- **health 远程可达性探针** [SPEC-OWNER:task-22.4-closeout-v0.15.0]：探针在收口 task；本 task 落 provider 骨架。
- **provider 配置选择 + 工厂主体** [SPEC-OWNER:task-22.1-provider-config-selection]：工厂在 22.1；本 task 仅落 `"remote"` 分支的具体 provider。
- **embedding 缓存** [SPEC-OWNER:task-22.2-embedding-cache]：缓存在 22.2；远程 provider 可被 22.2 缓存包裹（正交）。
- **远程 reranker provider** [SPEC-DEFER:phase-future.reranker]：reranker 属 Phase 21；本 task 仅 embedding。

## 4. Actors

- **主 agent**：实施 + PR 主理（含 R7 dep chore）。
- **`core/src/embedding/remote_provider.rs::RemoteEmbeddingProvider`**：本 task 新增的远程 HTTP provider 骨架，实现 `EmbeddingProvider`。
- **`build_request_body` / `parse_response` 纯函数**：契约级测试的断言对象（无网络）。
- **`core/src/embedding/factory.rs`**：task-22.1 工厂，本 task 在 feature 下落 `"remote"` 分支。
- **远程 API（OpenAI / Cohere）**：真实联调对象，本 task **不**真实命中（ADR-013，defer）。
- **下游 task-22.4**：health 远程探针 + closeout。

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/embedding/traits.rs:13-42`（`EmbeddingProvider` trait + `EmbeddingError` 含 `Backend{source}` / `Other` / `DimMismatch`）
- `core/src/embedding/fastembed_provider.rs`（feature-gated provider 范例 + rustls 口径）
- `core/Cargo.toml:94-99`（fastembed rustls feature 配置范例：`default-features=false` + `hf-hub-rustls-tls`）+ `core/Cargo.toml:107-116`（`[features]` 表）
- `core/Cargo.toml:29-33`（`serde` / `serde_json` 已 direct dep — 请求构造 / 响应解析用）
- `internal/config/config.go:48-54`（`RemoteProviderConfig{Enabled, Provider, Endpoint}` — 远程 opt-in 既有 seam）
- `docs/decisions/adr-004-local-first-privacy-baseline.md`（远程显式 opt-in）+ `docs/decisions/adr-008-core-library-selection.md` Amendment（rustls 而非 OpenSSL）+ `docs/decisions/adr-027-embedding-provider-abstraction.md`（D4 骨架 + D5 本地优先红线）+ `docs/decisions/adr-013` 风格（禁伪造凭据）

### 5.2 关键设计 — 纯函数拆分 + feature-gated + 不打网络

- 请求构造 / 响应解析拆为**纯函数**（`build_request_body` / `parse_response`），与网络 IO 解耦 → 契约级测试无需起 HTTP 测试替身、无需网络即可严格断言（ADR-013：deterministic 管道可验证）。
- `RemoteEmbeddingProvider` feature-gated（`embedding-remote`）：默认构建不编译此模块、不拉 HTTP client → 0 网络 dep（ADR-004 + ADR-008 D5）。
- 运行时显式 opt-in：工厂仅在配置 `provider="remote"` 且 feature 编入时返回；密钥从环境 / 配置读，不硬编码、不入日志（PRD §Constraints 安全基线）。
- 真实网络命中**不在本 task 验证**：CI 无密钥 / 网络，契约测试用 fixture；真实联调如实 defer（ADR-013）。

### 5.3 不变量

- `EmbeddingProvider` trait 不变（add-only，本 task 不改 trait）。
- 默认构建 0 网络 dep（`embedding-remote` feature off → `remote_provider.rs` 不编入，HTTP client 不拉）。
- 远程 provider 不在默认 / 缺省配置下激活（ADR-004 本地优先红线）。
- 契约测试不发起任何真实网络请求（ADR-013）。
- 密钥不入库、不入日志、不写进 spec / 测试 fixture（fixture 用伪造响应，承 PRD 安全基线）。

## 6. Acceptance Criteria

- [x] **AC1**: `build_request_body(model, texts, dim)` 对给定 model / texts / dim 构造期望 OpenAI/Cohere 风格 JSON（`model` / `input` / 可选 `dimensions` 字段正确）— verified by **TEST-22.3.1**
- [x] **AC2**: `parse_response(fixture_json)` 对固定 fixture 响应解析出期望向量（条数 / 维度 / 顺序与 fixture 一致）；向量值与 fixture 逐字节对应 — verified by **TEST-22.3.2**
- [x] **AC3**: 错误路径 — malformed JSON / 空 `data` / 缺 `embedding` 字段 → `parse_response` 返回明确 `EmbeddingError`（不 panic）；HTTP 错误映射到 `EmbeddingError::Backend` / `Other` — verified by **TEST-22.3.3**
- [x] **AC4**: feature-gated 与本地优先 — 默认构建（无 `embedding-remote`）不编入 `remote_provider.rs` 且 0 网络 dep；`embedding-remote` feature 下工厂 `"remote"` 分支返回 `RemoteEmbeddingProvider`；`name()` 携 `"remote-<provider>"` provenance；`dim()` 返配置 dim — verified by **TEST-22.3.4**
- [x] **AC5**: 既有不退化 + 真实联调如实 defer — 默认构建 `cargo test --workspace` + `go test ./...` 全 PASS；`embedding-remote` feature 下契约测试全 PASS（无网络）；真实网络联调 / 密钥 / 真实召回质量按 ADR-013 如实记录 defer（§8 R1 stop-condition），不伪造真实命中；D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-22.3.5** + §10 记录

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-22.3.1 | `build_request_body` 构造期望请求 JSON（model/input/dimensions） | `core/src/embedding/remote_provider.rs` `#[cfg(all(test, feature="embedding-remote"))]` | Done |
| TEST-22.3.2 | `parse_response` 解析 fixture 响应出期望向量（条数/维度/顺序） | `core/src/embedding/remote_provider.rs` `#[cfg(...)]` | Done |
| TEST-22.3.3 | 错误路径：malformed/空 data/缺字段 → 明确 `EmbeddingError` | `core/src/embedding/remote_provider.rs` `#[cfg(...)]` | Done |
| TEST-22.3.4 | feature-gated 编入 + 工厂 `"remote"` 分支 + name/dim | `core/src/embedding/{remote_provider,factory}.rs` | Done |
| TEST-22.3.5 | 默认构建 0 网络 dep 不退化 + feature 下契约测试 PASS + 真实联调 defer 记录 + D2 lint | 全 Rust + `scripts/spec_drift_lint.sh` + §10 | Done |

## 8. Risks

- **R1（高）远程真实联调需密钥 + 网络，CI 不可验证**：OpenAI / Cohere 真实可达性 + 真实召回质量需密钥 + 网络；无人值守 CI 无凭据。
  - **缓解**：本 task 做 feature-gated 骨架 + 纯函数契约测试（fixture 断言请求构造 / 响应解析 / 错误路径，不打真实网络），CI 可验证骨架契约正确性。**stop-condition**：远程密钥 / 网络不可得 → 契约测试跑通即视骨架达标，真实网络联调 / 召回质量按 `[SPEC-DEFER:phase-future.embedding-provider-remote]` 如实记录 defer，**不**标 `[x]` 真实命中、**不**据无网络环境伪造响应或 ratify（ADR-013）。
- **R2（中）HTTP client dep 引入网络供应链表面**：远程 provider 需 HTTP client（如 reqwest）。
  - **缓解**：optional dep（`embedding-remote` feature 下才编入），默认构建不拉（ADR-008 D5 + ADR-004）；rustls TLS（承 fastembed `hf-hub-rustls-tls` 口径）避 OpenSSL/pkg-config 系统依赖；R7 dep 变更走主 agent lockfile（§10 disclosed）。
- **R3（中）密钥泄露风险**：远程 provider 需 API 密钥。
  - **缓解**：密钥从环境变量 / 配置读，不硬编码、不入日志、不写进 spec / 测试 fixture（fixture 用伪造响应样本，承 PRD §Constraints 安全基线）；远程显式 opt-in（ADR-004）。
- **R4（低）OpenAI vs Cohere 请求 / 响应 schema 差异**：两家 API 请求 / 响应 shape 不同。
  - **缓解**：本 task 以 OpenAI 风格（`data[].embedding`）为主骨架 + 契约测试；Cohere 等其他 provider 的 schema 差异以 `provider` 配置分派 / 后续版本扩充（add-only），本 task 不穷举所有 provider。

## 9. Verification Plan

```bash
# 默认构建：0 网络 dep 不退化（remote_provider.rs 不编入）
cargo test --workspace
cargo tree -p contextforge-core | grep -i reqwest || echo "OK: no http client in default build"

# embedding-remote feature：契约级确定性测试（无网络）
cargo test --features embedding-remote -p contextforge-core embedding::remote_provider -- --nocapture

# Go 不退化（本 PR 零 Go delta）
go test ./...

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**: 2026-05-31。

- **改动文件**:
  - `core/src/embedding/remote_provider.rs`（新增，`#[cfg(feature="embedding-remote")]`）— `RemoteEmbeddingProvider`（endpoint/model/dim/api_key/name；Debug 不打印 api_key）；纯函数 `build_request_body`（OpenAI 风格 model/input/dimensions）/ `parse_response`（data[].embedding → 有序 Vec<Vec<f32>>，错误路径明确 Err）；`impl EmbeddingProvider`（embed 经 ureq POST，错误映射 Backend）；4 个契约测试 `#[cfg(test)]`。
  - `core/src/embedding/mod.rs` — feature-gated `pub mod remote_provider` + re-export。
  - `core/src/embedding/factory.rs` — `"remote"` 分支：feature 下从 env（endpoint/model/provider/api_key）构造 `RemoteEmbeddingProvider`、否则明确 feature-未启用 `Err`。
  - `core/Cargo.toml` + `Cargo.lock` — `embedding-remote` feature + `ureq 2.12.1` optional dep（R7 chore）。

- **commit 列表**: `273f528`（RED：骨架 + 测试，pure fns 无缓存实现，2 failed/2 passed）→ `adc6db3`（GREEN：build_request_body + parse_response，4/4 PASS）→ 本 docs 提交。

- **§9 Verification 结果**（实测，ADR-013）:
  - 默认构建 `cargo test --workspace` exit 0；`cargo tree -p contextforge-core | grep ureq` 空 → 默认 0 网络 dep（ADR-004 本地优先）。
  - feature 构建 `cargo test --features embedding-remote -p contextforge-core embedding::remote_provider` 4/4 PASS（TEST-22.3.1 请求构造 / 22.3.2 fixture 解析有序 / 22.3.3 错误路径 / 22.3.4 name+dim+factory）；ureq 2.12.1 在本 Windows MSVC 成功编译。
  - `go test ./...` 本 PR 零 Go delta（CI go-test gate 复核）。
  - D2 lint `--touched origin/master`：scoped touched 0 未标注命中（CI spec-lint gate 权威）。

- **设计取舍**:
  - 请求构造 / 响应解析拆纯函数 → 契约测试无网络可严格断言（ADR-013 deterministic 可验证）。
  - HTTP client 选 `ureq`（同步，匹配同步 `EmbeddingProvider` trait，免 tokio 运行时桥）+ rustls（承 ADR-008 D5，避 OpenSSL）；相较 reqwest+blocking 更轻（少 transitive）。R7 dep chore 已 §10 disclosed + Cargo.toml 注释（包名/版本/用途/替代方案）。
  - 工厂 `"remote"` 从 env 读 endpoint/model/api_key（配置 plumbing 同 22.1 server 缺省，留后续按需接入）；api_key 不入日志、不入 fixture（PRD 安全基线 + ADR-004 opt-in）。
  - name() 用静态字面（"remote-openai"/"remote-cohere"，match provider 选）满足 `&'static str` 约束 + "remote-<provider>" provenance。

- **剩余风险 + 下游影响（含 R1 stop-condition — ADR-013 诚实 defer）**:
  - **远程真实网络联调 / 真实 API 密钥 / 真实召回质量如实 defer** `[SPEC-DEFER:phase-future.embedding-provider-remote]`：CI / 无人值守环境无 OpenAI/Cohere 密钥 + 无网络，本 task **不打真实网络**、**不**伪造真实响应或召回数值（ADR-013）。契约测试（fixture，纯函数）跑通即视骨架达标 — **未**标真实命中。这是 §8 R1 设计内的诚实 stop-condition，非失败。
  - OpenAI 风格为主骨架；Cohere 等 schema 差异以 `provider` 配置分派 / 后续 add-only 扩充。
  - 下游：task-22.4 health 远程可达性探针（feature/opt-in）+ smoke v12 + closeout；远程 provider 可被 task-22.2 `CachingEmbeddingProvider` 正交包裹。
