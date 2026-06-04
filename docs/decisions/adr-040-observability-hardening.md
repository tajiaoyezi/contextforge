# ADR `040`: `observability-hardening`

**Status**: Accepted（v0.28.0 / task-35.3 closeout 据真实 CI 逐 D ratify；D1 rust-silent-failure-surfacing 🟢 + D2 go-silent-failure-surfacing 🟢（memstore nil-sink honest non-issue grounding 校正）+ D3 grounding 校正诚实 7→3-4 收敛 🟢 + D4 默认 0-dep/0-network/既有契约不变 🟢；Rust eprintln! stderr 输出 guard/inspection 不伪造断言、memstore nil-sink by-design honest-defer——见 §Ratification）

**Category**: 可观测性硬化（surface genuinely-swallowed errors）/ 热路径静默错误显式化（stderr surfacing）/ 契约诚实化（honest 7→3-4 grounding-correction）
**Date**: 2026-06-04
**Decided By**: 主 agent（ADR-012 自治）；tajiaoyezi ratification at v0.28.0 closeout（AskUserQuestion 2026-06-04 选「A 可观测性硬化(纯绿区)」+「规划+实现+发版(无人值守)」= 对 v0.28.0 tag/release 的显式授权）
**Related**: ADR-031（observability-hardening — 本 ADR 为其 v0.19.0 母 ADR 方向的延续：承其 stderr / best-effort surfacing 方向，add-only，不溯改其正文）/ ADR-036（governance-debt-cleanup — task-31.3 已确立 Go `fmt.Fprintf(os.Stderr)` audit-surfacing 模式，本 phase 镜像该模式；其 D3 nit 2 已修的 server.go:298 即本 phase grounding 校正 DROP 的站点）/ ADR-038（governance-debt-cleanup-2 — 本 phase 承 Phase 33 治理债血脉、第三轮债清理性质，边际递减据实陈述）/ ADR-039（vector-config-completeness — `setVectorEnv` 来自其 task-34.2，本 phase D2 在其 config.Load / Setenv 静默路径上补 surfacing）/ ADR-004（local-first-privacy-baseline — 默认行为 / proto / 既有契约不变 + 0 网络 + best-effort 仍 best-effort 不转 fail-fast）/ ADR-008（dep add-only — Phase 35 = 0 新依赖；不引入任何 log / tracing / metrics facility，Rust eprintln! / Go Fprintf 既有惯例）/ ADR-013（禁伪造红线 — honest 7→3-4 reduction 据实记录、不为 Rust 伪造 stderr-assert 证据、不夸大缺口）/ ADR-012（main-agent-governance-autonomy — tag/release outward-facing 须用户显式授权，v0.28.0 本轮已授权）/ ADR-014（D1-D4，第二十六次激活）/ roadmap §3.17 + §4

## Context

ContextForge 截至 Phase 33（governance-debt-cleanup-2, Done / v0.26.0）已完成第二轮治理债清扫。本 Phase 35 是承 Phase 31（governance-debt-cleanup, v0.24.0）/ Phase 33（governance-debt-cleanup-2, v0.26.0）治理债血脉的**刻意精简**小版本——据实陈述：这是第三轮债清理性质、边际递减（third debt-cleanup, diminishing returns），已向用户言明、用户经 AskUserQuestion（2026-06-04）选「A 可观测性硬化(纯绿区)」。据 ADR-013 取诚实优先于凑量（honest over padding）。主题一行：把热路径中被静默吞掉的真实错误显式化（surface genuinely-swallowed errors），镜像仓库既有 stderr 惯例（Rust `eprintln!` / Go `fmt.Fprintf(os.Stderr)`）。

**facility grounding（不引入任何新 logging / metrics framework，simplicity-first + ADR-004/008）**：

