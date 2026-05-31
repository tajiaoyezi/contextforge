# Task `25.3`: `closeout-v0.18.0 — 多 backend 生产选择矩阵（语料规模 × 部署形态 → 推荐 backend + caveat）+ scripts/console_smoke.sh v15 向量生产 backend 状态 smoke + v0.18.0 release docs（README/RELEASE_NOTES/evidence/artifacts）+ ADR-030 据真实结果 ratify + ADR-023/008 add-only Amendment（D3/D4 tier 推进，不溯改正文 D5）+ phase-25 §6 闭合 + adapter + ADR-014 第十六次激活`

**Status**: Draft

**Priority**: P2
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 25 (production-vector-backend)
**Dependencies**: task-25.1（qdrant 生命周期层：connect 配置 + health-probe + ensure-create 契约）/ task-25.2（lancedb 真实可构建性调查结论：构建通过或 stop-condition + 索引调参参数）/ task-23.3（closeout 模板 + smoke pattern + tag/backfill pattern）/ task-19.7（closeout 模板）/ ADR-030（production-vector-backend，本 phase 新 Proposed）/ ADR-023（vector-backend-default，本 phase 推进 D3/D4 tier）/ ADR-013（禁伪造）/ ADR-014 D1-D5（第十六次激活）

## 1. Background

task-25.1 已让 `vector-qdrant` feature 下 `QdrantBackend` 有 connect 配置 + health-probe + collection ensure-create 的生命周期层（契约层不连 live server 可单测；真实 KNN over live qdrant 诚实延后）；task-25.2 已给出 lancedb 真实 dev-box 可构建性结论（构建通过记真实凭据或诚实 stop-condition）+ 索引调参参数。本 task 收口 Phase 25：(1) 产出**多 backend 生产选择矩阵**（语料规模 × 部署形态 → 推荐 backend + 每档 caveat：live-server 依赖 / protoc 前置 / 平台限制）；(2) 把 smoke 升 v15，加向量生产 backend 状态 smoke 断言；(3) 产出 v0.18.0 release docs；(4) 据真实非合成结果 ratify ADR-030 + ADR-023/008 add-only Amendment 记录 D3/D4 tier 推进结果（不溯改正文，D5）；(5) 闭合 phase-25 §6 AC；(6) 更新 s2v-adapter。

承 v0.13.0–v0.16.0 收口模式：closeout = smoke final + release evidence/artifacts + README/RELEASE_NOTES + ADR 状态 + adapter；tag push 在无人值守授权下由主 agent 自主决断（ADR-012），post-tag-push backfill 填实 tag SHA / run ID / 镜像 digest。

## 2. Goal

产出多 backend 生产选择矩阵：据 ADR-023 D1-D4 tier + ADR-028 嵌入式/fallback 推进 + 本 phase qdrant/lancedb 推进结果，把四档（dev/小语料 → hnsw（含 ADR-028 持久化）；单机嵌入式持久 → sqlite-vec（ADR-028 MSVC 通过）；大语料嵌入式列存 → lancedb（task-25.2 可构建性结论）；hosted/multi-agent/scale-out → qdrant（task-25.1 生命周期层））按语料规模 × 部署形态收敛成可查矩阵 + 每档 caveat。`scripts/console_smoke.sh` 升 v15：既有 step 不退化 + 新增向量生产 backend 状态 smoke 断言（qdrant/lancedb 为 feature 层验证、非 console 热路径 + 默认构建 intact 断言，承 task-23.3 smoke pattern）。新增 `docs/releases/v0.18.0-{evidence,artifacts}.md` + `README.md` v0.18 段 + `RELEASE_NOTES.md` v0.18.0 段。`docs/decisions/adr-030-production-vector-backend.md` 据 task-25.1/25.2 真实结果 Status `Proposed → Accepted`（或受阻维度记录维持）+ ADR-023/008 add-only Amendment 记 D3/D4 tier 推进结果。`docs/specs/phases/phase-25-*.md` §6 AC1-5 全 `[x]` + Status `Draft → Done`。`docs/s2v-adapter.md` Phase 25 `Draft → Done` + Tasks `0 → 3` + ADR-030 索引 + ADR-023 D3/D4 推进记录。ADR-014 D1-D5 第十六次激活 closeout PR body。D2 lint 触及行 0 未标注命中。

