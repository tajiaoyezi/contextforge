# Task `31.3`: `eval-exporter-and-mcp-nits — eval case-results 子表（add-only migration 0018）+ exporter content="" 经新 ListAllChunks RPC 真实全文 + 真实 ContentHash + 3 MCP nits（protocolVersion 解析白名单 / audit.Write err 不吞 / allowlist 文件 mode warn）+ C2/C3/C4 诚实延后重申`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 31 (governance-debt-cleanup)
**Dependencies**: 既有 `core/src/eval/store.rs`（task-14.1 `SqliteEvalStore`，单表 `eval_runs` + `case_results_json` JSON blob）/ 既有 `migrations/0017_*.sql`（Phase 27 最新 migration，本 task add-only 接 0018）/ ADR-029（eval-quality-harness——case-results 子表为 add-only 子表演进，task-31.4 Amendment）/ 既有 `internal/exporter/source.go`（task-6.3 pseudo full-scan 导出，`content=""` 根因）/ task-6.3 §10:335-368（path B：`ListAllChunks` 全文路径，memory 记其留待后续 task）/ `proto/contextforge/v1/search.proto`（v1 search proto；SearchResponse 不携 chunk 全文）/ 既有 `internal/mcpadapter/server.go` + `allowlist.go`（task-7.1 MCP adapter）/ ADR-004（local-first-privacy-baseline，默认行为 + 既有契约不变）/ ADR-013（禁伪造数值 / run-id 红线）/ ADR-014 D1-D5（第二十二次激活）

## 1. Background

三处跨 Phase 累积的治理债 + 一组诚实延后重申，均 code-local（🟢 可单测），无 outward-facing：

- **C1 eval case-results 仍是 JSON blob**：`core/src/eval/store.rs` 的 `CaseResult`（`:17-25`）以序列化 JSON 存进单表 `eval_runs` 的 `case_results_json` 列——`update_case_results`（`:177-193`）写 `UPDATE ... SET case_results_json`，`row_to_run`（`:285`）读 `serde_json::from_str`，INSERT 以 `[]` 初始化（`:118`）。整列 blob 无法按 case 维度 SQL 过滤 / 聚合（如「跨 run 找某 case_id 历次 score」「统计 passed 比例」须全行反序列化在内存里筛）。ADR-029:54 + roadmap:228 记此为子表演进项。
- **D(a) exporter content="" 导致 ContentHash 是空串哈希**：`internal/exporter/source.go` 的 `loadRecords` 把 `content = ""`（`:85`），随后 `ContentHash = contentHash(content)`（`:96`）对空串求哈希。**根因**：v1 search proto 的 `SearchResponse`（`proto/contextforge/v1/search.proto`）不携 chunk 全文，导出热路径无处取正文。memory 记此 trade-off 留待后续 task（task-6.3 §10:335-368 documented path B）。
- **D(b) 3 个 MCP nits**：`internal/mcpadapter/server.go:187` 用裸字符串字典序比较 `protocolVersion < SupportedProtocolVersion`（脆弱——版本号字典序非语义序）；`server.go:270` 的 `writeAudit`（`:266-276`）以 `_ = audit.Write(...)` 吞掉错误（审计失败静默）；`internal/mcpadapter/allowlist.go` 的 `LoadAllowlist`（`:19-40`）`os.ReadFile`（`:20`）+ JSON parse 但从不 `Stat` 文件以在 world-readable/writable mode 时 warn（允许名单文件权限过松无告警）。
- **C2/C3/C4 诚实延后重申**（documented，非 code AC）：rust-native-eval-runner / multi-arch-native-runner / github-native-attestation 三项据 ADR-013 受阻 / 无驱动维度诚实重申延后，不伪造完成（见 §3 范围外）。

## 2. Goal

本 task 聚焦三处 code-local 债项的真实修复 + 三项延后的诚实重申：

