# ADR `036`: `governance-debt-cleanup`

**Status**: Proposed

**Category**: 治理 / 可观测性 / 缓存 / 部署 / eval
**Date**: 2026-06-02
**Decided By**: 主 agent（ADR-012 自治）；tajiaoyezi ratification at v0.24.0 closeout
**Related**: ADR-021 (event-bus / memory-bus-bridge — memstore-event-emit Go fallback parity + event-bus partition/capacity 经核 Phase 26 已交付的 add-only 更正记录) / ADR-027 (embedding-provider — L1 cache LRU/cap add-only) / ADR-029 (eval — case-results 子表 add-only) / ADR-031 (observability-hardening — D5 event-bus partition/capacity 已交付，本 ADR verify-only 引用) / ADR-033 (release-ci-hardening — multi-arch-native-runner / github-native-attestation 诚实重申延后) / ADR-004 (local-first-privacy-baseline — 默认行为 + 既有契约不变) / ADR-012 (main-agent-governance-autonomy) / ADR-013 (禁伪造凭据红线) / ADR-014 (D1-D5，第二十二次激活) / roadmap §3.13 + §4

## Context

ContextForge v0.1-v0.23 在「价值闭环优先」原则下，跨 Phase 累积了一批长尾治理债：roadmap §4 backlog 的若干条目、Phase 28（ADR-033）的 follow-up、以及历次 PR review 遗留的 code-local nits。这些项单独看都不阻断主线，但累积后形成可观测性 / 缓存 / 部署 / eval 几个维度的一致性缺口。本 ADR 把这批债项收敛为一个集中清理 Phase 的处理策略。

实测现状（按债项源码锚点）：

- **可观测性（Go fallback event parity）**：`internal/consoleapi/memstore.go` 的 `emitEvent`（:100-115）把事件追加进一个上限 1000 的环形缓冲，供 `GET /v1/observability/events` 的 fallback 路径读取。它已被 workspace/job 变更调用（`CreateWorkspace` :183 / `UpdateWorkspaceConfig` :232 / `EnqueueJob` :260 / `CancelJob` :308），但 memory 变更从不调用它：`MemMemoryStore` 的 `Pin`（:590-603）/ `Deprecate`（:605-616）/ `SoftDelete`（:618-629）/ `Unpin`（:631-645）/ `HardDelete`（:649-657）。这是 Go fallback 侧的真实债。Rust data-plane `MemoryServer` 已发射 `memory.*` 事件（`core/src/data_plane/memory.rs:52-106`），故 Rust 侧不动，缺口仅在 Go fallback。

- **关键校正（event-bus partition/capacity，ADR-013 诚实）**：roadmap §4（约 line 230/236）仍把 event-bus-partition 与 event-bus-capacity 列为 open backlog。经核，这两项**经 Phase 26 已交付**：`core/src/data_plane/events.rs:24-203` 定义 `EventBusConfig` / `Partition` / `from_config`（capacity 替换硬编码 1000、`memory.*` 与 `indexing.*` 可分区独立 broadcast channel），生产侧已 wire（`server.rs:602-603`），并有测试 `TEST-26.3.1a/b/c`（events.rs:549-605）覆盖。故本 phase 对这两项为 **verify-only**（既有 core 测试保持绿）+ roadmap §4 **add-only 更正记录**（剔除该 stale backlog 条目），不重复实现（ADR-013：经核 Phase 26 已交付，不伪造为新交付）。

- **缓存（无界增长）**：`core/src/embedding/cache.rs` 的 `CachingEmbeddingProvider.mem`（`Mutex<HashMap<String,Vec<f32>>>`，:23）无界，插入（:154/:170）无上限增长；L2 SQLite `INSERT OR REPLACE`（:99-104）亦无上限。长跑 daemon 下 L1 内存随唯一查询数单调增长。Go 侧 `internal/consoleapi/memstore.go` 的 `memStoreCacheDefaultCapacity = 256`（:49，doc comment :46-48 已带 `[SPEC-DEFER:phase-future.cache-cap-configurable]`）硬编码，`cacheCapacity` 字段（:41）在 `NewMemStore`（:57）设定、FIFO 强制（:73-77/:93-97），但不可配置。

- **部署（生产 compose 缺资源限 / TLS）**：`deploy/docker-compose.production.yml` 两个 service（:20-43、:45-68）未设 `mem_limit` / `cpus` / `deploy.resources`（多租户 / 共享主机下无资源护栏，锚点 `docs/deploy/production.md:383-391`）；`console-api` 以明文 `0.0.0.0:48181` 绑定（:50-58），无 TLS（锚点 `docs/deploy/production.md:165-167` + `v0.9.0-artifacts:107`）。真实 cert 签发须域名（🟡），compose-config 解析为 🟢。

