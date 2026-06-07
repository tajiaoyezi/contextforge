# Task `42.2`: `console-api-source-type-forward — console_data_plane.proto SearchRequest add-only repeated string source_type = 9（既有字段 1-8 号冻结，buf generate）+ core/src/data_plane/search.rs 按 req.source_type 对 populate 后的 hit 做 post-filter（空 → 不过滤 byte-equiv）+ Go internal/contractv1 SearchRequest add-only SourceType []string + internal/consoleapi handleSearch 解析 ?source_type=（query param + body 并集，镜像 ?semantic/?hybrid OR-merge）+ grpcclient 透传到 console_data_plane`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 42 (chunk-source-type-filter)
**Dependencies**: task-42.1（`classify_source_type` + 三构造点 populate `h.source_type`——console post-filter 用 populate 后的 `h.source_type`）/ 既有 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`（`SearchRequest` 字段 1-8 已用 :151-167 / `SearchResultItem.source_file_type=5` :195 响应侧已在）/ 既有 `core/src/data_plane/search.rs`（:337-342 BM25 分支 `SearchFilters::default()` / :374-382 `SearchResultItem.source_file_type: h.source_type`）/ 既有 `internal/consoleapi/handlers.go`（`handleSearch` `?semantic`/`?hybrid` OR-merge forward 范式）+ `internal/contractv1/contractv1.go:112-128`（`SearchRequest` add-only 字段范式，Semantic/Hybrid）+ grpcclient 映射点 / ADR-047（chunk-source-type-filter，本 task 即其 D3 原文实现）/ ADR-015（proto add-only，console SearchRequest source_type=9）/ ADR-024（console-api-semantic-forward）/ ADR-044（console-api-retrieval-signal-forward，请求侧 forward 范式承接）/ ADR-004（空 filter byte-equiv）/ ADR-008（dep add-only，0 新 dep）/ ADR-013 / ADR-012 / ADR-014 D1-D5（第三十三次激活）

## 1. Background

task-42.1 令 retriever 真实派生 + 过滤 source_type，v1 gRPC / v1 REST body 路径（`rest.go:137` 解码完整 proto SearchRequest 含 `filters`）立即生效。但 **console-api `/v1/search` 路径请求侧无法传 source_type filter**：

- **B1 console 响应侧已就绪（真实）**：console 数据面 `SearchResultItem.source_file_type=5`（`console_data_plane.proto:195`）+ `data_plane/search.rs:378 source_file_type: h.source_type.clone()`——task-42.1 populate 后 console 响应**立即显示**真实 source_type，无需改响应侧。
- **B2 console 请求侧缺 source_type（真实）**：console `SearchRequest`（`:151-167`）字段仅 query/workspace_id/agent_scope/retrieval_method/top_k/config_snapshot/semantic(7)/hybrid(8)——**无 source_type**；`data_plane/search.rs:337-342` BM25 分支用 `SearchFilters::default()`（不传 filter）。故 console 用户无法按 source_type 筛选。
- **B3 forward 范式已备（真实）**：`handleSearch` 已有 `?semantic`（ADR-024 / task-20.1）+ `?hybrid`（ADR-044 / task-39.2）的「query param + body 字段 OR-merge → grpcclient 透传」范式；`internal/contractv1.SearchRequest` 已有 Semantic/Hybrid add-only 字段范式。source_type 镜像此（差异：source_type 是 repeated，合并语义为并集）。

本 task 把 console_data_plane `SearchRequest` add-only `source_type=9` + Go `?source_type=` forward 接通，让 console `/v1/search` 用户可按 source_type 筛选。code-local 🟢 可单测，0 新 dep + proto add-only（既有字段号冻结）。

## 2. Goal

(1) **proto add-only**：`console_data_plane.proto` `SearchRequest` add-only `repeated string source_type = 9`（既有字段 1-8 号冻结，注释记 ADR-015 add-only + 空 → 不过滤 backward-compat）+ `buf generate`（Rust prost + Go pb.go）。
(2) **data_plane post-filter**：`core/src/data_plane/search.rs` 按 `req.source_type` 对 BM25/semantic/hybrid 三分支汇总后的 hit 做 post-filter（利用 task-42.1 已 populate 的 `h.source_type`；`req.source_type` 空 → 不过滤 byte-equiv）。在 data_plane 统一 post-filter（而非各分支内）覆盖三检索路径一致。
(3) **Go contractv1 + handleSearch + grpcclient**：`internal/contractv1/contractv1.go` `SearchRequest` add-only `SourceType []string` json `source_type`（镜像 Semantic/Hybrid）；`internal/consoleapi/handlers.go` `handleSearch` 解析 `?source_type=`（repeated query param）+ body `source_type` 并集合并 → `SearchRequest.SourceType`（镜像 `?semantic`/`?hybrid` OR-merge）；grpcclient 映射 `SearchRequest.SourceType` → console_data_plane `SearchRequest.source_type`。

pass bar：proto `SearchRequest.source_type` round-trip + prost wire-tag 字段号 9 in-crate 断言（🟢）；`handleSearch` `?source_type=code&source_type=doc` query param + body `source_type` 并集 → 转发到下游 SearchClient（capturingSearch 断言，镜像 `TestTask201/392`）（🟢）；`data_plane` source_type post-filter（空 → byte-equiv / 非空 → 仅留匹配桶）（🟢）；空 source_type → 不过滤 backward-compat；0 新 dep（ADR-008）；既有 Go / Rust 单测不退化；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`SearchRequest` add-only `repeated string source_type = 9`（注释：task-42.2 add-only，按 source_type 桶过滤 chunk 检索结果，空 → 不过滤 backward-compat，ADR-015 add-only + parity v1 search.proto SearchFilters.source_type=1）+ `buf generate`
- 改 `core/src/data_plane/search.rs`——按 `req.source_type` 对汇总后的 hit post-filter（利用 task-42.1 populate 的 `h.source_type`；空 → 不过滤；用 `classify_source_type` 派生值比对——注：`h.source_type` 已是派生值，直接比对 `req.source_type.contains(&h.source_type)`）
- 改 `internal/contractv1/contractv1.go`——`SearchRequest` add-only `SourceType []string` json `source_type`（镜像 Semantic/Hybrid add-only 字段 + 注释）
- 改 `internal/consoleapi/handlers.go`——`handleSearch` 解析 `?source_type=`（`r.URL.Query()["source_type"]` repeated）+ body `source_type` 并集合并到 `req.SourceType`（镜像 `?semantic`/`?hybrid` OR-merge）
- 改 grpcclient 映射点（`internal/consoleapi/grpcclient*.go` 或对应 SearchClient 实现）——`SearchRequest.SourceType` → console_data_plane `SearchRequest.source_type`
- 同源测试：proto `SearchRequest.source_type` round-trip + prost wire-tag 字段号 9（TEST-42.2.1）+ `handleSearch` `?source_type=` 解析 + body 并集 → 下游 + `data_plane` post-filter（TEST-42.2.2）

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- chunk-level `agent_scope` 真实过滤 [SPEC-DEFER:phase-future.chunk-agent-scope-filter]
- importer 显式 source_type 打标 [SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]
- v1 semantic 路径 retriever-内 source_type 过滤 [SPEC-DEFER:phase-future.semantic-path-source-type-filter]
- 真实 release tag / run-id / digest（v0.35.0）[SPEC-OWNER:task-42.3-closeout]（ADR-012 用户授权后回填）

## 4. Actors

- 主 agent（ADR-012 自治）
- console_data_plane `SearchRequest`（`console_data_plane.proto:151`，本 task add-only `source_type=9`）
- `data_plane::search`（`core/src/data_plane/search.rs`，本 task 加 source_type post-filter）
- `handleSearch`（`internal/consoleapi/handlers.go`，本 task 解析 `?source_type=` + body 并集，镜像 `?semantic`/`?hybrid`）
- grpcclient SearchClient（本 task 映射 `SourceType` → console_data_plane `source_type`）
- console UI / REST 用户（可经 `POST /v1/search?source_type=doc` 按来源类型筛选；响应 `source_file_type` 显示真实派生值）

## 5. Behavior Contract

### 5.1 Required Reading

- `proto/contextforge/console_data_plane/v1/console_data_plane.proto:151-167`（`SearchRequest` 字段 1-8，semantic=7/hybrid=8 add-only 范式，**下一空号 9**）+ `:190-212`（`SearchResultItem.source_file_type=5` 响应侧已在）
- `core/src/data_plane/search.rs:337-342`（BM25 分支 `SearchFilters::default()`）+ `:374-382`（`SearchResultItem.source_file_type: h.source_type` 响应已写，task-42.1 populate 后真实）
- `internal/consoleapi/handlers.go`（`handleSearch` `?semantic`（task-20.1）/ `?hybrid`（task-39.2）OR-merge forward 范式）+ `internal/consoleapi/search_semantic_test.go`（`TestTask201/392` capturingSearch 断言范式）
- `internal/contractv1/contractv1.go:112-128`（`SearchRequest` Semantic/Hybrid add-only 字段范式）
- `docs/decisions/adr-015-*.md`（proto add-only 字段号冻结）+ `adr-024-*.md` / `adr-044-*.md`（console 请求侧 forward 范式）+ `adr-047-chunk-source-type-filter.md §D3`（本 task 即其原文实现）

### 5.2 关键设计 — console 请求侧 source_type forward（add-only / 并集合并 / 空 byte-equiv）

- **B1 proto add-only（字段号冻结）**：console_data_plane `SearchRequest` 既有字段 1-8 号不动，source_type 取**下一空号 9**（`repeated string source_type = 9`，ADR-015 add-only）；空 source_type → 不过滤（backward-compat，既有 client 不传 → 行为不变）。
- **B2 data_plane post-filter（统一覆盖三路径，空 byte-equiv）**：`data_plane/search.rs` 在 BM25/semantic/hybrid 分支汇总 hit 后，按 `req.source_type` 统一 post-filter（`req.source_type` 空 → 不过滤；非空 → 仅留 `req.source_type.contains(&h.source_type)` 的 hit，`h.source_type` 是 task-42.1 已 populate 的派生值）。统一 post-filter 覆盖三检索路径一致（v1 retriever-内 BM25 过滤 + console data_plane post-filter 双重，结果一致）。
- **B3 Go forward（query param + body 并集，镜像 ?semantic/?hybrid）**：`handleSearch` 解析 `r.URL.Query()["source_type"]`（repeated query param）+ body decode 的 `req.SourceType`，**并集合并**（去重可选，下游集合语义幂等）；→ grpcclient 映射到 console_data_plane `SearchRequest.source_type`。与 `?semantic`/`?hybrid`（bool OR-merge）同构（差异：repeated 用并集而非 OR）。
- **设计定性**：source_type 是 repeated（多桶并集，任一桶匹配即留）；空 → 不过滤；与 v1 path（task-42.1 retriever-内过滤）+ console response（B1 已显示真实 source_type）形成完整请求-响应闭环。

### 5.3 不变量

- 空 source_type → 不过滤 byte-equiv（ADR-004）：`req.source_type` 空 → data_plane 不过滤、结果与改动前 byte-identical；既有 console client（不传 source_type）行为不变。
- proto add-only（ADR-015）：console_data_plane `SearchRequest` 既有字段 1-8 号 + 类型不变；source_type 取新号 9；既有 wire 兼容。
- 0 新代码依赖（ADR-008）：proto add-only 无新 dep；Go/Rust 无 Cargo/go.mod 依赖增量。
- 0 网络：source_type 过滤是本地检索决策。
- 三路径一致：BM25/semantic/hybrid 经 data_plane 统一 post-filter，source_type 过滤行为一致（利用 task-42.1 populate 的 `h.source_type`）。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（proto add-only source_type=9 + round-trip 🟢）: `console_data_plane.proto` `SearchRequest` add-only `repeated string source_type = 9`（既有字段 1-8 冻结，buf generate Rust prost + Go pb.go）；prost wire-tag 字段号 9 in-crate 断言（tag 0x4A） — verified by **TEST-42.2.1**
- [x] **AC2**（Go forward + data_plane post-filter 🟢）: `internal/contractv1.SearchRequest` add-only `SourceType []string`；`handleSearch` 解析 `?source_type=`（query param）+ body 并集 → 下游 SearchClient；grpcclient → console_data_plane `source_type`；`data_plane/search.rs` 按 `req.source_type` post-filter（空 → 不过滤 byte-equiv / 非空 → 仅留匹配桶） — verified by **TEST-42.2.2**
- [x] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-42.2.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-42.2.1 | console_data_plane `SearchRequest.source_type` prost wire-tag 字段号 9 in-crate 断言（`source_type:["x"]` → `[0x4A,0x01,0x78]`；空 → 0 字节 backward-compat） | `core/src/data_plane/search.rs`（`test_42_2_1_source_type_proto_field_number`） | Done |
| TEST-42.2.2 | `handleSearch ?source_type=code&source_type=doc`（query param）+ body `source_type` 并集 → 下游 SearchClient（`TestTask422_HandleSearchSourceTypeUnion`）+ grpcclient → pb.source_type（`TestTask422_GrpcClient_Search_ForwardsSourceType`）+ `data_plane/search.rs` source_type post-filter（`test_42_2_2_dataplane_source_type_filter`：空 → 2 hit byte-equiv / [doc] → 仅 .md / [code] → 仅 .rs） | `internal/consoleapi/search_semantic_test.go` + `internal/consoleapi/grpcclient/grpcclient_test.go` + `core/src/data_plane/search.rs` | Done |
| TEST-42.2.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（高）console proto 字段号冲突 / 非 add-only**：source_type 复用既有字段号或改既有字段破 wire 兼容（ADR-015）。
  - **缓解**：既有字段 1-8 号冻结，source_type 取下一空号 9（add-only）；TEST-42.2.1 prost wire-tag in-crate 断言字段号 9；空 → 不过滤 backward-compat。stop-condition：复用 / 改既有字段号则 AC1 不标 `[x]`。
- **R2（中）空 source_type 破 byte-equiv**：data_plane post-filter 在 source_type 空时仍生效破 backward-compat。
  - **缓解**：post-filter 仅 `!req.source_type.is_empty()` 时生效；空 → 不过滤；TEST-42.2.2 断言空 filter byte-equiv。stop-condition：空 filter 改变结果则 AC2 不标 `[x]`。
- **R3（中）buf generate 改无关 pb.go / 描述符位移**：buf generate 可能重排无关生成文件。
  - **缓解**：buf generate 内容幂等（仅 LF/CRLF + 新字段描述符增量）；diff review 仅留 source_type 相关增量，还原无关重排（Phase 32/39 经验：rawDesc 描述符位移 + 还原不相关 pb.go）。stop-condition：生成 diff 含无关语义改动则不合。
- **R4（中）handleSearch repeated query param 解析**：`?source_type=a&source_type=b` 须 `r.URL.Query()["source_type"]`（非 `.Get` 仅取首个）。
  - **缓解**：用 `r.URL.Query()["source_type"]` 取全部 + body 并集；TEST-42.2.2 断言多值 query param + body 并集。stop-condition：仅取首值 / 漏 body 并集则 AC2 不标 `[x]`。
- **R5（低）data_plane 双重过滤 vs v1 retriever-内过滤不一致**：v1 retriever-内 BM25 过滤（task-42.1）+ console data_plane post-filter 两机制。
  - **缓解**：两者均按 `classify_source_type`/populate 的 `h.source_type` 比对，确定性一致；console 用 post-filter 覆盖 semantic/hybrid（retriever-内仅 BM25）；TEST-42.2.2 断言 data_plane 过滤行为。stop-condition：两机制结果不一致则 AC2 不标 `[x]`。

## 9. Verification Plan

```bash
# 1. AC1 — proto round-trip + wire-tag 字段号 9
buf generate
cargo test -p contextforge-core data_plane::

