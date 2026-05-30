# Task `19.7`: `closeout-v0.12.0 — Phase 19 收口 + v0.12.0 release docs（端到端语义检索 ship；若 embedding provider 受阻则诚实缩范围）`

**Status**: Done（release docs + phase/adapter 收口完成；**v0.12.0 tag 已经用户授权 push** @ `dcbe09b`，`release.yml` run `26685041851` success——见 §6 AC6 / §10）

**Priority**: P0
**Owner**: 主 agent（ADR-012 自治；v0.12.0 outward-facing release tag push 为 USER-AUTHORIZED-ONLY，须用户显式授权方可 push）
**Related Phase**: Phase 19 (vector-retrieval-integration)
**Dependencies**: task-19.1（embedding provider trait + deterministic 缺省 provider + real provider feature-gated + spike evidence）/ task-19.2（默认 backend 接 `Retriever::with_vector_searcher` 生产热路径）/ task-19.3（`/v1/search?semantic=true` Go→Rust gRPC semantic 通路 + proto add-only 字段）/ task-19.4（smoke v9 30-step + `cmd/contextforge/eval.go --semantic` CLI）/ task-19.5（真实 dogfood embedding `SemanticRecall@K` 实测 + `docs/spikes/phase-19-real-recall.md`）/ task-19.6（ADR-023 Proposed→Accepted ratify 或据实测维持 + ADR-006 A1 转正 + ADR-008 amend + Phase 18 AC3/AC4 解决记录）/ ADR-014 D1-D5 第十次激活收口 / ADR-013（禁 fake-evidence）

## 1. Background

Phase 19 把 Phase 18 交付的**向量 backend 基础设施**（task-18.1 三 trait + task-18.2 harness + task-18.3–18.6 4 路真实数据 backend + task-18.7 ADR-023 Proposed + task-18.8 `SemanticRecall@K` 度量+门禁）推进到**生产语义检索**：task-19.1 补 embedding provider（deterministic 缺省 + real feature-gated）、task-19.2 把选定默认 backend 接生产 retriever 热路径、task-19.3 通 `/v1/search?semantic=true` Go→Rust gRPC 通路、task-19.4 smoke v9 30-step、task-19.5 真实召回评测、task-19.6 ratify ADR-023。

本 task 是 Phase 19 的**收口 + v0.12.0 release**：把 19.1–19.6 的交付汇成对外 minor release 文档（README v0.12 段 + RELEASE_NOTES v0.12.0 段 + `docs/releases/v0.12.0-{evidence,artifacts}.md`），勾选 phase-19 §6 AC + §8 DoD，更新 `docs/s2v-adapter.md`（Phase 19 Draft→Done / Tasks 0→7 / ADR-023 状态 / BDD row / Phase 18 forward-ref 解除），合入后 push v0.12.0 annotated tag 触发 `release.yml` ghcr 镜像构建（**tag push 经用户授权**）。

承 task-18.9（v0.11.0 closeout）建立的 release-docs pattern（`docs/releases/v0.11.0-{evidence,artifacts}.md`）。**诚实口径（ADR-013）**：v0.12.0 的范围取决于 task-19.1/19.5 的真实交付——若 real embedding provider 在两平台均可构建且 task-19.5 产出真实 `SemanticRecall@K`，则 v0.12.0 = 「端到端语义检索 live + ADR-023 ratify」；若 embedding provider 受平台/模型门槛（Phase 19 §7 R1 stop-condition）受阻，则 deterministic 缺省 provider 跑通 wiring/smoke、real recall + ADR ratify 据实测诚实缩范围 defer，仿 task-18.9 缩范围 pattern。本 task 据 19.1–19.6 落地的真实状态填，**不预先断言 Done/Accepted**。

## 2. Goal

落 v0.12.0 release docs（README §v0.12.0 + RELEASE_NOTES §v0.12.0 + `docs/releases/v0.12.0-evidence.md` + `docs/releases/v0.12.0-artifacts.md`）+ phase-19 §6 AC1-6 据 19.1–19.6 真实交付勾选（含 AC5 ratify 结论或 documented 未决）+ §8 DoD + `docs/s2v-adapter.md`（Phase 19 Draft→Done / Tasks 0→7 / ADR-023 状态 / BDD row 追加 / Phase 18 forward-ref `phase-future.{vector-retrieval-integration,embedding-provider-full}` 解除）+ `docs/s2v-adapter.md` 19.7 行；合入后据用户授权 push v0.12.0 annotated tag 触发 `release.yml`。`cargo test --workspace`（默认 feature）+ `go test ./...` 不退化；D2 lint `--touched master` 触及行 0 未标注命中。README v0.12 Quick start 加语义召回 example（`contextforge search --semantic "<query>"`）反映 task-19.3/19.4 落地的真实 CLI/通路。