- **Rust core 仅用 `eprintln!` 写 stderr**：`core/Cargo.toml` **无** log crate、**无** tracing crate、**无** metrics facility；原子量（`AtomicU64/Usize`）仅存在于测试代码。镜像范本：`core/src/data_plane/search.rs:108-113`（`if let Err(e) = ... { eprintln!("WARN ...: {e}") }` best-effort，不中断 caller）+ `core/src/server.rs:669`（`eprintln!("INFO ...")`）。severity 是 message string 的前缀（`WARN` / `INFO`），无 severity framework。
- **Go 用 `fmt.Fprintf(os.Stderr, ...)` 显式化 best-effort 失败**：镜像范本 `internal/daemon/rest.go:101-122`（`audit.Write` best-effort，非阻断，前缀 `"contextforge audit: %v"` 写 stderr）+ `log.Printf` `[warning]` 前缀（`internal/importer/importer.go:59`）。无 metrics facility。

**HONEST 7→3-4 REDUCTION（本 phase 的 ADR-013 核心价值——据实记录，不夸大缺口）**：survey 列出 7 处「静默吞错」候选，grounding 复核后 **4 处 DROP**（已显式化 / 设计有意为之，无代码改动）、**3-4 处 KEEP**（确为静默、值得显式化）：

DROPPED（already-done 或 intentional-by-design，无代码改动，记为 grounding 校正）：
- `core/src/data_plane/search.rs:109` — **已**经 `eprintln!("WARN search_persist.put failed (key={key}); hot cache still updated: {e}")` 显式化；唯一「缺口」是结构化计数器，但 core **无** metrics facility（仅测试原子量）→ 加一个 = over-engineering。DROP（already-surfaced）。
- `internal/mcpadapter/server.go:298`（`writeAudit`）— 自 task-31.3（ADR-036 D3 nit 2）起**已**经 `fmt.Fprintf(os.Stderr, "mcp: audit write failed ...")` 显式化。DROP（already-done）。
- `internal/mcpadapter/allowlist.go:31` — warning **已**经 `fmt.Fprintf(os.Stderr, "mcp: warning: allowlist file ... overly permissive ...")` 触发；其「POSIX-only」是**有意的、已记录的平台事实**（Windows 上 perm bits 无意义；令 Windows ACL 有意义须 `golang.org/x/sys/windows` = 新 dep、破 0-dep）。DROP（intentional caveat, not debt）。
- `core/src/jobs/index_session_backend.rs:193`（`eb.send`）— `let _ = eb.send(...)` 是**有意**的：既有注释说明 no subscribers 时 `SendError` 被吞是可接受的；broadcast send 在无订阅者时返 `Err` 是**正常状态**而非失败，此处加 `WARN` 会变噪声。LEAVE AS-IS（不改）。

KEPT（确为静默、值得显式化）：
- **task-35.1（Rust）**：(a) `core/src/jobs/index_session_backend.rs:201` 的 `let _ = store.append(...)` 吞掉真实持久化错误（`indexing_event_store` SQLite append 失败：disk full / lock）；(b) `core/src/retriever/mod.rs:415` 的 `Err(_) => continue` 在 Tantivy 与 SQLite 不同步时静默跳过一条命中、吞掉真实错误。
- **task-35.2（Go）**：(a) `cmd/contextforge/main.go:297`（`setVectorEnv`）的 `config.Load` 错误被静默吞掉（malformed/unreadable config.toml），及 :308 `os.Setenv` 失败被静默丢弃；(b) `internal/consoleapi/memstore.go:579`（`emitMemoryEvent`）在 `s.emit == nil` 时 no-op（degraded observability）——此项 **🟡 BORDERLINE**，须在实施时 grounding `MemMemoryStore` 是否在生产路径接线（详 D2）。

本 ADR 把上述「Rust / Go 静默错误显式化 + honest 7→3-4 grounding 校正 + 默认零依赖 / 既有契约不变守线」收敛为一个精简硬化 + 诚实化 Phase 的处理策略。surfacing 为 **observability-only**——best-effort 仍 best-effort，**不**把 best-effort 转为 fail-fast（MUST NOT），不阻断热路径 / RPC happy-path。0 new dep、0 network、0 proto、0 migration。全部改动遵守 ADR-004 默认行为 / proto / 既有契约不变 + 0 网络 + ADR-008 0 新依赖（不引 log/tracing/metrics facility）+ ADR-013 受阻 / 非问题项诚实分级不伪造（不为 Rust 伪造 stderr-assert）。

## Decision