## 3. Scope

### In Scope

- **产出多 backend 生产选择矩阵**：语料规模 × 部署形态 → 推荐 backend + caveat 的可查矩阵（hnsw / sqlite-vec / lancedb / qdrant 四档，每档据 ADR-023 tier + ADR-028/本 phase 推进结果 + caveat），写入 release docs（v0.18.0-evidence）+ adapter；矩阵为 add-only 指南，不溯改 ADR-023 D1-D6 tier 排序。
- **修改 `scripts/console_smoke.sh`**：v15 注释段 + 新增向量生产 backend 状态 smoke 断言（qdrant/lancedb 为 feature-gated backend 层、非 console 热路径 + 默认构建 intact 断言，承 task-23.3 step 32 pattern）；既有 step 标号 / 断言不动语义；终态 marker 保留。
- **新增 `docs/releases/v0.18.0-evidence.md` + `docs/releases/v0.18.0-artifacts.md`**：承 v0.13.0–v0.16.0 模板（合入记录 / S2V 状态 / 验证证据 / CI / AC 达成 / 平台矩阵 / 生产 backend 选择矩阵 / upgrade-rollback / ADR-014 record / tag+镜像 SHA backfill 段）。
- **修改 `README.md`**：v0.18 段——生产规模向量 backend（qdrant 生命周期层 feature 下 / lancedb 可构建性态如实记录）+ 选择矩阵摘要。
- **修改 `RELEASE_NOTES.md`**：v0.18.0 段（task 表 + qdrant 生命周期 / lancedb 可构建性结论 / 选择矩阵 + upgrade/rollback）。
- **修改 `docs/decisions/adr-030-production-vector-backend.md`**：据 task-25.1/25.2 真实结果 Status `Proposed → Accepted`（或受阻维度记录维持）+ 回填 Ratification Amendment；ADR-023 D3/D4 tier 推进 + ADR-008 依赖变更（若 task-25.1/25.2 引入新 crate）以 add-only Amendment 记录（不溯改 ADR-023/008 正文，D5）。
- **修改 `docs/specs/phases/phase-25-production-vector-backend.md`**：§6 AC1-5 全 `[x]` + Status `Draft → Done` + §8 DoD 勾选。
- **修改 `docs/s2v-adapter.md`**：Phase 25 行 `Draft → Done` + `Tasks 0 → 3` + Task 索引 25.1-25.3 Done + ADR-030 索引行 + BDD phase-25 feature 行 + ADR-023 D3/D4 推进注。
- **新增 `test/features/phase-25-production-vector-backend.feature`**（≥3 scenario）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- **qdrant 生命周期层实现** [SPEC-OWNER:task-25.1-qdrant-server-lifecycle]：本 task 在矩阵 / release docs 引用其交付，不实现。
- **lancedb 可构建性调查实现** [SPEC-OWNER:task-25.2-lancedb-buildability-and-index-tuning]：本 task 引用其可构建性结论 + 索引调参参数，不重做调查。
- **qdrant 真实 live-server 集成 / KNN** [SPEC-DEFER:phase-future.qdrant-server-lifecycle]：CI 无 live server，矩阵记 qdrant 档 caveat，真实集成延后。
- **lancedb 真实 ANN 索引性能 / compaction 执行** [SPEC-DEFER:phase-future.lancedb-index-tuning] / [SPEC-DEFER:phase-future.lancedb-schema-compaction]：矩阵记 lancedb 档 caveat，真实性能延后。
- **把 qdrant/lancedb 接进 `core/src/server.rs` 语义热路径** [SPEC-DEFER:phase-future.vector-retrieval-integration]：backend 生命周期/可构建性先行，热路径接入后续。
- **v0.18.0 tag push 实际执行**：closeout PR 合入后，在无人值守授权下主 agent 自主 push `v0.18.0` annotated tag 触发 release.yml（承 v0.16.0 自主授权 pattern）；post-tag-push backfill 填实 tag SHA / run ID / 镜像 digest 由独立 backfill PR 承接。
- **multi-arch 镜像 / 签名 / SBOM** [SPEC-DEFER:phase-future.multi-arch-image] / [SPEC-DEFER:phase-future.image-signing-and-sbom]：发布硬化项，独立推进。