## 3. Scope

### In Scope

- **修改 `README.md`** — 顶部新增 `## What's new in v0.12.0` 段：端到端语义检索（embedding provider + 默认 backend 生产 wiring + `/v1/search?semantic=true` + eval `--semantic`）；Quick start 加语义召回 example `contextforge search --semantic "<query>"`（命令形态以 task-19.3/19.4 实际落地为准）；据 19.1/19.5 真实状态写范围（live recall + ADR ratify，或 deterministic 缺省 provider 跑通 + real recall defer 的诚实声明）。
- **修改 `RELEASE_NOTES.md`** — 顶部新增 `## v0.12.0 (<date>) — vector-retrieval-integration` 段（Highlights / Upgrade path / Rollback path / contract 版本声明）。
- **新建 `docs/releases/v0.12.0-evidence.md`** — 合入记录（task-19.1–19.7 PR 列表）+ 真实 `SemanticRecall@K` 实测表（数据源 `docs/spikes/phase-19-real-recall.md`，或诚实记 deterministic 缺省 provider 数据 + real defer）+ 验证证据（cargo/go test + smoke v9 + D2 lint）+ 平台矩阵 + ADR-014 第十次激活 record + tag/镜像 SHA 待填（post-tag-push backfill，承 v0.8/v0.10/v0.11 pattern）。
- **新建 `docs/releases/v0.12.0-artifacts.md`** — 二进制档 / 源码档 / 验证脚本 / 镜像推送 / ADR 状态记录表 / 版本声明 / 存档指引（仿 `docs/releases/v0.11.0-artifacts.md` 结构）。
- **修改 `docs/specs/phases/phase-19-vector-retrieval-integration.md`** — §6 AC1-6 据 19.1–19.6 真实交付勾选 `[x]`（AC5 含 ratify 结论或据实测 documented 未决，禁据合成 ratify，ADR-013）；§8 Definition of Done 据真实状态勾选；顶部 `**Status**: Draft → Done`。
- **修改 `docs/s2v-adapter.md`** — §Phase 索引 Phase 19 `Draft → Done` + `Tasks 0 → 7`；§Task 总索引追加 19.1–19.7 行；§ADR 索引 ADR-023 状态更新（Accepted 或据实测维持）；§BDD Feature 索引追加 `test/features/phase-19-vector-retrieval-integration.feature` 行；Phase 18 forward-ref `[SPEC-OWNER:phase-future.vector-retrieval-integration]` + `[SPEC-DEFER:phase-future.embedding-provider-full]` 解除标注（**不溯改 Phase 18 spec 正文，D5**）。
- **修改 `scripts/console_smoke.sh`** — v9 final 校准（承 task-19.4 30-step；closeout 范围内仅做一致性确认，step 内容由 task-19.4 落地）。
- **修改 `docs/specs/tasks/task-19.7-closeout-v0.12.0.md`** — 本 spec 顶部 `Status` + §6 AC + §7 追踪表 + §10 Completion Notes 据实回填。

### Out of Scope（[SPEC-DEFER] / [SPEC-OWNER]）