可观测性硬化采用 **「Rust eprintln! 静默错误显式化 + Go Fprintf 静默错误显式化 + honest 7→3-4 grounding 校正 + 默认零依赖 / best-effort 守线」** 策略，分 4 个决策点：

### D1 — rust-silent-failure-surfacing（`index_session_backend` append + retriever desync-skip 经 eprintln! WARN 显式化；best-effort 保形；guard 测试）（task-35.1）🟢

为两处真实静默错误补 `eprintln!` WARN（镜像 `search.rs:109`）：

- `core/src/jobs/index_session_backend.rs:201`：`let _ = store.append(&job_id_context, "indexing", evt.processed_files, evt.total_files, "")` 改为 `if let Err(e) = store.append(...) { eprintln!("WARN indexing-event persist failed (job={job_id_context}): {e}"); }`。**仍 best-effort**（不阻断 indexing）——observability only，镜像 `search.rs:109`。
- `core/src/retriever/mod.rs:415`：`Err(_) => continue` 改为 `Err(e) => { eprintln!("WARN retriever: chunk {chunk_id} present in index but missing from SQLite (desync), skipping: {e}"); continue; }`。**行为（skip）保留**——observability only，镜像既有 retriever `eprintln!` 惯例（filter no-op 路径附近）。
- `core/src/jobs/index_session_backend.rs:193` 的 `eb.send`（no-subscribers `SendError`）**LEAVE AS-IS**（intentional，无订阅者返 `Err` 是正常状态，加 WARN 是噪声）。

**0 新依赖、0 schema migration、0 proto 改动、不引入 metrics facility**。Tests（据实——Rust std unit test 断言 stderr 输出 awkward，仓库既有 `eprintln!` 站点不断言其输出 → 用 GUARD / behavior-preservation 测试，与仓库惯例 + 既有 verify-only guard task（如 task-34.3）一致）：TEST-35.1.1（注入失败的 `IndexingEventStore` test-double（append 返 `Err`），断言 index session 仍成功完成、新错误分支被行使、best-effort 保形）+ TEST-35.1.2（构造 `chunk_id` 在 Tantivy index 中但缺于 SQLite chunks 表的状态，run `query()`，断言该命中被跳过 **且** query 无错返回其余有效命中，behavior-lock guard）。

**理由**：两处确为静默吞掉真实错误（SQLite append 失败 / Tantivy⇄SQLite desync），`eprintln!` WARN 是「make-silent-failures-explicit」的忠实解读；镜像 `search.rs:109` best-effort 范本最 surgical（行为保形、observability only、0 dep）。**HONEST CAVEAT（ADR-013）**：Rust 侧 **不**断言 stderr 输出（仓库无此惯例，强断言 awkward 且脆弱）——AC 据实写「错误经 `eprintln!` 显式化（inspection，与仓库惯例一致）+ 行为保形（automated guard test）」，**不**声称 Rust stderr-output 被断言。

### D2 — go-silent-failure-surfacing（`setVectorEnv` config.Load/Setenv 经 Fprintf 显式化；stderr-capture RED→GREEN；memstore nil-sink 🟡 impl-grounding）（task-35.2）🟢

为 `setVectorEnv` 静默错误补 `fmt.Fprintf(os.Stderr)`（镜像 `daemon/rest.go:110`）：

- `cmd/contextforge/main.go:297`：`setVectorEnv` 中 `cfg, err := config.Load(dataDir); if err != nil { return restore }` 在返回 restore 前经 `fmt.Fprintf(os.Stderr, "contextforge: vector config load failed (%s): %v\n", dataDir, err)` 显式化 malformed/unreadable config.toml 错误。
- `cmd/contextforge/main.go:308`：`os.Setenv` 失败被静默丢弃（仅 success 时记 restore）——可选在 setenv 失败时补一条 WARN。**仍 best-effort**（失败时 env-only 路径不变）——observability only，镜像 `daemon/rest.go:110`。
- `internal/consoleapi/memstore.go:579`（`emitMemoryEvent`）`s.emit == nil` no-op = **🟡 BORDERLINE**：可能 by-design（`MemMemoryStore` 是 fallback/test double，sink 可选）。task-35.2 须在**实施时** ground `MemMemoryStore` 是否在「sink 被期望」的生产路径接线；IF yes → 加一次性（`sync.Once`）degraded-observability WARN（首次因 nil sink 丢事件时，0-dep、非噪声）；IF fallback/test-only by-design → DROP 并记为 honest 非问题。此项 **🟡 pending impl-grounding**，**不**断言为必交付。