- **eval（case 结果不可查询）**：`core/src/eval/store.rs` 的 `CaseResult`（:17-25）以序列化 JSON 存于单表 `eval_runs` 的 `case_results_json` 列（`update_case_results` :177-193 写 `UPDATE ... SET case_results_json`、`row_to_run` :285 经 `serde_json::from_str` 读、INSERT 初始化 `[]` :118）。per-case 结果无法 SQL 过滤 / 聚合（锚点 ADR-029:54 + roadmap:228）。

- **exporter content="" + 3 个 MCP nits**：`internal/exporter/source.go` 的 `loadRecords` 把 `content = ""`（:85），再对空串算 `ContentHash = contentHash(content)`（:96）——根因是 v1 search proto（`proto/contextforge/v1/search.proto`）的 `SearchResponse` 不携带 chunk 全文（此项按 memory 留待后续 task，task-6.3 §10:335-368 记录路径 B）。另 3 个 MCP nits：`internal/mcpadapter/server.go:187`（`protocolVersion` 与 `SupportedProtocolVersion` 做原始字符串字典序比较，脆弱）、`server.go:270`（`writeAudit` :266-276 以 `_ = audit.Write(...)` 吞错）、`internal/mcpadapter/allowlist.go` `LoadAllowlist`（:19-40）`os.ReadFile`（:20）+ JSON 解析但从不 `Stat` 文件以对 world-readable/writable mode 告警。

- **诚实延后维度（ADR-033 重申）**：rust-native-eval-runner（`core/src/eval/runner.rs:26-41` 为占位、无 consumer，Go harness 为单一事实源）、multi-arch-native-runner（`release.yml:57` 仅 amd64，QEMU emulation 实测不可行，须原生 arm64 runner）、github-native-attestation（私有仓库不可用，cosign keyless 在用，verify-image 已于 PR #193 修）。

本 ADR 记录上述清理的处理策略。改动**多为 code-local 🟢 可单测**（Go fallback event / cache eviction / cache cap config / eval 子表 / exporter RPC / MCP nits）；compose 真实 TLS cert 须域名（🟡），compose-config parse 🟢；github-native-attestation 受私有仓库限制 🔴。全部改动遵守 ADR-004 默认行为不变 + ADR-013 受阻 / 无驱动维度诚实不伪造。

## Decision

治理债清理采用 **「先核实再行动 + add-only 演进 + 诚实重申延后」** 策略，分 5 个决策点：

### D1 — memstore-event-emit Go fallback parity + event-bus partition/capacity verify-only 更正（task-31.1）

Go fallback 的 memory 变更（`MemMemoryStore` 的 Pin / Deprecate / SoftDelete / Unpin / HardDelete）经 `emitEvent` 向 fallback ring 发射 `memory.pin` / `memory.deprecate` / `memory.soft_delete` / `memory.unpin` / `memory.hard_delete`，与 workspace/job 路径及 Rust data-plane 路径对齐；Rust 侧不动（已发射）。event-bus partition/capacity 为 **verify-only**：既有 `TEST-26.3.1a/b/c` 保持绿，roadmap §4 以 add-only 更正剔除该 stale backlog 条目。

**理由**：Go fallback memory ops 与 workspace/job + Rust 路径的 event 对齐是观测一致性的真实缺口（fallback 模式下 memory 变更对 `GET /v1/observability/events` 不可见）。event-bus partition/capacity 经核 **Phase 26 / ADR-031 D5 已交付**（`events.rs` `from_config` + `server.rs:602-603` + `TEST-26.3.1a/b/c`）→ roadmap §4 add-only 更正剔除，如实不重复实现（ADR-013）。

### D2 — cache + deploy hardening：embedding-cache LRU + 可配置 cache cap + compose 资源限 + 可选 TLS proxy（task-31.2）

`core/src/embedding/cache.rs` 的 L1（`mem` HashMap）加 LRU（或容量上限）驱逐策略，bound 内存增长（L2 SQLite 可选同步上限）。`internal/consoleapi/memstore.go` 的 `memStoreCacheDefaultCapacity`（硬编码 256）经 config/env 暴露，unset 时保留默认值。`deploy/docker-compose.production.yml` 加（文档化 / 可选）`mem_limit` + `cpus`，并加可选 TLS-terminating reverse-proxy service（caddy/traefik）或文档化 cert-mount。

**理由**：长跑 daemon 的 embedding 缓存（`cache.rs:23` 无界 HashMap）须 LRU/cap bound 防内存无界增长；Go memstore cap 硬编码 256（`memstore.go:49`）须可配置；生产 compose 缺资源限 / TLS（多租户 / 共享主机风险）。真实 cert 须域名（🟡），compose-config parse 🟢。

### D3 — eval case-results subtable + exporter full-content + MCP nits（task-31.3）

`core/src/eval/store.rs` 的 per-case 结果从 `case_results_json` JSON blob 提升为可查询子表 `eval_case_results`（FK `eval_run_id`）+ add-only migration 0018（当前最新为 0017）；既有 `eval_runs` 读不受影响。exporter 经新增 `ListAllChunks(collection_id)` RPC（返回 chunk 全文，即 task-6.3 §10:335-368 记录的路径 B）或 `GetSourceChunk` body 取回，填实 `content` + 真实 `ContentHash`（同时修 `internal/exporter/fidelity.go` `CalcFidelity`）。3 个 MCP nits：`protocolVersion` 改解析 / 白名单已知版本（非字典序）；`audit.Write` 错误 surface（log / 至少 stderr warn，不吞）；`LoadAllowlist` 加 `Stat` + 对过宽 file-mode warn（或拒绝）。

