# Task `35.3`: `closeout-v0.28.0 — observability-hardening grounding 校正（诚实 7→3-4 缩减：search.rs:109 already-surfaced / server.go:298 already-done task-31.3 / allowlist.go:31 intentional POSIX-only / eb.send:193 intentional no-subscribers，DROP 不伪造）+ v0.28.0 closeout（smoke v25 step [44/44] + TestTask353 + release docs + ADR-040 据 D1-D4 ratify + ADR-031 add-only Phase 35 Amendment + roadmap §3.17+§4 + adapter）`

**Status**: Draft

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 35 (observability-hardening)
**Dependencies**: task-35.1（rust-silent-failure-surfacing — `core/src/jobs/index_session_backend.rs:201` `store.append(...)` indexing-event persist 失败由 `let _ =` 静默吞 → 改 `if let Err(e) = ... { eprintln!("WARN indexing-event persist failed ...: {e}") }`，best-effort 不阻断 indexing；`core/src/retriever/mod.rs:415` Tantivy/SQLite 失同步时 `Err(_) => continue` 静默跳过命中 → 改 `Err(e) => { eprintln!("WARN retriever: chunk ... desync, skipping: {e}"); continue }`，skip 行为不变；均仿 `core/src/data_plane/search.rs:108-113` eprintln! WARN 惯例；`index_session_backend.rs:193` `let _ = eb.send(...)` LEAVE AS-IS（no-subscribers intentional））/ task-35.2（go-silent-failure-surfacing — `cmd/contextforge/main.go:297` `setVectorEnv` 内 `config.Load(dataDir)` err 静默吞 → 仿 `internal/daemon/rest.go:110` `fmt.Fprintf(os.Stderr, "contextforge: vector config load failed (%s): %v\n", dataDir, err)` 显式化（仍 best-effort，失败时 env-only 路径不变）+ `main.go:308` `os.Setenv` 失败可选 WARN；`internal/consoleapi/memstore.go:579` `emitMemoryEvent` nil-sink no-op 🟡 实施时 grounding：若 MemMemoryStore 接在期望 sink 的生产路径 → 加一次性（`sync.Once`）degraded-observability WARN，否则 fallback/test-double by-design → DROP 记 honest non-issue）全 Done / ADR-040（observability-hardening，本 task ratify）/ ADR-031（observability-hardening，v0.19.0 母 ADR——本 task add-only Phase 35 Amendment：承其 stderr/best-effort surfacing 方向，把热路径中被静默吞掉的真实错误显式化，不溯改正文 ADR-014 D5）/ ADR-036（governance-debt-cleanup，task-31.3 确立的 Go stderr audit-surfacing pattern——本 phase 镜像之）/ ADR-004（默认行为 / 既有契约不变——surfacing observability-only，best-effort 仍 best-effort 不转 fail-fast）/ ADR-008（dep add-only，Phase 35 = 0 new dep）/ ADR-012（tag/release outward-facing 须用户显式授权；本轮经 AskUserQuestion 2026-06-04 已授权 v0.28.0）/ ADR-013（禁伪造红线——honest 7→3-4 缩减如实记录，Rust eprintln! 不伪造 stderr-assert 已验）/ ADR-014 D1-D4（第二十六次激活）

## 1. Background

Phase 35（observability-hardening）是一个刻意**小**的版本——承 Phase 31（governance-debt-cleanup，v0.24.0）/ Phase 33（governance-debt-cleanup-2，v0.26.0）的治理债血脉，绿色 backlog 已偏薄，这是第三轮债清理性质、边际递减（diminishing returns）；故据 ADR-013「诚实优于充数」据实陈述，并经 AskUserQuestion（2026-06-04）用户选「A 可观测性硬化（纯绿区）」+「规划+实现+发版（无人值守）」，即对 v0.28.0 tag/release 的显式授权（ADR-012）。主题一行：把热路径中被静默吞掉的真实错误显式化（surface genuinely-swallowed errors），镜像仓库既有 stderr 惯例（Rust `eprintln!` / Go `fmt.Fprintf(os.Stderr, ...)`）；0 new dep、0 network、默认行为 + 既有 best-effort 契约不变（observability-only，不阻断热路径/RPC happy-path）。