**0 新依赖、不引入 metrics facility**。Tests：TEST-35.2.1（**最强测试**——Go 测试可经 `os.Pipe` 重定向捕获 `os.Stderr` → genuine RED→GREEN：写 MALFORMED config.toml 入 temp dataDir、调 `setVectorEnv`、断言 captured stderr 含 WARN 行；valid/missing config 断言**无** WARN；断言 env-wins + restore 行为保留，扩展 task-34.2 既有 `TestSetVectorEnv`）。memstore nil-sink（🟡 impl-grounding 决策）折入 task-35.2 叙事：若 KEEP 则在 memstore 测试中得一条断言，若 DROP 则记为 honest 非问题——**不**为其预分配独立通过的 TEST-ID。

**理由**：`setVectorEnv`（来自 ADR-039 task-34.2）的 `config.Load` 错误静默是真实缺口，`fmt.Fprintf(os.Stderr)` 是「make-silent-failures-explicit」的忠实解读；镜像 `daemon/rest.go:110` audit-surfacing 范本（ADR-036 task-31.3 确立）最 surgical。stderr-capture 经 `os.Pipe` 是仓库可达的 genuine RED→GREEN（Go 侧可断言 stderr，与 Rust 侧不同——据实区分）。memstore nil-sink 🟡 据 ADR-013 须实施时 grounding 决定 KEEP/DROP、不预断言。

### D3 — grounding correction（honest 7→3-4 reduction，据实记录）（task-35.3）🟢 / grounding-correction

survey 列出的 7 处候选经 grounding 复核 **4 处 DROP**，据实记录（这是本 phase 的 ADR-013 价值）：

- `search.rs:109` — **already-surfaced**（已 `eprintln!` WARN；加结构化计数器 = over-engineering，core 无 metrics facility）→ DROP。
- `server.go:298`（`writeAudit`）— **already-done**（task-31.3 / ADR-036 D3 nit 2 已 `fmt.Fprintf(os.Stderr)`）→ DROP。
- `allowlist.go:31` — **intentional POSIX-only platform caveat**（Windows perm bits 无意义；令其有意义须 `golang.org/x/sys/windows` 新 dep 破 0-dep）→ DROP。
- `eb.send:193` — **intentional no-subscribers**（broadcast 无订阅者返 `Err` 是正常状态，加 WARN 是噪声）→ LEAVE AS-IS。
- **不引入新 metrics facility**（core `eprintln!` / Go `Fprintf` only）。

**理由**：据 ADR-013，对 already-done / intentional-by-design 项诚实记录、不伪造为新工作、不夸大缺口。把 already-surfaced 的 `search.rs:109` 或 already-done 的 `server.go:298` 重做、把 intentional 的 `allowlist.go:31`（Windows-ACL）/ `eb.send:193`（no-subscribers）当 gap 改，都是违 Simplicity-First + ADR-013。据实 7→3-4 reduction 正是本 phase 的核心价值。

### D4 — 默认行为 + 0-dep + 0-network + 既有契约不变（all tasks）🟢

所有改动保持默认行为 / proto / 既有契约不变 + 0 网络（ADR-004）+ 0 新依赖（ADR-008，Phase 35 = 0 dep，不引 log/tracing/metrics facility）：

- surfacing 为 **observability-only**：best-effort 仍 best-effort，**不** fail-fast（indexing 不阻断、query 续行、热路径非阻断、env-only 路径失败不变）。
- D1 `eprintln!` WARN（Rust）/ D2 `fmt.Fprintf(os.Stderr)`（Go）均镜像既有 stderr 惯例，无 severity framework、无 metrics facility。
- 0 proto / 0 migration / 0 新 dep。
- 既有 `cargo-test` / `go-test` / `lint` / `spec-lint` 四门不退化。