- **Reranker (cross-encoder)** [SPEC-DEFER:phase-future.reranker]：承 Phase 19 §不在 scope。
- **Hybrid scoring (BM25 + Vector fusion)** [SPEC-DEFER:phase-future.hybrid-scoring]：本 phase ship 语义路径单独 + BM25 fallback，fusion 后续。
- **Remote embedding provider（OpenAI / Cohere）** [SPEC-DEFER:phase-future.embedding-provider-remote]：本 phase 仅本地 provider。
- **多 backend 同时生产可用** [SPEC-DEFER:phase-future.multi-backend-production]：仅选定 1 默认 backend。
- **Vector index 增量更新** [SPEC-DEFER:phase-future.vector-incremental-index]：承 Phase 18，默认全量 reindex。
- **Console UI 端语义召回 explain panel** [SPEC-OWNER:phase-future.console-semantic-explain]：cross-repo Console 领域，本 task 仅评估通知。
- **embedding provider trait / deterministic + real provider 落地** [SPEC-OWNER:task-19.1-spike-embedding-provider]：本 task 仅消费其交付做 release 文档。
- **默认 backend 生产 wiring** [SPEC-OWNER:task-19.2-default-backend-wiring]。
- **proto semantic flag + gRPC semantic path + Go handler** [SPEC-OWNER:task-19.3-semantic-search-api]。
- **smoke v9 30-step + eval `--semantic` CLI** [SPEC-OWNER:task-19.4-smoke-v9]。
- **真实 dogfood `SemanticRecall@K` 实测** [SPEC-OWNER:task-19.5-real-recall-eval]。
- **ADR-023 ratify + ADR-006 A1 转正 + ADR-008 amend** [SPEC-OWNER:task-19.6-adr-023-ratify]。

## 4. Actors

- **主 agent**：closeout 文档 + release docs + v0.12.0 tag push（经用户授权）。
- **`release.yml`**：v0.12.0 annotated tag push（`v*` pattern）触发 Docker Buildx → ghcr login → build + push `ghcr.io/tajiaoyezi/contextforge-daemon:v0.12.0` + `:latest`。
- **下游 task-19.1–19.6**：本 task 消费其交付（embedding provider / 默认 backend wiring / semantic API / smoke v9 / 真实 recall / ADR ratify）汇成 release。
- **post-tag-push backfill PR**：填实 tag SHA + release.yml run ID（承 v0.8/v0.10/v0.11 pattern）。
- **用户**：v0.12.0 outward-facing tag push 授权方（USER-AUTHORIZED-ONLY）。

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/specs/phases/phase-19-vector-retrieval-integration.md`（§6 AC1-6 / §8 DoD / §7 R1 stop-condition）
- `docs/specs/tasks/task-18.9-release-v0.11.0-closeout.md`（closeout task pattern 模板）
- `docs/releases/v0.11.0-evidence.md` + `docs/releases/v0.11.0-artifacts.md`（release docs 结构模板）
- 各 sibling task spec：`../tasks/task-19.1-spike-embedding-provider.md` / `../tasks/task-19.2-default-backend-wiring.md` / `../tasks/task-19.3-semantic-search-api.md` / `../tasks/task-19.4-smoke-v9.md` / `../tasks/task-19.5-real-recall-eval.md` / `../tasks/task-19.6-adr-023-ratify.md`（消费其 §10 Completion Notes 真实状态）
- `docs/decisions/adr-023-vector-backend-default.md`（task-19.6 ratify 后的 Status）+ `docs/decisions/adr-006-recall-eval-acceptance-gate.md`（Amendment A1 转正状态）+ `docs/decisions/adr-008-core-library-selection.md`（embedding provider crate amend）
- `docs/decisions/adr-013-no-fake-evidence.md`（禁据合成 recall ratify / 禁 fake `[x]`）+ `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（D1-D5 第十次激活）
- `.github/workflows/release.yml`（v* tag push → ghcr）+ `docs/spikes/phase-19-real-recall.md`（task-19.5 真实数据，feed evidence 表）

### 5.2 诚实口径（ADR-013，据 19.1/19.5 真实交付二选一）

- **路径 A（端到端语义检索 live）**：real embedding provider 两平台可构建 + task-19.5 产出真实 `SemanticRecall@K`。则 v0.12.0 = 「端到端语义检索 ship」；phase-19 §6 AC1-6 全 `[x]`；ADR-023 据 task-19.6 翻 `Accepted`（若实测达 SemanticRecall@10 ≥0.70 gate）；README 加 `--semantic` live example。
- **路径 B（诚实缩范围 defer）**：real embedding provider 受平台/模型门槛受阻（Phase 19 §7 R1 stop-condition）。则 deterministic 缺省 provider 跑通 wiring/smoke（AC1-4 met，标注用 deterministic 缺省 provider）；real recall + ADR-023 ratify 据实测 documented 未决（AC5 partial/deferred），仿 task-18.9 缩范围声明。**不把未达项标 `[x]`，不据合成 recall 翻 ADR Accepted（ADR-013）**。
- 路径选择据 task-19.1 §10 + task-19.5 §10 + task-19.6 §10 的真实结论，本 task closeout 时定。

