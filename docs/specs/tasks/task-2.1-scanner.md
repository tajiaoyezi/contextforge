# Task `2.1`: `scanner — 文件扫描 + denylist/allowlist 过滤 + secret 扫描`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 2 (index-core)
**Dependencies**: Phase 1（canonical schema + proto）

## 1. Background

数据面入口：扫描本地目录，按 denylist/allowlist 过滤，并做 secret 扫描 + redaction（PRD §Constraints 安全 / §Technical Risks R4）。secret redaction 不改原文件，结果保留 `[REDACTED:<TYPE>]` 类型标签。

## 2. Goal

`scanner` 能遍历指定路径（ignore/walkdir），命中 denylist 路径默认跳过，allowlist 模型可配置；secret pattern 检测命中后产出 redacted 内容 + `redaction_status`，原文件不被修改；超大单文件走流式 + 大小上限保护。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Constraints 安全 + §User Flow secret 命中示例）
- `docs/specs/phases/phase-2-index-core.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/specs/tasks/task-1.2-config.md`
- `docs/decisions/adr-004-local-first-privacy-baseline.md`
- `test/features/scanner.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Implementation Phases Phase 2 Exit Criteria): `.env`、`.ssh/`、`.git/objects/`、`node_modules/`、`target/` 等 denylist 路径默认跳过，不进扫描结果。
- [ ] **AC2** (PRD §Constraints 安全): allowlist 路径导入模型生效；用户覆盖 denylist 须显式确认。
- [ ] **AC3** (PRD §Technical Risks R4 / §User Flow): secret pattern（API key / Bearer token / private key / AWS / GitHub token / 通用 password / cookie）命中后产出 redacted 内容 + `redaction_status`，**原文件不被修改**，保留 `[REDACTED:<TYPE>]` 类型标签。
- [ ] **AC4** (PRD §Technical Risks R4): 提供 `scan --dry-run` 预检（列出将被 redact 的命中，不写索引）。
- [ ] **AC5** (PRD §User Flow 边界场景): 超大单文件（如 100MB 日志）走流式 + 大小上限保护，内存不爆。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 denylist 默认跳过 | SCEN-2.1.1 | TEST-2.1.1 | - | unit-test | Not Started |
| AC2 allowlist 模型 | SCEN-2.1.2 | TEST-2.1.2 | - | unit-test | Not Started |
| AC3 secret redact 不改原文件 | SCEN-2.1.3 | TEST-2.1.3 | - | unit-test | Not Started |
| AC4 scan --dry-run 预检 | SCEN-2.1.4 | TEST-2.1.4 | - | unit-test | Not Started |
| AC5 超大文件流式保护 | SCEN-2.1.5 | TEST-2.1.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R4**（secret redaction 漏检或误报）：denylist 第一道防线；pattern 可扩展；dry-run 预检；override 写 audit log（audit 在 task 5.3）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit（adapter 其余 §Commands 占位，按 init.md 步 8 §9 规则省略）。

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
