# Task `23.3`: `closeout-v0.16.0 — 向量增量索引评估（最小实现或如实延后 [SPEC-DEFER:phase-future.vector-incremental-index]）+ scripts/console_smoke.sh v13 向量持久化/跨平台 smoke + v0.16.0 release docs（README/RELEASE_NOTES/evidence/artifacts）+ ADR-028 据真实结果 ratify + ADR-023/008 add-only Amendment + phase-23 §6 闭合 + adapter`

**Status**: Draft

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 23 (vector-persistence-and-cross-platform)
**Dependencies**: task-23.1（hnsw 图持久化往返 + rebuild-on-load）/ task-23.2（sqlite-vec 跨平台调查结论：落地或 stop-condition）/ task-19.4（smoke 30-step 基线）/ task-19.7（closeout 模板 + tag/backfill pattern）/ ADR-028（vector-persistence-strategy，本 phase 新 Proposed）/ ADR-023（vector-backend-default，本 phase 推进其 Follow-ups）/ ADR-013（禁伪造）/ ADR-014 D1-D5（第十四次激活）

## 1. Background

task-23.1 已让 `vector-hnsw` feature 下 hnsw 图可持久化往返 + rebuild-on-load；task-23.2 已给出 sqlite-vec Windows MSVC 跨平台真实调查结论（某路径落地或诚实 stop-condition）。本 task 收口 Phase 23：(1) 评估**向量增量索引**——单 chunk 追加 / 删除不触发全量 reindex 的最小实现，或确证依赖未明时如实延后（`[SPEC-DEFER:phase-future.vector-incremental-index]`，承 Phase 18/19 默认全量 reindex 语义）；(2) 把 smoke 升 v13，加向量持久化 / 跨平台相关断言；(3) 产出 v0.16.0 release docs；(4) 据真实非合成结果 ratify ADR-028 + ADR-023/008 add-only Amendment 记录推进结果（不溯改正文，D5）；(5) 闭合 phase-23 §6 AC；(6) 更新 s2v-adapter。

承 v0.12.0 / v0.13.0 收口模式：closeout = smoke final + release evidence/artifacts + README/RELEASE_NOTES + ADR 状态 + adapter；tag push 经用户授权后由 release.yml 触发 + post-tag-push backfill。

## 2. Goal

