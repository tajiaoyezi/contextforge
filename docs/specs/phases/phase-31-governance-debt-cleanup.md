# Phase 31 · governance-debt-cleanup

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 清理跨 Phase 累积的治理债（长尾 backlog + Phase 28 follow-up + 旧 nits）：**观测一致性**（`internal/consoleapi/memstore.go` Go fallback memory 变更操作未发 `memory.*` event，与 workspace/job + Rust 数据面路径不对齐）、**缓存硬化**（`core/src/embedding/cache.rs:23` 长跑 daemon 的 L1 `Mutex<HashMap>` 无界增长须 LRU/cap bound；Go memstore cap 硬编码 256 须可配置）、**部署硬化**（生产 compose 缺资源限 / 缺 TLS 终结）、**eval 可查询性**（per-case 结果现序列化为单表 JSON blob → 可查询子表 + add-only migration 0018）、**exporter 全文**（`internal/exporter/source.go:85` content="" 根因 v1 search proto 无 chunk 全文 → 新 `ListAllChunks` RPC 真实全文 + 真实 ContentHash）、以及 **3 个 MCP nits**（protocolVersion 字符串字典序脆弱 / `audit.Write` 吞 err / allowlist 文件 mode 未 warn）。多数为 code-local 🟢 可单测；compose 真实 TLS cert 须域名 🟡；GitHub 原生 attestation 私有仓库 🔴 经核诚实重申延后。并含一项关键校正（ADR-013 诚实复核）：`event-bus-partition` / `event-bus-capacity` **经核 Phase 26 已交付**（`core/src/data_plane/events.rs:24-203` `EventBusConfig`/`Partition`/`from_config` + 生产接线 `server.rs:602-603` + 测试 `TEST-26.3.1a/b/c`），**非治理债** → 本 phase 对其 **verify-only**（既有 core 测试维持绿）+ `docs/roadmap.md §4` add-only 更正剔除该过期 backlog 条目，**不重复实现**（ADR-013）。
>
> **入读顺序（必读）**：本 phase spec → `docs/roadmap.md §3.13 + §4 backlog` → 各债项源码锚点（`internal/consoleapi/memstore.go` Pin/Deprecate/SoftDelete/Unpin/HardDelete + emitEvent / `core/src/embedding/cache.rs:23` / `core/src/eval/store.rs` CaseResult / `internal/exporter/source.go:85` / `internal/mcpadapter/server.go:187,270` + `allowlist.go` / `deploy/docker-compose.production.yml` + `docs/deploy/production.md`）→ `core/src/data_plane/events.rs:24-203`（event-bus partition/capacity 经核 Phase 26 已交付——verify-only 锚点）→ AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5，第二十二次激活）→ `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`（禁伪造红线：exporter 真实 ContentHash / eval 子表真实查询 / cache 真实驱逐 / compose docker compose config 真实 parse，真实 cert / 原生 runner / 原生 attestation 受阻维度如实记录不伪造）。
>
> **ADR 影响面（已识别）**：
> - **ADR-036 governance-debt-cleanup（新，Proposed）**：记 memstore-event-emit Go fallback parity + event-bus partition/capacity verify-only 更正（D1）+ cache LRU + cap configurable + compose resource-limits + 可选 TLS proxy（D2）+ eval case-results 子表 + exporter full-content + 3 MCP nits（D3）+ honest defer 重申（D4）+ 默认行为 / 既有契约不变（D5）。
> - 触及 **ADR-021（event-bus / memory-bus-bridge）**：memstore-event-emit Go fallback parity 使 Go 降级路径 memory 变更与 workspace/job + Rust 数据面对齐；并记 event-bus partition/capacity **经核 Phase 26 已交付**的 add-only 更正（不溯改正文，以 Amendment 记录，ADR-014 D5）。
> - 触及 **ADR-027（embedding-provider）**：`CachingEmbeddingProvider.mem` 无界 L1 加 LRU/cap 驱逐——以 add-only Amendment 记录缓存有界化（默认值 / 既有契约不变）。
> - 触及 **ADR-029（eval）**：per-case 结果由单表 JSON blob 升级到可查询子表 `eval_case_results`（FK + add-only migration 0018）——以 add-only Amendment 记录（既有 `eval_runs` 读不受影响）。
> - 触及 **ADR-033（release）**：multi-arch-native-runner / github-native-attestation 经核受阻维度诚实重申延后——以 add-only Amendment 记录重申（不伪造完成）。
> - 触及 **ADR-004（默认行为 + 既有契约不变）**：exporter `ListAllChunks` 为 add-only RPC、cache cap 默认值不变、compose 限值可选、eval 子表 add-only migration、MCP nits 不破协议——默认行为 / proto / 既有契约均不变（守线，非推翻）。

