# Phase 45 · v1.0-api-cli-freeze

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本 phase 是 **v1.0 收口冲刺的第一步**（承 ADR-050 v1.0 定义）：立 v1.0 锚点（ADR-050）+ API/CLI 冻结准备（D2 维度）。**项目从未立过 v1.0 锚点**（PRD/roadmap/README 四处查证一致，ADR-013 grounding）——PRD 的 P0 是 v0.1 的（早已满足且远超，recall@5/@10=1.0 超 PRD 北极星 75%/85%），PRD 里 "v1.0" 只在分发维度。本 phase 立 ADR-050 正式定义 v1.0（= 功能成熟度收口 + API/CLI 冻结 + 文档对齐 + GitHub Release 流程；**不含** multi-user/自动更新/arm64 native，推 v2.0），并交付 D2 API/CLI 冻结的两个 P0 阻塞项：(1) daemon REST 移除 2 个 501 未实装 端点（`POST /v1/import` + `POST /v1/eval/run`，§2A 决策 B 有意留下，console-api 已完整覆盖）+ 实装 `chunk_count`（真实 COUNT，非 placeholder 0）；(2) CLI 加 `--version`/`version` + 顶层 `--help`（修复 `-h` 落 unknown subcommand exit 2）+ example.toml 补全 4 个检索 section。code-local 🟢 可单测，0 新 dep（ADR-008）+ 0 migration + daemon REST 移除是 v1.0 前允许的 breaking change（major version 边界）。**诚实定性（ADR-013）**：v1.0 不含 multi-user/认证身份/自动更新/arm64 native——honest-defer 推 v2.0；现有 SPEC-DEFER 项不阻塞 v1.0，列"已知限制"。默认行为 / proto（已 FROZEN）/ 既有契约不变；既有三门不退化。

> **入读顺序**：本 phase spec → ADR-050（v1.0 定义，本 phase 立）→ roadmap §v1.0 锚点段 → 源码锚点（`internal/daemon/rest.go:48-60`（5 路由含 2 个 501）+ `:188-240`（handleCollections chunk_count placeholder + handleImport/handleEval 501）+ `internal/cli/cli.go:23-54`（subcommands + Execute）+ `:119-127`（unknown subcommand exit 2）+ `contextforge.example.toml`（16 行缺 4 section））→ ADR-014（D1-D5，第三十六次激活）→ ADR-013（v1.0 锚点真实可验证；501 移除是 major 边界 breaking；honest-defer 推 v2.0）。

## 1. 阶段目标

v0.37.0 ship 后，启动 v1.0 收口冲刺。本 phase（Phase 45 / v0.38.0）立 v1.0 锚点（ADR-050）+ 交付 D2 API/CLI 冻结的两个 P0 阻塞项：daemon REST 清理（移除 501 + 实装 chunk_count）+ CLI（--version + --help + example.toml）。code-local 🟢 可单测，0 新 dep + 0 migration。v1.0 不含项据实延后推 v2.0。既有三门不退化。

**具体 exit criteria（§6 AC）**：
1. **ADR-050 v1.0 定义**：立 v1.0 锚点（4 维度 + 不含清单 + v2.0 路线）+ roadmap §v1.0 锚点段（AC1）
2. **daemon REST 冻结**：移除 2 个 501 未实装（import/eval/run）+ 实装 chunk_count（真实 COUNT 非 0）；移除是 v1.0 前 breaking（AC2）
3. **CLI 冻结**：`--version`/`version` 子命令 + 顶层 `--help`（不 exit 2）+ example.toml 补全 4 检索 section（AC3）
4. **v0.38.0 closeout**：smoke v35[54/54] + release docs + ADR-050 部分 ratify（D1/D2 维度）+ roadmap/adapter（AC4）
5. ADR-014 D1-D5（第三十六次激活）全通过（AC5）

**版本号**：v0.38.0（Phase 45，承 v0.37.0），theme v1.0-api-cli-freeze。minor release（v1.0 收口第一步：立锚点 + API/CLI 冻结；daemon REST 移除 2 端点是 v1.0 前 breaking change）。

## 2. 业务价值

v1.0 收口第一步——立 v1.0 锚点 + 清 API/CLI 冻结的 P0 阻塞项：

### 45.1 v1.0-definition（🟢）
ADR-050 立 v1.0 锚点（D1 能力已满足 / D2 API/CLI 冻结 / D3 文档对齐 Phase 46 / D4 发布 Phase 46-47 + 不含清单推 v2.0）。承 ADR-017 悬空的 "v1.0 release gate" 提法，正式承接为可执行锚点。

### 45.2 daemon-rest-api-freeze（🟢，含 v1.0 前 breaking）
- 移除 `POST /v1/import` + `POST /v1/eval/run` 2 个 501 未实装（§2A 决策 B 有意留下；console-api `/v1/index-jobs` + `/v1/eval-runs` 已完整覆盖）。
- 实装 `chunk_count`（打开 collection metadata.sqlite COUNT 查询，非 placeholder 0）。
- **HONEST（ADR-013）**：移除是 v1.0 前允许的 breaking change（major 边界）；daemon REST 留 search/chunks/collections 3 个真实端点。

### 45.3 cli-version-help（🟢）
CLI `--version`/`version` 子命令（打印版本）+ 顶层 `--help`（修复 `-h` exit 2）+ example.toml 补全 `[embedding]`/`[vector]`/`[reranker]`/`[retrieval]`。