评估向量增量索引并据可行性最小实现或如实延后：在支持行级 insert/delete 的 backend（sqlite-vec `vec0` 行级 / brute-force 追加）上落地单 chunk 追加 / 删除不全量重建的最小增量路径 + deterministic 单测可断言；建图类 backend（hnsw）受 crate 全量建图限制则如实延后并文档化评估口径。`scripts/console_smoke.sh` 升 v13：既有 step 不退化 + 新增向量持久化 / 跨平台 smoke 断言（feature 下 hnsw 持久化往返 smoke，或据 task-23.2 调查结论如实标注 sqlite-vec 跨平台态）。新增 `docs/releases/v0.16.0-{evidence,artifacts}.md` + `README.md` v0.16 段 + `RELEASE_NOTES.md` v0.16.0 段。`docs/decisions/adr-028-vector-persistence-strategy.md` 据 task-23.1/23.2 真实结果 Status `Proposed → Accepted`（或记录维持）+ ADR-023/008 add-only Amendment 记推进结果。`docs/specs/phases/phase-23-*.md` §6 AC1-5 全 `[x]` + Status `Draft → Done`。`docs/s2v-adapter.md` Phase 23 `Draft → Done` + Tasks `0 → 3` + ADR-028 索引 + ADR-023 Follow-ups 推进记录。ADR-014 D1-D5 第十四次激活 closeout PR body。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **评估 + 修改 `core/src/retriever/vector/`（向量增量索引最小实现或文档化评估）**：在支持行级增量的 backend 上落地单 chunk 追加 / 删除不全量 reindex 的最小路径 + deterministic 单测；不支持行级增量的 backend（建图类）如实延后并在 spec §10 / spike 文档化评估口径（`[SPEC-DEFER:phase-future.vector-incremental-index]`）。
- **修改 `scripts/console_smoke.sh`**：v13 注释段 + 新增向量持久化 / 跨平台 smoke 断言（feature 下 hnsw 持久化往返 smoke step，或据 task-23.2 调查结论如实标注 sqlite-vec 跨平台态）；既有 step 标号 / 断言不动语义；终态 marker 保留。
- **新增 `docs/releases/v0.16.0-evidence.md` + `docs/releases/v0.16.0-artifacts.md`**：承 v0.12.0/v0.13.0 模板（合入记录 / S2V 状态 / 验证证据 / CI / AC 达成 / 平台矩阵 / upgrade-rollback / ADR-014 record / tag+镜像 SHA backfill 段）。
- **修改 `README.md`**：v0.16 段——向量持久化（hnsw feature 下图持久化）+ sqlite-vec 跨平台态如实记录。
- **修改 `RELEASE_NOTES.md`**：v0.16.0 段（task 表 + hnsw 持久化 / sqlite-vec 跨平台结论 / 增量索引评估 + upgrade/rollback）。
- **修改 `docs/decisions/adr-028-vector-persistence-strategy.md`**：据 task-23.1/23.2 真实结果 Status `Proposed → Accepted`（或记录维持）；ADR-023 Follow-ups 推进 + ADR-008 依赖变更（若 task-23.2 落地替代绑定）以 add-only Amendment 记录（不溯改 ADR-023/008 正文，D5）。
- **修改 `docs/specs/phases/phase-23-vector-persistence-and-cross-platform.md`**：§6 AC1-5 全 `[x]` + Status `Draft → Done` + §8 DoD 勾选。
- **修改 `docs/s2v-adapter.md`**：Phase 23 行 `Draft → Done` + `Tasks 0 → 3` + Task 索引 23.1-23.3 Done + ADR-028 索引行 + BDD phase-23 feature 行 + ADR-023 Follow-ups 推进注。
- **新增 `test/features/phase-23-vector-persistence-and-cross-platform.feature`**（≥3 scenario）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **hnsw 图持久化实现** [SPEC-OWNER:task-23.1-hnsw-graph-persistence]：本 task 在 smoke / release docs 引用它，不实现。
- **sqlite-vec 跨平台调查实现** [SPEC-OWNER:task-23.2-sqlite-vec-cross-platform]：本 task 引用其调查结论，不重做调查。
- **向量增量索引的完整 backend 级增量（全 backend 行级 + 建图增量）** [SPEC-DEFER:phase-future.vector-incremental-index]：本 task 落最小可行增量或如实延后，完整增量属后续版本。
- **v0.16.0 tag push 实际执行**：closeout PR 合入后，据用户明确授权 push `v0.16.0` annotated tag 触发 release.yml（沿用历史 release 流；用户授权前不 push）。post-tag-push backfill 填实 tag SHA / run ID / 镜像 digest 由独立 backfill PR 承接（仿 v0.10.0/v0.11.0/v0.12.0 pattern）。
- **hybrid / reranker / remote provider** [SPEC-DEFER:phase-future.hybrid-scoring] / [SPEC-DEFER:phase-future.reranker] / [SPEC-DEFER:phase-future.embedding-provider-remote]：其他候选版本。
- **multi-arch 镜像 / 签名 / SBOM** [SPEC-DEFER:phase-future.multi-arch-image] / [SPEC-DEFER:phase-future.image-signing-and-sbom]：发布硬化项，独立推进。

## 4. Actors

- **主 agent**：实施 + PR 主理 + closeout 决策（增量索引最小实现 vs 延后 + ADR-028 ratify vs 维持）。
- **`core/src/retriever/vector/`**：向量 backend 层，本 task 评估增量索引。
- **`scripts/console_smoke.sh`**：端到端 C1 兜底 smoke，本 task 升 v13。
- **`docs/releases/` + `README.md` + `RELEASE_NOTES.md`**：v0.16.0 release 文档面。
- **`docs/decisions/adr-028-*.md`**：本 phase 新 ADR，本 task ratify；ADR-023/008 add-only Amendment。
- **`docs/s2v-adapter.md`**：Phase/task/ADR/BDD 索引。
- **用户**：v0.16.0 tag push 授权（stop-condition）。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/tasks/task-19.7-closeout-v0.12.0.md`（closeout 模板 + tag/backfill pattern）+ `docs/specs/tasks/task-18.9-release-v0.11.0-closeout.md`（诚实缩范围 closeout pattern）
- `docs/releases/v0.12.0-{evidence,artifacts}.md` + `docs/releases/v0.11.0-{evidence,artifacts}.md`（release 文档结构 + 平台矩阵 + backfill 段）
- `scripts/console_smoke.sh`（既有 step + 终态 marker）
- `docs/specs/tasks/task-23.1-hnsw-graph-persistence.md` + `task-23.2-sqlite-vec-cross-platform.md`（本 phase 上游交付）
- `docs/decisions/adr-028-vector-persistence-strategy.md`（本 phase ADR）+ `docs/decisions/adr-023-vector-backend-default.md`（Follow-ups + Amendment pattern）+ `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）
- `core/src/retriever/vector/{hnsw,sqlite_vec,brute_force}.rs`（各 backend `delete` 全量 reindex 语义 — 增量索引评估基线）
- `docs/s2v-adapter.md` §Phase / §Task / §ADR / §BDD 索引

