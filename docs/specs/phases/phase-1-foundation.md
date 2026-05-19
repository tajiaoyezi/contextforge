# Phase 1 · foundation

**Status**: Done

> Phase Spec（s2v full-standard §8.2）。本文件由 `/s2v-init` 生成。
> **Phase 1 已收口（chore/phase-1-done，主 agent 域，2026-05-19）**：1.1/1.2/1.3/1.4
> 全 Done 并 merge；§6 端到端 smoke 已填实且经 team §4 Gate 3 实跑全绿；R1 缓解
> 落地（proto/canonical schema 冻结 + Go↔Rust gRPC 端到端打通）。§8 DoD 全满足。
> 收口三步：PR#9 chore-closeout（§6+Status→In Progress）→ PR#8 task-1.4（§4 Gate 0-4）
> → 本 chore-done（Status→Done + DoD）。

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

**端到端 smoke**（主 agent 在 team §4 Gate 3 对 task-1.4 PR 分支执行，须全过；落点 = task-1.4 AC5/TEST-1.4.5，真集成非 mock）：

```bash
# 前置：Go toolchain + Rust stable/cargo（见 docs/s2v-adapter.md §Constraints Runtime target）
set -euo pipefail

# (1) 双二进制可构建
cargo build -p contextforge-core
go build -o /tmp/cf-smoke ./cmd/contextforge

# (2) AC1 — contextforge init 在隔离 HOME 生成 ~/.contextforge/ 配置+目录，不联网，幂等可重跑
SMOKE_HOME="$(mktemp -d)"
HOME="$SMOKE_HOME" /tmp/cf-smoke init
HOME="$SMOKE_HOME" /tmp/cf-smoke init            # 二次重跑须幂等不报错
test -f "$SMOKE_HOME/.contextforge/config.toml"

# (3) AC2/AC3/AC5 — daemon 拉起 contextforge-core → local gRPC Health=SERVING；
#     core 异常退出基础版自动重启；端到端 init→core→health 串通。
#     internal/daemon TestMain 真 `cargo build -p contextforge-core` + 真子进程
#     + 真 Go↔Rust gRPC（非 mock）：
go test ./internal/daemon/ -run 'TestTask14_AC[235]' -count=1 -v

# (4) 清理
rm -rf "$SMOKE_HOME" /tmp/cf-smoke
echo "Phase 1 端到端 smoke: PASS"
```

## 7. 阶段级风险

- 关联 PRD §Technical Risks **R1**（Go↔Rust local gRPC 边界复杂度）：本 phase 即为 R1 缓解落点 —— 必须在此冻结 canonical record + gRPC proto 契约并版本化；契约变更走 proto 兼容规则（仅加字段、不删不改 tag）。

## 8. Phase Definition of Done

- [x] 本 phase 全部 task spec Status=Done 或 Waived（按 §12.3 登记）—— 1.1/1.2/1.3/1.4 均 Done 且 merge（PR#1/2/3/8）；无 Waived
- [x] §6 阶段级 AC 全部满足、端到端 smoke 已填实且执行全过（`s2v_preflight_phase` 通过）—— PR#9 填实；PR#8 §4 Gate 3 实跑：`s2v_preflight_phase` exit 0 + §6 smoke 全过（init 幂等 + AC2/AC3/AC5 真集成 PASS）
- [x] 关联风险 R1 缓解措施已落地（proto/canonical schema 版本化冻结）—— task-1.1 proto/canonical schema v0.1 冻结；task-1.4 端到端验证 Go↔Rust local gRPC（health SERVING + 崩溃自动重启）
- [x] adapter §Phase 状态索引该行 Status 同步更新 —— 本 chore-done 同步 phase-1 → Done
- [x] team §4 Gate 3 phase smoke gate 通过后方可 merge 最后一个 task —— task-1.4 §4 Gate 0-4 全过后 merge（master `2a083b8`）
