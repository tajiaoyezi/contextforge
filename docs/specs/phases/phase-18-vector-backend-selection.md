# Phase 18 · vector-backend-selection

**Status**: Draft

> Phase Spec（s2v full-standard §8.2）。本 phase 解决 PRD §Open Questions **O2 向量后端最终选型**（源自 D2 / 技术 TBD：SQLite vec ext / Qdrant local / LanceDB / 内嵌 HNSW，需核心开发在 Phase 5-6 期间做 spike 压测后定）。
>
> ⚠️ **Status: Draft** — 本 spec 由 `/s2v-add phase` 占位渲染，业务字段全部 `<TBD-by-user>`。主 agent / 用户按 ADR-012 §2A 业务承诺审核 + 填实下述各节后改 Status: Draft → Ready，方可进入 §3 工作流（spike task 起草）。
>
> **入读顺序（必读）**：本 phase spec → `docs/prds/context-forge.prd.md` §Open Questions O2 + §Decisions Log D2 + §Constraints Performance/Compatibility → AGENTS.md §3 / §4 Gate / §8 卡住协议 → `docs/decisions/adr-014-cross-phase-exit-criteria-validation.md`（自 Phase 10 起本 phase 必须激活 D1-D5 cross-validation gate）。
>
> **本 phase 关键拍板点（§2A 用户领域，不可由主 agent 自决）**：
> 1. **候选集**：是否锁 4 路（SQLite vec / Qdrant local / LanceDB / 内嵌 HNSW）或剔补
> 2. **评测口径**：10 万 chunk recall@5/10 + P95 latency + 单机内存 RSS + cold-start time + 索引重建耗时 — 哪几项是 must / nice-to-have
> 3. **判据排序**：性能优先 / 嵌入门槛低优先 / 单文件可移植优先 / 单机资源占用优先 — 当 4 backend 在不同维度交错胜出时的优先级
> 4. **集成深度**：spike 阶段是否就推 trait 抽象 + 1 backend 默认实现（其他留 v0.x.y），还是先 4 backend 全跑数据再回头决策抽象层
> 5. **数据集来源**：合成数据 / O6 golden questions / 真实仓库 corpus — spike 跑哪份；golden questions 数据集 O6 是否成熟度足够（若不够，本 phase 是否同时启动 O6）
>
> **本 phase 不引入新 ADR 的前提**：spike 结论指向既有 D2 「向量后端做 provider 抽象，v0.1 不强依赖」延伸；如 spike 推翻 D2（如发现某 backend 必须深度耦合 SQLite metadata 路径无法做干净 trait 抽象），则本 phase 收口 PR 需新增 ADR-023（spike 数据驱动的决策）。

## 1. 阶段目标

<TBD-by-user>

<!-- 渲染规则（§2A 业务承诺审核期填）：
     - 一句话陈述本 phase 完成后应该成立的事实（例：「v0.x ship 后 ContextForge 自带向量召回 trait + 1 个默认 backend 实现 + spike 数据 evidence 文档 + recall eval gate 含语义召回路径」）
     - 列出 ≥3 个具体可观测的 phase exit criteria（spike 结果 / trait 抽象 / 默认 backend 实现 / eval 接入），每条对应 §6 一条 AC
     - 明确 v0.x 是 minor (v0.11) 还是 patch；如延期 v1.x ship 标记
-->

## 2. 业务价值

<TBD-by-user>

<!-- 渲染规则（§2A 审核期填）：
     - 直接对接 PRD §Core Capabilities / §Success Metrics — 哪条 metric 因本 phase 提升 ≥X%
     - 直接对接 PRD §Decisions Log D2 — 是 D2 「provider 抽象」的延伸还是修正
     - 反指标自觉（PRD §Anti-metrics）：本 phase 不能破坏「不能为提升命中率牺牲可解释性」/ secret redaction / 本地优先 3 条
     - 列出本 phase 不在 scope 的关联工作（例：CJK tokenizer / golden questions 扩充 / reranker），标 [SPEC-DEFER:phase-future.<topic>]
-->

## 3. 涉及模块

<TBD-by-user>