## 1. 阶段目标

v0.23.0 ship 后，ContextForge 清理跨 Phase 累积的治理债：**观测路径一致**（Go fallback `MemMemoryStore` Pin/Deprecate/SoftDelete/Unpin/HardDelete 与 workspace/job + Rust 数据面一样向 fallback ring 发 `memory.*` event）、**缓存有界**（embedding L1 缓存 LRU/cap bound 防长跑 daemon 内存无界增长 + Go memstore cap 可配置）、**部署可加固**（生产 compose 文档化 / 可选 mem_limit + cpus + 可选 TLS 终结反代）、**eval 可查询**（per-case 结果升子表可 SQL 过滤聚合）、**exporter 全文保真**（导出 record.content 非空 + 真实 ContentHash）、**MCP 健壮**（protocolVersion 解析白名单 + audit 错误浮出 + allowlist 文件 mode 告警）。多数 code-local 🟢 可单测；compose 真实 cert 须域名 🟡；GitHub 原生 attestation 私有仓库 🔴 经核诚实重申。**关键校正**：event-bus partition/capacity 经核 Phase 26 已交付——本 phase verify-only + roadmap §4 add-only 更正，不重复实现（ADR-013）。默认行为 / proto / 既有契约不变（ADR-004）；既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。

**具体可观测的 phase exit criteria（对应 §6 AC）**：

1. **观测一致性 + event-bus verify-only 更正**：`internal/consoleapi/memstore.go` Go fallback `MemMemoryStore` 的 Pin / Deprecate / SoftDelete / Unpin / HardDelete 经 `emitEvent` 向 fallback ring 发 `memory.pin` / `memory.deprecate` / `memory.soft_delete` / `memory.unpin` / `memory.hard_delete`（与 workspace/job fallback + Rust 数据面 `core/src/data_plane/memory.rs:52-106` 对齐；Rust 侧不动）；event-bus partition/capacity **经核 Phase 26 已交付**（`events.rs:24-203` + `server.rs:602-603` + `TEST-26.3.1a/b/c` 维持绿）→ verify-only + roadmap §4 add-only 更正剔除过期 backlog，不重复实现（AC1）
2. **缓存 + 部署硬化**：`core/src/embedding/cache.rs` L1 `mem` 加 LRU/cap 驱逐（超 cap oldest 驱逐 + 驱逐 key 上 inner 重新被调）；`internal/consoleapi/memstore.go:49` cap 由硬编码 256 改 config/env 可配置（未设时默认值不变）；`deploy/docker-compose.production.yml` 加文档化 / 可选 mem_limit + cpus + 可选 TLS 终结反代服务（`docker compose config` 真实 parse 🟢；真实 cert 须域名 🟡 延后）（AC2）
3. **eval 子表 + exporter 全文 + MCP nits + honest defer 重申**：`core/src/eval/store.rs` per-case 结果升子表 `eval_case_results`（FK + add-only migration 0018）可 SQL 查询；`internal/exporter/source.go` 经新 `ListAllChunks` RPC 取 chunk 全文使 record.content 非空 + 真实 ContentHash（修 `fidelity.go::CalcFidelity`）；3 MCP nits 修（protocolVersion 解析白名单 / `audit.Write` 错误浮出 / allowlist 文件 mode stat + warn）；rust-native-eval-runner / multi-arch-native-runner / github-native-attestation 经核受阻 / 无驱动维度诚实重申延后（§3 范围外，非 code AC）（AC3）
4. **默认行为不变 + v0.24.0 closeout**：默认行为 / proto / 既有契约不变（ADR-004）；v0.24.0 release docs + `scripts/console_smoke.sh` [40/40] + ADR-036 据真实测试 ratify + ADR-021/027/029/033 add-only Amendment + roadmap §4 add-only 更正 + phase §6 闭合（AC4）
5. ADR-014 D1-D5（第二十二次激活）全通过（AC5）