**理由**：ADR-004 local-first + ADR-008 dep add-only——默认行为 / proto / 既有契约不变 + 0 网络 + 0 新依赖（不引 log/tracing/metrics facility）是不可让渡 baseline。本 phase 为可观测性硬化 + 诚实化——非默认行为演进。observability-only（best-effort 保形不转 fail-fast）/ 镜像既有 stderr 惯例（无新 framework）/ 0 proto / 0 migration 使既有用户与既有契约零感知。

## Consequences

- **Positive**: 两处 Rust 真实静默错误显式化（D1 `index_session_backend` append 失败 + retriever Tantivy⇄SQLite desync-skip 经 `eprintln!` WARN，镜像 `search.rs:109`，best-effort 保形，guard 测试 TEST-35.1.1/35.1.2，0 dep / 0 proto / 0 migration）；`setVectorEnv` config.Load/Setenv 静默错误显式化（D2 经 `fmt.Fprintf(os.Stderr)`，镜像 `daemon/rest.go:110`，stderr-capture genuine RED→GREEN TEST-35.2.1，env-wins + restore 保形）；honest 7→3-4 reduction 据实记录（D3：`search.rs:109` already-surfaced / `server.go:298` already-done(task-31.3) / `allowlist.go:31` intentional POSIX-only / `eb.send:193` intentional no-subscribers 全 DROP，不引新 metrics facility）；全部 0-dep / 0-network / observability-only（best-effort 保形不转 fail-fast），默认行为 / proto / 既有契约不变（ADR-004 / ADR-008），既有四门不退化。
- **Negative / open**（受阻 / 非问题项如实，不伪造、不夸大）：memstore nil-sink degraded-observability warn（D2）须实施时 ground `MemMemoryStore` 是否生产路径接线 → 🟡 impl-grounding `[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]`（若 production-wired 则加一次性 `sync.Once` WARN，若 fallback/test-only by-design 则 DROP 记 honest 非问题、不预断言）；structured metrics/counter facility for observability（core 无此 facility，stderr surfacing 是忠实 scope）→ honest-defer `[SPEC-DEFER:phase-future.observability-metrics-facility]`；already-surfaced / intentional 的 DROP 站点（`search.rs:109` 计数器 / `server.go:298` / `allowlist.go:31` Windows-ACL / `eb.send` no-subscribers）**不**重做（grounding 校正）；Rust 侧 stderr-output **不**被断言（仓库无此惯例，行为保形 guard 测试 + inspection，据 ADR-013 不伪造 stderr-assert）——以上据 ADR-013 如实分级、不伪造完成、不夸大缺口。
- **Ratification**: 本 ADR **Accepted**（v0.28.0 / task-35.3 closeout）。task-35.1（#229）/ 35.2（#230）通过后据真实 CI / 实测产物逐 D ratify Proposed→Accepted（见 §Ratification）；memstore nil-sink 🟡 impl-grounding 经 grounding 解析为 **by-design honest non-issue**（生产唯一调用点 `console_api_serve.go:109` 紧随无条件 `SetEventSink`:112）→ DROP，据实记录（ADR-013：禁据合成 / 伪造 ratify；不为 Rust 伪造 stderr-assert 证据）。
- **Follow-ups**: structured metrics/counter facility for observability（core 现无 facility，引入须 simplicity-first + exposure path 评估后）`[SPEC-DEFER:phase-future.observability-metrics-facility]`；memstore nil-sink degraded warn（IF grounding 示 `MemMemoryStore` sink optional-by-design 则 honest 非问题，IF production-wired 则一次性 warn）`[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]`。ADR-031（stderr / best-effort surfacing 方向）以 add-only 引用承接、不溯改正文（ADR-014 D5）；ADR-036 / ADR-038 / ADR-039 / ADR-004 / ADR-008 / ADR-013 引用均不溯改其正文。

## Ratification（v0.28.0 / task-35.3）

本 ADR 于 v0.28.0 closeout（task-35.3）据 task-35.1（#229 squash 9a57647）/ 35.2（#230 squash 69fc367）/ 35.3（this PR）真实 CI（四门绿：cargo-test / go-test / lint / spec-lint）逐 D ratify Proposed→Accepted（ADR-013 不预填、不据合成 ratify、不为 Rust 伪造 stderr-assert）：