<!-- 渲染规则（§2A 审核期填）：
     - 每个 task 的核心改动文件列出（按 Source areas 路径）— 例 core/src/retriever/vector.rs / core/src/embedding/<provider>.rs / proto/contextforge/v1/<service>.proto
     - 新增 trait + 实现路径
     - 新增 SQLite migration（如选 sqlite-vec 需要 .so / .dll 动态加载）
     - 新增依赖（Cargo.toml）— 注意 R7：subagent 不得自改 lockfile
     - eval harness 扩展点（internal/eval/eval.go 加语义召回评测）
     - 新增 BDD feature 文件路径
     - 涉及 ADR：D2 是否需 amendment ADR-023
-->

## 4. 任务清单

<TBD-by-user>

<!-- 渲染规则（§2A 审核期填，待 /s2v-add task 起草后补真表）：
     Spec 列路径用 `../tasks/task-X.Y-<name>.md` phase 相对路径

| Task | 模块 | Spec |
|---|---|---|
| 18.1 | <例：retriever/vector-trait> | `../tasks/task-18.1-<name>.md` |
| 18.2 | <例：spike-harness> | `../tasks/task-18.2-<name>.md` |
| ... | ... | ... |

建议 task 拆分维度（4 backend 4 task + harness + trait + integration，按 §2A 决策剪裁）：
- 18.1 vector retrieval trait 抽象 + 占位 NoopVectorBackend + retriever 接口扩展（不引入真 backend；先冻结契约）
- 18.2 spike harness — 10 万 chunk 合成 + golden corpus + recall/P95/RSS/cold-start measurement runner
- 18.3 backend spike A（例：SQLite vec ext）
- 18.4 backend spike B（例：Qdrant local 嵌入式）
- 18.5 backend spike C（例：LanceDB）
- 18.6 backend spike D（例：内嵌 HNSW — hnswlib-rs / hora-search）
- 18.7 decision ADR-023（如需）+ default backend 选定 + 集成路径
- 18.8 eval harness 语义召回评测 + recall gate 接入（D6 一等验收门扩展）
- 18.9（收口）phase smoke / release v0.11 prep
-->

## 5. 依赖关系

<TBD-by-user>

<!-- 渲染规则（§2A 审核期填）：
     - **依赖**：列出本 phase 启动前必须 merged 的 phase / task / ADR / 外部信号
       - Phase 4（retrieval-explain）— 复用 retriever 抽象与 result schema；冻结契约
       - Phase 6（cli-api-export）— 复用 daemon / CLI 端点；语义召回经现有 search 接口 plug-in
       - Phase 8（eval-and-reliability）— 复用 internal/eval recall harness；本 phase 扩 SemanticRecall 指标
       - PRD §Open Questions O2（resolved by Phase 18 closeout） / O6（golden questions 成熟度；本 phase 是否同时 unlock 需 §2A 决定）
       - ADR-008 core-library-selection — 引入新 backend dep 时同步 amendment
       - ADR-006 recall-eval-acceptance-gate — 语义召回纳入门禁的影响面
     - **可并行**：本 phase 内 spike task 之间默认互相独立（共享 harness 数据集 + 评测 runner，写文件互不相交）
     - **Phase 内推荐序**：trait 冻结 (18.1) → harness 就绪 (18.2) → 4 spike 任一序 (18.3-18.6 并行) → decision (18.7) → eval 接入 (18.8) → 收口 (18.9)
-->

## 6. 阶段级验收标准 + 端到端 smoke

<TBD-by-user>

<!-- 渲染规则（C1 集成兜底门，**禁止留 <TBD>** 进入实施 — 必须 §2A 审核期填实）：
     每条 AC 显式 owner（ADR-014 D3）：`verified by phase-smoke step M` 或 `verified by task-X.Y §6 AC M (file:line)`

阶段级验收标准（每条对应 §1 phase exit criteria + 含 verified by owner）：