**v0.x 版本号决策**：v0.24.0（Phase 31，承 v0.23.0；roadmap §1.1 Phase N→v0.(N-7).0）。minor release（治理债清理 + nits 修；多为 code-local，exporter `ListAllChunks` 为 add-only RPC、cache cap 默认值不变、compose 限值可选、eval 子表 add-only migration、MCP nits 不破协议，默认行为 / proto / 既有契约 / 默认构建 0 新依赖 + 0 网络不变）。

## 2. 业务价值

清理 roadmap §4 长尾 backlog + Phase 28 follow-up + 旧 nits，补齐观测 / 缓存 / 部署 / eval / 导出 / MCP 健壮性缺口，且经核诚实更正一处过期 backlog：

- **观测一致性（memstore-event-emit）**：`internal/consoleapi/memstore.go` 的 `emitEvent` helper 已为 workspace/job 变更（CreateWorkspace / UpdateWorkspaceConfig / EnqueueJob / CancelJob）发 event 入 fallback ring（供 `GET /v1/observability/events` fallback 服务），但 `MemMemoryStore` 的 memory 变更（Pin/Deprecate/SoftDelete/Unpin/HardDelete）**从未发** event——Go 降级路径下 memory 操作在观测面不可见，与 workspace/job + Rust 数据面（`core/src/data_plane/memory.rs:52-106` 已发 `memory.*`）不一致。本 phase 补齐 Go fallback 侧 parity（Rust 侧不动）。
- **event-bus partition/capacity 经核更正（ADR-013 诚实）**：roadmap §4 仍把 `event-bus-partition` / `event-bus-capacity` 列为开放 backlog，但复核源码证实二者经 Phase 26 / ADR-031 D5 已交付（`events.rs:24-203` `from_config` + `server.rs:602-603` 生产接线 + `TEST-26.3.1a/b/c`）。本 phase verify-only（既有测试维持绿）+ roadmap §4 add-only 更正剔除过期条目，如实不重复实现。
- **缓存有界化（cache-lru / cache-cap-configurable）**：`core/src/embedding/cache.rs:23` L1 `Mutex<HashMap>` 无界（插入 :154/:170 无界增长；L2 SQLite `INSERT OR REPLACE` :99-104 亦无界），长跑 daemon 风险内存无界增长；Go memstore cap 硬编码 256（memstore.go:49，doc comment 已带 `[SPEC-DEFER:phase-future.cache-cap-configurable]`）须可配置。本 phase 加 LRU/cap 驱逐 + 配置化 cap。
- **部署硬化（compose-resource-limits / compose-tls-termination）**：`deploy/docker-compose.production.yml` 两服务无 mem_limit/cpus（多租户 / 共享主机资源风险），console-api 明文绑 `0.0.0.0:48181`（无 TLS）。本 phase 加文档化 / 可选资源限 + 可选 TLS 终结反代（caddy/traefik 或文档化 cert-mount）；真实 cert 须域名 🟡。
- **eval 可查询性（case-results-subtable）**：`core/src/eval/store.rs` CaseResult 现序列化为单 `eval_runs` 表的 `case_results_json` 列，无法按 case SQL 过滤 / 聚合。本 phase 升可查询子表 `eval_case_results`（FK + add-only migration 0018）。
- **exporter 全文保真（exporter content="" follow-up）**：`internal/exporter/source.go:85` `loadRecords` 把 content 置 `""` 后 ContentHash 算空串 hash（:96）——根因 v1 search proto `SearchResponse` 不携 chunk 全文。本 phase 加 `ListAllChunks(collection_id)` RPC（task-6.3 §10:335-368 文档化 path B）取真实全文，使 record.content 非空 + 真实 ContentHash（亦修 `fidelity.go::CalcFidelity`）。
- **MCP 健壮（3 nits）**：`internal/mcpadapter/server.go:187` protocolVersion 用原始字符串字典序比较（脆弱）；server.go:270 `writeAudit` 丢弃 `audit.Write` 错误；`allowlist.go::LoadAllowlist` 读文件但从不 stat 警告过宽 mode。本 phase 解析白名单 + 错误浮出 + 文件 mode stat + warn。

**不在本 phase 范围**：

