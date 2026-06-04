# language: en
# Maps to:
#   - docs/specs/phases/phase-35-observability-hardening.md
#   - docs/specs/tasks/task-35.1-rust-silent-failure-surfacing.md
#   - docs/specs/tasks/task-35.2-go-silent-failure-surfacing.md
#   - docs/specs/tasks/task-35.3-closeout-v0.28.0.md
#
# 轻量 BDD（s2v §9.2）；Phase 35 observability-hardening（承 Phase 31/33 治理债血脉，把热路径中被静默吞掉的真实错误显式化——刻意小版本，第三轮债清理性质、边际递减，诚实优先于凑数，ADR-013 + ADR-040 + ADR-031 母 ADR add-only）。Scenario ID 在各 task spec §7 追踪表映射到测试 / 真实 run。
# Status: Draft（规划稿，本文件随 phase/task spec 一并 Draft）。
# 既有 best-effort / RPC happy-path 契约不变（surfacing 仅 observability，绝不把 best-effort 改成 fail-fast）；0 new dep / 0 网络 / 0 proto / 0 migration（ADR-004/008）。镜像仓库既有 stderr 惯例（Rust eprintln! / Go fmt.Fprintf(os.Stderr)），不引入任何新日志 / metrics 框架。
# 受阻 / 据实非问题维度均以 [SPEC-DEFER:phase-future.<name>] 标注（结构化 metrics/counter facility core 无、stderr surfacing 即忠实 scope；memstore nil-sink 若 grounding 显示 sink 本就 optional-by-design 则记诚实非问题），据真实测试回填，绝不预填 stderr 断言或 release 数值（ADR-013）。