1. **C1**：把 per-case 结果提升为可查询子表 `eval_case_results`（FK `eval_run_id` → `eval_runs`）+ add-only migration `0018`（当前最新为 Phase 27 的 `0017`），既有 `eval_runs` 读路径不受影响（向后兼容）。
2. **D(a)**：新增 `ListAllChunks(collection_id)` RPC（返回 chunk 全文——task-6.3 §10 documented path B）或 `GetSourceChunk` body 取，使 exporter `loadRecords` 填真实 `content` + 真实 `ContentHash`（同步修 `internal/exporter/fidelity.go` 的 `CalcFidelity`）。
3. **D(b)**：3 MCP nits 修——`protocolVersion` 改解析 / 白名单已知版本（非字典序）；`writeAudit` 的 `audit.Write` 错误至少 stderr warn / 上抛（不吞）；`LoadAllowlist` 加 `Stat` + 在过松 mode 时 warn（或拒绝）。

pass bar：eval 子表 per-case SQL 可查 + 既有 `eval_runs` 读不变；exporter 导出 record.content 非空 + ContentHash 匹配真实正文（非 sha256-of-empty）；3 MCP nits 修且不破协议；C2/C3/C4 范围外条目各带 `[SPEC-DEFER]` tag；默认行为 / proto 既有字段 / 既有契约不变（ADR-004）；D2 lint 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- **C1 case-results-subtable**：add-only migration `0018_eval_case_results.sql` 建子表 `eval_case_results`（列至少：`eval_run_id` FK、`case_id`、`query`、`expected_chunks`、`actual_chunks`、`score`、`passed`）；`core/src/eval/store.rs` 的 `update_case_results` 在写 `case_results_json`（保留向后兼容读）之外，向子表写 per-case 行；新增按 case 维度查询的读方法（如 `query_case_results(eval_run_id)` / 跨 run 按 `case_id` 聚合）。既有 `row_to_run`（`:285`）整 run 读路径不变。
- **D(a) exporter full-content**：新增 `ListAllChunks(collection_id)`（或 `GetSourceChunk` body 取）RPC 返回 chunk 全文（task-6.3 §10:335-368 path B）；`internal/exporter/source.go` 的 `loadRecords` 改用该 RPC 填真实 `content`（取代 `:85` 的 `content = ""`）+ 真实 `ContentHash`（`:96`）；同步使 `internal/exporter/fidelity.go` 的 `CalcFidelity` 基于真实正文计算。
- **D(b) 3 MCP nits**：`internal/mcpadapter/server.go:187` `protocolVersion` 改 parse / 白名单已知版本（非裸字典序比较）；`server.go:270` `writeAudit`（`:266-276`）的 `audit.Write` 错误 stderr warn / 上抛（不再 `_ =` 吞）；`internal/mcpadapter/allowlist.go` `LoadAllowlist`（`:19-40`）加 `os.Stat` + 在 world-readable/writable mode 时 warn（或拒绝）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **C2 rust-native-eval-runner** [SPEC-DEFER:phase-future.rust-native-eval-runner]——`core/src/eval/runner.rs:26-41` 为占位无 consumer，Go harness 是单一事实源；据 ADR-013 无驱动维度诚实重申延后，不伪造完成。
- **C3 multi-arch-native-runner** [SPEC-DEFER:phase-future.multi-arch-native-runner]——`release.yml:57` 单 amd64，QEMU emulation 实测不可行（task-28.1 已据实延后），须原生 arm64 runner；据 ADR-013 受阻维度重申延后。
- **C4 github-native-attestation** [SPEC-DEFER:phase-future.github-native-attestation]——`release.yml` cosign keyless 在用，`actions/attest-*` 在用户私有仓库不可用（task-28.2 run 26789731232 failure 实测确认），cosign-verify 已于 PR #193 修；据 ADR-013 受阻维度重申延后。
- cache / deploy 硬化（embedding-cache LRU / compose 资源限 / TLS proxy）[SPEC-OWNER:task-31.2-cache-and-deploy-hardening]
- observability + memstore event parity [SPEC-OWNER:task-31.1-observability-memstore-event-parity]
- v0.24.0 closeout / release tag [SPEC-OWNER:task-31.4-closeout-v0.24.0]

## 4. Actors