两个实现 task 全 Draft（实施授权另行）：35.1（rust-silent-failure-surfacing——Rust core 仅用 `eprintln!` 到 stderr，`core/Cargo.toml` 无 log/tracing/metrics facility，atomics 仅在 test code，故仿 `core/src/data_plane/search.rs:108-113`（`if let Err(e) = ... { eprintln!("WARN ...: {e}") }` best-effort 不 abort caller）+ `core/src/server.rs:669`（`eprintln!("INFO ...")`）：(a) `index_session_backend.rs:201` `store.append(...)` 吞 indexing_event_store SQLite append 真实错误（磁盘满 / 锁）→ 加 `if let Err(e)` WARN 分支，仍 best-effort 不阻断 indexing；(b) `retriever/mod.rs:415` `Err(_) => continue` 在 Tantivy 与 SQLite 失同步时静默跳过 search hit → 改 `Err(e) => { eprintln!("WARN retriever: chunk ... desync, skipping: {e}"); continue }`，skip 行为保留；`index_session_backend.rs:193` `let _ = eb.send(...)` LEAVE AS-IS（既有注释言明 no-subscribers SendError 可接受，broadcast 无订阅者返 Err 是正常态非失败，加 WARN = 噪声））/ 35.2（go-silent-failure-surfacing——Go 用 `fmt.Fprintf(os.Stderr, ...)` 做 best-effort 失败显式化，仿 `internal/daemon/rest.go:101-122`（audit.Write best-effort 非阻断、stderr 前缀 `"contextforge audit: %v"`）：(a) `cmd/contextforge/main.go:297` `setVectorEnv` 内 `config.Load(dataDir)` 静默吞 malformed/unreadable config.toml 错误 → 在 return restore 前 `fmt.Fprintf(os.Stderr, "contextforge: vector config load failed (%s): %v\n", dataDir, err)` 显式化 + `main.go:308` `os.Setenv` 失败可选 WARN，仍 best-effort（失败时 env-only 路径不变）；(b) `internal/consoleapi/memstore.go:579` `emitMemoryEvent` 在 `s.emit == nil` 时 no-op 🟡 BORDERLINE，实施时 grounding：若 MemMemoryStore 接在期望 sink 的生产路径 → 加一次性 `sync.Once` degraded-observability WARN（0-dep 非噪声），若 fallback/test-double by-design → DROP 记 honest non-issue））。

本 task 兼两职：(A) 一组经 grounding 校正为 **诚实 7→3-4 缩减 / DROP 不伪造** 的项——4 处 survey 候选经核为 already-surfaced 或 intentional-by-design，记为 grounding 校正（这是本 phase 的 ADR-013 核心价值）；(B) 收口 v0.28.0：smoke v25 + release docs + ADR-040 据真实结果 ratify + ADR-031 add-only Phase 35 Amendment + roadmap §3.17 推进记录 + §4 add-only backlog + phase §6 闭合 + adapter + feature。

**(A) 诚实 7→3-4 缩减 = grounding 校正（DROP 不写新代码，ADR-013 核心价值）**：survey 把 7 处候选表述为待硬化点，经核其中 4 处为 already-surfaced 或 intentional-by-design，DROP 不实现新代码——
- `core/src/data_plane/search.rs:109` 已经 `eprintln!("WARN search_persist.put failed (key={key}); hot cache still updated: {e}")` 显式化；唯一「缺口」是结构化计数器，但 core 无 metrics facility（仅 test atomics），加之 = over-engineering。DROP（already-surfaced）。
- `internal/mcpadapter/server.go:298`（`writeAudit`）自 task-31.3（ADR-036 D3 nit 2）起即经 `fmt.Fprintf(os.Stderr, "mcp: audit write failed ...")` 显式化。DROP（already-done）。
- `internal/mcpadapter/allowlist.go:31` warning 已经 `fmt.Fprintf(os.Stderr, "mcp: warning: allowlist file ... overly permissive ...")` 触发；其「POSIX-only」是 INTENTIONAL 文档化平台事实（Windows 上 perm bits 无意义，让 Windows ACL 有意义需 `golang.org/x/sys/windows` = new dep 破 0-dep）。DROP（intentional caveat，非债）。
- `core/src/jobs/index_session_backend.rs:193`（`eb.send`）`let _ = eb.send(...)` 为 INTENTIONAL：既有注释言明无订阅者时 SendError 可接受；broadcast send 在无订阅者时返 Err 是正常态非失败，此处加 WARN = 噪声。LEAVE AS-IS（不改）。

**(B) v0.28.0 closeout**：smoke v24 step `[43/43]`（Phase 34 live）顺接 v25 step `[44/44]`（banner v24→v25，staging `cf-v27-cfg`，offset +2）+ `TestTask353`（mirror `TestTask343`，无回归 `[37/37]`..`[43/43]`）+ `docs/releases/v0.28.0-{evidence,artifacts}.md`（`<backfill>` 待回填）+ README v0.28 段 + RELEASE_NOTES v0.28.0 段 + ADR-040 Proposed→Accepted（per-D ratify）+ ADR-031 add-only Phase 35 Amendment + roadmap §3.17 + §4 + phase-35 §6 闭合 + adapter + feature。

## 2. Goal

(A) 把 observability-hardening 的 survey 候选据实 **grounding 校正为诚实 7→3-4 缩减**（4 处 DROP / LEAVE AS-IS）：`search.rs:109` already-surfaced（结构化计数器 = over-engineering，core 无 metrics facility）/ `server.go:298` already-done（task-31.3 ADR-036 D3）/ `allowlist.go:31` intentional POSIX-only 平台 caveat（Windows ACL 需 new dep 破 0-dep）/ `index_session_backend.rs:193` `eb.send` intentional no-subscribers（broadcast 无订阅者返 Err 是正常态）。**不写新代码**——4 处校正即本 task 的 ADR-013 价值（dropped 不 faked），须在 phase spec §2、ADR-040 Context + D3、本 task §范围外如实记录。0 新 dep。