### 5.3 release docs 内容口径

- `docs/releases/v0.12.0-evidence.md`：合入记录（19.1–19.7 PR）+ 真实 `SemanticRecall@K` 实测表（task-19.5 数据源；路径 B 则记 deterministic 缺省 provider 数据 + real defer）+ cargo/go test + smoke v9 + D2 lint + 平台矩阵 + ADR-014 第十次激活 record。
- `docs/releases/v0.12.0-artifacts.md`：二进制档（Go 控制面 + Rust 数据面，embedding/backend feature gate 说明）+ 源码档（master tag SHA + Phase 19 代码路径）+ 验证脚本 + 镜像推送 + ADR 状态记录表 + 版本声明（contract bump 评估：若 SearchResponse 加 `vector_score`/`embedding_provider` add-only 字段落生产）+ 存档指引。
- tag SHA / release.yml run ID 在 closeout PR 合入时**待填**（先于 tag push），post-tag-push backfill PR 填实。

## 6. Acceptance Criteria

- [x] **AC1**: v0.12.0 release docs 四文件（`README.md` §v0.12.0 含语义召回 Quick start example + `RELEASE_NOTES.md` §v0.12.0 + `docs/releases/v0.12.0-evidence.md` 新 + `docs/releases/v0.12.0-artifacts.md` 新）落地，**路径 A（端到端语义检索 live）**诚实写：语义路径 opt-in（`/v1/search?semantic=true` + `eval run --semantic`），默认构建用 0-dep deterministic provider + brute-force（wiring），real fastembed provider feature-gated（real recall @10=0.9333）。**注：`contextforge search --semantic` 不存在**——Quick start 用实际落地的 REST `?semantic=true` + `eval run --semantic`（ADR-013：不写不存在的命令）— verified by **TEST-19.7.1**
- [x] **AC2**: phase-19 §6 AC1-6 全 `[x]`（据 19.1–19.6 真实交付；AC5 ADR-023 ratify Accepted 据真实非合成 recall @10=0.9333，ADR-013）+ §8 DoD 真实状态 + 顶部 `Status: Draft → Done` — verified by **TEST-19.7.2**（无 fake `[x]`）
- [x] **AC3**: `docs/s2v-adapter.md` Phase 19 `Draft → Done` + `Tasks 7` + 19.1–19.7 Task 行（19.7 Done）+ ADR-023 Accepted + BDD `phase-19-vector-retrieval-integration.feature` 行（PR #141 已建）+ Phase 18 forward-ref `→ Phase 19 解除`标注（adapter 内，不溯改 Phase 18 spec 正文，D5）— verified by **TEST-19.7.3**
- [x] **AC4**: 既有不退化 — 本 closeout 纯文档（零代码改动）；默认 `cargo test --workspace`（embedding/backend feature 默认不启用）+ `go test ./...` 不退化（CI cargo-test/go-test gate 复核）— verified by **TEST-19.7.4**
- [x] **AC5**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched master` PR 触及行 0 未标注命中（D1-D5 第十次激活：D1 mapping / D2 lint / D3 verified-by / D4 自治 / D5 Phase 1-18 不溯改，Phase 18 spec diff 空）— verified by **TEST-19.7.5**（CI spec-lint gate + 本 PR body 记 D1-D5）
- [x] **AC6**: v0.12.0 annotated tag push（**经用户授权——USER-AUTHORIZED-ONLY，stop-condition (c) 已获授权 2026-05-30**）触发 `release.yml` → ghcr `ghcr.io/tajiaoyezi/contextforge-daemon:v0.12.0` + `:latest` 构建推送 — verified by **TEST-19.7.6**：tag SHA `dcbe09bba4bc6f636186d7df1f32447ab96d1ddc` + `release.yml` run `26685041851`（**success**）+ 镜像 digest `sha256:6f0ae8fbf956dcdeaeea29e0f0a98f9dbbada8d3ca8bf0ae5c3c79fa448eca6d`（本 backfill PR 填实）。

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-19.7.1 | v0.12.0 release docs 4 文件 + 路径 A 诚实口径（实际命令，无 `search --semantic`） | README / RELEASE_NOTES / v0.12.0-{evidence,artifacts}.md | Done |
| TEST-19.7.2 | phase-19 §6 AC1-6 [x] + Status Done（据实，无 fake `[x]`） | docs/specs/phases/phase-19-vector-retrieval-integration.md | Done |
| TEST-19.7.3 | adapter Phase 19 Done / Tasks 7 / ADR-023 Accepted / BDD row / Phase 18 forward-ref 解除 | docs/s2v-adapter.md | Done |
| TEST-19.7.4 | 零代码改动（cargo/go test 天然不退化，CI gate 复核） | 全 workspace | Done |
| TEST-19.7.5 | D2 lint --touched master 0 hits + D1-D5 第十次激活 | scripts/spec_drift_lint.sh + closeout PR body | Done |
| TEST-19.7.6 | v0.12.0 tag push + release.yml ghcr 构建 | git tag + GHA release.yml | Done（tag `dcbe09b` + run `26685041851` success + digest `sha256:6f0ae8…d2990`） |

## 8. Risks

- **R1（高）embedding provider 受阻 → 缩范围 vs phase 原始 AC**：若 task-19.1 real provider 两平台不可构建（Phase 19 §7 R1 stop-condition），AC5（真实 recall + ADR ratify）未达原始标准。
  - **缓解**：诚实标 partial/deferred（非 fake `[x]`，ADR-013）；deterministic 缺省 provider 跑通 wiring/smoke（AC1-4 met，标注）；real recall + ADR-023 ratify 据实测 documented 未决，仿 task-18.9 缩范围 closeout pattern；README/RELEASE_NOTES 范围口径与 19.1/19.5 真实交付一致。
- **R2（中）v0.12.0 tag push 对外发布**：`release.yml` → ghcr 镜像 + GitHub Release，outward-facing。
  - **缓解**：tag push USER-AUTHORIZED-ONLY，须用户显式授权方 push；镜像 = 默认 feature 构建（embedding/backend feature 默认关闭则镜像行为同 v0.11.0 BM25-only baseline；若默认含 hnsw 则文档明记）；可 `git tag -d v0.12.0` + 删 release 回退。
- **R3（中）release docs tag SHA / run ID 待填**：closeout PR 合入先于 tag push。
  - **缓解**：待填值 + post-tag-push backfill PR 填实（承 v0.8/v0.10/v0.11 pattern）；Dockerfile workspace member COPY 完整性预检（承 v0.11.0 release.yml `bench/` manifest 教训，PR #139）。
- **R4（低）contract 字段变更对外**：若 task-19.3 SearchResponse 加 `vector_score`/`embedding_provider` add-only 字段落生产，需 contract 版本声明 + cross-repo Console 通知评估。
  - **缓解**：仿 ADR-015/022 add-only pattern；conformance test 守既有 endpoint 不破坏（task-19.3 落地）；artifacts §版本声明记 contract bump 与否；cross-repo Console 通知 [SPEC-OWNER:phase-future.console-semantic-explain]。

## 9. Verification Plan

```bash
cargo test --workspace        # 默认 feature，embedding/backend gated 不入编译，0 failed
go test ./...                 # 含 internal/eval + Go semantic handler，全 PASS
bash scripts/console_smoke.sh # v9 30-step（承 task-19.4），semantic search + eval --semantic
bash scripts/spec_drift_lint.sh --touched master   # 0 unannotated hits
# 合入后（经用户授权 push）：
git tag -a v0.12.0 -m "v0.12.0 — Phase 19 vector-retrieval-integration (end-to-end semantic search)"
git push origin v0.12.0       # → release.yml ghcr 构建 ghcr.io/tajiaoyezi/contextforge-daemon:v0.12.0 + :latest
```

## 10. Completion Notes (s2v 6 项标准)

- **完成日期**：2026-05-30（release docs + 收口；tag push 待用户授权）
- **改动文件**：`README.md`（§v0.12.0 + 语义 Quick start）/ `RELEASE_NOTES.md`（§v0.12.0）/ `docs/releases/v0.12.0-evidence.md`（新）/ `docs/releases/v0.12.0-artifacts.md`（新）/ `docs/specs/phases/phase-19-vector-retrieval-integration.md`（§6 AC1-6 [x] + Status Draft→Done）/ `docs/s2v-adapter.md`（Phase 19 Done + Tasks 7 + ADR-023 Accepted + Phase 18 forward-ref 解除 + 19.7 行）/ `docs/specs/tasks/task-19.7-closeout-v0.12.0.md`（本 spec）。注：`scripts/console_smoke.sh` v9 由 task-19.4 落地，本 closeout 一致性确认**无需改动**；本 closeout 纯文档（零代码改动）。
- **commit 列表**：见本 task PR（分支 `chore/phase-19-closeout-v0.12.0`）；合入后以 merge commit 为准
- **路径选择（§5.2）**：**路径 A（端到端语义检索 live）**——real provider（fastembed）两平台可构建（task-19.1 R1 未触发）+ task-19.5 真实 `SemanticRecall@10=0.9333≥0.70` + task-19.6 ADR-023 Accepted。无缩范围。诚实限定：默认构建语义路径用 0-dep deterministic provider + brute-force（wiring 正确性），real-model 召回需 `--features embedding-fastembed`；`contextforge search` 无 `--semantic`（语义入口 = REST `?semantic=true` + `eval run --semantic`），README 据实写。
- **§9 Verification 结果**：本 closeout 纯文档 → cargo/go test 天然不退化（CI cargo-test/go-test gate 复核）；D2 lint `--touched master` 0 未标注命中（CI spec-lint gate）。smoke v9（task-19.4 #145 落地）诚实记录：`bash -n` + 标号 + step 29/30 行为经 task-19.4/19.3 单测验证；本地 WSL REAL 复跑 step 1–25 + 迁号 `[21/30]…[26/30]` 跑通，**既有 step 26**（task-16.1 daemon `kill-9`-restart，非 Phase 19）在非交互 WSL 下停住（exit 0，无 FAIL 断言），未达 step 29/30——完整 daemon-restart REAL marker 由合规 Linux/release smoke 复跑定（见 `docs/releases/v0.12.0-evidence.md` §3b）。**v0.12.0 tag SHA + release.yml run ID + 镜像 digest 待 post-tag-push backfill 填**（tag push 经用户授权后）。
- **AC6 / tag push 状态（stop-condition c）**：closeout docs PR（#148）三门绿后自主合入；**v0.12.0 annotated tag push 经用户显式授权（2026-05-30）** 后 push（tag SHA `dcbe09bba4bc6f636186d7df1f32447ab96d1ddc`）→ `release.yml` run `26685041851` **success** → `ghcr.io/tajiaoyezi/contextforge-daemon:v0.12.0` + `:latest` @ `sha256:6f0ae8fbf956dcdeaeea29e0f0a98f9dbbada8d3ca8bf0ae5c3c79fa448eca6d`。本 backfill PR 填实 AC6 / TEST-19.7.6 + evidence §7 + artifacts §4。
- **剩余风险 / 未做项**：路径 A 无缩范围。reranker [SPEC-DEFER:phase-future.reranker] / hybrid scoring [SPEC-DEFER:phase-future.hybrid-scoring] / remote provider [SPEC-DEFER:phase-future.embedding-provider-remote] / multi-backend production [SPEC-DEFER:phase-future.multi-backend-production] / hnsw graph persistence [SPEC-DEFER:phase-future.hnsw-graph-persistence] 各自后置；Console 语义召回 explain [SPEC-OWNER:phase-future.console-semantic-explain]（cross-repo，仅评估通知）；console-api `/v1/search` 转发 `?semantic=true` 到 gRPC（现仅 daemon rest.go 转发）属 task-19.5 follow-up 评估。
- **下游 task 影响**：v0.12.0 release ship 后端到端语义检索 live（opt-in）；后继 phase 消费 embedding provider seam + 生产语义 wiring；post-tag-push backfill PR 填 tag SHA / run ID / digest。
- **ADR-014 D1-D5 第十次激活**：D1 task-19.7 §6 AC1-6 ↔ phase-19 §6 AC6 mapping；D2 lint 0 unannotated（CI spec-lint）；D3 每 AC verified-by TEST-19.7.x；D4 主 agent 自治据真实数据收口；D5 Phase 1-18 spec 未溯改（Phase 18 AC3/AC4 解决记录写在 ADR-023 Amendment + adapter 行，`git diff --stat master -- docs/specs/phases/phase-18-*` 空）。
