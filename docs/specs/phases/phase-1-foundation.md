# Phase 1 · foundation

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。本文件由 `/s2v-init` 生成，Status=Draft。
> §6 含 `<TBD-by-user>` 端到端 smoke 占位 —— 本 phase 最后一个 task 完工/合并前必须填实（`s2v_preflight_phase` C1 集成兜底门禁，team §4 Gate 3 强制）。

## 1. 阶段目标

`contextforge init` 跑通；Go CLI（`contextforge`）↔ Rust core（`contextforge-core`）双二进制经 local gRPC 打通；canonical record schema + denylist/allowlist 配置定型。来源：PRD §Implementation Phases Phase 1。

## 2. 业务价值

奠定整个 ContextForge 的契约地基：所有后续 phase（索引/导入/检索/治理/迁移）都依赖本 phase 冻结的 canonical record schema 与 gRPC proto。没有稳定契约，多 phase 并行开发会持续返工（对应 PRD §Vision「统一、本地优先、可解释、可评测的 Context Hub」的前置）。

## 3. 涉及模块

- `cli`（Go）：CLI 入口骨架 + `contextforge init`
- `config`（Go）：TOML 配置 + denylist/allowlist 加载
- `daemon`（Go skeleton）：本地服务骨架 + 启动 contextforge-core + gRPC client
- `contextforge-core`（Rust skeleton）：gRPC server + health
- `proto/`：gRPC + canonical-record proto 契约
- 文件锚点：`cmd/contextforge/` · `internal/cli/` · `internal/config/` · `internal/daemon/` · `core/src/` · `proto/contextforge/v1/`

## 4. 任务清单

| Task | 模块 | Spec |
|---|---|---|
| 1.1 | proto | `../tasks/task-1.1-proto.md` |
| 1.2 | config | `../tasks/task-1.2-config.md` |
| 1.3 | core | `../tasks/task-1.3-core-skeleton.md` |
| 1.4 | cli | `../tasks/task-1.4-cli-init.md` |

## 5. 依赖关系

- **依赖**：无（PRD depends_on = `-`）
- **可并行**：否（基础设施，所有后续 phase 依赖本 phase 冻结的契约）
- **Phase 内顺序**：1.1 proto 先行 → 1.2 config ∥ 1.3 core-skeleton（均 dep 1.1）→ 1.4 cli-init（dep 1.1/1.2/1.3，端到端 `init`）

## 6. 阶段级验收标准 + 端到端 smoke

**阶段级验收标准（参考 — agent 据 PRD §Implementation Phases Phase 1 Exit Criteria 提供，用户审定后落实）**：

- `contextforge init` 能生成默认配置与本地数据目录（`~/.contextforge/` 结构见 PRD §Technical Approach 本地数据目录结构 v0.1）
- `contextforge-core` 能由 daemon 启动
- Go daemon 能通过 local gRPC health check Rust core
- canonical record schema v0.1 与 proto 契约冻结（字段见 PRD §Technical Approach Canonical Record v0.1 最小 schema）
- denylist / allowlist 默认配置可被 CLI 读取

**端到端 smoke**：`<TBD-by-user>`（本 phase 最后一个 task=1.4 完工/合并前填实，例：`contextforge init && contextforge-core 由 daemon 拉起 + gRPC health 返回 SERVING` 的可执行 smoke 命令序列）

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R1**（Go↔Rust local gRPC 边界复杂度）：本 phase 即为 R1 缓解落点 —— 必须在此冻结 canonical record + gRPC proto 契约并版本化；契约变更走 proto 兼容规则（仅加字段、不删不改 tag）。

## 8. Phase Definition of Done

- [ ] 本 phase 全部 task spec Status=Done 或 Waived（按 §12.3 登记）
- [ ] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过（`s2v_preflight_phase` 通过）
- [ ] 关联风险 R1 缓解措施已落地（proto/canonical schema 版本化冻结）
- [ ] adapter §Phase 状态索引该行 Status 同步更新
- [ ] team §4 Gate 3 phase smoke gate 通过后方可 merge 最后一个 task