**不在本 phase 范围**：D3 文档对齐（Phase 46）/ D4 GitHub Release 流程（Phase 46）/ v1.0 正式发版（Phase 47）/ multi-user/认证身份/自动更新（v2.0）。

## 3. 涉及模块

- **45.1**：`docs/decisions/adr-050-v1.0-definition.md`（新增）+ `docs/roadmap.md`（§v1.0 锚点段 + §3.27）
- **45.2**：`internal/daemon/rest.go`（移除 handleImport/handleEval + 路由 :58/:59；实装 chunk_count）+ `internal/daemon/rest_test.go`（移除 501 测试 + 加 chunk_count 真实值测试）
- **45.3**：`internal/cli/cli.go`（version 子命令 + 顶层 --help）+ `cmd/contextforge/main.go`（版本常量）+ `contextforge.example.toml`（补全 4 section + 头部刷新）+ `internal/cli/cli_test.go`（TEST-45.3）
- **45.4**：smoke v34→v35[54/54] + TestTask454 + release docs + ADR-050 部分 ratify + roadmap/adapter
- BDD：`test/features/phase-45-v1.0-api-cli-freeze.feature`

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 45.1 | ADR-050 v1.0 定义 + roadmap §v1.0 锚点段 | `../tasks/task-45.1-v1.0-definition.md` |
| 45.2 | daemon REST 移除 2 个 501 未实装 + 实装 chunk_count + rest_test 更新 | `../tasks/task-45.2-daemon-rest-api-freeze.md` |
| 45.3 | CLI --version + 顶层 --help + example.toml 补全 + TEST-45.3 | `../tasks/task-45.3-cli-version-help.md` |
| 45.4 | smoke v35[54/54] + v0.38.0 closeout + ADR-050 部分 ratify + roadmap/adapter | `../tasks/task-45.4-closeout-v0.38.0.md` |

## 5. 依赖关系

- 45.1（定义）无 dep，可先行。
- 45.2（daemon REST）dep 既有 rest.go（501 未实装 + chunk_count placeholder）+ console-api 已覆盖 import/eval（ADR-017）；与 45.3 并行无冲突。
- 45.3（CLI）dep 既有 cli.go（Execute + subcommands）+ main.go（版本注入）；与 45.2 并行。
- 45.4（closeout）dep 45.1+45.2+45.3。
- ADR-050（新 Proposed）/ ADR-007（v1.0 分发定义收窄 add-only Amendment）/ ADR-017（悬空 v1.0 gate 正式承接）/ ADR-015（proto FROZEN）/ ADR-004/008/013/014/012 守线。

## 6. 阶段级验收标准 + 端到端 smoke

- [x] **AC1**（ADR-050 v1.0 定义 🟢）: ADR-050 Proposed（4 维度 + 不含清单 + v2.0 路线）+ roadmap §v1.0 锚点段 — verified by **TEST-45.1.1**（ADR-050 在场 + 4 维度 + 不含清单）
- [x] **AC2**（daemon REST 冻结 🟢，含 v1.0 前 breaking）: 移除 2 个 501 未实装 + 实装 chunk_count（真实 COUNT 非 0） — verified by **TEST-45.2.1**（501 端点移除 + rest_test）+ **TEST-45.2.2**（chunk_count 真实值）
- [x] **AC3**（CLI 冻结 🟢）: `--version`/`version` + 顶层 `--help`（不 exit 2）+ example.toml 4 section — verified by **TEST-45.3.1**（version 输出）+ **TEST-45.3.2**（--help 不 exit 2）+ **TEST-45.3.3**（example.toml 4 section 在场）
- [x] **AC4**（v0.38.0 closeout + ADR-050 部分 ratify）: smoke v35[54/54] + release docs + ADR-050 部分 ratify（D1/D2）+ roadmap/adapter — verified by **TEST-45.4.1**
- [x] **AC5**（ADR-014 cross-validation gate）: D1-D5（第三十六次激活）— verified by task-45.4 PR body + LAST TEST

## 7. 阶段级风险

- **R1（中）移除 501 端点破既有调用方**：daemon REST 移除 import/eval/run 端点可能破既有调用（如有）。
  - **缓解**：v1.0 前 major 边界允许 breaking；console-api 已完整覆盖（`/v1/index-jobs` + `/v1/eval-runs`）；release notes 显式记 breaking change；rest_test 更新守护。stop-condition：若 v1.0 后才发现破生产调用则需 v1.0.1 兼容垫片。
- **R2（低）chunk_count 实装性能**：打开每个 collection metadata.sqlite COUNT 查询，collection 多时偏慢。
  - **缓解**：COUNT(*) 是 SQLite 索引扫描（快）；collection 数通常 <10；best-effort（单 collection 失败不阻断列表）。stop-condition：若 collection 极多导致超时则加 cache 或 honest-defer。

## 8. Definition of Done

- 4 task spec 顶部 Status Done；§6 AC1-5 全 [x]；smoke 全 PASS。
- ADR-050 Proposed（部分 ratify D1/D2；D3/D4 在 Phase 46/47 + v1.0.0 完整 ratify）；ADR-007 add-only Amendment（v1.0 分发定义收窄）；roadmap §v1.0 锚点段 + §3.27 + adapter。
- release：v0.38.0-{evidence,artifacts}.md + RELEASE_NOTES + README v0.38 段（含 breaking change 记录）。
- smoke：v35[54/54] + TestTask454。
- follow-up：D3 文档对齐（Phase 46）/ D4 GitHub Release（Phase 46）/ v1.0 正式发版（Phase 47）/ v2.0 路线（multi-user/自动更新/arm64）。