**理由**：case 结果 JSON blob（`store.rs`）→ 可查询子表（SQL 过滤聚合）；exporter `content=""`（`source.go:85`，根因 v1 search proto 无 chunk 全文）须经新 `ListAllChunks` RPC 真实全文 + 真实 `ContentHash`（修 sha256-of-empty）；3 MCP nits（`protocolVersion` 字符串字典序脆弱 / `audit.Write` 吞 err / allowlist 文件 mode 未 warn）修。RPC 为 add-only、不破 proto / 既有契约。

### D4 — 诚实延后维度重申（task-31.3，无 code AC，文档于 §范围外）

rust-native-eval-runner、multi-arch-native-runner、github-native-attestation 三项在 task-31.3 §范围外以 `[SPEC-DEFER]` tag 重申延后，不作为本 phase 的 code AC。

**理由**：据 ADR-013，对受阻 / 无驱动维度诚实重申延后、不伪造完成：rust-native-eval-runner（`runner.rs:26-41` 占位、无 consumer，Go harness 为单一事实源）；multi-arch-native-runner（QEMU emulation 实测不可行，须原生 arm64 runner，承 ADR-033 D1 DEFERRED）；github-native-attestation（私有仓库不可用，cosign keyless 在用，verify-image 已于 PR #193 修，承 ADR-033 §Follow-ups）。

### D5 — 默认行为 + 既有契约不变（所有 task）

所有改动保持默认行为 / proto / 既有契约不变：exporter `ListAllChunks` 为 add-only RPC、cache cap 默认值不变、compose 限值可选、eval 子表为 add-only migration、MCP nits 不破协议；既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。

**理由**：ADR-004。本 phase 为纯硬化 + nits 修——非功能行为演进。add-only / 默认值保留 / 可选化使既有用户与既有契约零感知，符合 ADR-004 本地优先 / 隐私基线与既有部署契约。

## Consequences

- **Positive**: Go fallback memory 变更经 `GET /v1/observability/events` 可见（与 workspace/job + Rust 路径观测一致）；event-bus partition/capacity 的 stale backlog 条目经核更正（roadmap §4 add-only，不重复实现）；长跑 daemon 的 embedding L1 缓存内存有界（LRU/cap）；Go memstore cap 可配置；生产 compose 得（可选）资源限 + TLS proxy；eval per-case 结果可 SQL 查询（子表）；exporter 导出 record.content 非空 + ContentHash 落在真实内容（非 sha256-of-empty）；3 MCP nits 修。全部 add-only / 默认值保留 / 可选化，默认行为 + proto + 既有契约不变（ADR-004），既有三门不退化。
- **Negative / open**: compose 真实 TLS cert 须域名 / DNS / ACME（🟡，无域名环境只能验 compose-config parse + 文档化 cert-mount，真实签发延后 `[SPEC-DEFER:phase-future.compose-tls-auto-cert]`，受阻维度如实不伪造）；github-native-attestation 私有仓库受阻（🔴，cosign 在用，承 ADR-033）；multi-arch-native-runner 须原生 arm64 runner（QEMU emulation 实测不可行，承 ADR-033 D1）；rust-native-eval-runner 无 consumer 驱动（Go harness 单一事实源）——以上受阻 / 无驱动维度据 ADR-013 如实记录、不伪造完成。
- **Ratification**: 本 ADR **Proposed**。task-31.1..31.3 通过后于 v0.24.0 closeout（task-31.4）据真实测试 / 实测 ratify Proposed→Accepted（ADR-013：禁据合成 / 伪造 ratify）；TLS 真实 cert / native arm64 runner / github-native-attestation（私有仓库受阻）等维度据已达维度 ratify + 如实记录受阻，不强 ratify。
- **Follow-ups**: compose 真实 TLS cert 自动签发（ACME / 域名后）`[SPEC-DEFER:phase-future.compose-tls-auto-cert]`；rust-native-eval-runner 接入（有 consumer 后）`[SPEC-DEFER:phase-future.rust-native-eval-runner]`；multi-arch 原生 arm64 runner `[SPEC-DEFER:phase-future.multi-arch-native-runner]`；GitHub 原生 attestation（仓库改公开 / 升 GHEC 后）`[SPEC-DEFER:phase-future.github-native-attestation]`；L2 SQLite 缓存上限 / 全局清理策略（若 D2 仅 bound L1）`[SPEC-DEFER:phase-future.l2-cache-eviction]`。ADR-021 / ADR-027 / ADR-029 / ADR-033 的扩展面均以各自 add-only Amendment 于 task-31.4 记录（不溯改正文，ADR-014 D5）。