(B) 据 35.1/35.2 **真实 CI / 实测产物**收口 v0.28.0：ADR-040 `Proposed → Accepted`（逐 D 如实——D1 rust-silent-failure-surfacing（`index_session_backend.rs:201` append WARN + `retriever/mod.rs:415` desync-skip WARN，仿 `search.rs:109`，best-effort 保留，guard tests；`eb.send:193` 留置 intentional no-subscribers）、D2 go-silent-failure-surfacing（`setVectorEnv` `config.Load`/`Setenv` 仿 `daemon/rest.go:110` stderr 显式化，stderr-capture RED→GREEN test；memstore nil-sink 🟡 impl-grounding：production-wired → one-time warn，否则 honest non-issue）、D3 grounding 校正诚实 7→3-4 缩减（4 处 DROP/LEAVE，无新 metrics facility）、D4 默认行为 + 0-dep + 0-network + 既有契约不变（ADR-004/008，best-effort 不转 fail-fast））；ADR-031 add-only Phase 35 Amendment（承其 stderr/best-effort surfacing 方向，把静默吞掉的真实错误显式化，不溯改正文 ADR-014 D5）；roadmap §3.17（Phase 35 推进记录）+ §4 add-only（新 backlog）；phase-35 §6 AC 置 `[x]` + Status Done；smoke v25 step `[44/44]`；release docs（tag/run/digest 用 `<backfill>`）；adapter（Phase 35 Done + Tasks 3 + ADR-040 Accepted + feature 行）。**真实 v0.28.0 tag/release 须用户显式授权**（本轮经 AskUserQuestion 2026-06-04 已授权 v0.28.0；不自行越界 tag，ADR-012）。

pass bar：(A) 4 处 grounding 校正如实记录于 §范围外 + ADR-040 D3（DROP/LEAVE 不写新代码）；(B) smoke `bash -n` 过 + `go test -run TestTask353` 过 + 文档闭合人工核 + ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 4 处 observability survey 候选的 **grounding 校正记录**（DROP / LEAVE AS-IS，不写新生产代码）：(a) `search.rs:109` already-surfaced（结构化计数器 over-engineering，core 无 metrics facility）；(b) `server.go:298` already-done（task-31.3）；(c) `allowlist.go:31` intentional POSIX-only 平台 caveat；(d) `index_session_backend.rs:193` `eb.send` intentional no-subscribers。**记录于 §范围外 + ADR-040 Context + D3，不改任何生产代码**（survey overstatement 校正即 ADR-013 价值）。
- `scripts/console_smoke.sh`——banner v24→v25 + v25 changelog 块 + step `[44/44]`（doc/status 断言 observability-hardening baseline：rust-silent-failure-surfacing + go-silent-failure-surfacing + grounding 校正诚实 7→3-4；default build init baseline 不变 + denominator 不溯改 ADR-014 D5），staging `cf-v27-cfg`（offset +2）。当前 live 脚本 v24 `[43/43]`（Phase 34）；故 Phase 35 顺接 `[44/44]`。
- `internal/cli/smoke_syntax_test.go`——新增 `TestTask353_SmokeV25ObservabilityHardeningStep`（mirror `TestTask343`，断言 `v25 (task-35.3)` header + `[44/44]` + 标记（`observability-hardening` / `TEST-35.1.` / `TEST-35.2.` / `TEST-35.3.` / `eprintln` / `setVectorEnv`）+ 无回归既有 `[37/37]`..`[43/43]`，denominator 不溯改 ADR-014 D5 + `bash -n` 语法）。
- 新增 `docs/releases/v0.28.0-{evidence,artifacts}.md`（tag SHA / run id / digest 用 `<backfill>` 待回填）+ `README.md` v0.28 段 + `RELEASE_NOTES.md` v0.28.0 段。
- `docs/decisions/adr-040-observability-hardening.md`——Status Proposed→Accepted（per-D 限定）+ `## Ratification（v0.28.0 / task-35.3）` 节（逐 D 真实依据；D1 `index_session_backend.rs:201`/`retriever/mod.rs:415` eprintln! WARN best-effort 保留 + guard tests + `eb.send:193` intentional 留置、D2 `setVectorEnv` stderr-capture RED→GREEN + memstore nil-sink 🟡 impl-grounding、D3 grounding 校正诚实 7→3-4 缩减（4 处 DROP/LEAVE，无新 metrics facility）、D4 默认行为 + 0-dep + 0-network 不变）。
- add-only Amendment（不溯改正文，ADR-014 D5）：`docs/decisions/adr-031-observability-hardening.md`——`## Amendment (Phase 35 / v0.28.0)`（承其 stderr/best-effort surfacing 方向——把热路径中被静默吞掉的真实错误（`index_session_backend.rs:201` indexing-event persist / `retriever/mod.rs:415` Tantivy-SQLite desync / `setVectorEnv` config.Load）经 `eprintln!` / `fmt.Fprintf(os.Stderr)` 显式化，observability-only best-effort 不转 fail-fast；不溯改 ADR-031 D1-Dn 正文 + 既有 Amendment 正文）。
- `docs/roadmap.md`——§3 新增 §3.17 Phase 35 推进记录 + §4 add-only（新 backlog 条目：observability-metrics-facility（结构化计数器，core 现无）/ memstore-degraded-observability-warn（若 grounding 显 sink optional-by-design）；add-only 不删旧条目正文）。
- `docs/specs/phases/phase-35-observability-hardening.md`——Status Draft→Done + §6 AC `[x]`（honest per-item：35.2 setVectorEnv stderr-capture 强测 🟢 / 35.1 Rust eprintln! guard+inspection 🟢 / memstore nil-sink 🟡 如实标注）。
- `docs/s2v-adapter.md`——§Phase 35 In Progress→Done + Tasks 2→3；§Task +35.3；§ADR 040 Proposed→Accepted；§BDD +phase-35 行。
- `test/features/phase-35-observability-hardening.feature`（已创建）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER] / DROPPED honest record）