# 2. AC2 — Go forward + data_plane post-filter
go test ./internal/consoleapi/...
cargo test -p contextforge-core data_plane::

# 3. 不退化（全量）
cargo test --workspace
go test ./...
cargo clippy --workspace --all-targets -- -D warnings
go vet ./...

# 4. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.console-source-type-forward-defer-note]：本 task 交付 console-api `/v1/search` 请求侧 source_type forward（proto add-only source_type=9 + data_plane post-filter + Go `?source_type=` query/body 并集 forward + grpcclient），🟢 可单测，0 新 dep + proto add-only（既有字段号冻结）+ 空 → 不过滤 byte-equiv。chunk-level `agent_scope` 过滤 `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`、importer 显式 source_type 打标 `[SPEC-DEFER:phase-future.chunk-importer-source-type-tagging]`、v1 semantic 路径 retriever-内过滤 `[SPEC-DEFER:phase-future.semantic-path-source-type-filter]` 均不在本 task 范围。本 task dep task-42.1 的 `classify_source_type` + populate（console post-filter 用 populate 的 `h.source_type`）；实测产物（v0.35.0）真实跑出后回填（ADR-013 不预填）。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification（feat/task-42.2-console-source-type-forward，真实证据）**：
- AC1：`cargo test -p contextforge-core --lib data_plane::search::test_42_2_1` —— `test_42_2_1_source_type_proto_field_number` PASS（`SearchRequest{source_type:["x"]}.encode_to_vec() == [0x4A,0x01,0x78]`，field 9 repeated string wire-tag；空 SearchRequest → 0 字节 backward-compat）。`buf generate proto` 重生 `console_data_plane.pb.go`（SourceType 字段 + getter + rawDesc 字节重排——SearchRequest 是靠前 message，插入描述符字节令后续 rawDesc 行整体重排，属正常）。
- AC2：`cargo test -p contextforge-core --lib data_plane::search::test_42_2_2` —— `test_42_2_2_dataplane_source_type_filter` PASS（.rs+.md fixture：空 → 2 hit byte-equiv / `[doc]` → 仅 b.md（`source_file_type="doc"`）/ `[code]` → 仅 a.rs）；`go test ./internal/consoleapi/...` PASS（`TestTask422_HandleSearchSourceTypeUnion` query/body 并集 + `TestTask422_GrpcClient_Search_ForwardsSourceType` grpcclient → pb.source_type）。
- AC3：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- 0 新 dep（proto add-only 无新 dep）/ 0 网络 / 空 source_type → 不过滤 byte-equiv；`cargo test --workspace` 全绿 + `clippy -D warnings` clean + `go test ./...` 全绿 + `go vet` clean + gofmt clean（CR-strip 后）。