## 4. Actors

- **主 agent**：实施 + PR 主理 + closeout 决策（选择矩阵 caveat + ADR-030 ratify vs 受阻维度记录维持）+ tag push 自主决断（ADR-012）。
- **`scripts/console_smoke.sh`**：端到端 C1 兜底 smoke，本 task 升 v15。
- **`docs/releases/` + `README.md` + `RELEASE_NOTES.md`**：v0.18.0 release 文档面 + 生产 backend 选择矩阵。
- **`docs/decisions/adr-030-*.md`**：本 phase 新 ADR，本 task ratify；ADR-023/008 add-only Amendment。
- **`docs/s2v-adapter.md`**：Phase/task/ADR/BDD 索引。
- **release.yml**：tag push 触发的发布流（主 agent 自主授权 push）。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/tasks/task-23.3-closeout-v0.16.0.md`（向量 closeout 模板 + smoke v13 pattern + ADR ratify/Amendment pattern + tag/backfill）+ `docs/specs/tasks/task-19.7-closeout-v0.12.0.md`（closeout 模板）
- `docs/releases/v0.16.0-{evidence,artifacts}.md`（最近向量 release 文档结构 + 平台矩阵 + backfill 段）
- `scripts/console_smoke.sh`（既有 step + `/32]` 标号 + 终态 marker + task-23.3 step 32 向量持久化/跨平台 pattern）
- `docs/specs/tasks/task-25.1-qdrant-server-lifecycle.md` + `task-25.2-lancedb-buildability-and-index-tuning.md`（本 phase 上游交付）
- `docs/decisions/adr-030-production-vector-backend.md`（本 phase ADR，D1-D4 + 待回填的 Ratification Amendment 段）+ `docs/decisions/adr-023-vector-backend-default.md`（D3/D4 tier + Amendment pattern）+ `docs/decisions/adr-028-vector-persistence-strategy.md`（嵌入式/fallback 两档前置结论）+ `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5）
- `core/src/retriever/vector/{qdrant,lance_db,hnsw,sqlite_vec,brute_force}.rs`（四档 backend + 默认 0-dep brute-force — 选择矩阵依据）
- `docs/s2v-adapter.md` §Phase / §Task / §ADR / §BDD 索引

### 5.2 关键设计 — 选择矩阵 + smoke v15 + ADR ratify