Feature: phase-35-observability-hardening
  In order to 让热路径中被 `let _ = ...` / `Err(_) => continue` 静默吞掉的真实错误显式化（磁盘满 / 锁 / 索引与 SQLite 失步），并据实记录 7→3-4 的诚实裁剪（已显式化 / 设计内有意 site 不返工），保持既有 best-effort 与 RPC happy-path 契约不变（ADR-013 的诚实价值 + ADR-040 + ADR-031 母 ADR）
  As Phase 35 内核（Rust index_session_backend / retriever desync 经 eprintln! WARN 显式化 + Go setVectorEnv 经 fmt.Fprintf(os.Stderr) WARN 显式化 + grounding-correction 据实裁剪 + v0.28.0 closeout）
  I want core/src/jobs/index_session_backend.rs:201 的 `let _ = store.append(...)`（吞 indexing-event SQLite 持久化真实错误）改为 `if let Err(e) = store.append(...) { eprintln!("WARN indexing-event persist failed (job=...): {e}"); }`（仍 best-effort，不阻断 indexing，镜像 search.rs:109）+ core/src/retriever/mod.rs:415 的 `Err(_) => continue`（索引命中 chunk 在 Tantivy 有但 SQLite 失步时静默跳过）改为先 `eprintln!("WARN retriever: chunk ... desync, skipping: {e}")` 再 continue（skip 行为不变，镜像 retriever 既有 eprintln! 惯例）+ Go cmd/contextforge/main.go:297 setVectorEnv 的 `config.Load` err（损坏/不可读 config.toml 被静默吞）经 `fmt.Fprintf(os.Stderr, "contextforge: vector config load failed (%s): %v\n", ...)` 显式化（env-only 路径在失败时不变，镜像 daemon/rest.go:110），main.go:308 os.Setenv 失败可选地 WARN + memstore.go:579 emitMemoryEvent nil-sink 在实施期 grounding 后据实定夺（🟡 production-wired → 一次性 sync.Once 降级 WARN；fallback/test-only by design → 据实裁剪记非问题），且默认构建仍 0 new dep / 0 网络 / 0 proto / 0 migration / 默认行为不变（ADR-004/008——surfacing 仅 observability，best-effort 保持 best-effort、indexing 不阻断、query 继续、热路径非阻塞），grounding-correction（search.rs:109 已显式 / server.go:298 task-31.3 已做 / allowlist.go:31 有意 POSIX-only 平台 caveat / eb.send:193 有意 no-subscribers）如实裁剪不返工不伪造（ADR-013）

  # ---
  # Maps to: docs/specs/tasks/task-35.1-rust-silent-failure-surfacing.md (TEST-35.1.1)
  Scenario: SCEN-35.1.1 — 对应 AC1（index_session_backend persist 失败经 eprintln! WARN 显式化，indexing 仍完成 = best-effort 保持）
    Given core/src/jobs/index_session_backend.rs:201 的 `let _ = store.append(&job_id_context, "indexing", evt.processed_files, evt.total_files, "")` 静默吞掉 indexing_event_store SQLite append 的真实错误（磁盘满 / 锁），无任何出口；而 core/src/data_plane/search.rs:108-113 已是 `if let Err(e) = ... { eprintln!("WARN search_persist.put failed (key={key}); hot cache still updated: {e}") }` 的 best-effort 显式化成熟范式（不 abort caller）；core/Cargo.toml 无 log / tracing / metrics dep（severity 仅消息前缀 WARN/INFO，无 severity 框架，atomics 仅存在于 test code）
    When  把该 site 改为 `if let Err(e) = store.append(...) { eprintln!("WARN indexing-event persist failed (job={job_id_context}): {e}"); }`（仍 best-effort，不阻断 indexing），并注入 append 返回 Err 的 IndexingEventStore test-double，跑一次完整 index session
    Then  index session 仍成功完成（best-effort 保持，新增的 error 分支被走到）+ 真实错误经 eprintln! WARN 显式化（按仓库惯例靠 inspection，绝不断言 Rust stderr 输出，ADR-013）+ best-effort 未被改成 fail-fast（indexing 不阻断）+ 0 new dep / 无新 metrics facility [SPEC-DEFER:phase-future.observability-metrics-facility]（TEST-35.1.1，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-35.1-rust-silent-failure-surfacing.md (TEST-35.1.2)
  Scenario: SCEN-35.1.2 — 对应 AC2（retriever desync 命中经 eprintln! WARN 显式化 + skip + query 返回其余命中 = 行为锁定）
    Given core/src/retriever/mod.rs:415 的 `let (file_path, content, indexed_at) = match row { Ok(t) => t, Err(_) => continue }` 在 Tantivy 索引与 SQLite chunks 表失步时静默跳过一条 search hit，吞掉真实错误；retriever/mod.rs 在 filter no-op 路径附近已有 eprintln! 惯例可镜像
    When  把该分支改为 `Err(e) => { eprintln!("WARN retriever: chunk {chunk_id} present in index but missing from SQLite (desync), skipping: {e}"); continue; }`（skip 行为不变），构造一个 chunk_id 在 Tantivy 索引存在但 SQLite chunks 表缺失的状态，调用 query()
    Then  失步命中被跳过 + query 返回其余有效命中且无 error（skip 行为锁定，behavior-lock guard）+ 真实失步错误经 eprintln! WARN 显式化（按仓库惯例靠 inspection，不断言 Rust stderr 输出，ADR-013）+ best-effort/继续语义未被改成 fail-fast + 0 new dep（TEST-35.1.2，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-35.2-go-silent-failure-surfacing.md (TEST-35.2.1)
  Scenario: SCEN-35.2.1 — 对应 AC3（setVectorEnv 损坏 config.toml 经 stderr WARN 显式化 + env-only 路径不变 = env-wins 保持，stderr-capture RED→GREEN）
    Given cmd/contextforge/main.go:297 的 setVectorEnv 中 `cfg, err := config.Load(dataDir); if err != nil { return restore }` 静默吞掉损坏 / 不可读 config.toml 的真实错误（main.go:308 的 `os.Setenv` 失败亦被静默丢弃，仅成功时记录 restore）；而 internal/daemon/rest.go:101-122 已是 audit.Write best-effort、non-blocking、经 `fmt.Fprintf(os.Stderr, "contextforge audit: %v")` 前缀显式化的范式；setVectorEnv 来自 task-34.2（ADR-039），既有 TestSetVectorEnv 可扩展
    When  在 return restore 前加 `fmt.Fprintf(os.Stderr, "contextforge: vector config load failed (%s): %v\n", dataDir, err)`（os.Setenv 失败可选地 WARN），并以 os.Pipe 重定向捕获 os.Stderr：往 temp dataDir 写一个 MALFORMED config.toml 调 setVectorEnv 断言 WARN 行出现；以 valid / missing config 断言无 WARN；并复核 env-wins + restore 行为（扩展 task-34.2 既有 TestSetVectorEnv）
    Then  损坏 config.toml → 捕获的 stderr 含 WARN 行（Go 可经 os.Pipe 重定向断言，本 phase 最强测试，genuine RED→GREEN）+ valid / missing config → 无 WARN + env-only 路径在 config.Load 失败时不变（已显式设置的 env 覆盖 config，env-wins 保持，向后兼容）+ best-effort 未被改成 fail-fast + 0 new dep（镜像 daemon/rest.go:110）（TEST-35.2.1，真实测试通过后回填）

  # ---
  # Maps to: docs/specs/tasks/task-35.2-go-silent-failure-surfacing.md (memstore nil-sink 🟡 impl-grounding，折入本 task 叙事，不预分配独立通过 TEST-ID)
  Scenario: SCEN-35.2.2 — 对应 AC4（memstore emitMemoryEvent nil-sink 🟡 实施期 grounding 后据实定夺，不预断言为已交付）
    Given internal/consoleapi/memstore.go:579 的 emitMemoryEvent 在 `s.emit == nil` 时为 no-op（无 sink 接线时降级 observability）；此为 BORDERLINE——可能是 MemMemoryStore 作为 fallback/test double 其 sink 本就 optional-by-design
    When  task-35.2 在实施期 ground MemMemoryStore 是否被接入到期望 sink 存在的 production 路径：IF 是 → 加一次性（sync.Once）降级 observability WARN（首次因 nil sink 丢事件时，0-dep、non-noisy）并在 memstore 测试断言；IF 设计上 fallback/test-only → 据实裁剪记诚实非问题
    Then  本项以 🟡 pending impl-grounding 记于 spec（不预断言为已交付，ADR-013）+ 若 production-wired → 一次性降级 WARN 落地并被断言；若 optional-by-design → 据实记非问题 [SPEC-DEFER:phase-future.memstore-degraded-observability-warn] + 无论哪个分支 0 new dep / best-effort 不变（折入 TEST-35.2.1 memstore 叙事，不预分配独立通过 TEST-ID，真实定夺后回填）

  # ---
  # Maps to: docs/specs/phases/phase-35-observability-hardening.md §2 + docs/decisions/adr-040-observability-hardening.md (D3)
  Scenario: SCEN-35.GC — grounding-correction（7→3-4 诚实裁剪：已显式化 / 设计内有意 site 不返工，ADR-013 本 phase 核心价值）
    Given 调研初列 7 个静默 site，逐一 grounding 后据实裁掉 4 个：core/src/data_plane/search.rs:109 已经 `eprintln!("WARN search_persist.put failed ...")` 显式化（唯一"缺口"是结构化 counter，但 core 无 metrics facility，加一个 = 过度工程）/ internal/mcpadapter/server.go:298 writeAudit 自 task-31.3（ADR-036 D3 nit 2）已 `fmt.Fprintf(os.Stderr, "mcp: audit write failed ...")` / internal/mcpadapter/allowlist.go:31 warning 已 fire 且 "POSIX-only" 是有意记录的平台事实（Windows perm bits 无意义，让 Windows ACL 有意义需 golang.org/x/sys/windows = new dep，破 0-dep）/ core/src/jobs/index_session_backend.rs:193 的 `let _ = eb.send(...)` 是有意（broadcast 无 subscriber 时 SendError 属 NORMAL 而非 failure，加 WARN 即 noise）
    When  对这 4 个 site 不做任何代码改动，将"已显式化 / 设计内有意"的裁剪如实记于 phase spec §2、ADR-040 Context + D3、与 task-35.3 grounding-correction（不引入任何新 metrics facility，仅 core eprintln! / Go Fprintf）
    Then  search.rs:109 already-surfaced + server.go:298 already-done(task-31.3) + allowlist.go:31 intentional POSIX-only 平台 caveat + eb.send:193 intentional no-subscribers 全部据实裁剪不返工（net-zero 代码）+ 7→3-4 诚实裁剪记于 ADR-040 D3（这是本 phase 的 ADR-013 诚实价值）+ A1 metrics facility / A2 fail-fast / A3 改裁剪 site 三 alternative 均 REJECTED（SCEN-35.GC 据实记录，不伪造 stderr 断言，真实 closeout 后回填）

  # ---
  # Maps to: docs/specs/tasks/task-35.3-closeout-v0.28.0.md (TEST-35.3.1)
  Scenario: SCEN-35.3.1 — 对应 AC5（默认行为 + 既有契约不变 + v0.28.0 closeout + grounding-correction 诚实）
    Given task-35.1 + task-35.2 全 Done（Rust index_session_backend / retriever desync 经 eprintln! WARN 显式化 + Go setVectorEnv 经 fmt.Fprintf(os.Stderr) WARN 显式化），current Phase 33/34 smoke v24[43/43]；ADR-040 据 D1-D4 须逐 D Proposed→Accepted（D1 rust-silent-failure-surfacing：append + desync-skip 经 eprintln! 镜像 search.rs:109，best-effort 保持，guard 测试，eb.send:193 as-is；D2 go-silent-failure-surfacing：setVectorEnv 经 fmt.Fprintf 镜像 daemon/rest.go:110，stderr-capture RED→GREEN，memstore nil-sink 🟡 impl-grounding；D3 grounding-correction 7→3-4 诚实裁剪，不引入 metrics facility；D4 默认 + 0-dep + 0-网络 + 既有契约不变 ADR-004/008）
    When  跑 scripts/console_smoke.sh banner v24→v25 + 新增 step → [44/44]（smoke_syntax_test.go TestTask353_SmokeV25ObservabilityHardeningStep 镜像 TestTask343，no-regression [37/37]..[43/43] 不溯改，staging dir cf-v27-cfg offset +2），产出 v0.28.0 release docs（docs/releases/v0.28.0-evidence.md + v0.28.0-artifacts.md + README v0.28 + RELEASE_NOTES v0.28.0，tag/run/digest 为 <backfill> 待回填 markers），ADR-040 逐 D Proposed→Accepted + ADR-031 母 ADR add-only Phase 35 Amendment（stderr/best-effort surfacing 方向延续，不溯改正文 ADR-014 D5）+ ADR-036/038/039 Related add-only + roadmap §3.17/§4 add-only + s2v-adapter add-only + phase §6 闭合
    Then  默认行为 / proto / 既有契约不变（ADR-004——surfacing 仅 observability，best-effort 保持 best-effort、indexing 不阻断、query 继续、热路径非阻塞，0 proto / 0 migration、core 仍无 metrics facility）+ smoke v25[44/44]（既有 step 不退化，denominators 不溯改 ADR-014 D5）+ ADR-040 逐 D 如实 ratify（memstore nil-sink 🟡 impl-grounding 据实定夺，metrics facility honest-defer，grounding-correction 7→3-4 据实记于 D3）+ ADR-014 D1-D5 第 26 次激活全通过（TEST-35.3.1 + 各 task LAST TEST TEST-35.1.3 / TEST-35.2.2 / TEST-35.3.2 = `bash scripts/spec_drift_lint.sh --touched origin/master` 0 unannotated hits，真实跑出后回填）