- **D1（rust-silent-failure-surfacing）→ Accepted 🟢**：task-35.1（#229）`index_session_backend.rs` **4 处** `store.append`（progress/index-error/commit-error/cancelled，grounding 发现是 4 处 emit 点非 1，一致显式化）`let _ =` → `if let Err(persist_err) { eprintln!("WARN indexing-event persist failed …: {persist_err}") }`（best-effort 保留，不阻断 indexing）+ `retriever/mod.rs:415` `Err(_) => continue` → `Err(e) => { eprintln!("WARN retriever: … desync …"); continue }`（skip 保留）；`eb.send` 各处保留 as-is（no-subscribers intentional）。TEST-35.1.1（真实 `SqliteIndexingEventStore` 接线 best-effort 行为锁）+ TEST-35.1.2（删 chunks 行造 desync → `query()` 优雅跳过返回）绿（`cargo test -p contextforge-core --lib` 209→212）。**HONEST（ADR-013）**：`SqliteIndexingEventStore` 是具体类型无 trait → 注入失败 double 须引 trait = scope creep，不做；error 分支 eprintln! 为机械改动 inspection-verified，仓库惯例不断言 eprintln! 输出 `[SPEC-DEFER:phase-future.rust-stderr-output-assertion]`；retriever `:373` surgical 留 as-is `[SPEC-DEFER:phase-future.tantivy-docstore-read-surface]`。
- **D2（go-silent-failure-surfacing）→ Accepted 🟢**：task-35.2（#230）`setVectorEnv` `config.Load` 错误（`if !errors.Is(err, os.ErrNotExist) { fmt.Fprintf(os.Stderr, "contextforge: vector config load failed (%s): %v\n", dataDir, err) }`，missing 静默/malformed 报警）+ `os.Setenv` 失败补 WARN，镜像 `daemon/rest.go:110`，best-effort 保留。TEST-35.2.1 stderr-capture（`os.Pipe`）真 RED→GREEN：malformed→WARN / missing→no WARN / valid→no WARN + env-wins + restore 保持（`go test ./cmd/contextforge/ -run TestSetVectorEnv` 6/6）。**grounding 校正（ADR-013）**：(a) config.Load 对 MISSING config.toml 也返 error（`os.Open` 失败）→ 据实加 `os.ErrNotExist` 守护避免无配置常见场景误报；(b) `memstore.go:579` nil-sink = **honest non-issue（DROP，`memstore.go` 0 改动）**——`NewMemMemoryStore()` 唯一生产调用点 `internal/cli/console_api_serve.go:109` 紧随**无条件** `:112 SetEventSink(store.EmitEvent)`，nil-sink 仅测试/by-design，加一次性 WARN 只在测试触发=噪声无生产价值 `[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]`。
- **D3（grounding correction 7→3-4）→ Accepted 🟢**：survey 7 候选经 grounding 复核 4 处 DROP/LEAVE 不改代码——`search.rs:109` already-surfaced（core 无 metrics facility，加计数器=over-engineering）/ `mcpadapter/server.go:298` already-done（task-31.3）/ `mcpadapter/allowlist.go:31` intentional POSIX-only 平台 caveat / `index_session_backend.rs:193` `eb.send` intentional no-subscribers；**不引入新 metrics facility** `[SPEC-DEFER:phase-future.observability-metrics-facility]`。
- **D4（默认行为 + 0-dep + 0-network + 既有契约不变）→ Accepted 🟢**：surfacing observability-only（best-effort 仍 best-effort，不转 fail-fast——indexing 不阻断 / query 续行 / env-only 路径失败不变 / daemon 不阻断）；0 新 dep（不引 log/tracing/metrics facility）/ 0 proto / 0 migration；`memstore.go` 0 改动；既有 `cargo test --workspace` + `go test ./...` + lint + spec-lint 四门不退化。

真实 v0.28.0 tag/run/digest 经用户授权后由 post-tag-push backfill 填实（release docs `<backfill>`，ADR-013 不预填）。本 closeout 本地四门绿：`cargo test -p contextforge-core --lib` 212 + `go test ./...` 全过 + `cargo clippy --workspace --all-targets -- -D warnings` 0 warning + `bash -n scripts/console_smoke.sh` exit 0。

## Alternatives

