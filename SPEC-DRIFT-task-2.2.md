# SPEC-DRIFT — task-2.2 (parser)

**触发**：PR #6 review round (主 agent 独立 co-review)  
**日期**：2026-05-20  
**涉及字段**：§5.2 Imports（业务契约字段，主 agent 域）

## 漂移详情

- **字段**：`docs/specs/tasks/task-2.2-parser.md` §5.2
  - 原文（spec 冻结时）：
    ```toml
    tree_sitter = "0.22"
    pulldown-cmark = "0.11"
    thiserror（旧号，未 pin）
    ```
  - 现实（PR#11 `chore/dep-parser-crates` merge 后 master 锁定）：
    ```toml
    tree-sitter = "0.26.8"
    pulldown-cmark = "0.13.3"
    thiserror = "2.0.18"
    + tree-sitter-{go,rust,python,typescript,javascript} 对应版本
    ```

- **严重性**：主版本差异巨大（tree-sitter 0.22 → 0.26.8 差 4 个主版本；pulldown-cmark 0.11 → 0.13.3 差 2 个）。pulldown-cmark 0.13 的 `Tag::Heading` / `Tag::CodeBlock` 已从 tuple struct 变为 named-field struct，代码实现完全按 0.13 编写，与 §5.2 声明版本**不向后兼容**。

## 原因

1. task-2.2 实施时（2026-05-17）§5.2 按当时最新稳定建议号填写（R7 预估）。
2. 实施中发现需真实 crates，立即按 R7 流程创建 `NEEDS-DEP-task-2.2.md`。
3. 主 agent 走独立 `chore/dep-parser-crates` PR（PR#11），实证兼容集并锁定 0.26.8 / 0.13.3 / 2.0.18（见 core/Cargo.toml L25-40 详细注释："NEEDS 建议号已过时——以实证锁定为准"）。
4. task-2.2 rebase 到该 base 后，**仅消费**已锁版本，未自行改 lockfile。
5. 实施 agent 在 §10 末尾追加了 "§5.2 Imports 版本说明" 脚注承认漂移，但**未走正式 SPEC-DRIFT 单一通道**（adapter §派工模板硬约束："禁止修改 §5.2 Imports / 如发现漂移写 SPEC-DRIFT-task-X.Y.md 交主 agent"）。

## 证据（单一事实源）

- `core/Cargo.toml`（master @ db4366b + 本 PR rebase 后）
- `Cargo.lock` 中精确 pin
- PR#11 `chore/dep-parser-crates` commit 及主 agent 注释
- `cargo check --workspace` + `cargo test --workspace` 实测通过（tree-sitter 5 语言 grammar + pulldown-cmark 0.13 API）

## 影响范围

- **正向**：无（实际代码使用的是正确新版本；§9 验证、独立 review 均已确认真绿、无回归）。
- **风险**：
  - 未来 agent / 协作者直接读 §5.2 会拿到错误版本号，导致本地复现失败或安全基线漂移。
  - 下游 task-2.3 chunker、task-2.4 indexer 若引用 §5.2 作为依赖契约会被误导。
  - 审计链不完整（S2V 要求单一通道记录所有契约变更）。

## 建议修复路径（主 agent 决策）

1. 主 agent 新建 `chore/spec-drift-task-2.2` 分支。
2. 在该分支上**仅修改** `docs/specs/tasks/task-2.2-parser.md` §5.2（把版本号更新为实测锁定值 + 语言 crates 列表），**不改任何实现代码**。
3. 自 PR 自 merge（走 §4 gate，附带本 SPEC-DRIFT 作为证据）。
4. 通知本 PR（task-2.2）rebase 到新 master。
5. 本 PR 继续走 §4 gate（此时 §5.2 已与现实一致，SPEC-DRIFT 历史可追溯）。

## 关联

- AGENTS.md §5（异常处理）+ adapter §派工模板（"禁止修改 §5.2 / 漂移走 SPEC-DRIFT"）
- task-2.2 §10（已记录版本说明，但不取代正式 SPEC-DRIFT 文件）
- PR#11（R7 依赖引入的唯一事实源）
- 未来任何依赖版本演进均须走同样通道

**本文件由 task-2.2 实施 agent 在 review 要求下创建，决策权完全上交主 agent。** 
**严禁 task agent 直接编辑 §5.2。**

---

**主 agent 裁决记录区**（由主 agent 填写后 commit）：

- [x] 接受 → 走 `chore/spec-drift-task-2.2` PR（PR#12 merged 2026-05-20，master=83e063d）
- [ ] 拒绝 + 其他方案
- 签字 / 日期：主 agent，2026-05-20（依据：PR#12 merge）