- 主 agent（ADR-012 自治；本 task 全 code-local，无 outward-facing）
- `core/src/eval/store.rs`（`SqliteEvalStore`——case-results 子表写 / 读）
- `migrations/0018_eval_case_results.sql`（add-only 子表 DDL + FK）
- `internal/exporter/source.go` + `fidelity.go`（exporter——经 `ListAllChunks` RPC 取真实全文）
- `proto/contextforge/v1/search.proto`（新增 `ListAllChunks` / `GetSourceChunk` RPC 契约——add-only）
- `internal/mcpadapter/server.go` + `allowlist.go`（MCP adapter——3 nits 修）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/eval/store.rs:17-25`（`CaseResult` 结构）+ `:177-193`（`update_case_results` 写 `case_results_json`）+ `:285`（`row_to_run` 读 `serde_json::from_str`）+ `:118`（INSERT 以 `[]` 初始化）
- `internal/exporter/source.go:85`（`content = ""`）+ `:96`（`ContentHash = contentHash(content)` 对空串）+ `internal/exporter/fidelity.go`（`CalcFidelity` 基于 content）
- `proto/contextforge/v1/search.proto`（`SearchResponse` 不携 chunk 全文——根因；新增 `ListAllChunks(collection_id)` / `GetSourceChunk` RPC 处）
- task-6.3 §10:335-368（path B：`ListAllChunks` 全文路径 documented；memory 记留待后续 task）
- `internal/mcpadapter/server.go:187`（`protocolVersion < SupportedProtocolVersion` 裸字符串字典序）+ `:270`（`writeAudit` 的 `_ = audit.Write(...)` 吞 err，体 `:266-276`）
- `internal/mcpadapter/allowlist.go:19-40`（`LoadAllowlist` `os.ReadFile :20` + JSON parse，无 `Stat` / mode warn）
- ADR-029:54 + roadmap:228（case-results 子表演进项锚点）

### 5.2 关键设计 — add-only 子表 + add-only RPC + nits 修（不破既有契约）

- **C1 子表 add-only，双写向后兼容**：migration `0018` 仅 `CREATE TABLE eval_case_results`（不动 `eval_runs` / `case_results_json` 列）。`update_case_results` 双写——保留写 `case_results_json`（既有 `row_to_run :285` 读路径不变）+ 向子表写 per-case 行。新增读方法走子表做 SQL 维度查询。`0018` 是 add-only，旧库经 migration 升级后既有 run 读不受影响（旧 run 的子表行可能为空，整 run 读仍走 JSON blob 兼容）。
- **D(a) RPC add-only，根因在 proto 缺全文**：`SearchResponse` 不携 chunk 全文是根因——新增 `ListAllChunks(collection_id)`（或 `GetSourceChunk` body 取）为**新增 RPC**，不改既有 `Search` RPC / `SearchResponse` 既有字段（add-only proto，ADR-004 既有契约不变）。`loadRecords` 改调新 RPC 填真实 `content` + 真实 `ContentHash`；`CalcFidelity` 随真实正文得真实保真度（取代 sha256-of-empty 假象）。
- **D(b) nits 不破协议**：`protocolVersion` 比较从裸字典序改为「解析 / 已知版本白名单」——拒绝不在白名单的版本，但**对当前 `SupportedProtocolVersion` 行为不变**（不破已握手客户端）。`writeAudit` 的 `audit.Write` err 从 `_ =` 吞改为 stderr warn / 上抛——审计失败可见，但**不改 happy-path**。`LoadAllowlist` 加 `os.Stat` + 过松 mode warn（或拒绝）——既有「文件缺 = 空名单拒绝所有」语义不变。

### 5.3 不变量

- 默认行为 / `eval_runs` 既有读路径（`row_to_run :285`）/ `SearchResponse` 既有字段 / MCP 既有握手协议**不变**（ADR-004）；C1 子表 + D(a) RPC 均 add-only，D(b) nits 不破协议。
- migration `0018` 为 add-only（仅新建子表 + FK，不 DROP / ALTER 既有列）；接 Phase 27 最新 `0017` 序号连续。
- 0 伪造数值 / run-id（ADR-013）：真实 SQL 子表查询结果 / 真实导出 content / 真实 ContentHash 待实测回填，不预填数值。
- C2/C3/C4 为诚实延后重申（documented，非 code AC），各带 `[SPEC-DEFER]` tag——不伪造完成。
- 0 outward-facing（全 code-local + proto add-only + migration add-only；无 tag / release / GHCR）。

## 6. Acceptance Criteria

- [ ] AC1（eval case-results 子表）: add-only migration `0018_eval_case_results.sql` 建子表 `eval_case_results`（FK `eval_run_id`）；`store.rs` `update_case_results` 双写子表 + 保留 `case_results_json`；per-case 行 SQL 可查询；既有 `eval_runs` / `row_to_run` 读路径不受影响 — verified by **TEST-31.3.1**（🟢；真实 SQL 子表查询结果待实测回填）
- [ ] AC2（exporter full content via ListAllChunks RPC）: 新增 `ListAllChunks(collection_id)`（add-only proto RPC）；`source.go` `loadRecords` 填真实 `content`（非 `""`）+ 真实 `ContentHash`（非 sha256-of-empty）；`fidelity.go` `CalcFidelity` 随真实正文 — verified by **TEST-31.3.2**（🟢；真实导出 content / ContentHash 待实测回填）
- [ ] AC3（3 MCP nits 修）: `server.go:187` `protocolVersion` 解析 / 白名单（非字典序）；`server.go:270` `audit.Write` err stderr warn / 上抛（不吞）；`allowlist.go` `LoadAllowlist` `Stat` + 过松 mode warn；均不破既有协议 — verified by **TEST-31.3.3**（🟢）
- [ ] AC4（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-31.3.4**（🟢；CI spec-lint 权威）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-31.3.1 | eval case-results 子表 `eval_case_results`（FK + add-only migration 0018）——per-case SQL 可查；既有 `eval_runs` 读不受影响 | `core/src/eval/store.rs` + `migrations/0018_eval_case_results.sql` | Planned |
| TEST-31.3.2 | exporter 经新 `ListAllChunks` RPC 取真实全文——导出 record.content 非空 + ContentHash 匹配真实正文（非 sha256-of-empty） | `proto/contextforge/v1/search.proto` + `internal/exporter/source.go` + `internal/exporter/fidelity.go` | Planned |
| TEST-31.3.3 | 3 MCP nits——`protocolVersion` 解析 / 白名单（非字典序）；`audit.Write` err 不吞；allowlist 文件 mode `Stat` + 过松 warn | `internal/mcpadapter/server.go` + `internal/mcpadapter/allowlist.go` | Planned |
| TEST-31.3.4 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）proto add-only RPC 牵动 buf generate + 多语言桩**：新增 `ListAllChunks` RPC 须 `buf generate` 重生 Rust / Go 桩，daemon 侧须实现 server handler。
  - **缓解**：严格 add-only（新 RPC，不动 `Search` / `SearchResponse` 既有字段，ADR-004）；buf generate 后跑全量 `cargo test --workspace` + `go test ./...` 验既有契约不退化；handler 真实取 chunk 全文（非合成），真实导出 content 待实测回填（ADR-013）。stop-condition：daemon 侧无可达 chunk store 取全文 → 如实记录受阻，AC2 不标 `[x]`，不伪造。
- **R2（中）migration 0018 序号 / FK 与既有库兼容**：旧库（仅到 `0017`）升级须幂等且不破既有 run。
  - **缓解**：`0018` 纯 `CREATE TABLE`（add-only，无 ALTER / DROP）；旧 run 子表行可能空，整 run 读仍走 `case_results_json` JSON blob 兼容（`row_to_run :285` 不变）；测试覆盖「旧 run JSON 读 + 新 run 子表查」并存。
- **R3（低）MCP `protocolVersion` 白名单收窄误拒合法客户端**：白名单化可能比裸字典序更严。
  - **缓解**：白名单须含当前 `SupportedProtocolVersion`（及历史已支持版本），对既有握手客户端行为不变；测试覆盖「当前版本仍通过 + 未知版本被拒」。
- **R4（低）C2/C3/C4 误被读为本 task 交付**：三项为诚实延后重申非交付。
  - **缓解**：§3 范围外各带 `[SPEC-DEFER]` tag，非 code AC；ADR-036 §D4 + ADR-033 add-only Amendment（task-31.4）如实记录，不伪造完成（ADR-013）。

## 9. Verification Plan

```bash
# 1. AC1 — eval case-results 子表（add-only migration 0018 + 双写 + per-case SQL 查）
#    真实子表查询结果待实测回填（ADR-013，不预填）
cargo test -p contextforge-core eval::store

