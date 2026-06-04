# Task `35.2`: `go-silent-failure-surfacing — setVectorEnv config.Load 错误（main.go:297）+ os.Setenv 失败（main.go:308）静默吞掉 → fmt.Fprintf(os.Stderr) WARN 显式化（镜像 daemon/rest.go:110 audit best-effort stderr 惯例）；memstore.go:579 emitMemoryEvent nil-sink 退化 🟡 待实施期 impl-grounding（若 MemMemoryStore 接入 production sink-expected 路径则一次性 sync.Once 退化告警，否则据实记 honest non-issue）；observability-only（best-effort env-only 路径失败时不变 / 不阻断热路径 / 不转 fail-fast，ADR-004）；0 新 dep（ADR-008）/ 0 proto / 0 migration`

**Status**: Done

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 35 (observability-hardening)
**Dependencies**: 既有 `cmd/contextforge/main.go:280-317` `setVectorEnv`（task-34.2 / ADR-039 D2 交付的跨进程 env-bridge——best-effort `config.Load(dataDir)` → 逐变量 `os.LookupEnv` env-wins 守卫 + `os.Setenv` + restore 闭包；本 task 在其 `config.Load` 错误分支 `:297-300` + `os.Setenv` 失败分支 `:308-310` 加 stderr 显式化）/ 既有 `internal/daemon/rest.go:101-122` `authMiddleware`（`audit.Write` best-effort non-blocking surfacing 范式——`if err := audit.Write(...); err != nil { fmt.Fprintf(os.Stderr, "contextforge audit: %v\n", err) }`，本 task 镜像此惯例）/ 既有 `internal/consoleapi/memstore.go:576-583` `emitMemoryEvent`（best-effort 观测 emit，`s.emit == nil` 时 no-op；本 task 🟡 待实施期 ground 其是否接入 production sink-expected 路径）/ 既有 `internal/config/config.go` `config.Load`（task-1.2 手写 TOML codec，malformed/unreadable config.toml 返回 error）/ ADR-040（observability-hardening；本 task = 其 §D2 原文实现）/ ADR-036（governance-debt-cleanup；task-31.3 确立的 Go `fmt.Fprintf(os.Stderr, "mcp: ..."）` audit-surfacing 范式，本 task 同向镜像）/ ADR-031（observability-hardening 母 ADR，v0.19.0；本 task 承其 stderr/best-effort surfacing 方向，add-only）/ ADR-039（vector-config-completeness；`setVectorEnv` 即其 task-34.2 落点）/ ADR-004（local-first-privacy-baseline，默认行为 + 既有 best-effort 契约不变；surfacing 不把 best-effort 转 fail-fast）/ ADR-008（dep add-only，Phase 35 = 0 新 dep）/ ADR-013（禁伪造红线——memstore nil-sink 🟡 据实标 impl-grounding 未定论，不预断为「已交付」；stderr-capture RED→GREEN 为真实可断言测试）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D5（第二十六次激活）

## 1. Background

Phase 35（observability-hardening，承 Phase 31 / Phase 33 治理债血脉）的主题是**把热路径中被静默吞掉的真实错误显式化**（surface genuinely-swallowed errors），镜像仓库既有 stderr 惯例（Rust `eprintln!` / Go `fmt.Fprintf(os.Stderr)`），0 新 dep、0 network、默认行为 + 既有 best-effort 契约不变（observability-only，不阻断热路径/RPC happy-path，**不**把 best-effort 转 fail-fast）。本 task 是该主题的 Go 侧落点（task-35.1 是 Rust 侧落点）：