- **多 backend 生产选择矩阵**：据 ADR-023 D1-D4 tier + ADR-028 + 本 phase 推进结果，列「语料规模 × 部署形态 → 推荐 backend」——dev/小语料 / 单机嵌入式持久 / 大语料嵌入式列存 / hosted multi-agent scale-out 各档对应 hnsw / sqlite-vec / lancedb / qdrant，每档记 caveat（hnsw 重建图成本 / sqlite-vec 单机 MSVC 凭据 / lancedb protoc 前置 + 可构建性结论 / qdrant live-server 依赖 + CI 无 server）。矩阵 add-only，不溯改 ADR-023 tier 排序。
- **smoke v15**：新增向量生产 backend 状态 smoke——qdrant/lancedb 为 feature-gated backend 层、非 console 热路径，step 诚实文档化（Rust feature 层 TEST-25.1.*/25.2.* 验证 + 默认构建 intact 断言），不伪造 console 生产 backend 路径（承 task-23.3 step 32 pattern，ADR-013）。既有 step 断言不动；终态 marker 保留。
- **ADR-030 ratify（ADR-013）**：据 task-25.1 真实契约单测（config/health-probe/ensure-create 决策，不连 server）+ task-25.2 真实 dev-box 构建结果（通过或 stop-condition）Proposed→Accepted；若某维度受阻（如 lancedb 构建在本平台受阻 / qdrant 无 live server 不能跑 KNN）则据「已达维度 ratify + 受阻维度如实记录」处理，不据合成/伪造 ratify。
- **ADR-023/008 add-only Amendment**：D3 qdrant tier（生命周期层推进）/ D4 lancedb tier（可构建性结论）以 add-only Amendment 记录在 ADR-023，不溯改 D1-D6 正文（D5）；若 task-25.1/25.2 引入新 crate 则 ADR-008 add-only 记依赖变更（基线：qdrant-client/lancedb/arrow-array/futures 均既有 optional dep，0 新 direct dep）。

### 5.3 不变量

- smoke 既有 step 不退化（仅新增向量生产 backend 状态 step + v15 注释）。
- release docs 诚实口径（承 task-23.3 §10）：deterministic 默认 / feature 本地 / 受阻三态如实标；qdrant live-server 依赖 + lancedb 可构建性据 task-25.1/25.2 真实结论记录，不伪造。
- ADR-030 ratify 仅在 task-25.1/25.2 真实落地后（ADR-013：据真实非合成）；受阻维度不强 ratify。
- 默认构建 0 vector 依赖 + BM25-only baseline 行为不变（ADR-023 D5 / ADR-004）。
- 选择矩阵 add-only，不溯改 ADR-023 D1-D6 tier 排序（D5）。

## 6. Acceptance Criteria

- [ ] **AC1**: 多 backend 生产选择矩阵产出（语料规模 × 部署形态 → 推荐 backend + 每档 caveat：hnsw/sqlite-vec/lancedb/qdrant）写入 release docs + adapter；`scripts/console_smoke.sh` v15 通过 `bash -n`（exit 0）+ 向量生产 backend 状态 smoke 断言（qdrant/lancedb feature 层 + 默认构建 intact）+ 既有 step 不退化 — verified by **TEST-25.3.1**
- [ ] **AC2**: v0.18.0 release docs 齐备（`docs/releases/v0.18.0-{evidence,artifacts}.md` + `README.md` v0.18 段 + `RELEASE_NOTES.md` v0.18.0 段）；evidence 含 task 表 / CI / AC 达成 / 平台矩阵 / 生产 backend 选择矩阵 / upgrade-rollback / §tag-backfill 待回填段 — verified by **TEST-25.3.2**
- [ ] **AC3**: ADR-030 据 task-25.1/25.2 真实结果 Status `Proposed → Accepted`（或受阻维度记录维持）+ Ratification Amendment 回填；ADR-023/008 add-only Amendment 记 D3/D4 tier 推进结果（不溯改正文）；phase-25 §6 AC1-5 全 `[x]` + Status `Draft → Done`；adapter Phase 25 `Draft → Done` + Tasks `0 → 3` + ADR-030 索引 + ADR-023 D3/D4 推进注 — verified by **TEST-25.3.3**
- [ ] **AC4**: 既有不退化 — 默认 `cargo test --workspace` + `go test ./...` 全 PASS；`cargo test --workspace --features vector-qdrant`（+ 可构建前提下 `--features vector-lancedb`）不退化 — verified by **TEST-25.3.4** + §10
- [ ] **AC5**: ADR-014 D1-D5 第十六次激活全通过（D1 phase§6↔task§6 mapping 表 + D2 lint `--touched origin/master` 0 未标注命中 + D3 verified-by + D4 自治 + D5 历史 Phase 1-24 不溯改）— verified by **TEST-25.3.5** + 本 closeout PR body

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-25.3.1 | 生产 backend 选择矩阵 + smoke v15 `bash -n` + 向量生产 backend 状态断言 | `docs/releases/v0.18.0-evidence.md` + `internal/cli/smoke_syntax_test.go` + `scripts/console_smoke.sh` | Planned |
| TEST-25.3.2 | v0.18.0 release docs 齐备 + 结构校验 | `docs/releases/v0.18.0-*.md` + README + RELEASE_NOTES | Planned |
| TEST-25.3.3 | ADR-030 ratify + ADR-023/008 Amendment + phase-25 闭合 + adapter | `docs/decisions/adr-030-*.md` + phase-25 spec + s2v-adapter | Planned |
| TEST-25.3.4 | 默认 `cargo test --workspace` + `go test ./...` + feature build 0 failed | 全 Rust + Go | Planned |
| TEST-25.3.5 | ADR-014 D1-D5 record（mapping + D2 lint，第十六次激活） | 本 closeout PR body | Planned |