- rust-native-eval-runner（`core/src/eval/runner.rs:26-41` 无 consumer，Go harness 为单一事实源）[SPEC-DEFER:phase-future.rust-native-eval-runner]
- multi-arch-native-runner（`release.yml:57` amd64-only，QEMU emulation 不可行，须原生 arm64 runner）[SPEC-DEFER:phase-future.multi-arch-native-runner]
- github-native-attestation（`actions/attest-*` 私有仓库不可用，cosign keyless 在用，verify-image 已修 PR #193）[SPEC-DEFER:phase-future.github-native-attestation]
- compose 真实 cert 自动签发 / ACME 续期（须真实域名）[SPEC-DEFER:phase-future.compose-tls-auto-cert]
- L2 SQLite 缓存有界化 / 跨进程 LRU [SPEC-DEFER:phase-future.cache-l2-bounded]

## 3. 涉及模块

### 31.1 observability + memstore event parity（task-31.1）

- 修改 `internal/consoleapi/memstore.go`——`MemMemoryStore` 的 Pin(:590-603) / Deprecate(:605-616) / SoftDelete(:618-629) / Unpin(:631-645) / HardDelete(:649-657) 经 `emitEvent` helper(:100-115) 向 capped 1000 fallback ring 发 `memory.pin` / `memory.deprecate` / `memory.soft_delete` / `memory.unpin` / `memory.hard_delete`（parity workspace/job 既有发 event 站点 + Rust 数据面）
- event-bus partition/capacity = **verify-only**（经核 Phase 26 已交付——`core/src/data_plane/events.rs:24-203` `EventBusConfig`/`Partition`/`from_config` + `server.rs:602-603` 生产接线 + `TEST-26.3.1a/b/c`）：既有 core 测试维持绿，**不重复实现**
- Rust 数据面 `core/src/data_plane/memory.rs:52-106` 已发 `memory.*`——**不动**（gap 仅 Go fallback 侧）
- `docs/roadmap.md §4` add-only 更正：剔除 `event-bus-partition` / `event-bus-capacity` 过期 backlog 条目（注明经核 Phase 26 已交付）
- 同源验证（≥2，🟢：Go 单测断言 fallback 模式 Pin 后 ring 增长 / 既有 `TEST-26.3.1a/b/c` 维持绿）

### 31.2 cache + deploy hardening（task-31.2）

- 修改 `core/src/embedding/cache.rs`——`CachingEmbeddingProvider.mem`（`Mutex<HashMap<String,Vec<f32>>>` :23）加 LRU（或 capacity-capped）驱逐策略（插入站点 :154/:170）；超 cap oldest 驱逐、驱逐 key 上 inner provider 重新被调
- 修改 `internal/consoleapi/memstore.go`——`memStoreCacheDefaultCapacity = 256`（:49，doc comment :46-48 已带 `[SPEC-DEFER:phase-future.cache-cap-configurable]`）经 config/env 暴露（cacheCapacity 字段 :41 在 NewMemStore :57 设、FIFO 强制 :73-77/:93-97）；未设时默认 256 不变
- 修改 `deploy/docker-compose.production.yml`——两服务（:20-43, :45-68）加文档化 / 可选 mem_limit + cpus；console-api 明文绑 `0.0.0.0:48181`(:50-58) 加可选 TLS 终结反代服务（caddy/traefik）或文档化 cert-mount。锚点 `docs/deploy/production.md:383-391` + `:165-167`
- `docker compose config` 真实 parse 🟢；真实 cert 须域名 🟡 延后 `[SPEC-DEFER:phase-future.compose-tls-auto-cert]`
- 同源验证（≥2，🟢：cache LRU 驱逐单测 / Go memstore cap config 单测 / `docker compose config` parse；🟡 真实 cert defer）

### 31.3 eval case-subtable + exporter full-content + MCP nits（task-31.3）

