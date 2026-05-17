# Task `4.1`: `retriever — BM25 / metadata / filter 检索`

> ⚠️ **Status: Draft** — 禁止进入实施。进入前清零 `<TBD-by-user>`、审 §6/§7/§9、Status→Ready。详见 `docs/s2v/standard.md` §10.5.1。

**Status**: Draft

**Priority**: P0
**Owner**: `<TBD-by-user>`
**Related Phase**: Phase 4 (retrieval-explain)
**Dependencies**: Phase 2 (index-core)

## 1. Background

可解释检索的检索内核：在 Phase 2 的 Tantivy + SQLite 索引上做 BM25 全文 + metadata + filter 检索（PRD §Decisions Log D2 P0 = 可解释 BM25/metadata baseline，不依赖向量）。

## 2. Goal

`retriever` 支持 BM25 全文检索 + metadata 检索 + filter（source_type / language / collection / agent_scope / time），返回 Top-K；空/错误 query 返回空结果不 panic；满足 PRD §Constraints 性能（10 万 chunk P95 < 500ms，不含 embedding/远程）。

## 3. Scope

### In Scope

- `<TBD-by-user>`

### Out Of Scope

- `<TBD-by-user>`

## 4. Users / Actors

- `<TBD-by-user>`

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Decisions Log D2 / §Constraints 性能 / §Technical Approach REST/MCP search 契约）
- `docs/specs/phases/phase-4-retrieval-explain.md`
- `docs/specs/tasks/task-2.4-indexer.md`
- `docs/decisions/adr-002-sqlite-tantivy-layered-storage.md`
- `test/features/retriever.feature`

### 5.2 Imports

- `<TBD-by-user>`

### 5.3 函数签名

- `<TBD-by-user>`

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：完整写出 AC；`- [ ] **AC<N>** (PRD §<ref>): <内容>`；PRD 未写标 `(本 task 新增)`；review 改内容不删注释；严禁混合写法 -->

- [ ] **AC1** (PRD §Decisions Log D2): BM25 全文检索 + metadata 检索在 Tantivy+SQLite 索引上可返回 Top-K（v0.1 P0，不依赖向量/embedding）。
- [ ] **AC2** (PRD §Technical Approach REST/MCP 契约): filter 支持 source_type / language / collection / agent_scope / time，与 search 请求契约一致。
- [ ] **AC3** (PRD §Implementation Phases Phase 4 Exit Criteria): 错误/空 query 返回空结果，不 panic。
- [ ] **AC4** (PRD §Constraints 性能 / §Success Metrics 次指标): 已索引、未调 embedding/reranker/远程 时 10 万 chunk 内 BM25/metadata/filter P95 < 500ms（基准在 Phase 8 回归）。
- [ ] **AC5** (PRD §Technical Risks R8): 支持 configurable tokenizer + path/filename/symbol 单独 field 并 boost + exact phrase/symbol search 接口（CJK-aware/n-gram fallback 接入点）。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 BM25+metadata Top-K | SCEN-4.1.1 | TEST-4.1.1 | - | unit-test | Not Started |
| AC2 filter 契约一致 | SCEN-4.1.2 | TEST-4.1.2 | - | unit-test | Not Started |
| AC3 空/错误 query 不 panic | SCEN-4.1.3 | TEST-4.1.3 | - | unit-test | Not Started |
| AC4 性能 P95<500ms | SCEN-4.1.4 | TEST-4.1.4 | - | unit-test | Not Started |
| AC5 tokenizer/boost/exact | SCEN-4.1.5 | TEST-4.1.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R3**（召回率）+ **R8**（中英文/代码符号检索）：tokenizer 可配置、symbol field boost；分场景 recall eval 在 Phase 8。关联 PRD §Open Questions **O11**。

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