以下经 grounding 校正为 **DROPPED / LEAVE AS-IS / honest-defer，不实现新代码**（survey overstatement 校正即本 task 的 ADR-013 价值，须在 spec 与 ADR-040 D3 如实记录）：

- **`search.rs:109` = ALREADY-SURFACED（不写新代码）**：自既有实现起即 `eprintln!("WARN search_persist.put failed (key={key}); hot cache still updated: {e}")` 显式化；唯一「缺口」是结构化计数器，但 core 无 metrics facility（仅 test atomics）→ 加之 = over-engineering。DROP，记 ADR-040 D3。
- **`internal/mcpadapter/server.go:298`（writeAudit）= ALREADY-DONE（task-31.3，不写新代码）**：自 task-31.3（ADR-036 D3 nit 2）即 `fmt.Fprintf(os.Stderr, "mcp: audit write failed ...")` 显式化。DROP，记 ADR-040 D3。
- **`internal/mcpadapter/allowlist.go:31` = INTENTIONAL POSIX-only 平台 caveat（不写新代码）**：warning 已 `fmt.Fprintf(os.Stderr, "mcp: warning: allowlist file ... overly permissive ...")` 触发；POSIX-only 是文档化平台事实（Windows perm bits 无意义）。DROP（非债，intentional），记 ADR-040 D3。
- **`core/src/jobs/index_session_backend.rs:193`（eb.send）= INTENTIONAL no-subscribers（LEAVE AS-IS）**：`let _ = eb.send(...)` 既有注释言明无订阅者 SendError 可接受；broadcast 无订阅者返 Err 是正常态非失败，加 WARN = 噪声。LEAVE AS-IS，记 ADR-040 D3。

其余范围外：
- 真实 v0.28.0 tag push + release run（cosign 真签 + GHCR 推送）[SPEC-OWNER:user-authorized-release]——outward-facing 不可逆已经 AskUserQuestion（2026-06-04）获本轮用户授权（ADR-012）；post-tag-push backfill 填实 tag SHA / run id / digest，本 task body 不预填真实凭据。
- 结构化 metrics / counter facility for observability（core 现无；stderr surfacing 是忠实 scope）[SPEC-DEFER:phase-future.observability-metrics-facility]——加 metrics facility 破 simplicity-first（ADR-004）+ 需 exposure path（proto/health）= scope creep；stderr 显式化是 make-silent-failures-explicit 的忠实诠释，honest-defer。
- memstore nil-sink 一次性 degraded warn IF grounding 显 MemMemoryStore sink optional-by-design [SPEC-DEFER:phase-future.memstore-degraded-observability-warn]——task-35.2 实施时 grounding；若 MemMemoryStore 接在期望 sink 的生产路径 → 加 `sync.Once` WARN（落 35.2），若 fallback/test-double by-design → DROP 记 honest non-issue，不预指派独立通过 TEST-ID。

## 4. Actors