## 8. Risks

- **R1（中）qdrant/lancedb 某维度依赖受阻**（qdrant 无 live server / lancedb 经 task-25.2 在本平台构建受阻）：ratify 须真实结果。
  - **缓解**：ADR-030 据「qdrant 生命周期契约层已达 + lancedb 可构建性据 task-25.2 真实结论」处理——已达维度 ratify，受阻维度如实记录（ADR-013），不据合成 ratify；选择矩阵记每档 caveat（live-server 依赖 / protoc 前置 / 平台限制）。AC3 以「已达维度 ratify + 受阻维度记录维持」满足。
- **R2（低）smoke v15 生产 backend 在 CI 默认构建不可跑**（feature-gated + qdrant 需 live server / lancedb 需 protoc）：默认 CI 无 `vector-qdrant` / `vector-lancedb`。
  - **缓解**：生产 backend 状态 smoke step 诚实文档化（Rust feature 层 TEST-25.1.*/25.2.* 验证 + 默认构建 intact 断言），默认 CI 跑既有 step 不退化 + `bash -n` 语法门；如实标 feature/live-server/protoc 依赖（ADR-013），不伪造 console 生产 backend 路径。
- **R3（低）v0.18.0 tag 在 release docs 未齐前 push**：release stop-condition。
  - **缓解**：closeout PR 先备齐 release docs；tag push 在无人值守授权下由主 agent 自主决断（ADR-012，承 v0.16.0 pattern），合入后自主 push v0.18.0 tag → release.yml → backfill。

## 9. Verification Plan

```bash
# smoke v15 语法 + step 标号
bash -n scripts/console_smoke.sh
go test ./internal/cli/... -run 'TestTask25|TestTask233' -v

# 既有不退化
go test ./...
cargo test --workspace

# feature 下生产 backend 契约（vector-qdrant 不连 server / 可构建前提下 vector-lancedb）
cargo test --workspace --features vector-qdrant
cargo test --workspace --features vector-lancedb

# 端到端 smoke（合规环境）
bash scripts/console_smoke.sh        # 期望末行 CONSOLE_REAL_SMOKE_EXIT=0

# D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes (s2v 6 项标准)

- **Status**: 待实施（Draft）。实施完成后按 6 项回填：完成日期 / 改动文件 / commit 列表 / §9 Verification 实测结果（ADR-013 真实非合成：smoke v15 + 上游 task-25.1/25.2 真实凭据）/ 设计取舍（选择矩阵 caveat + smoke v15 诚实文档化 + ADR-030 ratify 维度 + ADR-023 D3/D4 add-only Amendment + tag push 自主决断）/ 剩余风险 + 下游影响（qdrant live-server 集成 / lancedb 真实索引性能延后 + tag/release backfill）。
