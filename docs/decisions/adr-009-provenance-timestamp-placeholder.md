# ADR `009`: `provenance-timestamp-placeholder`

**Status**: Accepted
**Category**: 协议接口 / Schema gap 治理
**Date**: 2026-05-23
**Decided By**: tajiaoyezi
**Related**: task-6.1 §2A 决策 E / task-4.2 §10 schema gap 留档 / PR #37

## Context

可解释检索结果链路里，时间字段在 `core/src/chunker/mod.rs::Provenance` 与 `proto/contextforge/v1/context.proto::Provenance` 两端类型不一致：

- `chunker::Provenance.imported_at` / `source_modified_at` 实际类型是 `String`（RFC3339 with `Z` 后缀；indexer SQLite TEXT 列直存原值，task-2.4 落定）。
- `proto::Provenance.imported_at` / `source_modified_at` 是 `google.protobuf.Timestamp`（Rust 走 `prost_types::Timestamp { seconds, nanos }`）。

task-6.1 wire `CoreService::search` 端到端时需要把 `Vec<retriever::SearchResult>` 映射到 `Vec<proto::RetrievalResult>`，因此必须为 `chunker::Provenance → proto::Provenance` 提供 field mapping helper。但 `chrono` / `time` crate 都不在 `core/Cargo.toml` direct dep（`time` 只是 Cargo.lock transitive 依赖，不可直接 use），没有现成的 RFC3339 解析器把 `String` 转成 `prost_types::Timestamp`。

约束：

- R7 严格通道：worker 不能改 `Cargo.toml` / `Cargo.lock` 加新 dep（lockfile-protect 强制），只有主 agent 在专门 `chore/dep-*` PR 中能加 — 这会阻塞或延后 Phase 6 派工。
- Phase 6 P0 时序：task-6.1 是 Phase 6 首个 task，task-6.2 REST / task-6.3 export / task-7.1 MCP / task-8.1 eval 全部依赖其 gRPC Search wire 落地。
- proto schema frozen：`proto/contextforge/v1/*.proto` 在 task-1.1 / phase23-start-gate 已 freeze，不能为兼容当前实现就把 Timestamp 改回 `string`（add-only field tag 原则）。

类似的 schema gap 治理先例：task-4.2 §10 已为 `context_id` / `source_type` / `agent_scope` / `redaction_status` 4 个上游 schema 未到位的字段选「v0.1 用 default 常量 placeholder，下游 task 再补齐」路径，并在 §10 显式留档。本 ADR 是同一治理模式在时间字段上的推广。

## Decision

v0.1 用 `prost_types::Timestamp::default()`（`seconds=0, nanos=0`，对应 epoch `1970-01-01T00:00:00Z`）作为 `proto::Provenance.imported_at` / `source_modified_at` 的 placeholder 值；不引入 `chrono` / `time` crate 解析 RFC3339；CLI text 渲染路径直接读 `chunker::Provenance` 的 String 字段保留原 RFC3339 值；`--json` 输出 placeholder 时间清晰可辨（`1970-01-01T00:00:00Z`），不会与真实时间混淆。

mapping helper 形态（task-6.1 §5.3 已落定）：

```rust
fn provenance_to_proto(p: &chunker::Provenance) -> PbProvenance {
    PbProvenance {
        importer: p.importer.clone(),
        original_path: p.original_path.clone(),
        imported_at: Some(prost_types::Timestamp::default()),
        source_modified_at: Some(prost_types::Timestamp::default()),
    }
}
```

## Rationale