- 主 agent（ADR-012 自治；真实 release 本轮经 AskUserQuestion 2026-06-04 已获用户授权）
- `index_session_backend` `store.append` / `eb.send`（`core/src/jobs/index_session_backend.rs:201`/`:193`，task-35.1 落地——`:201` 加 eprintln! WARN best-effort，`:193` LEAVE AS-IS intentional no-subscribers，本 closeout 经 ADR-040 D1 ratify）
- `retriever` Tantivy-SQLite desync-skip（`core/src/retriever/mod.rs:415`，task-35.1 落地——`Err(e)` 加 eprintln! WARN，skip 行为保留，本 closeout 经 ADR-040 D1 ratify）
- `setVectorEnv` config.Load / Setenv（`cmd/contextforge/main.go:297`/`:308`，task-35.2 落地——`config.Load` 失败 `fmt.Fprintf(os.Stderr)` 显式化，本 closeout 经 ADR-040 D2 ratify）
- `emitMemoryEvent` nil-sink（`internal/consoleapi/memstore.go:579`，task-35.2 🟡 impl-grounding，本 closeout 据 grounding 结果如实记 ADR-040 D2）
- 4 处 grounding 校正候选（`search.rs:109` / `server.go:298` / `allowlist.go:31` / `index_session_backend.rs:193`，本 closeout 经 ADR-040 D3 DROP/LEAVE 如实记录）
- closeout 文档集（smoke / release docs / ADR-040 ratify / ADR-031 add-only Phase 35 Amendment / roadmap §3.17+§4 / phase spec / adapter / feature）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/data_plane/search.rs:108-113`（`if let Err(e) = ... { eprintln!("WARN ...: {e}") }` best-effort 不 abort caller——task-35.1 镜像源 + grounding 校正锚点 `:109` already-surfaced）+ `core/src/server.rs:669`（`eprintln!("INFO ...")`——severity 是 message 字符串前缀 WARN/INFO，无 severity framework）
- `core/src/jobs/index_session_backend.rs:201`（`let _ = store.append(&job_id_context, "indexing", evt.processed_files, evt.total_files, "")` 静默吞 indexing_event_store SQLite append 真实错误锚点——task-35.1 改 `if let Err(e)` WARN 分支 best-effort 保留）+ `:193`（`let _ = eb.send(...)` intentional no-subscribers——LEAVE AS-IS 锚点，既有注释言明可接受）
- `core/src/retriever/mod.rs:415`（`let (file_path, content, indexed_at) = match row { Ok(t) => t, Err(_) => continue }` Tantivy/SQLite 失同步静默跳过命中锚点——task-35.1 改 `Err(e) => { eprintln!("WARN retriever: chunk ... desync, skipping: {e}"); continue }`，skip 行为保留，仿 retriever 既有 eprintln! 惯例（filter no-op 路径附近））
- `core/Cargo.toml`（无 log/tracing/metrics crate——atomics 仅 test code；surfacing 仅 eprintln! 是忠实 scope，无新 facility，ADR-004/008）
- `internal/daemon/rest.go:101-122`（`audit.Write` best-effort 非阻断、stderr 前缀 `"contextforge audit: %v"`——task-35.2 `setVectorEnv` 镜像源）+ `internal/importer/importer.go:59`（`log.Printf` `[warning]` 前缀——Go warning 惯例参照）
- `cmd/contextforge/main.go:297`（`setVectorEnv` 内 `cfg, err := config.Load(dataDir); if err != nil { return restore }` 静默吞 malformed config.toml 锚点——task-35.2 加 `fmt.Fprintf(os.Stderr, "contextforge: vector config load failed (%s): %v\n", dataDir, err)`）+ `:308`（`os.Setenv` 失败仅 success 时记 restore——可选 WARN）
- `internal/consoleapi/memstore.go:579`（`emitMemoryEvent` `s.emit == nil` no-op 🟡 BORDERLINE——task-35.2 实施时 grounding MemMemoryStore 是否接生产 sink 路径）
- `internal/mcpadapter/server.go:298`（`writeAudit` 已 `fmt.Fprintf(os.Stderr, "mcp: audit write failed ...")` since task-31.3——grounding 校正 already-done 锚点）+ `internal/mcpadapter/allowlist.go:31`（warning 已 `fmt.Fprintf(os.Stderr, "mcp: warning: allowlist file ...")`，POSIX-only intentional——grounding 校正锚点）
- `docs/specs/tasks/task-35.1-rust-silent-failure-surfacing.md §10` + `task-35.2-go-silent-failure-surfacing.md §10`（真实测试结果 + 结论——ADR-040 ratify 依据）
- `docs/decisions/adr-040-observability-hardening.md`（§D1-D4 + Consequences Ratification 条款）
- `docs/decisions/adr-031-observability-hardening.md`（§Decision——本 task add-only Phase 35 Amendment 落点：承其 stderr/best-effort surfacing 方向）+ `docs/decisions/adr-036-governance-debt-cleanup.md §D3`（task-31.3 确立的 Go stderr audit-surfacing pattern——本 phase 镜像之）
- `internal/cli/smoke_syntax_test.go`（`TestTask343_SmokeV24VectorConfigCompletenessStep`——本 task `TestTask353` mirror 源）+ `scripts/console_smoke.sh`（v24 `[43/43]` 块 + banner，cf-v26-cfg → 本 task cf-v27-cfg offset +2）
- `docs/releases/v0.27.0-{evidence,artifacts}.md`（release docs 模板）

### 5.2 关键设计 — grounding 校正诚实 7→3-4 + 诚实 per-D ratify + backfill 待回填

- **grounding 校正诚实 7→3-4 缩减（不写新代码，ADR-013 核心价值）**：survey 列 7 处 observability 候选，经核 4 处为 already-surfaced 或 intentional-by-design——`search.rs:109` already-surfaced（结构化计数器 over-engineering，core 无 metrics facility）/ `server.go:298` already-done（task-31.3 ADR-036 D3）/ `allowlist.go:31` intentional POSIX-only 平台 caveat（Windows ACL 需 new dep 破 0-dep）/ `index_session_backend.rs:193` `eb.send` intentional no-subscribers（broadcast 无订阅者返 Err 是正常态）。**4 处 DROP/LEAVE 不写生产代码**（symmetry honestly），grounding 校正记 §范围外 + ADR-040 Context + D3。剩 3-4 处 KEPT（35.1 两处 + 35.2 一处确 + 一处 🟡 impl-grounding）genuinely silent worth surfacing。pass bar 4 处校正如实记录、0 生产代码改动。0 新 dep。
- ADR-040 ratify **逐 D 项据真实结果**：D1（rust-silent-failure-surfacing——`index_session_backend.rs:201` `store.append` eprintln! WARN best-effort 保留（不阻断 indexing）+ `retriever/mod.rs:415` desync-skip eprintln! WARN（skip 行为保留），仿 `search.rs:109`，guard tests 验 best-effort/skip 行为保留 🟢；`eb.send:193` LEAVE AS-IS intentional no-subscribers；**Rust eprintln! stderr 输出在 std 单测中难断言、仓库既有 eprintln! 站点也不断言输出，故据实用 guard/behavior-preservation 测试 + inspection，不伪造 stderr-assert 已验 ADR-013**）/ D2（go-silent-failure-surfacing——`setVectorEnv` `config.Load`/`Setenv` 仿 `daemon/rest.go:110` `fmt.Fprintf(os.Stderr)` 显式化达成 🟢；**Go 测试可经 `os.Pipe` 重定向 `os.Stderr` 捕获 → 真实 RED→GREEN（本 phase 最强测试）**：malformed config.toml → 断言 WARN 行存在，valid/missing → 断言无 WARN，env-wins + restore 行为保留（扩展 task-34.2 既有 `TestSetVectorEnv`）；memstore nil-sink 🟡 `[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]` impl-grounding）/ D3（grounding 校正诚实 7→3-4 缩减——4 处 DROP/LEAVE 如实，无新 metrics facility（core eprintln! / Go Fprintf only））/ D4（默认行为 + 0-dep + 0-network + 既有契约不变 ADR-004/008——surfacing observability-only，best-effort 仍 best-effort 不转 fail-fast，0 新 dep，0 proto，0 migration）。各 D 真实测试 / 实测结果待 35.1-35.2 实施后跑出再回填，不为「全 Accepted」伪造 Rust stderr-assert 已验（ADR-013）。
- ADR-031 add-only Phase 35 Amendment 为 **add-only 注记**（不删/不改 ADR-031 正文 + 既有 Amendment 正文）：承其 stderr/best-effort surfacing 方向——把热路径中被静默吞掉的真实错误（`index_session_backend.rs:201` indexing-event persist / `retriever/mod.rs:415` Tantivy-SQLite desync / `setVectorEnv` config.Load）经 `eprintln!` / `fmt.Fprintf(os.Stderr)` 显式化，observability-only best-effort 不转 fail-fast。ADR-031（v0.19.0 母 ADR）确立的 stderr/best-effort surfacing 经 Phase 35 在更多热路径站点兑现。
- tag SHA / release run id / 镜像 digest 在 release docs 用 `<backfill: ...>` 待回填——真实 v0.28.0 tag/release 是 closeout 合入后的**用户授权步**（本轮经 AskUserQuestion 2026-06-04 已授权），post-tag-push backfill PR 填实（承 v0.8–v0.27 pattern）。
- smoke step `[44/44]` 为文档/状态步：验 default build init baseline 不变（ADR-004）+ 文档化三 task 状态（rust-silent-failure-surfacing + go-silent-failure-surfacing + grounding 校正诚实 7→3-4），staging `cf-v27-cfg`（offset +2）。

### 5.3 不变量

- 默认行为不变（ADR-004）：surfacing observability-only——`index_session_backend.rs:201` append 失败仍不阻断 indexing；`retriever/mod.rs:415` desync 仍 skip 该 hit、query 返回其余有效命中无错误；`setVectorEnv` `config.Load` 失败仍走 env-only 路径返 restore；best-effort 仍 best-effort，不转 fail-fast；4 处 grounding 校正站点 0 代码改动（行为不变）。
- closeout 0 行为变更 / 0 新依赖（Phase 35 = 0 new dep，ADR-008——35.1 复用 Rust 既有 `eprintln!`（无 log/tracing/metrics crate）；35.2 复用 Go 既有 `fmt.Fprintf(os.Stderr)` + 标准 `os.Pipe`；0 proto / 0 migration；smoke 既有 step + denominator 不溯改 ADR-014 D5）。
- best-effort 守恒（ADR-004）：surfacing MUST NOT turn best-effort into fail-fast——indexing 不阻断、query 续行、env-only 路径不变；severity 仅是 message 字符串前缀（WARN/INFO），无 severity framework，无新 metrics facility。
- ADR-014 D5：历史 Phase 1-34 spec 不溯改；ADR-031 add-only Phase 35 Amendment 不改正文 + 既有 Amendment 正文；roadmap §4 新 backlog 为 add-only 条目不删旧条目正文。
- add-only 显式化（`index_session_backend.rs:201`/`retriever/mod.rs:415` 加 eprintln! WARN 分支不改行为 + `setVectorEnv` 加 `fmt.Fprintf(os.Stderr)` 不改返回路径）不破既有契约（ADR-004/008）；Rust 无 log/tracing/metrics crate 保持、Go 无 metrics facility 保持。
- honest 守线（ADR-013）：4 处 grounding 校正（`search.rs:109` already-surfaced / `server.go:298` already-done / `allowlist.go:31` intentional POSIX-only / `eb.send:193` intentional no-subscribers）如实记录于 §范围外 + ADR-040 D3，**DROP/LEAVE 不写新代码**；Rust eprintln! stderr-assert 难在 std 单测断言 → 据实用 guard/inspection，不伪造已验；memstore nil-sink 🟡 impl-grounding 不预断已交付。
- 真实 tag/release 经用户授权后执行（本轮经 AskUserQuestion 2026-06-04 已授权，ADR-012）；release docs tag/run/digest backfill 待回填，不预填伪造凭据。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [ ] **AC1**（grounding 校正诚实 7→3-4 缩减如实记录 🟢）: 4 处 survey 候选经核 DROP/LEAVE 不写新代码——`search.rs:109` already-surfaced（结构化计数器 over-engineering，core 无 metrics facility）/ `server.go:298` already-done（task-31.3）/ `allowlist.go:31` intentional POSIX-only 平台 caveat / `index_session_backend.rs:193` `eb.send` intentional no-subscribers；如实记录于 §范围外 + ADR-040 Context + D3（dropped 不 faked，ADR-013）；**0 生产代码改动**；0 新 dep — verified by **TEST-35.3.1**。
- [ ] **AC2**（v0.28.0 closeout 🟢🟡）: smoke banner v24→v25 + step `[44/44]`（observability-hardening baseline + default build baseline intact，staging `cf-v27-cfg` offset +2）+ `TestTask353_SmokeV25ObservabilityHardeningStep`（含无回归既有 `[37/37]`..`[43/43]`，denominator 不溯改）；v0.28.0 release docs（`v0.28.0-{evidence,artifacts}.md` `<backfill>` + README v0.28 段 + RELEASE_NOTES v0.28.0 段）+ ADR-040 per-D ratify `Proposed→Accepted`（D1 `index_session_backend.rs:201`/`retriever/mod.rs:415` eprintln! WARN best-effort 保留 + guard tests + `eb.send:193` intentional 留置 + Rust 不伪造 stderr-assert；D2 `setVectorEnv` stderr-capture RED→GREEN + memstore nil-sink 🟡 impl-grounding；D3 grounding 校正诚实 7→3-4 + 无新 metrics facility；D4 默认行为 + 0-dep + 0-network）+ ADR-031 add-only Phase 35 Amendment（承 stderr/best-effort surfacing）+ roadmap §3.17 推进记录 + §4 add-only 新 backlog + phase-35 §6 AC `[x]` + Status Done + adapter（Phase 35 Done/Tasks 3/ADR-040 Accepted）+ feature — verified by **TEST-35.3.2**。
- [ ] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中（CI spec-lint 权威）— verified by **TEST-35.3.2**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-35.3.1 | grounding 校正诚实 7→3-4 缩减：4 处 survey 候选 DROP/LEAVE 不写新代码——`search.rs:109` already-surfaced（counter over-engineering，core 无 metrics facility）/ `server.go:298` already-done（task-31.3）/ `allowlist.go:31` intentional POSIX-only 平台 caveat / `index_session_backend.rs:193` `eb.send` intentional no-subscribers；如实记录 §范围外 + ADR-040 Context+D3（dropped 不 faked，ADR-013）；0 生产代码改动；0 新 dep | `docs/specs/tasks/task-35.3-closeout-v0.28.0.md`（§范围外）+ `docs/decisions/adr-040-observability-hardening.md`（Context + D3） | Draft |
| TEST-35.3.2 | smoke v25 step `[44/44]`（observability-hardening baseline + rust-silent-failure-surfacing/go-silent-failure-surfacing/grounding-校正 标记 `observability-hardening`/`TEST-35.1.`/`TEST-35.2.`/`TEST-35.3.`/`eprintln`/`setVectorEnv` + 无回归既有 denominator，staging `cf-v27-cfg`）+ `bash -n` 过 + `go test -run TestTask353` 过 + v0.28.0 release docs + ADR-040 per-D ratify Accepted（D1 eprintln! WARN best-effort + Rust 不伪造 stderr-assert / D3 grounding 校正诚实 7→3-4 如实）+ ADR-031 add-only Phase 35 Amendment + roadmap §3.17+§4 + phase-35 §6 闭合 + adapter + feature + D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/console_smoke.sh` + `internal/cli/smoke_syntax_test.go` + release/ADR-040/ADR-031/roadmap/phase/adapter/feature + `scripts/spec_drift_lint.sh` | Draft |

