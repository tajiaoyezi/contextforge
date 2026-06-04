# Task `35.1`: `rust-silent-failure-surfacing — core 热路径中两处被静默吞掉的真实错误显式化（index_session_backend.rs:201 store.append 的 let _ = 持久化失败 + retriever/mod.rs:415 Err(_)=>continue 的 Tantivy/SQLite desync），镜像仓库既有 eprintln! WARN 惯例（search.rs:109 / server.rs:669）；best-effort 契约不变（不阻断 indexing / query 继续 skip）= observability-only，非 fail-fast；0 新 dep / 0 schema migration / 0 network；eb.send:193（no-subscribers 故意 swallow）按原样保留不动`

**Status**: Done

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 35 (observability-hardening)
**Dependencies**: 既有 `core/src/jobs/index_session_backend.rs:196-209`（task-33.3 / ADR-038 D3 replay-source 进度行持久化，`:201` `let _ = store.append(...)` best-effort 写——本 task 把 `let _` 替换为 `if let Err(e)` 显式化，best-effort 不变）/ `core/src/retriever/mod.rs:413-416`（task-19.x 语义检索 SQLite JOIN-by-chunk_id 热路径，`:415` `Err(_) => continue` 静默 skip desync hit——本 task 在 `continue` 前显式化）/ `core/src/data_plane/search.rs:108-113`（task-16.1 write-through best-effort `eprintln!("WARN ...")` 惯例——本 task 镜像源）/ `core/src/server.rs:669`（`eprintln!("INFO ...")` 启动期惯例）/ ADR-040（observability-hardening §D1，本 task 即其原文实现，ratify @ task-35.3 closeout）/ ADR-031（observability-hardening v0.19.0 母 ADR——本 phase 承其 stderr/best-effort surfacing 方向 add-only）/ ADR-036（governance-debt-cleanup，task-31.3 确立 Go stderr audit-surfacing pattern，本 phase 镜像其形态）/ ADR-004（local-first-privacy-baseline，默认行为 + 既有契约不变 + 默认 build 0 新 dep）/ ADR-008（dep add-only，本 task = 0 新 dep）/ ADR-013（禁伪造红线——Rust eprintln! 输出据实声明「不断言 stderr 输出」，仅 guard/behavior-preservation 测试，不夸大；honest 7→3-4 site 缩减据实记入）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D4（第二十六次激活）

## 1. Background

ADR-031（v0.19.0 observability-hardening）确立的 stderr/best-effort surfacing 方向下，core 热路径仍残留若干处把**真实**错误静默吞掉的点——经 grounding 核实后从初列的 7 处诚实收敛为 vector 侧 task-35.2 与本 task（Rust 侧）的 **2 处真静默**，其余皆已显式化或属故意设计（见 §3 范围外 + ADR-040 D3 honest 缩减）。本 task 聚焦 core 两处把真实错误吞掉的 `let _` / `Err(_)`，镜像仓库既有 `eprintln!` WARN 惯例显式化，best-effort 契约不变：