- [ ] AC1：<例：vector retrieval trait 抽象 ship — trait Vector{Backend,Indexer,Searcher} 三 trait 落地 core/src/retriever/vector/，含 NoopVectorBackend 占位实现 + 既有 BM25 检索不退化（cargo test --workspace 0 failed）> — verified by `core/src/retriever/vector/mod.rs::tests` + phase-smoke step 1
- [ ] AC2：<例：spike harness 跑通 — 10 万合成 chunk + golden corpus 双数据集；recall@5/10 + P95 + RSS + cold-start 4 维测量；至少 4 backend 各 1 次完整跑通 evidence 落 docs/spikes/phase-18-<backend>.md> — verified by phase-smoke step 2
- [ ] AC3：<例：spike 决策落 ADR-023 — Status: Proposed → Accepted at closeout PR；含 4 backend trade-off matrix + 选定的默认 backend + 排除理由> — verified by closeout PR diff 含 ADR-023
- [ ] AC4：<例：默认 backend 集成实现 + retriever 端集成 + smoke v9 — N step 含 vector recall path + 既有 27/28 step Phase 16/17 不退化> — verified by phase-smoke step 3
- [ ] AC5：<例：eval harness 语义召回评测纳入 — internal/eval/eval.go 加 SemanticRecall@K 指标 + recall gate 含 D6 阈值（具体阈值在 §2A 拍板）> — verified by phase-smoke step 4
- [ ] AC6：ADR-014 cross-validation gate 全套通过 — D1 mapping table + D2 lint 0 unannotated hits + D3 verified-by 显式 + D4 主 agent 自治 + D5 历史 Phase 1-17 不溯改 — verified by 本 PR body 含 D1 + D2 + D3 + D5 diff

端到端 smoke：

```bash
# step 1 — vector trait ship 不退化 cargo test
cargo test --workspace
# 期望：所有 既有测试通过 + 新 vector trait 测试通过

# step 2 — spike harness 跑通 4 backend
bash scripts/spike_vector_backends.sh
# 落 docs/spikes/phase-18-{sqlite-vec,qdrant-local,lancedb,hnsw}.md 4 份 evidence

# step 3 — default backend 集成 smoke v9
bash scripts/console_smoke.sh
# 27 step 不退化 + step 28 Phase 17 不退化 + step 29-30 加入 vector search roundtrip

# step 4 — eval harness 语义召回评测
contextforge eval run --golden test/fixtures/eval/golden-v0.11.jsonl --semantic
# 期望：报告含 SemanticRecall@5 / @10 + 阈值 PASS

# step 5 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
# 0 unannotated hits

# step 6 — release smoke
bash scripts/release_smoke.sh
# v0.11.0 prep ok
```
-->

## 7. 阶段级风险

<TBD-by-user>

<!-- 渲染规则（§2A 审核期填）：
     列出 phase 级风险（不在 task 级 §8）+ 缓解措施：
     - 选定的 backend 在 Linux / macOS 编译困难（如 sqlite-vec .so 平台二进制；Qdrant 嵌入式资源占用）
     - eval harness 数据集 (O6) 不够 → 退化为合成 corpus 评测可信度下降
     - trait 抽象层加 retriever 路径性能损耗（动态分派 vs 静态泛型）
     - 新 dep 引入触发 ADR-008 amendment（如 Qdrant 走 sled / RocksDB 与 D2 SQLite/Tantivy 分层冲突）
     - cross-platform compatibility — Phase 1 §Constraints Supported platforms 约束（Linux/WSL2 P0；macOS/Windows nice-to-have）
     - v0.11 ship 时间线 — spike 失败回退到 D2 「provider 抽象但不实装」的影响
-->

## 8. Definition of Done

<TBD-by-user>

<!-- 渲染规则（§2A 审核期填，与 §6 AC 形成完整收口）：
     - 所有 task spec Status: Done
     - §6 phase AC 全 [x]
     - 端到端 smoke 全 PASS
     - 涉及 ADR Status: Accepted（如新增 ADR-023）
     - PRD §Open Questions O2 标记 resolved by Phase 18 closeout
     - adapter §Phase 状态索引 Phase 18 Status: Done
     - v0.11.0 (or v0.10.x patch) RELEASE_NOTES + evidence + artifacts 落盘
     - spike evidence 文档 docs/spikes/phase-18-*.md ≥4 份
     - cross-repo follow-up（如涉及 Console UI 端配套字段，例如 SearchResponse 新增 vector_score 字段需 Console PR 适配）
-->