- **R7 严格通道避免 chore-dep PR 阻塞 Phase 6**：v0.1 P0 时序压力大（task-6.1 → 6.2 → 6.3 → 7.1 → 8.1 链上 5 个下游消费者等 Search wire 端到端打通），临时引一个新 crate 需要走完整 chore-dep PR + 通知所有存活 worktree rebase，会让 Phase 6 派工延后 ≥1 个周期。placeholder 路径让 task-6.1 此刻就能无阻塞实施。
- **CLI text 用户场景不丢信息**：用户最常用入口是 `contextforge search "<query>"` 默认 human-readable 输出，该路径直接 print `chunker::Provenance.imported_at` String，仍能看到原始 RFC3339 时间戳；只有 `--json` 自动化下游会拿到 placeholder。「人不丢信息 + 机器明显占位」是干净 trade-off。
- **治理路径与 task-4.2 §10 一致**：task-4.2 已经为 `context_id` / `source_type` / `agent_scope` / `redaction_status` 4 字段选了「v0.1 placeholder + 下游 task 再补齐」路径并显式留档；时间字段沿用相同治理模式 → 全局 schema gap 治理路径单一、可预期、易复盘。

## Alternatives

- **选项 A — 引 `chrono` crate**：主流 RFC3339 解析库，API 成熟。拒绝因为：(1) R7 严格通道需先开 `chore/dep-chrono` PR 走完整 PR → merge → rebase 流程，延后 Phase 6 派工 ≥1 周期；(2) `chrono` 不在 Cargo.lock transitive，是全新供应链面，supply chain audit 成本最高。
- **选项 B — 引 `time` crate**：Rust 官方推荐的现代时间库，且 `time` 已在 Cargo.lock 作 transitive 依赖（`time = 0.3.x`，由 `tokio` / `tantivy` 间接引入）。拒绝因为：v0.1 P0 仍需开 chore-dep PR 加 `time` 到 `core/Cargo.toml` direct dep，时序代价同选项 A — 即便供应链面较小，仍阻塞当前 Phase 6 派工。**Rollback 阶段优先选项**（见下方 Rollback Plan）。
- **选项 C — stdlib 写 inline RFC3339 parser**：~30 行手写 parser 把 `YYYY-MM-DDTHH:MM:SS[.fff]Z` 切成 `seconds` + `nanos`。拒绝因为：(1) 时间解析有大量边界（leap second / 闰年 / 时区偏移 / 小数秒精度），手写很难无 bug；(2) 测试维护负担长期摊销不划算；(3) 一旦 task-6.3 需要真实保真还是要换 crate，相当于做两遍。
- **选项 D（选定） — v0.1 placeholder `Timestamp::default()`**：见 Decision 段。
- **选项 E — 改 proto `Provenance.imported_at` 类型为 `string`**：拒绝因为：(1) proto 已 frozen 在 task-1.1 / phase23-start-gate（add-only field tag 原则）；(2) 把强类型契约降级为字符串会让下游所有消费者（REST / MCP / export / eval）都失去 `Timestamp` semantic（时区 / 比较 / 排序 / 时间范围 filter），是更大的 schema 倒退；(3) 与 ADR-003「result schema 单一源」原则冲突。

## Consequences

- **正向**：
  - Phase 6 不被 R7 chore-dep PR 阻塞，task-6.1 worker 终端可直接进入 RED → GREEN（实际已在 PR #37 review 接受后落实）；
  - CLI text 用户体验零回归（直接走 chunker::Provenance String，看到的就是原始 RFC3339 时间戳）；
  - 决策可逆（rollback 路径 = R7 chore-dep PR 引 `time` crate + 新增 `parse_rfc3339_utc` helper + 改 `provenance_to_proto` 一处调用，scope 紧凑）；
  - ADR 留档让所有下游 task（6.3 / 7.1 / 8.1）能看到「这是 placeholder，不是真实数据」，不会因为看到 1970-01-01 误以为是 bug；
  - 治理模式与 task-4.2 §10 schema gap 一致，全局可复盘可预期。
- **负向 / 成本**：
  - `--json` 输出对自动化下游有误导潜力（程序消费者拿到的 `imported_at` / `source_modified_at` 都是 `1970-01-01T00:00:00Z`），需要文档显式提示；
  - task-6.3 export 阶段如要做迁移保真度校验，时间字段保真度从理论 100% 降到 ~91%（2 / 23 字段是 placeholder），不过仍落在 PRD §Success Metrics 次指标「跨 Agent 迁移保真 ≥ 80% 结构化字段」的 17% 容差内可吸收；
  - 真实迁移时间戳要等 SPEC-DRIFT-task-2.4 + 本 ADR Rollback Plan 双轨完成后才能上线。