## 8. Risks

- **R1（低）grounding 校正误写新生产代码**：本 task 的 4 处校正是 DROP/LEAVE（already-surfaced / already-done / intentional），若误改 `search.rs:109`（加 counter）/ `server.go:298` / `allowlist.go:31`（Windows ACL）/ `index_session_backend.rs:193`（eb.send）生产逻辑则越界破 0-dep/simplicity。
  - **缓解**：4 处仅记录于 §范围外 + ADR-040 Context+D3，不触生产代码；结构化 metrics facility `[SPEC-DEFER:phase-future.observability-metrics-facility]`。stop-condition：4 处 grounding 校正如实记录且 0 生产代码改动则 AC1 标 `[x]`。
- **R2（低）closeout 误报 already-surfaced 为本 task 新修 / 误报 Rust stderr-assert 为已验 / 误报 memstore nil-sink 为定交付**：诚实风险。
  - **缓解**：§范围外 + ADR-040 D3 逐项如实——4 处 already-surfaced/intentional DROP/LEAVE；D1 Rust eprintln! stderr-assert 难在 std 单测断言 → 据实用 guard/inspection 不伪造已验（ADR-013）；memstore nil-sink 🟡 `[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]` impl-grounding，不预指派独立通过 TEST-ID。stop-condition：任何「本 task 新修 already-surfaced」/「Rust stderr-assert 已验」/「memstore 已交付」表述须有真实凭据，否则标受阻维度 / backfill。