- 修改 `core/src/eval/store.rs`——per-case 结果（CaseResult :17-25，现 `case_results_json` 列序列化 JSON；update_case_results :177-193 写 UPDATE / row_to_run :285 读 `serde_json::from_str` / INSERT seed `[]` :118）升可查询子表 `eval_case_results`（FK `eval_run_id`）+ add-only migration 0018（当前最新 0017，承 Phase 27）。锚点 `ADR-029:54` + `roadmap:228`
- 修改 `internal/exporter/source.go`——`loadRecords` content="" (:85) / ContentHash 算空串 (:96) 根因 v1 search proto 无 chunk 全文 → 经新 `ListAllChunks(collection_id)` RPC（task-6.3 §10:335-368 path B）取真实全文 → record.content 非空 + 真实 ContentHash（亦修 `internal/exporter/fidelity.go::CalcFidelity`）
- 修改 `internal/mcpadapter/server.go` + `allowlist.go`——3 nits：(a) :187 protocolVersion `< SupportedProtocolVersion` 原始字符串字典序比较 → 解析 / 白名单已知版本；(b) :270 `writeAudit`(:266-276) `_ = audit.Write(...)` 吞 err → 至少 stderr warn / propagate；(c) `LoadAllowlist`(:19-40) `os.ReadFile`(:20) + JSON parse 但从不 Stat → 过宽（world-readable/writable）mode stat + warn（或拒绝）
- C2/C3/C4 honest defer 重申（§3 范围外带 tag，非 code AC）：rust-native-eval-runner / multi-arch-native-runner / github-native-attestation
- 同源验证（≥2，🟢：eval 子表 FK + migration 0018 单测 / exporter content 非空 + ContentHash 匹配真实全文单测 / 3 MCP nits 单测）

### 31.4 closeout-v0.24.0（task-31.4）

- 修改 `scripts/console_smoke.sh`——banner v20→v21 + v21 changelog block + 新 step [40/40]（doc/status：断言 default-build init baseline + 有运行时面的治理债修复如 memstore-event-emit parity 可达则断言、否则 doc/status；current live [37/37]、Phase 29 计划 [38/38]、Phase 30 计划 [39/39] → Phase 31 顺位 [40/40]）
- 修改 `internal/cli/smoke_syntax_test.go`——新 Test 断言 [40/40] + no-regression（denominators 不溯改，ADR-014 D5）
- 新增 `docs/releases/v0.24.0-evidence.md` + `v0.24.0-artifacts.md`（tag SHA / run id / digest 为 angle-bracket backfill marker）+ `README.md` v0.24 段 + `RELEASE_NOTES.md` v0.24.0 段
- 修改 `docs/decisions/adr-036-governance-debt-cleanup.md`——Status Proposed→Accepted（逐 D 如实：TLS 真实 cert / 原生 runner / attestation 受阻维度部分 ratify）+ 新 `## Ratification（v0.24.0 / task-31.4）`
- add-only Amendment（非正文改，ADR-014 D5）：`adr-021`（memstore-event-emit Go parity + event-bus partition/capacity 经核 Phase 26 已交付的更正记录）/ `adr-027`（cache LRU）/ `adr-029`（case-results subtable）/ `adr-033`（multi-arch-native-runner / github-native-attestation defer 重申）；`docs/roadmap.md §4` add-only 更正剔除 event-bus-partition/capacity（经核 Phase 26 已交付）
- 修改 `docs/specs/phases/phase-31-governance-debt-cleanup.md`——Status Draft→Done + §6 AC 勾选（逐维如实）
- 修改 `docs/s2v-adapter.md`——Phase 31 行 + Task 行 + ADR-036 行 + BDD 行

### BDD feature

- 新增 `test/features/phase-31-governance-debt-cleanup.feature`（≥4 scenario：memstore-event-emit Go parity + event-bus verify-only 更正 / cache LRU + cap configurable + compose 硬化 / eval 子表 + exporter 全文 + MCP nits / v0.24.0 收口）

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 31.1 | `internal/consoleapi/memstore.go` `MemMemoryStore` Pin/Deprecate/SoftDelete/Unpin/HardDelete emit `memory.*` 入 fallback ring（parity workspace/job + Rust 数据面）+ event-bus partition/capacity verify-only（经核 Phase 26 已交付）+ roadmap §4 add-only 更正 | `../tasks/task-31.1-observability-memstore-event-parity.md` |
| 31.2 | `core/src/embedding/cache.rs` L1 LRU/cap 驱逐 + `internal/consoleapi/memstore.go:49` cap config/env 可配置 + `deploy/docker-compose.production.yml` 可选 mem_limit/cpus + 可选 TLS 终结反代 | `../tasks/task-31.2-cache-and-deploy-hardening.md` |
| 31.3 | `core/src/eval/store.rs` per-case 子表 `eval_case_results`（FK + migration 0018）+ `internal/exporter/source.go` 新 `ListAllChunks` RPC 全文 + 真实 ContentHash + 3 MCP nits（protocolVersion 白名单 / audit.Write 错误浮出 / allowlist 文件 mode warn）+ honest defer 重申 | `../tasks/task-31.3-eval-exporter-and-mcp-nits.md` |
| 31.4 | smoke [40/40] + v0.24.0 closeout + ADR-036 ratify + ADR-021/027/029/033 add-only Amendment + roadmap §4 更正 | `../tasks/task-31.4-closeout-v0.24.0.md` |