- **A1（引入 metrics/counter facility）**：在 core 加一个 metrics/counter facility 暴露 observability。否决：core **无** metrics facility（仅测试原子量），加一个破 simplicity-first + 须 exposure path（proto/health）= scope creep；据 D1/D3，`eprintln!` / `fmt.Fprintf(os.Stderr)` stderr surfacing 是「make-silent-failures-explicit」的忠实解读，最 surgical 且 0-dep。
- **A2（把 best-effort 转为 fail-fast / 返回错误）**：把静默站点改为返回 / 传播错误。否决：破既有契约（indexing 不阻断、query 续行、热路径非阻断，ADR-004）；据 D4 surfacing 为 observability-only（best-effort 仍 best-effort，MUST NOT 转 fail-fast）。
- **A3（也改 DROPPED 站点）**：按 survey 重做 `search.rs:109` / `server.go:298` / `allowlist.go:31` / `eb.send:193`。否决：它们 already-surfaced（`search.rs:109` / `server.go:298` task-31.3）或 intentional-by-design（`allowlist.go:31` Windows-ACL 须新 dep / `eb.send:193` no-subscribers 正常状态）——重做是把 already-done 伪造为新工作 + 把 intentional 当 gap（违 Simplicity-First + ADR-013）；据 D3 据实 7→3-4 reduction、记 grounding 校正、不重做。

## 触及 ADR 关系

- **ADR-031（observability-hardening，v0.19.0 母 ADR）→ add-only 引用承接（不溯改）**：本 phase 承其 stderr / best-effort surfacing 方向（TraceStore / events / event-bus 之外，向热路径静默错误延伸），以 add-only 引用记，**不溯改 ADR-031 正文**（ADR-014 D5）。
- **ADR-036（governance-debt-cleanup）→ 镜像 + 据实引用（不溯改）**：task-31.3 已确立 Go `fmt.Fprintf(os.Stderr)` audit-surfacing 模式（D3 nit 2），本 phase D2 镜像该模式；其已修的 `server.go:298` 即本 phase D3 grounding 校正 DROP（already-done）的站点，据实引用、不溯改其正文。
- **ADR-038（governance-debt-cleanup-2）→ 承接血脉引用（不溯改）**：本 phase 承 Phase 33 治理债血脉、第三轮债清理性质（边际递减据实陈述），承其诚实优先于凑量方向，不溯改其正文。
- **ADR-039（vector-config-completeness）→ 据实引用（不溯改）**：D2 的 `setVectorEnv` 来自其 task-34.2，本 phase 在其 `config.Load` / `Setenv` 静默路径上补 surfacing（observability only，不改 env-wins / restore 契约），承其方向、不溯改其正文。
- **ADR-004（local-first-privacy-baseline）→ 守线**：默认行为 / proto / 既有契约不变 + 0 网络 + observability-only（best-effort 保形不转 fail-fast）（D4）守 ADR-004 baseline。
- **ADR-008（dep add-only）→ 守线**：本 phase 加 **0 新依赖**——不引 log / tracing / metrics facility（Rust `eprintln!` / Go `fmt.Fprintf(os.Stderr)` 既有惯例），memstore 一次性 warn（IF KEEP）用 std `sync.Once` 0-dep。
- **ADR-013（禁伪造红线）→ 守线**：honest 7→3-4 reduction 据实记录（4 DROP / 3-4 KEEP）；不为 Rust 伪造 stderr-assert 证据（行为保形 guard + inspection 据实）；memstore nil-sink 🟡 impl-grounding 实施时据实 KEEP/DROP、不预断言；DROP 站点据实记 already-done / intentional、不夸大为 gap（D1 / D2 / D3）。
- **ADR-014（cross-phase-exit-criteria-validation）→ 第二十六次激活**：D1-D4 mapping + 各 task LAST D2 lint（touched 行 0 未标注命中）+ D1/D2 verified-by（TEST-35.1.1/35.1.2 guard、TEST-35.2.1 stderr-capture）+ D4 自治 + D5 历史 Phase 1-34 不溯改（ADR 改动 add-only 引用、不溯改 ADR-031 正文）；本 ADR ratify 在 task-35.3 closeout，Proposed 阶段不 ratify。
