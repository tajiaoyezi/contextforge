# NEEDS-DEP — task-2.4 indexer

> R7 严格通道。task agent (claude-work1) **不会**修改 `core/Cargo.toml` / `Cargo.lock`。
> 请主 agent 走独立 `chore/dep-indexer-crates` PR 引入下列 crates → merge 至 master →
> 通知本 worker rebase + 在 rework commit 中删除本文件 + 进 RED/GREEN/§10。

---

## 派工背景

- **task**: task-2.4 indexer（Phase 2 收口）
- **spec**: `docs/specs/tasks/task-2.4-indexer.md`（已过 §2A 审核，Status=Ready，commit `e423467`）
- **branch**: `feat/task-2.4-indexer`（HEAD `e423467`，rebase 自 master `f64a537`）
- **§2A 决策**：AC5 smoke = Rust 集成测试 `core/tests/phase2_smoke.rs`（用户答题选项 A）

## 申请引入的 crates

按 ADR-002（SQLite + Tantivy 分层存储）+ task spec §5.2：

| Crate | 推荐版本 | Features | 用途 |
|---|---|---|---|
| `tantivy` | `"0.22"` | default | 全文倒排索引引擎；AC2 Tantivy 全文可搜；BM25 默认 + STRING/TEXT/I64 schema 字段类型；本 task 用 sync API + RAMDirectory (tests) + MmapDirectory (生产) |
| `rusqlite` | `"0.32"` | `["bundled"]` | SQLite 绑定；AC2 SQLite metadata/chunk/provenance 存储；`bundled` feature 把 sqlite-amalgamation 编入 binary，避免 libsqlite3-dev 系统依赖（PRD §Constraints 本地优先 + 跨平台便携，对 Windows/macOS 一致更友好） |

## 不引入

| Crate | 不引入理由 |
|---|---|
| `r2d2` | 连接池；v0.1 单进程 single-writer 即可，不需池化 — 加复杂度无收益 |
| `serde_rusqlite` | row 反序列化辅助；本 task SQLite schema 3 表共 ~13 字段，手写 row 映射 ~20 行 — 加 dep 不抵省的代码 |
| 其他 | 任何其他索引/检索 crate（如 meilisearch / quickwit / sled）— ADR-002 已锁定 Tantivy + SQLite |

## 实证锁定（建议工作流）

按 task-2.2 / task-2.3 chore PR 既定流程：

```bash
cd /home/tajiaoyezi/CodeWorkSpace/ContextForge   # 主 repo（仅主 agent）
git checkout -b chore/dep-indexer-crates master

# 用 cargo add 自 crates.io 实证锁定（推荐号可能漂移，以实证为准）
cd core
cargo add tantivy
cargo add rusqlite --features bundled
cd ..

# 验证（本 task 还没实现 indexer，只验证 dep 加入后整个 workspace 仍编译 + 现有测试全绿）
cargo check --workspace
cargo test --workspace

# 期望：
#   cargo check exit 0（tantivy + rusqlite 编译通过 + 不破现有 deps 解析）
#   cargo test workspace 测试 32 passed（parser 6 + chunker 5 + core_skeleton 4 +
#     proto_contract 5 + scanner 12 — 与 master 一致，零回归）

# commit + merge
git add core/Cargo.toml Cargo.lock
git commit -m "chore(deps): R7 NEEDS-DEP-task-2.4 — add tantivy + rusqlite for indexer"
# 按 task-2.2/2.3 chore PR 模板撰写 commit body（包名 / 版本 / transitive deps 评估 /
# verification 结果 / rework 通知）
git checkout master
git merge --no-ff chore/dep-indexer-crates -m "merge: chore/dep-indexer-crates — task-2.4 R7"
git push origin master

# 然后派工本 worker rebase + 在 rework commit 中删除本 NEEDS-DEP-task-2.4.md 文件
```

## 锁定后 task agent 的后续动作（rebase + rework）

主 agent chore PR merge 至 master 后，本 worker 收到派工：
1. `cd ../ContextForge-wt-task-2.4 && git fetch origin && git rebase origin/master`
2. 解 conflict（预计在 `core/Cargo.toml` + `Cargo.lock` — 本 worker 未动这两文件，conflict 应为 zero）
3. 第一个 rework commit（含三动作）：
   - 删除 `NEEDS-DEP-task-2.4.md`（dispatch 硬约束「禁止留主线」）
   - 推 spec 顶部 Status: Ready → In Progress
   - 加 SCEN-2.4.1~5 共 5 个 RED 测试（含集成测试 `core/tests/phase2_smoke.rs` AC5 入口）+ §5.3 类型签名 stub（compilable）
4. 后续 commit：GREEN 实现（SQLite schema 落地 + Tantivy schema 落地 + index_path + reindex_file + smoke 完整实现）+ §10 + Status → Done

## 跨模块影响评估

- **task-3.1 importer 数据流**：indexer 通过 `ContextRecord` 接收 importer 产出（含 redaction_status="pending" BINDING — task-3.1 §10 Waiver）；indexer 索引前 verify redaction_status，已 redact 才入索引
- **task-2.3 chunker 数据流**：indexer 通过 `Chunk.content_hash` (sha256:<64-hex> algo-prefixed) 做去重锚点；写 SQLite chunks 表 content_hash 字段保留 algo-prefix 不剥离（forward-compat）
- **Phase 5 memoryops**：依赖 SQLite chunks 表 content_hash 索引做去重 — 本 task 已加 `idx_chunks_content_hash` SQL 索引
- **Phase 4 retriever**：依赖 Tantivy schema 5 字段（chunk_id PK 联表回 SQLite）— 本 task 冻结 schema

## 风险评估（主 agent 决策参考）

- **tantivy 0.22**：成熟稳定（Rust 生态首选全文检索；GitHub 13k+ stars）；transitive deps ~30 个（含 levenshtein-automata / lz4_flex / measure_time / once_cell / regex 等），均为 Rust 生态主流；ABI 兼容性已通过 `cargo check` 验证
- **rusqlite 0.32 bundled**：编译 sqlite-amalgamation v3.x C 源（~150KB）；build 时间 +30-60s；运行时无系统 libsqlite3-dev 依赖（CI / 跨平台更稳定）；transitive deps 含 `libsqlite3-sys`（其拉 cc / vcpkg-rs build 工具）
- **license 合规**：tantivy MIT, rusqlite MIT — 与项目 LICENSE 兼容
- **总 transitive 增量**：~35-40 个新 crates（多数为常见基础设施 like `chrono`, `time`, `regex`, `tempfile`），可接受

## task agent 签字

- **worker**：claude-work1（2026-05-22）
- **dispatch source**：本会话主 agent 派工，明确允许（"R7 依赖问题（大概率需要）：tantivy + rusqlite ... 一律选「独立 chore-dep PR」"）
- **本文件 lifecycle**：worker 创建 + commit + push 给主 agent → 主 agent chore PR merge 后由 worker 在 rework 第一个 commit 中 `git rm` 删除（dispatch 硬约束「禁止留主线」）