### 5.2 关键设计 — 增量索引评估 + smoke v13 + ADR ratify

- **向量增量索引评估**：核实各 backend 增量能力——sqlite-vec `vec0` 支持行级 INSERT/DELETE（既有 `delete` 是 spike 全量语义，可改行级）；brute-force `rows` 可追加；hnsw `instant-distance` 全量建图无增量插入。最小实现优先在支持行级的 backend 落 deterministic 单测（追加/删除单 chunk → search 反映变更，不全量重建）；hnsw 增量受 crate 限制则如实延后 `[SPEC-DEFER:phase-future.vector-incremental-index]` + 文档化评估口径。AC 以「评估完成 + 最小实现或诚实延后」满足。
- **smoke v13**：新增向量持久化 / 跨平台 smoke——feature `vector-hnsw` 下 hnsw 持久化往返 smoke（index→save→重载→search 命中）；sqlite-vec 跨平台据 task-23.2 结论如实标（构建通过则加 MSVC 构建 smoke note，受阻则记录 stop-condition note，不伪造）。既有 step 断言不动；终态 marker 保留。
- **ADR-028 ratify（ADR-013）**：据 task-23.1 真实持久化往返 + task-23.2 真实跨平台构建结果 Proposed→Accepted；若某维度受阻（如 sqlite-vec MSVC 仍阻）则 ADR-028 据「已达维度 ratify + 受阻维度如实记录」处理，不据合成 / 伪造 ratify。
- **ADR-023/008 add-only Amendment**：推进结果（hnsw 持久化解除 D2「rebuild-on-restart」前提 / sqlite-vec 跨平台结论）以 add-only Amendment 记录在 ADR-023，不溯改 D1-D6 正文（D5）；若 task-23.2 落地替代绑定则 ADR-008 add-only 记依赖变更。

### 5.3 不变量

- smoke 既有 step 不退化（仅新增向量持久化 / 跨平台 step + v13 注释）。
- release docs 诚实口径（承 task-19.7 / task-18.9 §10）：deterministic 默认 / feature 本地 / 受阻三态如实标；sqlite-vec 跨平台据 task-23.2 真实结论记录，不伪造。
- ADR-028 ratify 仅在 task-23.1/23.2 真实落地后（ADR-013：据真实非合成）；受阻维度不强 ratify。
- 默认构建 0 vector 依赖 + BM25-only baseline 行为不变（ADR-023 D5）。

## 6. Acceptance Criteria