- **R3（低）smoke denominator 误溯改 / staging offset 错位**：新 step 须 `[44/44]`、staging `cf-v27-cfg`（offset +2），既有 `[37/37]`..`[43/43]` 不动。
  - **缓解**：`TestTask353` 无回归断言守护（mirror `TestTask343`）；ADR-014 D5；staging dir `cf-v27-cfg` 顺接 v25→cf-v27（offset +2）。
- **R4（低）ADR-031 Amendment 误溯改正文 / 既有 Amendment 正文 / surfacing 误转 fail-fast**：须 add-only 追加 `## Amendment (Phase 35 / v0.28.0)` 不删既有正文（D5），且 surfacing 严守 observability-only。
  - **缓解**：仅追加 Phase 35 Amendment 段（承 stderr/best-effort surfacing：`index_session_backend.rs:201`/`retriever/mod.rs:415`/`setVectorEnv`），不改 ADR-031 正文 + 既有 Amendment 正文；surfacing best-effort 不转 fail-fast（indexing 不阻断 / query 续行 / env-only 路径不变，ADR-004），不伪造 Rust stderr-assert（ADR-013）。

## 9. Verification Plan

```bash
# AC1 — grounding 校正诚实 7→3-4 缩减如实记录（DROP/LEAVE 不写新代码，人工核 §范围外 + ADR-040 Context+D3）
# search.rs:109 already-surfaced / server.go:298 already-done / allowlist.go:31 intentional POSIX-only / eb.send:193 intentional no-subscribers
# 验 4 处生产代码 0 改动（git diff 仅 docs/smoke/test）

# AC2 — smoke 语法 + syntax test
bash -n scripts/console_smoke.sh
go test ./internal/cli/ -run TestTask353

# AC2 — 文档闭合人工核（ADR-040 Accepted + per-D / ADR-031 add-only Phase 35 Amendment /
#        roadmap §3.17 + §4 新 backlog / phase §6 [x] / adapter Done / feature 存在）
# AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master

# 既有不退化（closeout 文档+smoke 不影响热路径；35.1 eprintln! WARN 分支 best-effort 保留 / 35.2 Go setVectorEnv add-only）
cargo test --workspace && go test ./...
```

> **outward-facing 红线**：真实 v0.28.0 tag push + release run（cosign 真签 + GHCR 推送）是 closeout 合入后的**用户授权步**（本轮经 AskUserQuestion 2026-06-04 已授权，ADR-012）；本 task body 不预填真实凭据，release docs 的 tag/run/digest 用 `<backfill>` 待 post-tag-push backfill 填实 [SPEC-OWNER:user-authorized-release]。
>
> **honest-defer / grounding 校正边界**：本 closeout 交付范围限于 4 处 grounding 校正如实记录（🟢 already-surfaced/intentional DROP/LEAVE，不写新代码）+ v0.28.0 closeout 文档/smoke；§范围外 grounding 校正（4 处 DROP/LEAVE / observability-metrics-facility `[SPEC-DEFER:phase-future.observability-metrics-facility]` / memstore-degraded-observability-warn `[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]`）**不实现新代码**，据 ADR-013 如实记录于 §范围外 + ADR-040 D3。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft — 待实施回填（35.1/35.2 实现 + v0.28.0 真实 tag/run/digest 经用户授权步后据真实 CI 实证回填，不预填伪造凭据 ADR-013）。