- **B1 `setVectorEnv` 把 `config.Load` 错误静默吞掉（`main.go:297`）**：`cmd/contextforge/main.go:287-317` 的 `setVectorEnv`（task-34.2 交付）best-effort 加载 `config.toml` 并把 `[vector]` 段桥接为 `CONTEXTFORGE_VECTOR_BACKEND` / `CONTEXTFORGE_VECTOR_DIM`。其 `:297-300` 段 `cfg, err := config.Load(dataDir); if err != nil { return restore }` ——当 `config.toml` malformed（手写 TOML codec 解析失败）/ unreadable（权限/IO 错误）时**静默返回 restore（不导出任何变量）**，用户得不到任何提示：他们写的 `[vector]` 配置被悄悄忽略、core daemon 退回 BruteForce 默认，而无人知道是配置文件坏了。这是一个真实的可观测性缺口（degraded silently）。
- **B2 `os.Setenv` 失败被静默丢弃（`main.go:308`）**：同一 helper 的 `:301-311` `setIfAbsent` 闭包里，`if os.Setenv(key, val) == nil { restores = append(...) }` ——仅在 `os.Setenv` **成功**时记录 restore；`os.Setenv` 失败（极少见，如非法 key/value）则被静默丢弃，既不导出、也不提示。虽然 `os.Setenv` 失败在常规路径几乎不发生，但「成功才记录、失败无声」与 surface-silent-failures 主题相悖，可顺带显式化为 WARN。
- **B3 仓库已证的 Go best-effort stderr surfacing 范式（`daemon/rest.go:110`）**：`internal/daemon/rest.go:101-122` `authMiddleware` 的 `audit.Write` 即 best-effort、non-blocking、surface-to-stderr 范式——`if err := audit.Write(...); err != nil { fmt.Fprintf(os.Stderr, "contextforge audit: %v\n", err) }`（PR #44 review FIX-2：「audit chain break 是 v0.1 operability signal，对响应保持 non-blocking，但 surface 到 stderr 让 ops 能注意到」）。task-31.3（ADR-036）进一步在 `internal/mcpadapter/server.go:298` 确立同向 `fmt.Fprintf(os.Stderr, "mcp: audit write failed ...")` 惯例。本 task **复用同一已证范式**——给 `setVectorEnv` 的两处静默分支加 `fmt.Fprintf(os.Stderr, ...)`，severity 以消息字符串前缀承载（无 severity 框架），不引入任何新 logging/metrics facility（simplicity-first + ADR-004/008）。
- **B4 `emitMemoryEvent` nil-sink 退化（`memstore.go:579`）🟡 待实施期 impl-grounding**：`internal/consoleapi/memstore.go:576-583` 的 `emitMemoryEvent` 是 best-effort 观测 emit，`s.emit == nil` 时 no-op（degraded observability when no sink wired）。这是 BORDERLINE 站点：它**可能**是 by-design（`MemMemoryStore` 是 fallback/test-double，其 sink 经 `SetEventSink` 可选接线，`CONSOLE_API_FALLBACK_INMEM=1` 才挂 `MemStore.EmitEvent`）。本 task 须在**实施期**（at implementation time）据实判断 `MemMemoryStore` 是否被接入某条 production 路径、且该路径期望 sink 已挂；**若是** ⇒ 加一次性（`sync.Once`）退化告警（首次因 nil sink 丢弃事件时 WARN 一行，0-dep、不刷屏 non-noisy）；**若它按设计仅作 fallback/test-double（sink 可选）** ⇒ 据实记为 honest non-issue 不改码 [SPEC-DEFER:phase-future.memstore-degraded-observability-warn]。本 task §spec 把它标 🟡 pending impl-grounding，**不**预断为「一定交付」（ADR-013 禁伪造）。

本 task 为 code-local 🟢 可单测（`setVectorEnv` 的 stderr-capture RED→GREEN：写 malformed config.toml → 断言 WARN 行出现在捕获的 stderr；valid/missing config → 断言无 WARN；env-wins + restore 行为不退化），0 新 dep（沿用 `fmt`/`os` 标准库 + 既有 `daemon/rest.go` stderr 惯例）、0 proto、0 migration；既有 best-effort 契约不变（env-only 路径在失败时行为不变，不阻断 daemon 启动）。

## 2. Goal

(1) **B1**：在 `setVectorEnv` 的 `config.Load` 错误分支（`main.go:297-300`）于 `return restore` 前加 `fmt.Fprintf(os.Stderr, "contextforge: vector config load failed (%s): %v\n", dataDir, err)`——把 malformed/unreadable `config.toml` 显式化为 stderr WARN（镜像 `daemon/rest.go:110` 的 `contextforge audit: %v` 前缀惯例）。**保持 best-effort**：surface 后仍 `return restore`（env-only 路径不变，不阻断 daemon 启动）——observability only。(2) **B2**：在 `setIfAbsent` 的 `os.Setenv` 失败分支（`main.go:308`）把「成功才记录」补成「失败则 WARN」——`os.Setenv` 返回非 nil err 时 `fmt.Fprintf(os.Stderr, ...)` 显式化（顺带项，best-effort 不变）。(3) **B4 🟡**：实施期据实 ground `MemMemoryStore.emitMemoryEvent` nil-sink（`memstore.go:579`）是否在 production sink-expected 路径——若是加一次性（`sync.Once`）退化 WARN，否则据实记 honest non-issue [SPEC-DEFER:phase-future.memstore-degraded-observability-warn]（不预断交付）。(4) **observability-only / 0-dep**：surfacing **不**把 best-effort 转 fail-fast（best-effort 仍 best-effort，daemon 启动不被阻断）；0 新 dep（`fmt`/`os` 标准库）/ 0 proto / 0 migration；既有 4 门（cargo-test/go-test/lint/spec-lint）不退化。