- [ ] **AC1**: 向量增量索引评估完成 — 支持行级的 backend 落最小增量实现（单 chunk 追加/删除不全量 reindex）+ deterministic 单测可断言，或确证依赖未明的 backend 如实延后并文档化评估口径（`[SPEC-DEFER:phase-future.vector-incremental-index]`）；`scripts/console_smoke.sh` v13 通过 `bash -n`（exit 0）+ 向量持久化/跨平台 smoke 断言 + 既有 step 不退化 — verified by **TEST-23.3.1**
- [ ] **AC2**: v0.16.0 release docs 齐备（`docs/releases/v0.16.0-{evidence,artifacts}.md` + `README.md` v0.16 段 + `RELEASE_NOTES.md` v0.16.0 段）；evidence 含 task 表 / CI / AC 达成 / 平台矩阵 / upgrade-rollback / §tag-backfill 待回填段 — verified by **TEST-23.3.2**
- [ ] **AC3**: ADR-028 据 task-23.1/23.2 真实结果 Status `Proposed → Accepted`（或受阻维度记录维持）；ADR-023/008 add-only Amendment 记推进结果（不溯改正文）；phase-23 §6 AC1-5 全 `[x]` + Status `Draft → Done`；adapter Phase 23 `Draft → Done` + Tasks `0 → 3` + ADR-028 索引 + ADR-023 Follow-ups 推进注 — verified by **TEST-23.3.3**
- [ ] **AC4**: 既有不退化 — 默认 `cargo test --workspace` + `go test ./...` 全 PASS；`cargo test --workspace --features vector-hnsw`（+ Linux `--features vector-sqlite`）不退化 — verified by **TEST-23.3.4** + §10
- [ ] **AC5**: ADR-014 D1-D5 第十四次激活全通过（D1 phase§6↔task§6 mapping 表 + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-22 不溯改）— verified by **TEST-23.3.5** + 本 closeout PR body

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-23.3.1 | 增量索引评估（最小实现或延后）+ smoke v13 `bash -n` + 向量持久化/跨平台断言 | `core/src/retriever/vector/` + `internal/cli/smoke_syntax_test.go` + `scripts/console_smoke.sh` | Planned |
| TEST-23.3.2 | v0.16.0 release docs 齐备 + 结构校验 | `docs/releases/v0.16.0-*.md` + README + RELEASE_NOTES | Planned |
| TEST-23.3.3 | ADR-028 ratify + ADR-023/008 Amendment + phase-23 闭合 + adapter | `docs/decisions/adr-028-*.md` + phase-23 spec + s2v-adapter | Planned |
| TEST-23.3.4 | 默认 `cargo test --workspace` + `go test ./...` + feature build 0 failed | 全 Rust + Go | Planned |
| TEST-23.3.5 | ADR-014 D1-D5 record（mapping + D2 lint） | 本 closeout PR body | Planned |

## 8. Risks

- **R1（中）向量增量索引各 backend 能力差异**（承 phase-23 §7 R3）：hnsw 全量建图无增量插入。
  - **缓解**：最小增量优先在 sqlite-vec `vec0` 行级 / brute-force 追加落地 + deterministic 单测；hnsw 增量受 crate 限制则如实延后 `[SPEC-DEFER:phase-future.vector-incremental-index]` + 文档化评估口径，AC1 以「评估完成 + 最小实现或诚实延后」满足。
- **R2（中）ADR-028 某维度依赖受阻**（sqlite-vec MSVC 经 task-23.2 仍阻）：ratify 须真实结果。
  - **缓解**：ADR-028 据「hnsw 持久化已达 + sqlite-vec 跨平台据 task-23.2 真实结论」处理——已达维度 ratify，受阻维度如实记录（ADR-013），不据合成 ratify。
- **R3（低）v0.16.0 tag 误在用户授权前 push**：release stop-condition。
  - **缓解**：closeout PR 仅备齐 release docs；tag push 经用户明确授权后单独执行（沿用历史 release 流）。
- **R4（低）smoke v13 持久化往返在 CI 默认构建不可跑**（feature-gated）：默认 CI 无 `vector-hnsw`。
  - **缓解**：持久化往返 smoke 在 feature 下本地 / 合规环境跑（🟡），默认 CI 跑既有 step 不退化 + `bash -n` 语法门；如实标 feature 依赖（ADR-013）。

## 9. Verification Plan

```bash
# smoke v13 语法 + step 标号
bash -n scripts/console_smoke.sh
go test ./internal/cli/... -run 'TestTask23|TestTask194' -v

# 既有不退化
go test ./...
cargo test --workspace

# feature 下持久化 / 跨平台（vector-hnsw / Linux vector-sqlite）
cargo test --workspace --features vector-hnsw
cargo test --workspace --features vector-sqlite

# 端到端 smoke（合规环境；feature 下持久化往返）
bash scripts/console_smoke.sh        # 期望末行 CONSOLE_REAL_SMOKE_EXIT=0

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: 待实施（Draft）。实施完成后按 6 项回填，含向量增量索引评估结论（最小实现 backend 列表 / 延后口径）+ smoke v13 实跑结论 + ADR-028 ratify 结论（含 sqlite-vec 跨平台维度据 task-23.2 真实态）+ ADR-023/008 Amendment 记录 + v0.16.0 tag/backfill 状态（用户授权后）。