## 5. 依赖关系

- **task-31.1**（observability + memstore parity）dep 既有 `internal/consoleapi/memstore.go` `emitEvent` helper + fallback ring（已在）+ `core/src/data_plane/events.rs`（verify-only 锚点，已在）；可独立先行（不依赖 31.2/31.3）。
- **task-31.2**（cache + deploy）dep 既有 `core/src/embedding/cache.rs`（CachingEmbeddingProvider 已在，ADR-027）+ `internal/consoleapi/memstore.go` cap 字段（已在）+ `deploy/docker-compose.production.yml`（已在）；与 31.1/31.3 并行无依赖。
- **task-31.3**（eval + exporter + MCP nits）dep 既有 `core/src/eval/store.rs`（ADR-029，migration 0017 承 Phase 27）+ v1 search proto（`ListAllChunks` 为 add-only RPC）+ `internal/exporter/*` + `internal/mcpadapter/*`（均已在）；与 31.1/31.2 并行无依赖。
- **task-31.4**（closeout）dep 31.1 + 31.2 + 31.3 全 Done；release docs / smoke [40/40] / ADR-036 ratify 据三 task 真实测试 / 实测产物。
- 外部：ADR-036（本 phase 新 Proposed）/ ADR-021（event-bus / memory-bus-bridge，memstore parity + event-bus 经核 Phase 26 已交付更正，add-only Amendment）/ ADR-027（embedding-provider，cache LRU add-only Amendment）/ ADR-029（eval，case-results subtable add-only Amendment）/ ADR-033（release，multi-arch-native-runner / github-native-attestation defer 重申 add-only Amendment）/ ADR-004（默认行为 + 既有契约不变）/ ADR-012（tag/release outward-facing 须用户显式授权）/ ADR-014 第二十二次激活 / ADR-013（禁伪造红线，真实测试 / 实测产物，受阻不伪造）。

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（每条 AC 含 ADR-014 D3 verified by 显式 owner；Draft 阶段未勾选，实施后逐条置 `[x]`）**：

