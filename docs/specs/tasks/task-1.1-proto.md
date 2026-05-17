# Task `1.1`: `proto — gRPC + canonical-record 契约冻结`

> ✅ **Status: Ready** — 已过 `/s2v-implement` §2A 前置审核（2026-05-17）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受。状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: Ready

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 1 (foundation)
**Dependencies**: 无（Phase 1 首个 task，所有后续 phase 依赖本 task 冻结的契约）

## 1. Background

ContextForge 是 Go 控制面 + Rust 数据面双二进制、经 local gRPC 通信的架构（PRD §Decisions Log D1）。所有 phase 都依赖统一的 canonical record schema 与 gRPC proto；契约不先冻结会导致多 phase 并行持续返工（PRD §Technical Risks R1）。本 task 定义并冻结 `proto/contextforge/v1/` 下的 proto 契约与 canonical record schema v0.1。

## 2. Goal

`proto/contextforge/v1/` 下 context / search / import / eval proto 定义完成并可由 Go + Rust 两侧 codegen；canonical record（SourceRecord / ContextRecord / Chunk / RetrievalResult）schema v0.1 与 proto 一一对应并打上 `schema_version="0.1"`；契约版本化，后续仅允许「加字段、不删不改 tag」。

## 3. Scope

### In Scope

- `proto/contextforge/v1/` 下 context / search / import / eval `.proto` 定义：`SourceRecord` / `ContextRecord` / `Chunk` / `RetrievalResult` 四类对象 + `SearchRequest` / `SearchResponse` + Phase 1 gRPC `Health` 契约
- canonical record schema v0.1 字段与 proto 一一对应，统一打 `schema_version="0.1"`
- Go（protoc-gen-go / grpc）+ Rust（tonic-build / prost）双侧 codegen 管线（buf 或 protoc）与「可生成」验证
- proto 版本化规则声明：仅加字段、不删不改 tag（契约冻结声明）
- 双侧 codegen 产物可被各自 import 的最小骨架（非业务实现）

### Out Of Scope

- gRPC service 方法业务实现（扫描 / 解析 / 索引 / 检索 / 导入 —— Phase 2+；本 task 仅冻结 message / service 契约）
- embedding / vector 相关字段（P1，v0.1 不强依赖，见 ADR-002）
- REST / MCP 传输层实现（Phase 6 / 7）
- 配置 / denylist 加载（task 1.2）、core daemon 启动逻辑（task 1.3）

## 4. Users / Actors

- 下游所有 phase 的实施 agent（消费本 task 冻结的 proto 契约）
- Go 控制面 `contextforge` 与 Rust 数据面 `contextforge-core` 的 codegen 工具链
- 跨 Agent 上下文迁移 / 审计 / 导出的契约消费者（exporter / memoryops / mcp-adapter）

## 5. Behavior Contract

描述 proto / canonical record 的外部契约。

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Technical Approach Canonical Record v0.1 最小 schema + REST/MCP 最小接口契约草案）
- `docs/specs/phases/phase-1-foundation.md`
- `docs/decisions/adr-001-go-rust-dual-binary-architecture.md`
- `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md`
- `docs/decisions/adr-003-cli-rest-mcp-grpc-interfaces.md`
- `test/features/proto.feature`

### 5.2 Imports

- proto 内部：`google/protobuf/timestamp.proto`（created_at / updated_at / expires_at 等时间字段）、`google/protobuf/struct.proto`（`metadata.extra` 任意字段）
- Go 生成侧：`google.golang.org/grpc`、`google.golang.org/protobuf`（生成包落 `proto/contextforge/v1` Go package）
- Rust 生成侧：`tonic`、`prost`（构建期 `tonic-build`，生成落 `core/src/pb/`）
- 本 task 是最上游契约源，**不** import 任何 internal / core 业务模块

### 5.3 函数签名

> proto 契约即本 task 的"签名"（message / service 定义）；下游按此 codegen + 实现。