- **B1 `store.append` 的 `let _ =` 吞掉真实持久化错误**：`core/src/jobs/index_session_backend.rs:201` 的 `let _ = store.append(&job_id_context, "indexing", evt.processed_files, evt.total_files, "")`（`:201-207`）把 `indexing_event_store` SQLite append 的**真实**失败（磁盘满 / 文件锁 / I/O 错误）静默丢弃——这与 `:193` 的 `eb.send`（broadcast no-subscribers 的 `Err` 属正常条件，见 B3）不同，store.append 返 `Err` 是真实持久化故障，被无声吞掉后运维无任何线索。本 task 把 `let _` 替换为 `if let Err(e) = store.append(...) { eprintln!("WARN ...: {e}"); }`，**仍 best-effort**（写失败不阻断 indexing，与改前一致）= observability-only，镜像 `search.rs:109`。
- **B2 retriever `Err(_) => continue` 吞掉 Tantivy/SQLite desync**：`core/src/retriever/mod.rs:415` 的 `let (file_path, content, indexed_at) = match row { Ok(t) => t, Err(_) => continue }`（`:413-416`）在 Tantivy 索引命中某 `chunk_id` 但 SQLite `chunks` 表无对应行（两库暂时不同步 / desync）时静默 skip 该 hit，吞掉真实错误。本 task 在 `continue` 前加 `eprintln!("WARN retriever: chunk ... (desync), skipping: {e}")`，**skip 行为保留**（query 继续返其余有效 hit）= observability-only，镜像 retriever 既有 eprintln! 惯例。
- **B3 `eb.send:193` 故意保留不动（intentional no-subscribers）**：`index_session_backend.rs:193` 的 `let _ = eb.send(pb_evt)` 是**故意**的——其上方 `:185` 既有注释明言「SendError swallowed since no subscribers is acceptable」。broadcast send 在无订阅者时返 `Err` 是**正常条件**而非失败，此处加 WARN 会变成噪声。本 task **按原样保留不改**（不是债，是故意设计）。
- **B4 facility = 仅 eprintln!（核实后据实声明，非引入新框架）**：Rust core **无** log crate / 无 tracing crate / 无 metrics facility（`core/Cargo.toml` 核实无日志依赖；`AtomicU64` / `AtomicUsize` 仅出现在测试代码）。本 task 镜像既有唯一惯例 `eprintln!` 到 stderr：`search.rs:108-113`（`if let Err(e) = ... { eprintln!("WARN ...: {e}") }` best-effort，不 abort caller）+ `server.rs:669`（`eprintln!("INFO ...")`）。严重级别是消息字符串前缀（`WARN` / `INFO`），**无**严重级别框架。引入任何 metrics/counter facility = over-engineering（simplicity-first + ADR-004/008），不在本交付内 [SPEC-DEFER:phase-future.observability-metrics-facility]。

经核 core 仅 `eprintln!` 一种 surfacing 惯例（`search.rs:108-113` / `server.rs:669`），本 task 两处显式化为 code-local 🟢，0 新 dep（无 facility 引入）+ 0 schema migration（纯分支改动，无表）。

## 2. Goal

(1) **B1**：`index_session_backend.rs:201` 把 `let _ = store.append(...)` 替换为 `if let Err(e) = store.append(...) { eprintln!("WARN indexing-event persist failed (job={job_id_context}): {e}"); }`——store.append 的真实持久化失败显式化到 stderr（WARN 前缀，镜像 `search.rs:109`），**best-effort 不变**（写失败不阻断 indexing，error 分支不向外传播）。(2) **B2**：`retriever/mod.rs:415` 把 `Err(_) => continue` 替换为 `Err(e) => { eprintln!("WARN retriever: chunk {chunk_id} present in index but missing from SQLite (desync), skipping: {e}"); continue; }`——Tantivy/SQLite desync 显式化，**skip 行为保留**（query 继续返其余有效 hit）。(3) **B3 据实**：`eb.send:193`（no-subscribers 故意 swallow）按原样保留不动，spec / ADR-040 D1 据实记其为故意设计非债（ADR-013 不把故意设计错记为缺口）。(4) **facility 据实**：仅用既有 `eprintln!`，不引入任何 log/tracing/metrics 框架；Rust eprintln! 输出**不断言**（仓库既有 eprintln! 站点亦不断言输出），用 guard / behavior-preservation 测试据实声明（ADR-013 不夸大为已断言 stderr 输出）。