- [x] **AC1**（observability + memstore parity；event-bus verify-only 更正 🟢）: `internal/consoleapi/memstore.go` Go fallback `MemMemoryStore` Pin/Deprecate/SoftDelete/Unpin/HardDelete 经 `emitEvent` 向 fallback ring 发 `memory.pin`/`memory.deprecate`/`memory.soft_delete`/`memory.unpin`/`memory.hard_delete`（parity workspace/job + Rust 数据面 `memory.rs:52-106`，Rust 侧不动）；event-bus partition/capacity **经核 Phase 26 已交付**（`events.rs:24-203` + `server.rs:602-603` + `TEST-26.3.1a/b/c` 维持绿）→ verify-only + roadmap §4 add-only 更正，不重复实现（ADR-013）— verified by **TEST-31.1.1**（Go fallback Pin 后 ring 增长）+ **TEST-31.1.2**（event-bus verify-only + roadmap §4 更正）+ phase-smoke step 1
- [x] **AC2**（cache + deploy hardening 🟢/🟡）: `core/src/embedding/cache.rs` L1 `mem` 加 LRU/cap 驱逐（超 cap oldest 驱逐 + 驱逐 key inner 重调）；`internal/consoleapi/memstore.go:49` cap 由硬编码 256 改 config/env 可配置（未设默认 256 不变）；`deploy/docker-compose.production.yml` 加可选 mem_limit + cpus + 可选 TLS 终结反代服务，`docker compose config` 真实 parse 🟢；真实 cert 须域名 🟡 延后 `[SPEC-DEFER:phase-future.compose-tls-auto-cert]` — verified by **TEST-31.2.1**（cache LRU 驱逐）+ **TEST-31.2.2**（cap config）+ **TEST-31.2.3**（compose parse / cert defer）+ phase-smoke step 2
- [x] **AC3**（eval 子表 + exporter 全文 + MCP nits 🟢 + honest defer 重申）: `core/src/eval/store.rs` per-case 升子表 `eval_case_results`（FK + add-only migration 0018，可 SQL 查询，既有 `eval_runs` 读不受影响）；`internal/exporter/source.go` 经新 `ListAllChunks` RPC 取全文 → record.content 非空 + ContentHash 匹配真实全文（非空串 hash）；3 MCP nits（protocolVersion 解析 / 白名单非字典序、`audit.Write` 错误浮出、allowlist 文件 mode stat + warn）修；rust-native-eval-runner / multi-arch-native-runner / github-native-attestation 经核诚实重申延后（§3 范围外带 `[SPEC-DEFER]` tag，非 code AC，不伪造完成）— verified by **TEST-31.3.1**（eval 子表）+ **TEST-31.3.2**（exporter 全文）+ **TEST-31.3.3**（3 MCP nits）+ phase-smoke step 3
- [x] **AC4**（默认行为不变 + v0.24.0 closeout）: 默认行为 / proto / 既有契约不变（ADR-004——exporter `ListAllChunks` add-only RPC、cache cap 默认值不变、compose 限值可选、eval 子表 add-only migration、MCP nits 不破协议）；v0.24.0 release docs（evidence/artifacts/README/RELEASE_NOTES）+ `scripts/console_smoke.sh` [40/40] + `internal/cli/smoke_syntax_test.go` markers 同步 + ADR-036 据真实测试 ratify（逐维如实：真实 cert / 原生 runner / attestation 受阻维度部分 ratify）+ ADR-021/027/029/033 add-only Amendment + roadmap §4 add-only 更正 + phase §6 闭合 — verified by **TEST-31.4.1**（smoke + smoke_syntax_test）+ **TEST-31.4.2**（docs + ADR ratify + Amendment + roadmap 更正 + adapter + feature）
- [x] **AC5**（ADR-014 cross-validation gate）: ADR-014 D1-D5（第二十二次激活）全通过 — D1 mapping + D2 lint `bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-30 不溯改（ADR 改动 add-only Amendment）— verified by task-31.4 closeout PR body + 各 task LAST TEST（TEST-31.1.3 / TEST-31.2.4 / TEST-31.3.4 / TEST-31.4.3）

**端到端 smoke（C1 集成兜底）**：(1) Go fallback `MemMemoryStore` 变更操作向 fallback ring 发 `memory.*` event（Pin 后 ring 增长）+ event-bus `TEST-26.3.1a/b/c` 维持绿（verify-only）全 PASS；(2) embedding cache LRU/cap 驱逐 + Go memstore cap 可配置 + `docker compose config` 真实 parse（含可选资源限 / TLS 服务）全 PASS（真实 cert 🟡 如实标注延后）；(3) eval 子表 SQL 可查询 + exporter record.content 非空且 ContentHash 匹配真实全文 + 3 MCP nits 修，默认行为 / 既有契约不变全 PASS（受阻 / 延后维度如实标注）。

## 7. 阶段级风险

- **R1（低）event-bus verify-only 误判为新债重复实现**：roadmap §4 过期条目易诱导把 partition/capacity 当作待实现治理债。
  - **缓解**：task-31.1 先复核源码锚点（`events.rs:24-203` `from_config` + `server.rs:602-603` 生产接线 + `TEST-26.3.1a/b/c`）确证经 Phase 26 已交付 → verify-only（既有测试维持绿）+ roadmap §4 add-only 更正剔除过期条目，**不写任何重复实现**（ADR-013 诚实）。stop-condition：若复核发现并未交付则升级为真实实现 task 并如实记录（不沿用 verify-only 结论）。
- **R2（中）exporter `ListAllChunks` add-only RPC 触及 v1 search proto / 既有 client 兼容**：根因 v1 search proto 无 chunk 全文，须新增 RPC 而非改既有响应。
  - **缓解**：task-31.3 以 add-only RPC（`ListAllChunks(collection_id)` 或 `GetSourceChunk` body fetch，task-6.3 §10:335-368 path B）实现，既有 `SearchResponse` shape 不动（ADR-004 既有契约不变）；exporter 改用新 RPC 取全文 + 真实 ContentHash。stop-condition：proto 改动须 add-only、既有 client 不破坏方标 AC3。
- **R3（中）embedding cache LRU 驱逐改动潜在 hit-rate / 行为回归**：L1 由无界 HashMap 改 LRU/cap 改变缓存命中语义。
  - **缓解**：task-31.2 LRU 单测断言「超 cap oldest 驱逐 + 驱逐 key 上 inner provider 重新被调」+ cap 默认值足够大不致常态 daemon 退化；L2 SQLite 有界化诚实延后 `[SPEC-DEFER:phase-future.cache-l2-bounded]`。stop-condition：默认 cap 下既有 cache 行为不退化方标 AC2。
- **R4（中）compose 真实 TLS cert 需真实域名 — 🟡 部分维度不可在 CI 内闭环**：`docker compose config` parse 可 CI 验，真实 cert 签发须域名 / ACME。
  - **缓解**：task-31.2 以「`docker compose config` 真实 parse（含可选 TLS 服务 / 资源限）🟢 + 真实 cert 自动签发 🟡 延后 `[SPEC-DEFER:phase-future.compose-tls-auto-cert]`」拆分；AC2 以「parse 通过 + 文档化 cert-mount」满足，真实 cert 维度如实标注延后（不伪造「TLS 已端到端达成」）。stop-condition：真实 cert 不在 CI 闭环则该维度不标达成。
- **R5（低）eval migration 0018 与既有 `eval_runs` 读兼容**：升子表须保既有单表读路径不破。
  - **缓解**：task-31.3 add-only migration 0018（仅加 `eval_case_results` 子表 + FK，不删 `case_results_json` 列）；既有 `row_to_run` 读路径维持绿。stop-condition：既有 `eval_runs` 读单测维持绿方标 AC3。

## 8. Definition of Done

- 4 task spec（31.1-31.4）顶部 `**Status**: Done`
- §6 阶段级 AC1-5 全 `[x]`（受阻 / 延后态按 ADR-013 如实记录，不伪造——如 compose 真实 cert 🟡 延后 / honest defer 三项重申 / event-bus verify-only 经核已交付）
- 端到端 smoke 3 step 全 PASS（含受阻 / 延后态如实标注）
- **ADR**：ADR-036 `Proposed → Accepted`（据真实测试 / 实测产物逐 D 项 ratify，真实 cert / 原生 runner / attestation 受阻维度据已达维度部分 ratify + 如实记录，不强 ratify）；ADR-021 / ADR-027 / ADR-029 / ADR-033 经 add-only Amendment 记录（memstore parity + event-bus 经核 Phase 26 已交付更正 / cache LRU / case-results subtable / defer 重申，不溯改正文，ADR-014 D5）；`docs/roadmap.md §4` add-only 更正剔除 event-bus-partition/capacity 过期 backlog 条目
- **adapter**：§Phase 索引 Phase 31 `Draft → Done` + `Tasks 0 → 4`；§ADR 索引 ADR-036；§BDD 追加 phase-31 feature 行；ADR-021/027/029/033 Amendment 记录
- **release**：`docs/releases/v0.24.0-{evidence,artifacts}.md` + `RELEASE_NOTES.md` v0.24 段 + README v0.24 段
- **smoke**：`scripts/console_smoke.sh` [40/40]（治理债修复 smoke + 既有 step 不退化，denominators 不溯改）+ `internal/cli/smoke_syntax_test.go` markers 同步
- **follow-up**：rust-native-eval-runner `[SPEC-DEFER:phase-future.rust-native-eval-runner]` + multi-arch-native-runner `[SPEC-DEFER:phase-future.multi-arch-native-runner]` + github-native-attestation `[SPEC-DEFER:phase-future.github-native-attestation]` + compose 真实 cert 自动签发 `[SPEC-DEFER:phase-future.compose-tls-auto-cert]` + L2 缓存有界化 `[SPEC-DEFER:phase-future.cache-l2-bounded]` 留 backlog
</content>
</invoke>