pass bar：`setVectorEnv` malformed `config.toml` ⇒ WARN 行出现在捕获的 stderr（stderr-capture RED→GREEN）、valid/missing `config.toml` ⇒ 无 WARN、env-wins + restore 行为不退化（扩展既有 `TestSetVectorEnv`，TEST-34.2.2 不退化）（🟢）；`os.Setenv` 失败分支 WARN 显式化（best-effort 不变）；`emitMemoryEvent` nil-sink 🟡 实施期据实决断（production-wired ⇒ 一次性 WARN / 设计 fallback-only ⇒ honest non-issue，不预断）；surfacing 不把 best-effort 转 fail-fast（daemon 启动不被阻断，env-only 失败路径行为不变，ADR-004）+ 既有契约（`setVectorEnv` 签名 / `config.Load` / `setDataDirEnv` / `daemon.Start`）不变；0 新 dep（ADR-008）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `cmd/contextforge/main.go` `setVectorEnv`（`:287-317`）——在 `config.Load` 错误分支（`:297-300`）于 `return restore` 前加 `fmt.Fprintf(os.Stderr, "contextforge: vector config load failed (%s): %v\n", dataDir, err)`（镜像 `daemon/rest.go:110` 前缀惯例）；在 `setIfAbsent`（`:301-311`）的 `os.Setenv` 分支把「成功才记录」补成「失败则 `fmt.Fprintf(os.Stderr, ...)` WARN」。两处均**保持 best-effort**（surface 后仍走原 return / 原流程，不阻断 daemon 启动）。
- import `fmt`（若 `main.go` 尚未 import；既有 `os` / `strconv` / `internal/config` 已在 task-34.2 引入）。
- 🟡 实施期据实 ground `internal/consoleapi/memstore.go:576-583` `emitMemoryEvent` nil-sink（`:579`）：判断 `MemMemoryStore` 是否被接入 production sink-expected 路径——若是加一次性（`sync.Once`）退化 WARN（`fmt.Fprintf(os.Stderr, ...)`，首次 nil-sink 丢弃事件时一行，non-noisy 0-dep）；若按设计仅 fallback/test-double（sink 可选）则据实记 honest non-issue 不改码 [SPEC-DEFER:phase-future.memstore-degraded-observability-warn]。
- 同源测试：`cmd/contextforge` 同包 test——扩展既有 `TestSetVectorEnv`（task-34.2）：写 malformed `config.toml` 到临时 dataDir → 经 `os.Pipe` 重定向捕获 `os.Stderr` → 断言 WARN 行出现；valid/missing config → 断言无 WARN；env-wins + restore 行为不退化（TEST-35.2.1）。memstore nil-sink 🟡 若实施期判定 production-wired 才在 `internal/consoleapi` 同包 test 加 nil-sink 退化 WARN 断言；若判 honest non-issue 则不加测试（据实，不为非 issue 造测试）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 为可观测性引入结构化 metrics/counter facility（core 无 metrics facility，stderr surfacing 是忠于「make-silent-failures-explicit」的范围）[SPEC-DEFER:phase-future.observability-metrics-facility]——加 facility 破 simplicity-first（ADR-004/008）+ 需暴露路径（proto/health）= scope creep，本 task 仅 stderr surfacing。
- `memstore.go:579` `emitMemoryEvent` nil-sink 一次性退化 WARN——**若**实施期 grounding 显示 `MemMemoryStore` sink 按设计可选（fallback/test-double-only）[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]，则据实记 honest non-issue 不改码（不预断交付，ADR-013）。
- 把 best-effort 路径转为 fail-fast / 返回 error（让 `setVectorEnv` config-load 失败阻断 daemon 启动）——破既有契约（`[vector]` file source 是 opt-in，load 失败非致命，ADR-004）[SPEC-DEFER:phase-future.vector-config-fail-fast]；本 task surfacing 仅 observability，best-effort 仍 best-effort。
- ADR-040 §D3 grounding correction 里被 DROP 的已显式化 / 设计意图站点（`internal/mcpadapter/server.go:298` 已经 task-31.3 surface / `internal/mcpadapter/allowlist.go:31` 的 POSIX-only 平台 caveat——Windows perm bits 不具语义、令其有义须引 `golang.org/x/sys/windows` = 新 dep 破 0-dep）均**不**返工（grounding correction，ADR-013）[SPEC-DEFER:phase-future.go-dropped-sites-no-rework]。
- 真实 release tag / run-id / digest（v0.28.0）[SPEC-OWNER:task-35.3-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `setVectorEnv`（`cmd/contextforge/main.go:287-317`，task-34.2 交付；本 task 在其 `config.Load` 错误分支 `:297` + `os.Setenv` 失败分支 `:308` 加 stderr WARN）
- `setIfAbsent`（`setVectorEnv` 内闭包 `:301-311`，本 task 在其 `os.Setenv` 分支加失败 WARN）
- `config.Load`（`internal/config/config.go`，malformed/unreadable `config.toml` 返回 error——本 task surface 其 error，不改其行为）
- `audit.Write` + `fmt.Fprintf(os.Stderr, "contextforge audit: %v\n", err)`（`internal/daemon/rest.go:101-122`，已证 best-effort stderr surfacing 范式，本 task 镜像，不改）
- `MemMemoryStore.emitMemoryEvent`（`internal/consoleapi/memstore.go:576-583`，nil-sink no-op；本 task 🟡 实施期据实 ground 是否 production-wired，不预断改码）
- 运维 / 部署者（写了 malformed `config.toml [vector]` 时，经 stderr WARN 能注意到配置被忽略 / core 退回 BruteForce，而非静默降级）
- spawned core daemon（`internal/daemon/daemon.go` `launch`——本 task 不改；surfacing 不阻断 daemon 启动，env-only 路径行为不变）

## 5. Behavior Contract

### 5.1 Required Reading

- `cmd/contextforge/main.go:280-317`（`setVectorEnv`——`:297-300` `cfg, err := config.Load(dataDir); if err != nil { return restore }` 静默吞 config.Load 错误是 B1 落点；`:301-311` `setIfAbsent` 的 `if os.Setenv(key, val) == nil { restores = append(...) }` 成功才记录、失败无声是 B2 落点；doc 注 `:283-286` 已记 env-wins / best-effort / config-load 错误非致命）
- `internal/daemon/rest.go:101-122`（`authMiddleware`——`:106-111` `if err := audit.Write(...); err != nil { fmt.Fprintf(os.Stderr, "contextforge audit: %v\n", err) }` 是本 task 镜像的已证 best-effort stderr surfacing 范式 + PR #44 review FIX-2 注释解释「non-blocking 但 surface 到 stderr 让 ops 注意」）
- `internal/consoleapi/memstore.go:570-583`（`SetEventSink` `:572-574` 经 `CONSOLE_API_FALLBACK_INMEM=1` 挂 `MemStore.EmitEvent` + `emitMemoryEvent` `:576-583` `if s.emit != nil` no-op——B4 🟡 grounding 站点；判断 `MemMemoryStore` 是否在 production sink-expected 路径）
- `internal/config/config.go` `config.Load`（malformed/unreadable `config.toml` 返回 error——本 task surface 其 error 到 stderr，不改其 error 语义）
- `internal/mcpadapter/server.go:298`（`writeAudit` `fmt.Fprintf(os.Stderr, "mcp: audit write failed ...")`，task-31.3 / ADR-036 D3 已 surface——同向惯例，本 task 不返工，grounding correction）+ `internal/mcpadapter/allowlist.go:31`（POSIX-only 平台 caveat，Windows perm bits 不具语义——intentional，不返工）
- `docs/decisions/adr-040-observability-hardening.md §D2`（本 task 即其原文实现）+ `§D3`（grounding correction：已显式化 / 设计意图站点 DROP）+ ADR-031（observability-hardening 母 ADR）/ ADR-036（Go stderr audit-surfacing 范式来源）/ ADR-004（best-effort 不转 fail-fast）/ ADR-008（0 新 dep）/ ADR-013（memstore nil-sink 🟡 据实标，不伪造）

### 5.2 关键设计 — setVectorEnv 静默错误 stderr 显式化（best-effort 不变 / 镜像 daemon/rest.go:110 / 0-dep）

- **B1 `config.Load` 错误 surface（镜像 `daemon/rest.go:110`）**：`setVectorEnv` 的 `:297-300` 分支改为——
  ```go
  cfg, err := config.Load(dataDir)
  if err != nil {
      fmt.Fprintf(os.Stderr, "contextforge: vector config load failed (%s): %v\n", dataDir, err)
      return restore // best-effort: malformed/unreadable config → env-only path unchanged
  }
  ```
  前缀 `contextforge: vector config load failed` 镜像 `daemon/rest.go:110` 的 `contextforge audit: %v` 形态（severity 以消息前缀承载，无 severity 框架）。surface 后仍 `return restore`——**best-effort 不变**（env-only 路径不变，不阻断 daemon 启动）；malformed/missing config ⇒ 不导出任何变量 ⇒ 两环境变量 unset ⇒ core `resolve_vector_backend` 收 `("", 0)` ⇒ BruteForce 字节等价（ADR-004，default 行为不变）。**注**：`config.Load` 对「文件不存在」与「文件 malformed」的区分由 `config.Load` 现行语义裁决——若 missing 文件不返 error（仅 malformed 返 error）则只有 malformed 触发 WARN；测试据 `config.Load` 实际语义断言（TEST-35.2.1 含 valid/missing → 无 WARN 断言）。
- **B2 `os.Setenv` 失败 surface（顺带项）**：`setIfAbsent`（`:301-311`）的 `os.Setenv` 分支由「成功才记录」补成「失败则 WARN」——
  ```go
  if err := os.Setenv(key, val); err != nil {
      fmt.Fprintf(os.Stderr, "contextforge: vector env setenv failed (%s): %v\n", key, err)
  } else {
      restores = append(restores, func() { _ = os.Unsetenv(key) })
  }
  ```
  仅显式化既有静默失败分支（restore 记录语义不变：成功才记 restore；失败 surface 后不记 restore），best-effort 不变。
- **B3 best-effort 不转 fail-fast（observability-only）**：两处 surface 均**不**改控制流为返回 error / 阻断 daemon 启动——`setVectorEnv` 仍返回 `func()`（restore 闭包），签名不变；`doServe` / `doMCP` 既有接线（`daemon.Start` 前调 `setVectorEnv`）行为不变（config 坏了 ⇒ WARN + 退回 env-only/BruteForce，daemon 照常起）。这是忠于「surface silent failures」而**不**破既有 best-effort 契约（ADR-004）的实现（MUST NOT turn best-effort into fail-fast）。
- **B4 `emitMemoryEvent` nil-sink 🟡 待实施期 impl-grounding**：实施期据实判断 `MemMemoryStore`（`memstore.go`）是否被接入某条 production 路径且该路径期望 sink 已挂——
  - **若 production sink-expected**：加一次性 `sync.Once` 退化告警——首次因 `s.emit == nil` 丢弃事件时 `fmt.Fprintf(os.Stderr, "contextforge: memory event dropped (no observability sink wired)\n")` 一行（`sync.Once` 保证不刷屏，0-dep non-noisy），其后静默（避免每次 emit 刷 stderr）。
  - **若按设计仅 fallback/test-double（sink 经 `SetEventSink` 可选，`CONSOLE_API_FALLBACK_INMEM=1` 才挂）**：据实记为 honest non-issue 不改码 [SPEC-DEFER:phase-future.memstore-degraded-observability-warn]——nil-sink no-op 是设计意图（fallback demo 模式不要求观测 sink），surface 它反成噪音。
  - **据实不预断（ADR-013）**：本 spec 把 B4 标 🟡 pending impl-grounding，**不**预先断言「已交付一次性 WARN」；实施期判定结果（哪条分支）在 task-35.3 closeout / §10 据实回填，并在 ADR-040 §D2 据实记录。
- **0 新 dep / 0 proto / 0 migration（ADR-008）**：仅用 `fmt`/`os` 标准库 + 既有 `daemon/rest.go` stderr 惯例 + （B4 若取一次性 WARN）`sync` 标准库；无第三方依赖增量、无 proto 改动、无 migration、无新 logging/metrics facility（severity 以消息前缀承载）。

### 5.3 不变量

- best-effort env-only 路径失败时不变（ADR-004）：`config.Load` 失败 ⇒ surface WARN 后仍 `return restore`（不导出变量 ⇒ env-only 路径不变 ⇒ unset ⇒ core 退回 BruteForce 字节等价）；`os.Setenv` 失败 ⇒ surface WARN 后不记 restore（既有「成功才记录」语义保持）；surfacing **不**把 best-effort 转 fail-fast——daemon 启动不被阻断，`setVectorEnv` 签名 / 控制流（返回 `func()` restore 闭包）不变（observability-only）。
- 既有契约不变：`setVectorEnv` 签名（`func setVectorEnv(dataDir string) func()`）/ env-wins 语义（显式 env 不被 config 覆盖，TEST-34.2.2 不退化）/ restore 闭包语义（成功 set 才记 restore）/ `config.Load` error 语义（本 task 只 surface 其 error、不改其返回）/ `setDataDirEnv` / `daemon.Start` 接线均不变（add-only stderr WARN 行）。
- observability-only（不阻断热路径 / RPC happy-path）：两处 surface 均为 stderr 旁路输出，不改任何返回值 / 控制流 / RPC 行为；core daemon 启动、`[vector]` env-bridge 拾取、Console memory ops（B4 若取一次性 WARN 也仅旁路一行）happy-path 不变（MUST NOT turn best-effort into fail-fast）。
- 0 新代码依赖（ADR-008）：Go 侧沿用 `fmt`/`os`（+ B4 若取 `sync.Once` 则 `sync`）标准库，无第三方依赖增量；0 proto / 0 migration / 无新 logging/metrics facility（severity 以消息字符串前缀承载，镜像 `daemon/rest.go:110` / `mcpadapter/server.go:298` 既有惯例）。
- memstore nil-sink 据实（ADR-013）：B4 标 🟡 pending impl-grounding，实施期据实判 production-wired（加一次性 WARN）/ 设计 fallback-only（honest non-issue [SPEC-DEFER:phase-future.memstore-degraded-observability-warn]），**不**预断交付、**不**为 honest non-issue 造测试或伪造 stderr 断言。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（`setVectorEnv` 静默错误 stderr 显式化 + best-effort 不变 🟢）: `config.Load` 错误分支（`main.go:297`）加 `fmt.Fprintf(os.Stderr, "contextforge: vector config load failed (%s): %v\n", dataDir, err)`（镜像 `daemon/rest.go:110`），surface 后仍 `return restore`（best-effort 不变，daemon 不阻断）；`os.Setenv` 失败分支（`main.go:308`）补 WARN 显式化（best-effort 不变）；malformed `config.toml` ⇒ 经 `os.Pipe` 捕获的 stderr 含 WARN 行；valid/missing config ⇒ 无 WARN；env-wins + restore 行为不退化（扩展既有 `TestSetVectorEnv`，TEST-34.2.2 不退化）；memstore nil-sink（`memstore.go:579`）🟡 实施期据实决断（production-wired ⇒ 一次性 `sync.Once` WARN 并加断言 / 设计 fallback-only ⇒ honest non-issue 不改码不造测试，[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]，不预断）；surfacing 不转 fail-fast（既有契约不变）；0 新 dep — verified by **TEST-35.2.1**（setVectorEnv malformed-config WARN via stderr-capture + valid/empty no-WARN + env-wins preserved）
- [x] **AC2**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-35.2.2**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-35.2.1 | `setVectorEnv` malformed-config WARN（stderr-capture RED→GREEN）：写 malformed `config.toml` 到临时 dataDir → 经 `os.Pipe` 重定向捕获 `os.Stderr` → 断言 `contextforge: vector config load failed` WARN 行出现；valid/missing config → 断言无 WARN；env-wins（显式 env 不被覆盖）+ restore 闭包恢复 不退化（扩展既有 `TestSetVectorEnv`，task-34.2）。memstore nil-sink 🟡 若实施期判 production-wired 才在 `internal/consoleapi` 同包 test 加一次性 WARN 断言；若判 honest non-issue 则不加（据实，不为非 issue 造测试） | `cmd/contextforge/main_test.go`（同包 test，扩展 `TestSetVectorEnv`）（+ 🟡 `internal/consoleapi/memstore_test.go` 条件性） | Done |
| TEST-35.2.2 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）surface 被误实现为 fail-fast（破 best-effort 契约）**：在 `config.Load` 错误分支加 surface 时若顺手改成返回 error / 阻断 daemon 启动，会破既有「`[vector]` file source opt-in、load 失败非致命」契约（ADR-004），令 malformed config 阻断 daemon。
  - **缓解**：surface 行紧贴 `return restore` 之前（add-only WARN，控制流不变）；`setVectorEnv` 签名（返回 `func()`）+ `doServe`/`doMCP` 接线不变；TEST-35.2.1 断言 malformed config 下 `setVectorEnv` 仍正常返回 restore（best-effort 保持）。stop-condition：若 surface 改了控制流 / 阻断启动则 AC1 不标 `[x]`。
- **R2（中）stderr-capture 测试改进程全局 `os.Stderr` 致并行测试串扰**：经 `os.Pipe` 重定向 `os.Stderr` 捕获 WARN 改进程全局 stderr，并行测试可能相互干扰。
  - **缓解**：测试用「保存 `os.Stderr` → 重定向到 pipe → 调 `setVectorEnv` → 读 pipe → defer 恢复 `os.Stderr`」惯例 + 不 `t.Parallel`（与既有 `TestSetVectorEnv` 同惯例，env 用 `t.Setenv` 自动恢复）；断言后恢复。
- **R3（中）memstore nil-sink 🟡 被预断为「已交付」（伪造）**：B4 是 BORDERLINE 站点，若在实施前就断言「已加一次性 WARN」会违 ADR-013（grounding 未做先下结论）。
  - **缓解**：spec §1 B4 / §2(3) / §5.2 B4 / §5.3 / §7 据实标 🟡 pending impl-grounding，明记「实施期据实判 production-wired（加 WARN）/ 设计 fallback-only（honest non-issue [SPEC-DEFER:phase-future.memstore-degraded-observability-warn]）」，不预断；实施结果在 §10 / task-35.3 据实回填，不为 honest non-issue 造测试或伪造 stderr 断言。
- **R4（低）surface 文案/前缀与仓库既有惯例不一致**：WARN 前缀若与 `daemon/rest.go:110` 的 `contextforge ...: %v` 形态不一致会增加 ops 解析负担、降低惯例一致性。
  - **缓解**：前缀镜像 `contextforge audit: %v`（`daemon/rest.go:110`）/ `mcp: audit write failed ...`（`mcpadapter/server.go:298`）形态——`contextforge: vector config load failed (%s): %v`（severity 以前缀承载，无 severity 框架）；§5.2 B1/B2 固化文案形态，TEST-35.2.1 断言前缀子串出现。
- **R5（低）被误读为引入新 logging/metrics facility**：surface 易被误读为新建观测装配，而非复用既有 stderr 惯例。
  - **缓解**：spec §1 B3 / §5.2 / §5.3 据实记「仅 `fmt.Fprintf(os.Stderr)` 镜像 `daemon/rest.go:110` / `mcpadapter/server.go:298` 既有惯例，无新 logging/metrics facility，0 新 dep」；结构化 metrics/counter facility 明 [SPEC-DEFER:phase-future.observability-metrics-facility]（ADR-040 §A1 REJECTED：core 无 metrics facility，加之破 simplicity-first + 需暴露路径 = scope creep）。

## 9. Verification Plan

```bash
# 1. AC1 — setVectorEnv malformed-config WARN（stderr-capture RED→GREEN）+ valid/empty no-WARN + env-wins 不退化
go test ./cmd/contextforge/ -run TestSetVectorEnv

# 2. AC1 — memstore nil-sink 🟡（仅当实施期判 production-wired 才有此测试；判 honest non-issue 则无）
go test ./internal/consoleapi/...

# 3. 不退化（全量 Go；surfacing observability-only 不改控制流，既有契约不变）
go test ./...
go vet ./cmd/... ./internal/consoleapi/...

# 4. 0 新 dep 确认（Go go.mod 无依赖增量）
git diff --stat go.mod go.sum   # 期望无变化（仅 fmt/os/sync 标准库）

# 5. AC2 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.go-silent-failure-defer-note]：本 task 仅交付 `setVectorEnv` 两处静默失败的 stderr 显式化（`config.Load` 错误 `main.go:297` + `os.Setenv` 失败 `main.go:308`，镜像 `daemon/rest.go:110` best-effort stderr 惯例，🟢 stderr-capture 可单测）+ memstore nil-sink 🟡 实施期据实决断（不预断）；结构化 metrics/counter facility [SPEC-DEFER:phase-future.observability-metrics-facility]、memstore nil-sink 一次性退化 WARN（若 grounding 显示 sink 设计可选则记 honest non-issue）[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]、把 best-effort 转 fail-fast [SPEC-DEFER:phase-future.vector-config-fail-fast]、ADR-040 §D3 grounding correction 里被 DROP 的已显式化/设计意图站点（`mcpadapter/server.go:298` 已 surface / `allowlist.go:31` POSIX-only Windows-ACL 平台 caveat）返工 [SPEC-DEFER:phase-future.go-dropped-sites-no-rework] 均不在本 task 范围。surfacing observability-only（best-effort 仍 best-effort，不转 fail-fast，daemon 启动不被阻断，ADR-004），0 新 dep（ADR-008）/ 0 proto / 0 migration；真实 release tag / run-id / digest（v0.28.0）[SPEC-OWNER:task-35.3-closeout] 实施授权后回填。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification 实证**（real evidence，本地全绿）：
- AC1：`go test ./cmd/contextforge/ -run TestSetVectorEnv -v` → 6/6 PASS（既有 `TestSetVectorEnv` 3 子测试 + 新 `TestSetVectorEnv_LoadErrorSurfacing` 3 子测试：malformed→WARN（stderr-capture 真 RED→GREEN）/ missing→no WARN（os.ErrNotExist 守护）/ valid→no WARN）。`go test ./...` 全过（无回归）；`go vet ./cmd/...` clean。
- AC2：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- gofmt：本机 `gofmt -l` 标 main.go/main_test.go = **纯 Windows autocrlf CRLF**（`gofmt -d` 整文件均匀 \r 差异、无任何缩进修正）；git 存 LF、CI（LF）gofmt 通过（以 CI/LF 为准）。

**grounding 校正（实施期，ADR-013）**：
- **config.Load 对 MISSING config.toml 也返 error**（`os.Open` 失败）→ 朴素 surface 会对「无配置」这一**常见默认**误报 WARN（噪声/UX 回归）。据实加 `errors.Is(err, os.ErrNotExist)` 守护：**仅** malformed/unreadable 报警，missing 静默（TEST-35.2.1 `missing→no WARN` 子测试守护）。
- **memstore nil-sink（`memstore.go:579`）= honest non-issue（DROP，不改码）**：grounding 复核 `NewMemMemoryStore()` 唯一**生产**调用点 `internal/cli/console_api_serve.go:109`，紧随**无条件** `:112 SetEventSink(store.EmitEvent)`——生产 fallback 路径 sink 总是接线；nil-sink 仅出现在测试/直接构造（设计内可选）。加一次性 degraded WARN 只会在测试触发=噪声无生产价值 → 据实记 honest non-issue 不改码 `[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]`（🟡 borderline 经 grounding 解析为 by-design）。
- **`os.Setenv` 失败 surface（顺带项）**：B2 在既有「成功才记 restore」分支补「失败则 WARN」，restore 记录语义不变。

**实际改动文件**：
- `cmd/contextforge/main.go`——`setVectorEnv`：`config.Load` 错误分支 add-only `if !errors.Is(err, os.ErrNotExist) { fmt.Fprintf(os.Stderr, "contextforge: vector config load failed (%s): %v\n", dataDir, err) }`（missing 静默、malformed 报警、best-effort `return restore` 不变）+ `os.Setenv` 失败分支 `if err := os.Setenv(...); err != nil { fmt.Fprintf(os.Stderr, ...) } else { restores = append(...) }`；import `errors`。
- `cmd/contextforge/main_test.go`——`captureStderr(t, fn)` helper（os.Pipe 重定向）+ `TestSetVectorEnv_LoadErrorSurfacing`（malformed→WARN / missing→no WARN / valid→no WARN）。
- `internal/consoleapi/memstore.go`——**0 改动**（nil-sink honest non-issue，grounding 校正）。
- 0 新 dep（`fmt`/`os`/`errors` 标准库）/ 0 proto / 0 migration / 默认行为 + 既有 best-effort 契约不变（env-only 路径失败时不变、env-wins/restore 保持，observability-only，ADR-004/008）。ADR-040 D2 ratify 依据（@ task-35.3 closeout）。