**实际改动文件**：
- `proto/contextforge/console_data_plane/v1/console_data_plane.proto`——`SearchRequest` add-only `repeated string source_type = 9` + `buf generate proto`（生成 `console_data_plane.pb.go`；Rust prost 由 build.rs 编译期重生）。
- `core/src/data_plane/search.rs`——`query()` 在 `hits` 装配后按 `req.source_type` post-filter（覆盖 BM25/semantic/hybrid 一致，利用 task-42.1 populate 的 `h.source_type`，空 → 不过滤）+ TEST-42.2.1/.2。
- `core/src/data_plane/mod.rs` + `core/tests/search_real_retriever.rs` + `core/tests/data_plane_integration.rs`——既有 `PbSearchRequest` struct 字面量补 `source_type: Vec::new()`（proto add-only 字段补全，非 `..Default` 的显式字面量）。
- `internal/contractv1/contractv1.go`——`SearchRequest` add-only `SourceType []string`。
- `internal/consoleapi/handlers.go`——`handleSearch` `?source_type=` query param + body 并集合并。
- `internal/consoleapi/grpcclient/grpcclient.go`——`SourceType: req.SourceType` 映射到 pb。
- `internal/consoleapi/search_semantic_test.go` + `internal/consoleapi/grpcclient/grpcclient_test.go`——TEST-42.2.2 Go 侧。

**grounding 校正（ADR-013 据实）**：(1) `buf generate` 须以 proto module 为输入（`buf generate proto`，module 根在 `proto/buf.yaml`）——裸 `buf generate` 从仓库根跑会 import-resolve 失败；(2) buf 重写的 4 个不相关 pb.go（search/service/service_grpc/console_data_plane_grpc，无字段变更 → 仅 EOL/注释 spurious diff）据实还原，仅留 `console_data_plane.pb.go` 真实变更（field+getter+rawDesc 字节重排，属正常非 churn）；(3) proto add-only 新字段令既有 `PbSearchRequest` 显式 struct 字面量（非 `..Default`）须补 `source_type: Vec::new()`（5 处 lib/integration test）——编译期暴露，据实补全。
