# Task `1.2`: `config — TOML 配置 + denylist/allowlist`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前：① 清零 `<TBD-by-user>`（§3/§4/§5.2/§5.3）② 审 §6/§7/§9 ③ Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 1 (foundation)
**Dependencies**: 1.1 (proto/canonical schema)

## 1. Background

ContextForge 默认本地优先、隐私基线（PRD §Constraints / §Decisions Log D4）。需要一份可被 CLI 读取的 TOML 配置 + 默认 denylist/allowlist，作为索引/导入的第一道安全防线（PRD §Technical Risks R4：denylist 路径优先）。

## 2. Goal

`~/.contextforge/config.toml` 默认配置可生成与读取；denylist 默认含 PRD §Constraints 列出的全部敏感路径；allowlist 路径导入模型可配置；config/token 文件权限受限（0600）。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Constraints 安全基线 + 本地数据目录结构 v0.1）
- `docs/specs/phases/phase-1-foundation.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/decisions/adr-004-local-first-privacy-baseline.md`
- `test/features/config.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：
     - init/add 基于 PRD 推导出 AC 内容，**完整写出**（不挂 <TBD-by-user> 前缀）
     - 每条 AC 加引用：`- [ ] **AC<N>** (PRD §<reference>): <内容>`
       - PRD 已写明 → 引用精确章节；PRD 没写、由 task 推导 → 标 `(本 task 新增)`
     - 用户 review 阶段：发现偏差直接改 AC 内容；review 通过**无需删除本注释**
     - **严禁** `- [ ] <TBD-by-user> AC<N>: 内容` 混合写法
-->

- [ ] **AC1** (PRD §Technical Approach 本地数据目录结构 v0.1): `contextforge` 能生成默认 `~/.contextforge/config.toml` 与目录骨架（collections/ logs/ runtime/）。
- [ ] **AC2** (PRD §Constraints 安全): 默认 denylist 包含 `.env` / `.env.*` / `*.pem` / `*.key` / `*.p12` / `*.pfx` / `id_rsa` / `id_ed25519` / `.ssh/` / `.git/objects/` / `node_modules/` / `target/` / `dist/` / `build/` / `.cache/` / `vendor/`，且可被 CLI 读取。
- [ ] **AC3** (PRD §Constraints 安全): collection 采用 allowlist 路径导入模型；用户覆盖 denylist 需显式确认。
- [ ] **AC4** (PRD §Constraints Local service security baseline): `config.toml` 与 token 文件权限为 `0600`（当前用户可读写）。
- [ ] **AC5** (PRD §Decisions Log D4 / 本 task 新增): 远程 provider 配置默认关闭，须显式 opt-in 字段才启用。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 默认配置/目录生成 | SCEN-1.2.1 | TEST-1.2.1 | - | unit-test | Not Started |
| AC2 默认 denylist 完整 | SCEN-1.2.2 | TEST-1.2.2 | - | unit-test | Not Started |
| AC3 allowlist 导入模型 | SCEN-1.2.3 | TEST-1.2.3 | - | unit-test | Not Started |
| AC4 文件权限 0600 | SCEN-1.2.4 | TEST-1.2.4 | - | unit-test | Not Started |
| AC5 远程 provider 默认关 | SCEN-1.2.5 | TEST-1.2.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R4**（secret 漏检）：denylist 是第一道防线，本 task 必须保证默认 denylist 完整且不可被静默绕过。
- 关联 PRD §Open Questions **O7**（v0.1 威胁模型边界）/ **O10**（本地 API/MCP 安全边界）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit：adapter 其余 §Commands 字段为占位，按 init.md 步 8 §9 规则省略。

## 10. Completion Notes

- **完成日期**：`<TBD-after-impl>`
- **改动文件**：`<TBD-after-impl>`
- **commit 列表**：`<TBD-after-impl>`
- **§9 Verification 结果**：
  - install: `<TBD-after-impl>`
  - typecheck: `<TBD-after-impl>`
  - unit-test: `<TBD-after-impl>`
- **剩余风险 / 未做项**：`<TBD-after-impl>`
- **下游 task 影响**：`<TBD-after-impl>`