# 2. AC2 — exporter 经新 ListAllChunks RPC 取真实全文（content 非空 + ContentHash 匹配真实正文）
#    buf generate 重生桩后 → daemon handler → exporter 导出；真实 content/hash 待实测回填
buf generate   # 重生 Rust/Go 桩（新增 add-only ListAllChunks RPC）
go test ./internal/exporter/...

# 3. AC3 — 3 MCP nits（protocolVersion 白名单 / audit.Write err 不吞 / allowlist mode warn）
go test ./internal/mcpadapter/...

# 4. AC1+AC2+AC3 — 既有契约不退化（add-only proto / add-only migration / nits 不破协议）
cargo test --workspace
go test ./...

# 5. AC4 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **0 outward-facing**：本 task 全 code-local（eval 子表 + add-only proto RPC + MCP nits）；无 tag / release / GHCR 推送。C2/C3/C4 为 documented 延后重申（§3 范围外，各带 `[SPEC-DEFER]` tag），非交付。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft（待实施）

**计划改动文件**：
- `migrations/0018_eval_case_results.sql`——新建（add-only）子表 `eval_case_results`（FK `eval_run_id` → `eval_runs` + per-case 列）。
- `core/src/eval/store.rs`——`update_case_results` 双写子表 + 保留 `case_results_json`；新增按 case 维度读方法；既有 `row_to_run`（`:285`）整 run 读不动。
- `proto/contextforge/v1/search.proto`——新增 `ListAllChunks(collection_id)`（或 `GetSourceChunk`）RPC（add-only，不改 `Search` / `SearchResponse` 既有字段）；buf generate 重生 Rust / Go 桩；daemon 侧实现 server handler。
- `internal/exporter/source.go`——`loadRecords` 改用 `ListAllChunks` RPC 填真实 `content`（取代 `:85` `content = ""`）+ 真实 `ContentHash`（`:96`）。
- `internal/exporter/fidelity.go`——`CalcFidelity` 基于真实正文。
- `internal/mcpadapter/server.go`——`:187` `protocolVersion` 解析 / 白名单（非字典序）；`:270` `writeAudit` 的 `audit.Write` err stderr warn / 上抛（不吞）。
- `internal/mcpadapter/allowlist.go`——`LoadAllowlist`（`:19-40`）加 `os.Stat` + 过松 mode warn（或拒绝）。

**§9 Verification 计划** (will record real evidence at impl)：
- AC1：`cargo test -p contextforge-core eval::store`——子表 per-case SQL 查询命中 + 既有 `eval_runs` 读不受影响（真实子表查询结果待实测回填，ADR-013，不预填）。
- AC2：`buf generate` 重生桩 + `go test ./internal/exporter/...`——导出 record.content 非空 + ContentHash 匹配真实正文（真实 content / hash 待实测回填）。
- AC3：`go test ./internal/mcpadapter/...`——`protocolVersion` 当前版本通过 + 未知版本拒；`audit.Write` err 可见；allowlist 过松 mode warn。
- 既有不退化：`cargo test --workspace` + `go test ./...` 全绿（add-only proto / migration / nits 不破既有契约）。
- AC4：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- C2/C3/C4：documented 延后重申（§3 范围外 + ADR-036 §D4 + ADR-033 add-only Amendment @ task-31.4），不伪造完成。