- `message SourceRecord { … }` / `message ContextRecord { … }`（含 §6 AC1 枚举的全部最小字段 + 可选 `title`，PRD JSON 有）/ `message Chunk { … }` / `message RetrievalResult { … }`
- `message SearchRequest { string query = 1; repeated string collections = 2; repeated string agent_scope = 3; int32 top_k = 4; SearchFilters filters = 5; bool explain = 6; }`
- `message SearchResponse { repeated RetrievalResult results = 1; }`
- `service ContextService { rpc Search(SearchRequest) returns (SearchResponse); rpc Health(HealthRequest) returns (HealthResponse); }`（Health 支撑 phase-1 §6 gRPC health check）
- codegen 入口：`buf generate`（或 `protoc`）产出 Go `proto/contextforge/v1/*.pb.go` + Rust `core/src/pb/contextforge.v1.rs`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：
     - init/add 基于 PRD 推导出 AC 内容，**完整写出**（不挂 <TBD-by-user> 前缀）
     - 每条 AC 加引用：`- [ ] **AC<N>** (PRD §<reference>): <内容>`
       - PRD 已写明 → 引用精确章节，例 `(PRD §AC.1)` / `(PRD §Behavior Contract)`
       - PRD 没写、由 task 推导 → 标 `(本 task 新增)`
     - 用户 review 阶段：发现偏差直接改 AC 内容；review 通过**无需删除本注释**
     - **严禁** `- [ ] <TBD-by-user> AC<N>: 内容` 混合写法（伪决策疲劳源）
-->

- [ ] **AC1** (PRD §Technical Approach Canonical Record v0.1 最小 schema): `ContextRecord` proto 含 PRD 列出的全部最小字段（id / schema_version / collection_id / source_type / source_provider / source_uri / agent_scope / content / content_hash / redaction_status / language / file_path / line_start / line_end / tags / provenance[] / security_labels / created_at / updated_at / expires_at / version / metadata.extra），未识别字段进 `metadata.extra` 不影响核心字段。
- [ ] **AC2** (PRD §Technical Approach): 额外定义 `SourceRecord` / `Chunk` / `RetrievalResult` 三类对象的 proto，四类对象边界与字段与 PRD §Technical Approach 一致。
- [ ] **AC3** (PRD §Technical Approach REST/MCP 最小接口契约草案): `search` proto 的请求/响应字段与 PRD 草案一致（query/collections/agent_scope/top_k/filters/explain → results[].chunk_id/context_id/source_type/file_path/line_start/line_end/score/retrieval_method/reason/agent_scope/redaction_status/provenance）。
- [ ] **AC4** (PRD §Decisions Log D1): proto 可由 Go（grpc-go）与 Rust（tonic）两侧 codegen 成功，无 FFI。
- [ ] **AC5** (PRD §Technical Risks R1 / 本 task 新增): proto 与 canonical record 标注 `schema_version="0.1"` 并写明版本化规则「仅加字段、不删不改 tag」（契约冻结声明）。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 ContextRecord 最小字段 | SCEN-1.1.1 | TEST-1.1.1 | - | unit-test | Not Started |
| AC2 四类对象 proto | SCEN-1.1.2 | TEST-1.1.2 | - | unit-test | Not Started |
| AC3 search 契约一致 | SCEN-1.1.3 | TEST-1.1.3 | - | unit-test | Not Started |
| AC4 Go+Rust codegen | SCEN-1.1.4 | TEST-1.1.4 | - | unit-test / typecheck | Not Started |
| AC5 schema 版本化冻结 | SCEN-1.1.5 | TEST-1.1.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R1**（Go↔Rust gRPC 边界 / 契约演进）：本 task 是 R1 主缓解落点，必须一次冻结契约 + 版本化规则。
- 关联 PRD §Open Questions **O9**（canonical record 最小 schema 边界、版本号、兼容策略最终冻结）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch  <!-- 与 adapter §Commands Install 一致 -->
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制：实施 agent 不允许 N/A -->

> 仅列 Install/Typecheck/Unit：adapter §Commands 其余字段（lint/integration/e2e/build/coverage/runtime-smoke）为 `<...>` 占位，按 init.md 步 8 §9 规则省略；用户在 adapter 补全对应字段后，如本 task 需要可在此补列。

## 10. Completion Notes

- **完成日期**：`<TBD-after-impl>`
- **改动文件**：`<TBD-after-impl>`
- **commit 列表**：`<TBD-after-impl>`
- **§9 Verification 结果**：
  - install: `<TBD-after-impl>`
  - typecheck: `<TBD-after-impl>`
  - unit-test: `<TBD-after-impl>`
- **剩余风险 / 未做项**：`<TBD-after-impl>`
- **下游 task 影响**：`<TBD-after-impl>`