pass bar：`index_session_backend` 注入返 `Err` 的 `IndexingEventStore` test-double 后 index session 仍成功完成（best-effort 保留，新 error 分支被触发）（🟢 guard）；retriever 构造「chunk_id 在 Tantivy 索引中、在 SQLite `chunks` 表中缺失」的状态后 `query()` skip 该 hit 且返其余有效 hit 不报错（behavior-lock guard）（🟢）；error 经 `eprintln!` surfacing（inspection，与仓库既有 eprintln! 惯例一致——据实声明非自动断言 stderr 输出，ADR-013）；`eb.send:193` 不动；0 新 dep（ADR-008）+ 0 schema migration + 既有 best-effort 契约不变（非 fail-fast，ADR-004）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `core/src/jobs/index_session_backend.rs:201`——`let _ = store.append(&job_id_context, "indexing", evt.processed_files, evt.total_files, "")` 替换为 `if let Err(e) = store.append(&job_id_context, "indexing", evt.processed_files, evt.total_files, "") { eprintln!("WARN indexing-event persist failed (job={job_id_context}): {e}"); }`（best-effort 不变——error 分支只 surfacing、不向外传播、不阻断 indexing）。镜像 `search.rs:109`。
- 改 `core/src/retriever/mod.rs:415`——`Err(_) => continue` 替换为 `Err(e) => { eprintln!("WARN retriever: chunk {chunk_id} present in index but missing from SQLite (desync), skipping: {e}"); continue; }`（skip 行为保留——`continue` 不变，仅在其前 surfacing）。镜像 retriever 既有 eprintln! 惯例。
- guard / behavior-preservation 测试：TEST-35.1.1（注入失败 `IndexingEventStore` test-double，断言 index session 仍成功完成 = best-effort 保留 + 新 error 分支被触发）+ TEST-35.1.2（构造 chunk_id 在 Tantivy 索引、在 SQLite 缺失的 desync 状态，断言 `query()` skip 该 hit 且返其余有效 hit 无错 = behavior-lock）。
- `eb.send:193`：**不改**（故意 no-subscribers swallow，§1 B3）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 为 core 引入 structured metrics/counter facility（core 现无任何 metrics facility，仅 `eprintln!`；stderr surfacing 是 make-silent-failures-explicit 的忠实范围，引入 counter facility 还需暴露路径 = scope creep）[SPEC-DEFER:phase-future.observability-metrics-facility]——本 task 用既有 `eprintln!` 显式化，不引入新框架（ADR-004/008 simplicity-first）。
- 自动断言 Rust eprintln! stderr 输出内容（std 单测断言 stderr 输出笨拙，且仓库既有 eprintln! 站点 `search.rs` / `server.rs` 均不断言输出——本 task 与仓库惯例一致用 guard / behavior-preservation 测试）[SPEC-DEFER:phase-future.rust-stderr-output-assertion]——本 task 据实声明 error 经 eprintln! surfacing（inspection）+ 行为保留（自动 guard 测试），不声称 stderr 输出被自动断言（ADR-013）。
- `eb.send:193` no-subscribers 路径改动（broadcast 无订阅者返 `Err` 属正常条件，加 WARN = 噪声，故意保留）[SPEC-DEFER:phase-future.eb-send-no-subscribers-warn]——本 task 据实记其为故意设计非债，按原样保留。
- 已显式化 / 故意设计的其余 site（`search.rs:109` 已 surfacing / Go `server.go:298` 已于 task-31.3 done / `allowlist.go:31` POSIX-only 平台 caveat）由 grounding correction 排除（见 ADR-040 D3）[SPEC-DEFER:phase-future.already-surfaced-sites-no-rework]——本交付改 Rust 侧两处真静默。
- 真实 release tag / run-id / digest（v0.28.0）[SPEC-OWNER:task-35.3-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `store.append` best-effort 持久化点（`core/src/jobs/index_session_backend.rs:199-208`，本 task 把 `:201` `let _ = store.append(...)` 替换为 `if let Err(e) = store.append(...) { eprintln!("WARN ...") }`）
- retriever SQLite JOIN-by-chunk_id 热路径（`core/src/retriever/mod.rs:401-416`，本 task 把 `:415` `Err(_) => continue` 替换为 `Err(e) => { eprintln!("WARN ...desync..."); continue; }`，`chunk_id` 于 `:404` 在作用域内）
- `eb.send` no-subscribers 点（`core/src/jobs/index_session_backend.rs:193`，本 task **不改**，故意 swallow，§1 B3）
- `eprintln!` 既有惯例镜像源（`core/src/data_plane/search.rs:108-113` WARN best-effort 不 abort caller / `core/src/server.rs:669` INFO）
- `IndexingEventStore` test-double（TEST-35.1.1 注入返 `Err` 的 append，触发新 error 分支 + 断言 best-effort 保留）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/jobs/index_session_backend.rs:196-209`（task-33.3 / ADR-038 D3 replay-source 进度行持久化块——`:199` `if let Some(store) = &indexing_store` / `:201-207` `let _ = store.append(...)` best-effort 写点，本 task 把 `:201` `let _` 替换为 `if let Err(e)` + `eprintln!("WARN ...")`，best-effort 不变）
- `core/src/jobs/index_session_backend.rs:185-194`（`:185` 注释「SendError swallowed since no subscribers is acceptable」 + `:193` `let _ = eb.send(pb_evt)`——本 task **不改**，故意 no-subscribers swallow，§1 B3）
- `core/src/retriever/mod.rs:400-416`（`:401-412` SQLite `SELECT file_path, content, indexed_at FROM chunks WHERE chunk_id = ?1` JOIN / `:404` `params![chunk_id]`（`chunk_id` 在作用域） / `:413-416` `match row { Ok(t) => t, Err(_) => continue }`——本 task 把 `:415` `Err(_) => continue` 替换为 `Err(e) => { eprintln!("WARN ...desync..."); continue; }`，skip 行为保留）
- `core/src/data_plane/search.rs:108-113`（write-through best-effort `if let Some(p) = self.persist.as_ref() { if let Err(e) = p.put(...) { eprintln!("WARN search_persist.put failed (key={key}); hot cache still updated: {e}") } }`——本 task `index_session_backend` 侧 WARN 形态的镜像源：best-effort、不 abort caller、WARN 前缀 + `{e}`）
- `core/src/server.rs:669`（`eprintln!("INFO orphan reaper: marked {} stale job(s) terminal at startup", reaped)`——core 既有 INFO 前缀 stderr 惯例，印证「严重级别 = 消息前缀，无 severity 框架」）
- `core/Cargo.toml`（核实 core 无 log crate / 无 tracing crate / 无 metrics facility——本 task 不引入任何新 facility，仅用既有 `eprintln!`）
- `docs/decisions/adr-040-observability-hardening.md §D1`（本 task 即其原文实现）+ `docs/decisions/adr-031-*.md`（observability-hardening v0.19.0 母 ADR，stderr/best-effort surfacing 方向，本 phase add-only 承之）+ `docs/decisions/adr-036-*.md`（task-31.3 Go stderr audit-surfacing pattern，本 phase 镜像形态）

### 5.2 关键设计 — `let _`/`Err(_)` → `eprintln!` WARN 显式化（best-effort 不变 / 0 facility / 0 dep / 0 migration）

- **B1 `store.append` 显式化镜像 `search.rs:109`**：`index_session_backend.rs:201` 的 `let _ = store.append(&job_id_context, "indexing", evt.processed_files, evt.total_files, "")` 替换为：
  ```rust
  if let Err(e) = store.append(
      &job_id_context,
      "indexing",
      evt.processed_files,
      evt.total_files,
      "",
  ) {
      eprintln!("WARN indexing-event persist failed (job={job_id_context}): {e}");
  }
  ```
  形态镜像 `search.rs:108-113`：error 分支只 `eprintln!` WARN（前缀 + `{e}`）后**不向外传播、不阻断 indexing**——store.append 的真实持久化失败（磁盘满 / 锁 / I/O）现可见于 stderr，best-effort 契约不变（observability-only，非 fail-fast）。
- **B2 retriever desync 显式化保留 skip**：`retriever/mod.rs:415` 的 `Err(_) => continue` 替换为：
  ```rust
  Err(e) => {
      eprintln!("WARN retriever: chunk {chunk_id} present in index but missing from SQLite (desync), skipping: {e}");
      continue;
  }
  ```
  `chunk_id`（`:404` `params![chunk_id]`）在作用域内可用于消息；`continue` 不变（skip 该 desync hit 后 query 继续返其余有效 hit）——Tantivy/SQLite desync 现可见于 stderr，behavior（skip）保留（observability-only）。镜像 retriever 既有 eprintln! 惯例。
- **B3 `eb.send:193` 故意保留**：`:185` 注释明言 no-subscribers 的 `SendError` swallow 是 acceptable；broadcast 无订阅者返 `Err` 属正常条件而非失败，加 WARN = 噪声——本 task **不改** `:193`（ADR-013 不把故意设计错记为缺口；spec / ADR-040 D1 据实记其为故意设计）。
- **B4 facility 据实仅 eprintln!（不引入新框架）**：core `Cargo.toml` 无 log/tracing/metrics 依赖（`Atomic*` 仅测试代码），本 task 镜像既有唯一 surfacing 惯例 `eprintln!`（WARN/INFO 前缀 + `{e}`）；不引入 metrics/counter facility（= over-engineering + 需暴露路径 scope creep，[SPEC-DEFER:phase-future.observability-metrics-facility]）。
- **testability 据实（不断言 stderr 输出）**：std 单测断言 eprintln! stderr 输出笨拙，且仓库既有 eprintln! 站点（`search.rs` / `server.rs`）均不断言其输出——本 task 与仓库惯例一致用 **guard / behavior-preservation** 测试：TEST-35.1.1 注入返 `Err` 的 `IndexingEventStore` test-double，断言 index session 仍成功完成（新 error 分支被触发 + best-effort 保留）；TEST-35.1.2 构造 desync 状态断言 `query()` skip 该 hit 且返其余有效 hit 无错（behavior-lock）。AC 据实声明：error 经 `eprintln!` surfacing（inspection，与仓库惯例一致）+ 行为保留（自动 guard 测试）——**不声称 stderr 输出被自动断言**（ADR-013，[SPEC-DEFER:phase-future.rust-stderr-output-assertion]）。

### 5.3 不变量

- best-effort 契约不变（ADR-004，**不**转 fail-fast）：`store.append` 失败仍**不阻断 indexing**（error 分支只 surfacing 不向外传播，与改前 `let _` 对 caller 可观察行为一致——index session 照常完成）；retriever desync hit 仍**被 skip**（`continue` 保留，query 继续返其余有效 hit）；surfacing = observability-only，MUST NOT 把 best-effort 变 fail-fast。
- 既有契约不变：`index_session` / `query` 公共签名与返回类型不变；`IndexingEventStore` trait 不改；retriever `query()` 在无 desync 时返回结果不变（既有 retriever / indexing 测试不退化）。
- 0 新代码依赖（ADR-008）：仅用既有 `eprintln!`（std 宏），无 Cargo 依赖增量、不引入 log/tracing/metrics facility；Rust core 默认 build dep 集不变（ADR-004 local-first）。
- 0 schema migration：纯分支改动（`let _`→`if let Err` / `Err(_)`→`Err(e)`），无表 / 无持久化结构变更，不加列、不 `ALTER`、不新增编号 migration。
- 0 network：surfacing 仅写本地 stderr，无网络调用。
- `eb.send:193` 不动：故意 no-subscribers swallow 按原样保留（§1 B3）；本交付改 `store.append:201` + `retriever:415` 两处真静默。
- testability 诚实边界（ADR-013）：Rust eprintln! 输出**不自动断言**（仓库惯例），用 guard / behavior-preservation 测试；不夸大为「stderr 输出已被断言」；自动断言 stderr 输出 → [SPEC-DEFER:phase-future.rust-stderr-output-assertion] 据实延后，不预填。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（两处 surfacing + best-effort 保留 guard 🟢）: `index_session_backend.rs:201` `let _ = store.append(...)` 替换为 `if let Err(e) = store.append(...) { eprintln!("WARN indexing-event persist failed (job={job_id_context}): {e}"); }`（best-effort 不变，error 分支不阻断 indexing，镜像 `search.rs:109`）；`retriever/mod.rs:415` `Err(_) => continue` 替换为 `Err(e) => { eprintln!("WARN retriever: chunk {chunk_id} ... (desync), skipping: {e}"); continue; }`（skip 行为保留）；`eb.send:193` 不动；error 经 `eprintln!` surfacing（inspection，与仓库既有 eprintln! 惯例一致——据实声明非自动断言 stderr 输出）+ 行为保留（自动 guard）；**0 新 dep + 0 schema migration + 0 network + best-effort 非 fail-fast** — verified by **TEST-35.1.1**（注入失败 `IndexingEventStore` test-double，index session 仍成功完成 = best-effort 保留 + 新 error 分支被触发）+ **TEST-35.1.2**（chunk_id 在 Tantivy 索引、SQLite `chunks` 缺失的 desync，`query()` skip 该 hit 且返其余有效 hit 无错 = behavior-lock guard）
- [x] **AC2**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-35.1.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-35.1.1 | 注入返 `Err` 的 `IndexingEventStore` test-double（append 失败），断言 index session 仍成功完成（best-effort 保留——error 分支只 surfacing 不阻断 indexing，新 `if let Err(e)` 分支被触发）；0 新 dep + 0 schema migration | `core/src/jobs/index_session_backend.rs`（同源 / 模块 test） | Done |
| TEST-35.1.2 | 构造 chunk_id 在 Tantivy 索引中、在 SQLite `chunks` 表缺失的 desync 状态，运行 `query()`，断言该 hit 被 skip（`continue` 保留）且返回其余有效 hit 无错（behavior-lock guard） | `core/src/retriever/mod.rs`（同源 / 模块 test） | Done |
| TEST-35.1.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）surfacing 误把 best-effort 变 fail-fast**：把 `let _` / `Err(_)` 改成 surfacing 时若误将 error 向外 `return`/`?` 传播，会破 best-effort 契约（阻断 indexing / 中断 query）。
  - **缓解**：B1 error 分支只 `eprintln!` 后**不传播**（无 `?` / 无 `return Err`），与改前 `let _` 对 caller 可观察行为一致；B2 `continue` 保留（skip 不变）；TEST-35.1.1 断言 index session 仍成功完成、TEST-35.1.2 断言 query 返其余有效 hit 无错。stop-condition：任一 guard 显示行为变 fail-fast 则 AC1 不标 `[x]`。
- **R2（中）retriever 消息引用的 `chunk_id` 不在作用域**：`Err(e) =>` 分支若 `chunk_id` 已被 move / 不可见，则消息无法引用。
  - **缓解**：`chunk_id` 于 `:404` `params![chunk_id]` 在同作用域可用（核实在 `match row` 同块内）；消息用 `{chunk_id}` + `{e}`。stop-condition：编译不过则不标 `[x]`。
- **R3（低）误把 `eb.send:193` 一并改动**：`eb.send` 与 `store.append` 同块相邻，易误把 no-subscribers swallow 一并显式化致噪声。
  - **缓解**：本 task scope 明确**只**改 `:201` + `retriever:415`；`:185` 注释 + ADR-040 D1 据实记 `eb.send:193` 为故意设计 → [SPEC-DEFER:phase-future.eb-send-no-subscribers-warn] 不改。stop-condition：`eb.send:193` 被改动则 review 退回。
- **R4（低）误读为「stderr 输出已被自动断言」**：guard / behavior-preservation 测试不断言 stderr 输出，易被夸大为「Rust 侧 stderr 输出已断言」。
  - **缓解**：spec §2 / §5.2 B4 / §5.3 + AC1 + ADR-040 D1 据实记「error 经 eprintln! surfacing（inspection，与仓库惯例一致）+ 行为保留（自动 guard），不断言 stderr 输出」；自动断言 stderr → [SPEC-DEFER:phase-future.rust-stderr-output-assertion] 据实延后（ADR-013 不夸大、不预填）。

## 9. Verification Plan

```bash
# 1. AC1 — index_session_backend best-effort 保留 guard（注入失败 IndexingEventStore）
cargo test -p contextforge-core jobs::index_session_backend

# 2. AC1 — retriever desync skip behavior-lock guard
cargo test -p contextforge-core retriever

# 3. 不退化（全量 + 既有 indexing / retriever 测试）
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# 4. AC2 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.rust-silent-failure-surfacing-defer-note]：本 task 仅把 core 两处真静默（`index_session_backend.rs:201` store.append `let _` / `retriever/mod.rs:415` `Err(_)=>continue` desync）镜像 `search.rs:109` 既有 `eprintln!` WARN 惯例显式化（🟢 guard 可测，0 新 dep + 0 schema migration + 0 network），best-effort 契约不变（observability-only，非 fail-fast）。Rust eprintln! 输出**不自动断言**（仓库既有 eprintln! 站点亦不断言，与惯例一致用 guard / behavior-preservation 测试，据实声明非已断言 stderr 输出，ADR-013）；自动断言 stderr 输出 → [SPEC-DEFER:phase-future.rust-stderr-output-assertion]；为 core 引入 structured metrics/counter facility → [SPEC-DEFER:phase-future.observability-metrics-facility]（core 现无 facility，stderr surfacing 是忠实范围）；`eb.send:193` no-subscribers WARN → [SPEC-DEFER:phase-future.eb-send-no-subscribers-warn]（故意设计非债，按原样保留）。Go 侧 silent-failure surfacing（`setVectorEnv` config.Load/Setenv + memstore nil-sink）见 task-35.2，非本 task 范围。实证数值真实跑出后回填（ADR-013 不伪造）。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification 实证**（real evidence，本地全绿）：
- AC1：`cargo test -p contextforge-core --lib test_35_1` → 2/2 PASS（`test_35_1_1_indexing_event_persist_best_effort_guard` + `test_35_1_2_retriever_desync_skip_guard`）。`cargo test -p contextforge-core --lib` 212 passed / 0 failed（既有 indexing / retriever 测试不退化）；`cargo clippy --workspace --all-targets -- -D warnings` 0 warning。
- AC2：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（本 PR 仅改 code + 本 task spec；CI spec-lint 权威）。

**grounding 校正（实施期，ADR-013）**：
- **`store.append` 实为 4 处非 1**：grounding 复核 `index_session_backend.rs` 发现 `let _ = store.append(...)` 共 4 个 emit 点（progress :201 / index-error / commit-error / cancelled），同一 SQLite persist 失败类、同 best-effort 形态——一致显式化全部 4 处（仅改 1 处会前后不一致）。`eb.send` 各处（progress/error/commit/cancelled）全保留 as-is（broadcast 无订阅者返 Err 是正常态 intentional，§1 B3）。
- **store 为具体类型无 trait → TEST-35.1.1 改 behavior-lock + inspection**：`indexing_event_store: Option<Arc<SqliteIndexingEventStore>>` 是具体类型（无 trait）；注入「返 Err 的 test-double」须引 trait = scope creep（破既有 ctor 契约），据 ADR-013 + simplicity-first **不做**。改交付 behavior-preservation guard：接真实 `SqliteIndexingEventStore` 跑 index → 完成 + 行持久化（`store.append` Ok 分支执行、best-effort 流程不破）；error 分支 eprintln! 为机械改动 inspection-verified（仓库惯例不断言 eprintln! 输出，[SPEC-DEFER:phase-future.rust-stderr-output-assertion]）。
- **retriever `:373` `searcher.doc()` 失败留 as-is**：`:415` 之外另有 `:373` `match searcher.doc(addr) { Err(_) => continue }`（Tantivy doc-store 读失败，不同子系统、更罕见）——本 task scope 限文档化的 `:415` Tantivy/SQLite desync，`:373` surgical 不扩展 [SPEC-DEFER:phase-future.tantivy-docstore-read-surface]。

**实际改动文件**：
- `core/src/jobs/index_session_backend.rs`——4 处 `let _ = store.append(...)` → `if let Err(persist_err) = store.append(...) { eprintln!("WARN indexing-event persist failed (job=..., stage=...): {persist_err}"); }`（best-effort 保留，不阻断 indexing）；`eb.send` 各处不改。+ 同源 `test_35_1_1_indexing_event_persist_best_effort_guard`（真实 store 接线 best-effort 行为锁）。
- `core/src/retriever/mod.rs`——`:415` `Err(_) => continue` → `Err(e) => { eprintln!("WARN retriever: chunk {chunk_id} ... (desync), skipping: {e}"); continue; }`（skip 行为保留）。+ 同源 `test_35_1_2_retriever_desync_skip_guard`（删 chunks 行造 desync → search 优雅跳过返回，非 fail-fast）。
- 0 新 dep / 0 proto / 0 schema migration / 0 network / 默认行为 + 既有 best-effort 契约不变（observability-only，ADR-004/008）。ADR-040 D1 ratify 依据（@ task-35.3 closeout）。