- **影响面**：
  - 上游：`core/src/server.rs::CoreService::search` mapping path（`provenance_to_proto` helper）；
  - 下游：task-6.1 cli-search / task-6.2 REST / task-6.3 exporter / task-7.1 MCP / task-8.1 eval-harness 全部消费 `RetrievalResult.Provenance`；
  - 不影响：retriever / indexer / chunker 内部存储（chunker::Provenance String 不变；indexer SQLite TEXT 列不变）— 该 ADR 仅约束 Rust → proto 映射边界。

## Rollback Or Migration Plan

**触发条件**（任一即可）：

1. task-6.3 exporter 实施时遇到「迁移 fixture 需校验时间字段真实值」需求；
2. 用户对自动化下游（`--json` / REST / MCP / eval）有 hard requirement 拿到真实时间；
3. SPEC-DRIFT-task-2.4 完成后 indexer schema 升级 imported_at / source_modified_at 列类型（如 REAL 双列 seconds+nanos），上游已有真实结构化时间可读 — 此时下游 placeholder 不再合理。

**回滚动作**（主 agent 走 R7 chore-dep PR）：

1. 主 agent 新开 `chore/dep-time-crate` branch；
2. 在 `core/Cargo.toml` `[dependencies]` 加 `time = { version = "0.3", features = ["parsing", "formatting"] }`（推荐 `time` 而非 `chrono`：(a) Cargo.lock 已有 transitive，供应链面零增加；(b) Rust 官方推荐现代库）；
3. 在 `core/src/server.rs` 新增 helper `fn parse_rfc3339_utc(s: &str) -> Option<prost_types::Timestamp>`：用 `time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)` 解析 → 拿 `unix_timestamp() + nanosecond()` → 组装 `prost_types::Timestamp`；空串 / 解析失败 → `None`（保持 placeholder fallback 兼容历史数据）；
4. 改 `provenance_to_proto` 把 `Timestamp::default()` 换成 `parse_rfc3339_utc(&p.imported_at).unwrap_or_default()`；
5. 沿 task-6.1 §10 §2A Decisions E 追加 rollback notes，并在本 ADR Status 字段后追加 `Superseded by: <PR>`，新开 ADR-XXX 记录最终决策；
6. 通知所有存活 worktree rebase（§4.1 STATUS-MAIN.md 协议）；
7. retriever / indexer / chunker 无需改（chunker::Provenance String 保留作为 indexer SQLite 原值持久化形态，proto 层做转换 — 向下兼容证明：`chunker::Provenance` 字段类型与本 ADR 决策时一致）。

scope 评估：rollback 改动仅 `core/Cargo.toml` 一行 + `core/src/server.rs` 两处（helper + 调用），不动 retriever / indexer / chunker / proto / CLI，紧凑且可逆。

## Follow-ups

- **关联 PRD §Success Metrics 次指标「跨 Agent 迁移保真 ≥ 80% 结构化字段」**：当前 2 / 23 字段 placeholder（~9% loss）落在 17% 容差内可吸收，但 task-8.1 eval-harness 应度量并报告 `populated rate` per field，避免容差被进一步 schema gap 蚕食。
- **关联 task-6.3 exporter spec §9 verification**：如要 fixture 校验时间字段真实保真（如 jsonl 迁移 round-trip 校验），需先触发本 ADR Rollback Plan；否则 fixture 阶段须显式 mark `imported_at` / `source_modified_at` 字段为 `placeholder-expected`。
- **关联 SPEC-DRIFT-task-2.4 indexer schema 扩展**：扩 indexer 时一并把 `imported_at` / `source_modified_at` 升级为 SQLite REAL 双列（unix epoch seconds + nanos）或新增结构化列 + 反向 backfill 历史 TEXT 行 — 真实保真的上游补，与本 ADR Rollback 形成双轨。
