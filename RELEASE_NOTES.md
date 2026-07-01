# ContextForge Release Notes

## v0.37.0 (2026-07-01) — memory-unpin-actor-propagation (闭环 pin/unpin actor 透传不对称 + audit/event source 归因：Phase 40 task-40.1 给 pin 加了 actor 透传，unpin 漏了；grounding 发现 store pinned=false 丢弃 actor → 单纯透传是空透传；真实落点是 emit_audit_and_event 加 actor 参数让 audit/event source 归因真实调用方（pin 顺带闭环）；proto UnpinMemoryRequest add-only actor=2 + Go X-Actor 透传链；默认 byte-equiv（空 actor 回落）+ 0 新 dep / 0 network / 0 migration / proto add-only + ADR-049 ratified)

| task | PR | delivery |
|---|---|---|
| 44.1 | #280 | unpin-actor-propagation：proto UnpinMemoryRequest add-only actor=2 + Rust unpin handler 透传 + emit_audit_and_event 加 actor 参数（audit/event source 归因）+ pin 顺带闭环 + Go 4 处透传（types/grpcclient/handlers X-Actor/memstore）+ degraded fallback + TEST-44.1.1/.2/.3/.4（lib 229→232） |
| 44.3 (this) | v0.37.0 closeout：smoke v33→v34 `[53/53]`（unpin X-Actor 端到端 / 不可达诚实归因 unit）+ `TestTask443`（无 [37/37]..[52/52] 回归）+ release docs + ADR-049 据 D1-D4 ratify（Proposed→Accepted；D1-D3 unit 🟢 / D4 byte-equiv 🟢 + 认证身份/其余 3 RPC 🔴 honest-defer）+ ADR-032/045 add-only Phase-44 Amendment + roadmap §3.26/§4 + adapter |

**Upgrade / Rollback**：v0.37.0 闭环 pin/unpin actor 透传不对称——`unpin` RPC 现接受 `actor` 字段（proto add-only `actor=2`），console-api `POST /v1/memory/{id}/unpin` 现读 `X-Actor` header 透传（镜像 pin）。**grounding 发现真实价值在 audit/event**：unpin 的 store 路径（`set_pinned_with_actor(pinned=false)`）清空 `pinned_by`，故 actor 的真实落点是 `emit_audit_and_event` source（audit log + event stream 归因真实调用方）；pin 顺带闭环（其 audit/event 也归因）。**默认 byte-equiv**：空 `actor` / 无 `X-Actor` header → audit source "console-api" / event source "contextforge-core"（各自 byte-identical 现状）。**0 新代码依赖** / **0 network** / **0 schema migration** / **proto add-only**（UnpinMemoryRequest actor=2，既有 memory_id=1 冻结）；既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。**认证身份 🟡 honest-deferred** `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`（X-Actor → 已认证 auth subject 须 console-api 鉴权层）；**deprecate/softdelete/harddelete actor 透传 🔴 honest-deferred** `[SPEC-DEFER:phase-future.memory-actor-all-rpc]`（Deprecate/SoftDelete 须 7 层+新 migration / HardDelete 须 audit 重设计——本 release 仅做 emit_audit_and_event actor 参数共用基础，这 3 RPC 未来顺带受益）。linux/amd64（承 v0.21–v0.36；arm64 仍 deferred）；cosign keyless sign + SBOM + provenance（承 task-28.2）。Rollback：`git tag -d v0.37.0` + 删 Release / ghcr tag；回退 v0.36.0 后 unpin actor 回到硬编码 "console-api"、audit/event source 回硬编码（行为回退现状）。tag SHA `6f91bcc689aa3d8b56f81eb2eb8090adce3859e8`（annotated tag obj `a786a95ba0e347a7e6fd10df4d259305a3dd9b92`）/ release run id `28551841199`（`success`）/ ghcr digest `sha256:c9fe6711435030cb642abd64fbd13e8f5d77a41eae666158dab5dc09b53d1043`（ghcr.io/tajiaoyezi/contextforge-daemon:v0.37.0 + :latest，linux/amd64）/ cosign Rekor tlog `2041944547`（sign）·`2041948153`（attest）（post-tag-push 回填，ADR-012/013）。

## v0.36.0 (2026-07-01) — governance-debt-cleanup-4 / indexing-replay-splice (第四轮治理债清扫，单聚焦 indexing-replay-e2e 拼接缺口：把 Phase 33 task-33.3 / ADR-038 D3 交付但从未在 live 路径调用的 indexing replay mapper 拼接进 `EventsServer::subscribe`；4 拼接缺口补全：`list_since(limit, since_ts)` + `DataPlaneStores.indexing_event_store` 字段 + `serve_full` 接线 + subscribe splice；since_ts>0 订阅者现可收到 missed indexing.progress/.cancelled/.error 事件（与 memory audit replay 对称）；默认 byte-equiv（since_ts<=0 / store=None）+ 0 新 dep / 0 network / 0 schema migration（复用 0019）/ 0 proto + ADR-048 ratified)

| task | PR | delivery |
|---|---|---|
| 43.1 | #276 | indexing-replay-splice：`indexing_events.rs` add `list_since(limit, since_ts)`（since_ts>0 时 `WHERE ts_unix >= ?` 镜像 `replay_events_from_audit`，since_ts<=0 不过滤 byte-equiv）+ `DataPlaneStores` add `indexing_event_store` 字段 + `full()` 第 10 参数 + `server.rs` serve_full `IndexSessionBackend` clone + `full()` 传入 + `events.rs` subscribe replay splice（audit 后、live 前）+ TEST-43.1.1/.2a/.2b/.2c（lib 225→229） |
| 43.3 (this) | v0.36.0 closeout：smoke v32→v33 `[52/52]`（indexing-replay-splice 可达断言 / 不可达诚实归因 unit TEST-43.1.2）+ `TestTask433`（无 [37/37]..[51/51] 回归）+ release docs（README v0.36 段 + evidence/artifacts，tag/run/digest `<backfill>` marker）+ ADR-048 据 D1-D4 ratify（Proposed→Accepted；D1-D3 unit 🟢 / D4 live daemon e2e 🟡 honest-defer）+ ADR-038 add-only Phase-43 Amendment（indexing-replay-e2e splice 维度兑现，live daemon e2e 续延后）+ ADR-031/021/004/008 守线引用 + roadmap §3.25/§4 + adapter + defer marker 更新 |

**Upgrade / Rollback**：v0.36.0 把 indexing event replay 接进 live subscribe 路径——`EventsServer::subscribe(since_ts>0)` 现先发送 missed 的 `indexing.*` 生命周期事件（`evt-idx-*`，id ASC）再发送 missed 的 memory audit 事件（`evt-audit-*`）再接 live 流，使重连订阅者能补齐 indexing 进度/取消/错误事件（与既有 memory audit replay 对称）。**默认 byte-equiv**：`since_ts<=0`（首连无 since_ts）→ 无 replay（行为 byte-identical）；`indexing_event_store=None`（旧 constructor / 单测）→ 无 indexing replay（退化现状）。仅 `serve_full` 生产路径（since_ts>0 + store=Some）时 indexing replay 生效。**0 新代码依赖** / **0 network** / **0 schema migration**（复用 Phase 33 migration 0019）/ **0 proto 改动**（纯内部 read 路径 splice）；既有 `cargo-test` / `go-test` / `spec-lint` 三门不退化。**live daemon restart-then-replay 端到端 e2e 🟡 honest-deferred** `[SPEC-DEFER:phase-future.indexing-replay-daemon-e2e]`（须 running daemon + 跨 restart 双窗口断言，ADR-013 不预填）；**memory-actor-all-rpc 据 ADR-013 honest-deferred** 留独立 phase（Deprecate/SoftDelete 7 层 + 新 migration / HardDelete 须 audit 重设计 = 非小债，roadmap §3.17/§3.22 "据实排小不凑数"）。linux/amd64（承 v0.21–v0.35；arm64 仍 deferred）；cosign keyless sign + SBOM + provenance（承 task-28.2）。Rollback：`git tag -d v0.36.0` + 删 Release / ghcr tag；回退 v0.35.0 后 indexing replay 回到未接 live 路径（mapper 仍在但 subscribe 不调用，行为回退现状，数据文件互通）。tag SHA `2cd59777bb04c4f57fb1e16cbbd6facfa21133bf`（annotated tag obj `bf14604f2f0d14f4ea236e571e661995bf135f51`）/ release run id `28526379425`（`success`）/ ghcr digest `sha256:b52d49a5c822ea8466cfcf1b44e8a0757a970bacbedca181fb1eb863fb258923`（ghcr.io/tajiaoyezi/contextforge-daemon:v0.36.0 + :latest，linux/amd64）/ cosign Rekor tlog `2037572029`（sign）·`2037574642`（attest）（post-tag-push 回填，ADR-012/013）。

## v0.35.0 (2026-06-07) — chunk-source-type-filter (把 chunk 检索的 `source_type` 过滤从 Phase 32 / ADR-037 据实记的 documented no-op 落地为真实过滤：`classify_source_type` 由 file_path 派生 0 schema migration + 三构造点 populate + `search()` BM25 post-filter + console-api `?source_type=` forward（proto add-only `source_type=9`）；agent_scope 据 grounding honest-defer 续 no-op + 0 新 dep / 0 network / 空 filter byte-equiv + ADR-047 ratified)

Phase 42 把 chunk 检索的 `source_type` 过滤从 Phase 32（task-32.3 / ADR-037）经核据实记的 **documented no-op** 落地为**真实过滤**（兑现 `[SPEC-DEFER:phase-future.chunk-source-type-filter]`）。`SearchFilters.source_type` + v1 proto `SearchFilters.source_type=1` + `RetrievalResult.source_type=3` 自 task-4.2 起就有契约，但 chunks 表无该列、`SearchResult.source_type` 恒 `DEFAULT_SOURCE_TYPE=""`，遂据实定为 no-op。**grounding 关键**：`source_type` 可由 `file_path` **确定性派生**（`classify_source_type`，镜像 `indexer::lang_hint_from_path`：扩展名 → 粗粒度桶 `code`/`doc`/`config`/`other`）→ 无须存储、无须 schema migration（chunks/files/provenance §5.3 **保持 FROZEN**），确定性派生 == 存储值。task-42.1 三构造点 populate 真实 source_type（填补 task-4.2 §2A v0.1 schema gap）+ `search()` BM25 source_type post-filter（镜像 language post-filter，空 → byte-equiv）；v1 `server.rs:440-453` 已映射 → v1 gRPC/REST body 立即生效。task-42.2 forward 到 console-api（proto add-only `source_type=9` + data_plane post-filter 覆盖 BM25/semantic/hybrid + Go `?source_type=` query/body 并集 + grpcclient），console 响应 `source_file_type` 早已就绪显示真实派生值。**诚实校正（ADR-013，本 phase 核心）**：`chunk-source-type-filter` 与 `chunk-agent-scope-filter` 经 grounding **不对称**——source_type 可派生真实落地；`agent_scope` 是 memory 层概念（`memory_items` 0013 / `ListMemory` scope / `memstore.go:629-635`）、chunks 无 agent 维度 → **本 phase 不伪造**，续 documented no-op + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`（须 ingest-path schema + 价值不明，镜像 Phase 32/34/35 grounding 校正手法）。source_type value 由空串变真实派生值系填补 v0.1 schema gap（契约本意），空 filter 过滤行为 byte-equiv（非破坏性默认变更）。**0 新依赖**（`classify_source_type` 纯 std）**/ 0 network / 0 schema migration**。

| task | 交付 |
|---|---|
| 42.1 (#267) | chunk-source-type-derivation-and-filter：`core/src/retriever/mod.rs` add `pub(crate) fn classify_source_type(file_path) -> &'static str`（扩展名确定性桶 code/doc/config/other，镜像 `indexer::lang_hint_from_path`，纯 std；无扩展名/未知/dotfile → other）+ 三构造点（`search()` BM25 / `get_chunk` / `assemble_vector_result`）`source_type` 真实派生 + `search()` BM25 source_type post-filter（SQLite JOIN 后，镜像 `:386` language，空 → 不过滤 byte-equiv）+ 窄化 no-op 块仅 agent_scope + 删除孤儿 `DEFAULT_SOURCE_TYPE`（grounding 校正）+ populate 连带更新 `test_4_2_1`/`test_6_2_e1`/`server.rs` wire/`phase4_smoke`/`phase6_smoke` 旧 schema-gap "" 断言→「有效桶」。TEST-42.1.1（classify 矩阵）+ TEST-42.1.2（真实过滤 + populate + agent_scope no-op）。0 新 dep（纯 std）/ 0 schema migration（§5.3 FROZEN） |
| 42.2 (#268) | console-api-source-type-forward：`console_data_plane.proto` `SearchRequest` add-only `repeated string source_type = 9`（既有字段 1-8 号冻结，ADR-015，`buf generate proto`）+ `core/src/data_plane/search.rs` hits 装配后按 `req.source_type` post-filter（覆盖 BM25/semantic/hybrid 一致，利用 populate 的 `h.source_type`，空 → 不过滤）+ Go `contractv1.SearchRequest` add-only `SourceType []string` + `handleSearch` `?source_type=` query param + body 并集（镜像 `?semantic`/`?hybrid`）+ grpcclient → `pb.source_type`。既有显式 `PbSearchRequest` 字面量补 `source_type: Vec::new()`（mod.rs + 2 integration test）。TEST-42.2.1（prost wire-tag field 9 → 0x4A）+ TEST-42.2.2（handleSearch 并集 + grpcclient 转发 + data_plane post-filter）。0 新 dep + proto add-only + 空 filter byte-equiv |
| 42.3 (this) | v0.35.0 closeout：smoke v31→v32 [51/51]（REAL：`index-job-real` fixture 全 markdown，`runner`/JobRunner 文档于 .md → source_type=doc → `?source_type=doc` 保留 JobRunner doc hit（`source_file_type=doc`）/ `?source_type=code` 过滤掉它——distinguishing：no-op 会两者皆返回该 chunk）+ TestTask423（无 [37/37]..[50/50] 回归）+ release docs（README v0.35 段 + evidence/artifacts，tag/run/digest `<backfill>` marker）+ ADR-047 据 D1-D4 ratify（Proposed→Accepted）+ ADR-037 add-only Phase-42 Amendment（source_type no-op superseded / agent_scope no-op reaffirmed）+ ADR-015/024/044/004/008 守线引用 + roadmap §3.24/§4 + adapter + defer marker 更新 |

**ADR**：ADR-047 (chunk-source-type-filter) 据 D1-D4 真实非合成验证 `Proposed → Accepted`（D1 source_type 由 file_path 派生 0 migration §5.3 FROZEN / D2 v1 retriever 真实过滤 + 三路径 populate 空 filter byte-equiv / D3 console proto add-only source_type=9 + data_plane post-filter + Go forward / D4 agent_scope honest-defer memory 层概念续 no-op）；ADR-037 (vector-backend-config-plumbing-and-completeness) add-only Phase-42 Amendment——标 source_type no-op 被真实过滤 superseded、agent_scope no-op reaffirmed，**NOT** 溯改 D-body / Ratification (v0.25.0)（ADR-014 D5）；ADR-015 (proto add-only) / ADR-024 / ADR-044 (console 请求侧 forward 范式) / ADR-004 (空 filter byte-equiv + source_type value 填补 v0.1 schema gap) / ADR-008 (0 新 dep) / ADR-013 守线。**ADR-014 cross-validation gate — 第三十三次激活**。

**Upgrade / Rollback**：v0.35.0 把 chunk 检索的 `source_type` 过滤落地为真实能力——用户可经 v1 `{"filters":{"source_type":["doc"]}}` / console `POST /v1/search?source_type=doc` 按来源类型（`code`/`doc`/`config`/`other`，由 `file_path` 扩展名派生）筛选检索结果。**空 source_type filter → 不过滤（byte-equivalent）**，既有 client 不传 → 行为不变。**source_type value 变化**：检索结果的 `source_type` / `source_file_type` 字段由历来的空串 `""` 变为真实派生桶值（填补 task-4.2 §2A v0.1 schema gap，契约本意是真实 source_type）——若客户端依赖空串需适配（绝大多数消费方读取该字段作展示/分类，不依赖空）。**agent_scope 仍是 documented no-op**（memory 层概念，chunks 无 agent 维度，honest-deferred）。**0 新代码依赖（`classify_source_type` 纯 std）/ 0 network / 0 schema migration**（chunks/files/provenance §5.3 FROZEN，source_type 由 file_path 派生不存储；v0.34↔v0.35 数据文件互通无需 reindex）；console_data_plane proto add-only `source_type=9`（既有字段号冻结，既有 gRPC client 兼容）。linux/amd64（承 v0.21–v0.34；arm64 仍 deferred）；cosign keyless sign + SBOM + provenance（承 task-28.2）。Rollback：`git tag -d v0.35.0` + 删 Release / ghcr tag；回退 v0.34.0 后 source_type 过滤回到 no-op、source_type value 回空串（数据文件互通，无需 reindex）。tag SHA `0fd4a75ea1c785324b1ba53b30054a1ce2887607`（annotated tag obj `6d8947bad4b0e75b5665e91cb6959a8c05a3f7de`）/ release run id `27086589784`（`success`）/ ghcr digest `sha256:6f8fb08a8ef86211e5db202e319132d5d2fa841e66eb7ed3968ecac1d9991029`（ghcr.io/tajiaoyezi/contextforge-daemon:v0.35.0 + :latest，linux/amd64）/ cosign Rekor tlog `1743945347`（sign）·`1743946599`（attest）（post-tag-push 回填，ADR-012/013）。

## v0.34.0 (2026-06-07) — tokenizer-default-on (做出 Phase 30 / ADR-035 §D3 据「翻默认是产品决策」诚实延后的产品决策：code/CJK tokenizer `code_cjk` 从 opt-in 翻为新建 collection 生产默认 `resolve_tokenizer()` + 生产索引两调用点接线 + Go `[retrieval] tokenizer` config 桥 `setTokenizerEnv`；首次刻意默认行为变更由 ADR-046 承接 + 既有 collection 不受影响 + opt-out + 实测 recall delta +0.1250 + 0 新 dep / 0 network + ADR-046 ratified)

Phase 41 做出 Phase 30 / ADR-035 §D3 据「翻默认是产品决策」诚实延后的产品决策（`[SPEC-DEFER:phase-future.tokenizer-default-on]`）：code/CJK 感知 analyzer `code_cjk`（task-24.1：camelCase/snake_case/dotted.path/kebab-case 拆子词 + 保留原 token + CJK bigram，纯 std 0-dep）自 v0.17.0 已实存但仅 opt-in；生产索引全走 `IndexSession::open(..)` → 新建 collection 绑 Tantivy 默认 `TEXT`。本版把 `code_cjk` 翻为**新建 collection 的生产默认**，使全体用户**默认**获更好的代码符号 / CJK 检索召回。**首次刻意默认行为变更**（新建 collection `content` 倒排词项 `TEXT`→`code_cjk`，**非 byte-equivalent**），由 **ADR-046** 显式承接 + 三重安全：(1) **既有 collection 不受影响**——`open_with_tokenizer` 对既有索引走 `Index::open_in_dir` 读回持久化 `meta.json` schema、忽略传入 tokenizer，老 `TEXT` collection 继续按原 analyzer 工作不被静默失效；(2) **opt-out** 回 legacy `TEXT`——`CONTEXTFORGE_TOKENIZER=default`（env）或 config.toml `[retrieval] tokenizer = "default"`；(3) **不自动迁移**——既有 collection 升级到 `code_cjk` 由用户经既有 `reindex_with_tokenizer`（Phase 30）主动触发。**实测真实 recall delta**（当前 16-题 golden）：before(default `TEXT`) recall@5/@10=0.8750 → after(`code_cjk`) recall@5/@10=**1.0000**，**delta +0.1250**（mrr +0.0625；与 Phase 30 `delta(seg−default)=+0.1250` 一致——ADR-029 §Negative 的 `+0.0909` 系 Phase 24 原始 11-题 golden，据实记当前数不沿用旧数，ADR-013）。**0 新依赖**（`code_cjk` 纯 std；jieba `cjk_segmenter` 仍 feature-gated——Phase 30 实测 jieba vs bigram delta=+0.0000 无增益，故默认翻 bigram `code_cjk` 而非 jieba，守 0-dep baseline）**/ 0 network**。**诚实校正（ADR-013）**：首次刻意默认变更据实定性非夸大 byte-equiv；jieba 默认不取 / 既有 collection 不自动迁移 / 大语料 recall / `RetrieverConfig.tokenizer` 路由据实保持延后不强行扩面。

| task | 交付 |
|---|---|
| 41.1 (#262) | tokenizer-default-on：`core/src/server.rs` add `resolve_tokenizer()`（pub fn）+ `parse_tokenizer(Option<&str>)`（pub(crate) 纯函数，镜像 `resolve_data_dir`/`resolve_vector_backend`）：unset/""→`code_cjk` 翻默认 / `"default"`→`DEFAULT_TOKENIZER` opt-out 回 legacy `TEXT` / `"code_cjk"` passthrough / `"cjk_segmenter"`→feature 在则 jieba·缺则 stderr WARN+`code_cjk` / unknown→stderr WARN+`code_cjk`（不静默落 TEXT）。生产索引两调用点 `server.rs:141` `CoreService::index` + `jobs/index_session_backend.rs:151` 改 `open_with_tokenizer(.., &resolve_tokenizer())`；`IndexSession::open`/`DEFAULT_TOKENIZER` 库 API+常量不动。首次刻意默认变更非 byte-equiv 由 ADR-046 D1/D4 承接；既有 collection 经 `open_in_dir` 自动安全。0 新 dep（`code_cjk` 纯 std）。TEST-41.1.1（`parse_tokenizer` env 矩阵）+ TEST-41.1.2（生产路径绑定 code_cjk + opt-out TEXT + 既有 collection 安全）；实测 recall delta +0.1250 |
| 41.2 (#263) | tokenizer-config-bridge：`internal/config/config.go` add-only `RetrievalConfig{Tokenizer}` + `Config.Retrieval` + `[retrieval]` 段 round-trip（镜像 `VectorConfig`/`[vector]`）+ `assignRetrieval`；`cmd/contextforge/main.go` add `setTokenizerEnv`（镜像 `setVectorEnv`：`[retrieval] tokenizer` 非空且 `CONTEXTFORGE_TOKENIZER` 未设→导出，env-wins、无段/空值不导出→Rust 默认 code_cjk）接线 doServe/doMCP；tokenizer 非密钥；Rust core 0 toml dep。TEST-41.2.1（round-trip code_cjk/default/cjk_segmenter + 既有段不受影响 + 向后兼容）+ TEST-41.2.2（env-wins / 无段不导出） |
| 41.3 (this) | v0.34.0 closeout：smoke v30→v31 [50/50]（REAL camel 子词 `runner`(of `JobRunner`) 经 `code_cjk` 默认命中——legacy `TEXT` 保 `jobrunner` 单 token 会 miss，distinguishing 断言）+ TestTask413（无 [37/37]..[49/49] 回归）+ release docs（README v0.34 段 + evidence/artifacts，tag/run/digest `<backfill>` marker）+ ADR-046 据 D1-D4 ratify（Proposed→Accepted）+ ADR-029 add-only Phase-41 Amendment（标默认开启维度 fulfilled）+ ADR-035 add-only Phase-41 Amendment（标 D3 产品决策 fulfilled）+ ADR-004/008 守线引用 + roadmap §3.23/§4 + adapter + defer marker 更新 |

**ADR**：ADR-046 (tokenizer-default-on) 据 D1-D4 真实非合成验证 `Proposed → Accepted`（D1 production 默认翻 `code_cjk` + 既有 collection schema-driven 安全 + 实测 recall delta +0.1250 / D2 `CONTEXTFORGE_TOKENIZER` env opt-out + Go `[retrieval]` config 桥 env-wins / D3 recall delta 复测 + honest-defer 边界 / D4 首次刻意默认变更承接 + 0-dep/0-network + opt-out byte-equiv）；ADR-029 (code-and-cjk-tokenizer-and-eval-hardening) add-only Phase-41 Amendment——标默认开启维度 fulfilled，**NOT** 溯改 D-body / §Negative（ADR-014 D5）；ADR-035 (cjk-true-segmenter-and-tokenizer-default) add-only Phase-41 Amendment——标 D3 full default flip 产品决策 fulfilled，**NOT** 溯改 D-body / D5 / Ratification（ADR-014 D5）；ADR-004 (local-first，刻意默认变更例外由 ADR-046 承接 + opt-out byte-equiv + 不自动迁移 safety intent 保持) / ADR-008 (0 新 dep) / ADR-013 守线。**ADR-014 cross-validation gate — 第三十二次激活**。

**Upgrade / Rollback**：**首次刻意默认行为变更（非 byte-equivalent，由 ADR-046 承接）**——v0.34.0 起，daemon 经生产索引路径**新建**的 collection 默认绑 `code_cjk`（代码符号子词 + CJK bigram，更好的代码/CJK 检索召回），不再是 legacy Tantivy `TEXT`。**既有 collection 不受影响**（经 `open_in_dir` 读回持久化 schema，继续以原 analyzer 索引/检索）。**opt-out 回 legacy `TEXT`**：环境变量 `CONTEXTFORGE_TOKENIZER=default` 或 config.toml `[retrieval] tokenizer = "default"`（env-wins）。**升级既有 collection 到 `code_cjk`**：用户主动经 `reindex_with_tokenizer`（Phase 30 起已备）；v0.34.0 **不自动迁移**。jieba 真分词仍需 `--features cjk-segmenter` + `CONTEXTFORGE_TOKENIZER=cjk_segmenter` opt-in（默认构建不含，0-dep baseline）。`[retrieval]` config 段 add-only（既有 config.toml 无 `[retrieval]` → zero value → core 默认 code_cjk）。**0 新代码依赖（`code_cjk` 纯 std）/ 0 network / 0 schema migration**（不改 chunks / Tantivy schema 字段集，仅 analyzer 绑定）。linux/amd64（承 v0.21–v0.33；arm64 仍 deferred）；cosign keyless sign + SBOM + provenance（承 task-28.2）。Rollback：`git tag -d v0.34.0` + 删 Release / ghcr tag；回退 v0.33.0 后新建 collection 重回 `TEXT` 默认（v0.34.0 期间新建的 `code_cjk` collection 在 v0.33.0 下仍可检索——retriever 无条件注册 `code_cjk` analyzer schema-driven 对称）；`embedding_cache` / chunks schema 未改 v0.33↔v0.34 数据文件互通；若仅不希望改默认无需回退——`CONTEXTFORGE_TOKENIZER=default` opt-out 即得 v0.33.0 行为。tag SHA `aa36c2c9eff20f6fb2bda5fd06ce784b36372c39`（annotated tag obj `ec5b2209a42831f85b4d54c01da4ad170650b85a`）/ release run id `27083795263`（`success`）/ ghcr digest `sha256:39b2db3b657ef2c89f75efe08ac10bef6a5585adb3f089670f5c46947ca3e49d`（ghcr.io/tajiaoyezi/contextforge-daemon:v0.34.0 + :latest，linux/amd64）/ cosign Rekor tlog `1742901000`（sign）·`1742902913`（attest）（post-tag-push 回填，ADR-012/013）。

## v0.33.0 (2026-06-07) — governance-debt-cleanup-3 (第三轮治理债清扫：兑现 ADR-032 `memory-actor-propagation`——memory pin actor 入口透传 `PinMemoryRequest.actor=3` + Go 参数链 + `X-Actor` header；兑现 ADR-038 `l2-cache-true-lru`——L2 embedding 缓存命中 bump 隐式 rowid 升访问序 LRU，据实更正 Phase 33「真 LRU 须加时间列」假设；0 新 dep / proto add-only / 0 schema migration / 默认 byte-equivalent + ADR-045 ratified)

Phase 40 第三轮治理债清扫（镜像 Phase 31 / 33 的「核实-诚实化-补全」打法），清两组在 grounding 中确认为**真实且 code-local 可单测**的跨 Phase 治理 marker。(1) **memory-actor-propagation**（ADR-032 §D1 defer）：`pin()` RPC 此前把调用 actor 硬编码 `"console-api"`——因入口到 store 透传链缺失（`PinMemoryRequest` 无 actor field、Go `MemoryClient.Pin` 无 actor 参数、`handleMemoryPin` 不读调用方标识；`set_pinned_with_actor` store 字段自 task-27.1 已在）。补 `PinMemoryRequest.actor=3`（add-only，既有 `memory_id=1` / `pin=2` 冻结）+ Go `Pin(id,pin,actor)` 参数链 + `grpcclient` 填 `Actor` + `handleMemoryPin` 读 `X-Actor` header + Rust `pin()` 空回落 `"console-api"`。console 部署在 auth 代理后可把 pin 归因真实调用方（写 `pinned_by`）。(2) **l2-cache-true-lru**（ADR-038 §A2/D4 defer）：Phase 33 给 L2 加 rowid-FIFO（插入序）驱逐但 `sqlite_get` 命中不重排——补命中即 bump 隐式 rowid 到表尾，使驱逐由插入序 FIFO 升访问序 LRU。**复用既有隐式 rowid、0 schema migration**——据实**更正** Phase 33「真 LRU 须加 created_at 列 + ALTER」假设（命中 bump 即得，与 Go memstore move-to-front 同技法）。**0 新 dep / proto add-only / 0 schema migration / 默认 byte-equivalent**（ADR-004/008/015）：默认无 `X-Actor` → 空 actor 回落 `"console-api"`；L2 命中 bump 仅有限 cap 生效且原样回写相同字节（返回结果不变）。**诚实校正（ADR-013）**：pin actor 调用方透传 vs **认证身份** honest-defer（`[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`）；L2 命中 bump 写放大 = 访问序 LRU 固有代价、`with_sqlite` 无生产调用点现网零影响（opt-in 路径语义补全非已确认线上问题）；其余 marker（`vector-dim-feature-enforce` / `tracestore-multi-workspace-strict` / `chunk-source-type-filter`）据实保持延后不强行扩面（焦点小版本，honest over padding）。

| task | 交付 |
|---|---|
| 40.1 (#257) | memory-actor-propagation：`PinMemoryRequest` add-only `string actor = 3`（既有 `memory_id=1` / `pin=2` 字段号冻结，ADR-015 D1）+ `buf generate`；Go `MemoryClient.Pin(id,pin)` → `Pin(id,pin,actor)`（interface + `memoryClient` / `MemMemoryStore` / `degradedMemory` 三实现）+ `grpcclient` 填 `pb.PinMemoryRequest.Actor` + `handleMemoryPin` 读 `r.Header.Get("X-Actor")`（缺省空串，ADR-022 D2 宽松 body 契约不改）；Rust `pin()` `set_pinned_with_actor(.., if req.actor.is_empty() { "console-api" } else { req.actor.as_str() })`（空回落 byte-equiv）。认证身份 honest-defer `[SPEC-DEFER:phase-future.memory-actor-authenticated-identity]`；0 新 dep / proto add-only。TEST-40.1.1（prost wire-tag actor=3 = `[0x1A,0x01,0x78]`）+ TEST-40.1.2（Rust 透传/空回落）+ TEST-40.1.3（Go X-Actor）+ TEST-40.1.4（grpcclient Actor） |
| 40.2 (#258) | l2-embedding-cache-true-lru：`core/src/embedding/cache.rs` `sqlite_get` 命中分支（仅 `l2_cap > 0`）`INSERT OR REPLACE` 原样回写命中行 bump 隐式 rowid 到表尾 → 既有 `sqlite_put` rowid 序驱逐由插入序 FIFO 升访问序 LRU；cap==0 不 bump（保插入序、零额外写）。复用既有隐式 rowid、0 新 dep / 0 schema migration；据实更正 Phase 33（ADR-038 A2/D4）真-LRU 假设，与 Go memstore move-to-front（task-33.2）同技法；写放大 + `with_sqlite` 无生产调用点现网零影响据实记。TEST-40.2.1（LRU 驱逐最久未用 vs FIFO）+ TEST-40.2.2（cap 门控 bump + 结果不变） |
| 40.3 (this) | v0.33.0 closeout：smoke v29→v30 [49/49]（banner；REAL 模式 `POST /v1/memory/{id}/pin` 带 `X-Actor: smoke-actor` → GET `"pinned_by":"smoke-actor"` 端到端断言）+ TestTask403（无 [37/37]..[48/48] 回归）+ release docs（README v0.33 段 + evidence/artifacts，tag/run/digest `<backfill>` marker）+ ADR-045 据 D1-D3 ratify（Proposed→Accepted）+ ADR-032 add-only Phase-40 Amendment（标 `memory-actor-propagation` fulfilled + 认证身份续延后）+ ADR-038 + ADR-027 add-only Phase-40 Amendment（标 `l2-cache-true-lru` fulfilled + 真-LRU 假设据实更正）+ ADR-015 add-only Amendment（proto add-only field）+ roadmap §3.22/§4 + adapter + defer marker 更新。多 agent 对抗审查（4 维度 × 3 skeptic）核实 0 真实缺陷 |

**ADR**：ADR-045 (governance-debt-cleanup-3) 据 D1-D3 真实非合成验证 `Proposed → Accepted`（D1 memory pin actor add-only 透传 / D2 L2 embedding 缓存访问序 LRU / D3 默认行为 + proto add-only + 0-dep/0-network + honest-defer 边界）；ADR-032 (memory-ops-hardening) add-only Phase-40 Amendment——标 `memory-actor-propagation` 入口透传维度 fulfilled + 认证身份续延后，**NOT** 溯改 D-body（ADR-014 D5）；ADR-038 (governance-debt-cleanup-2) + ADR-027 (embedding-provider-abstraction) add-only Phase-40 Amendment——标 `l2-cache-true-lru` fulfilled + 据实更正真-LRU 假设，**NOT** 溯改 D-body（ADR-014 D5）；ADR-015 (console-contract-v1) add-only Amendment（`PinMemoryRequest.actor=3`）；ADR-022 D2（memory pin lenient body 契约保持）/ ADR-004/008/013 守线。**ADR-014 cross-validation gate — 第三十一次激活**。

**Upgrade / Rollback**：默认行为 / proto 既有字段 / 既有契约不变（既有 v0.6–v0.32 client + 索引/数据无需改动），**0 新代码依赖、0 schema migration、默认 byte-equivalent**（ADR-004/008/015，proto 既有字段号 `memory_id=1` / `pin=2` 冻结）；改动均为 console_data_plane proto add-only（`actor=3`）+ Go `MemoryClient.Pin` add-only 参数 + `handleMemoryPin` 读 `X-Actor` + Rust `pin()` 空回落 + `cache.rs` `sqlite_get` 命中 bump + smoke/test/docs；默认（无 `X-Actor` header）→ 空 actor → `pinned_by="console-api"` 逐字节等价；L2 命中 bump 仅改 rowid 不改返回 vector；`embedding_cache` schema 未改 v0.32↔v0.33 数据文件互通。linux/amd64（承 v0.21–v0.32；arm64 仍 deferred）；cosign keyless sign + SBOM + provenance（承 task-28.2）。Rollback：`git tag -d v0.33.0` + 删 Release / ghcr tag；与 v0.32.0 行为兼容（默认无 `X-Actor` → `pinned_by="console-api"` byte-equivalent，`PinMemoryRequest.actor=3` 对 v0.32.0 client 无害——proto3 未知字段忽略）。tag SHA `1d43b65faac31bc2f55c5261ac1275e6c1b08f88`（annotated tag obj `0cf258d3313b312037103ce8d3eb68e51a19bc9a`）/ release run id `27080497210`（success）/ ghcr digest `sha256:8756155cf3dbab3004bbbfdd899bad9ece629857d1a364bbf03322c8690a14b1`（ghcr.io/tajiaoyezi/contextforge-daemon:v0.33.0 + :latest，linux/amd64）/ cosign Rekor tlog `1741487788`（sign）·`1741489556`（attest）（post-tag-push 回填，ADR-012/013）。

## v0.32.0 (2026-06-06) — console-api-retrieval-signal-forward (兑现 ADR-025 `console-api-hybrid-forward` honest-defer：对外 console-api `POST /v1/search?hybrid=true` 首次贯通 hybrid BM25+vector RRF 融合 + `hybrid_score` provenance；据实重界定 ADR-043 `console-api-rerank-forward`——reranker 保持 env 驱动、rerank `reason` provenance 对外 REST 可见、`?rerank` per-request superseded by ADR-043 D3；0 backend 算法改动「贯通而非重写」+ ADR-044 ratified)

Phase 39 把 hybrid 检索信号——自 Phase 21 即存在于内核（`server.rs` hybrid 路径 + `search_hybrid` + `hybrid_score`）但**对外 REST 不可达**——经 console_data_plane proto add-only + 数据面 dispatch + Go console-api 转发**首次贯通到对外 `POST /v1/search`**，镜像 Phase 20 `?semantic` 范式。Console / REST 用户现可 `?hybrid=true`（或 body `{"hybrid":true}`）请求 hybrid 融合检索，响应携 `retrieval_method="hybrid"` + `hybrid_score` 融合分 provenance（与 `v1 RetrievalResult.hybrid_score=15` parity）。**0 backend 算法改动「贯通而非重写」**：复用既有 `search_hybrid`（task-21.1）+ `reranker_from_env`（task-38.2）+ `?semantic` 转发范式（task-20.1）+ `vector_score` provenance 范式（task-32.3）。**诚实校正（ADR-013）**：历史 `console-api-rerank-forward` 设想 `?rerank=true` per-request 转发，但 Phase 38（ADR-043 D3）已确立 reranker 服务端 env 驱动（非 per-request）——per-request 与 env 驱动**冲突**，故据实**重界定**：reranker 保持 env 驱动、`?rerank` per-request 记为 superseded、不实现，改交付 rerank `reason` provenance 在对外 REST 的可见性。**0 新 dep / 0 migration / 默认 `hybrid=false` 字节等价**（ADR-004/008/015，proto 既有字段号 1-7 / 1-16 冻结）。

| task | 交付 |
|---|---|
| 39.1 (#252) | console-dataplane-hybrid-proto-and-dispatch：console_data_plane proto add-only `SearchRequest.hybrid=8`（镜像 `v1/search.proto:28`）+ `SearchResultItem.hybrid_score=17`（镜像 `v1 RetrievalResult.hybrid_score=15`），既有字段号 1-7 / 1-16 冻结（ADR-015 D1）+ `buf generate proto` 重生 Go 生成代码（rawDesc 重编码，无 message/service 重排）；`core/src/data_plane/search.rs` `query()` hybrid dispatch 分支（`if req.hybrid {..} else if req.semantic {..} else {BM25}`，hybrid 分支镜像 `server.rs` hybrid 路径 + 数据面 semantic 分支：`DeterministicEmbeddingProvider` + hardcoded `BruteForceVectorBackend` + `search_hybrid` + `retrieval_method="hybrid"` + 复用 `reranker_from_env` opt-in）+ `hybrid_score` 填充（镜像 `vector_score`：`if h.retrieval_method == "hybrid" { h.score } else { 0.0 }`）；默认 `hybrid=false` 字节等价；0 新 dep / 0 migration。TEST-39.1.1（数据面 hybrid dispatch）+ TEST-39.1.2（proto 字段号 8/17 via prost wire tag + `hybrid_score` 填充条件） |
| 39.2 (#253) | console-api-hybrid-forward-and-rerank-visibility：`contractv1.SearchRequest.Hybrid bool`（json `hybrid`，镜像 `Semantic`）+ `SearchResult.HybridScore float32`（json `hybrid_score`，镜像 `VectorScore`，cross-repo add-only signal ADR-014 D4）+ `handleSearch` `?hybrid` OR-merge（镜像 `?semantic`）+ `grpcclient.Search` 转发 `Hybrid` + `protoToSearchResult` 映射 `HybridScore`（`Reason` rerank provenance 既有映射保留）；对外 `POST /v1/search` 贯通 hybrid；reranker 保持 env 驱动、**不加 `?rerank`**（per-request superseded by ADR-043 D3）；默认 `hybrid=false` 字节等价；0 新 dep / 0 proto 再改。TEST-39.2.1（`?hybrid` OR-merge + json round-trip）+ TEST-39.2.2（`grpcclient` 转发 `Hybrid` + 映射 `HybridScore` + rerank `Reason`） |
| 39.3 (this) | v0.32.0 closeout：smoke v28→v29 [48/48]（banner，staging offset；REAL 模式断言 `POST /v1/search?hybrid=true` → `retrieval_method="hybrid"` + `hybrid_score`，rerank `reason` provenance 可见）+ TestTask393（无 [37/37]..[47/47] 回归）+ release docs（README v0.32 段 + `:350` 措辞替换 + evidence/artifacts，tag/run/digest `<backfill>` marker）+ ADR-044 据 D1-D4 ratify（Proposed→Accepted）+ ADR-025 add-only Phase-39 Amendment（标 `console-api-hybrid-forward` fulfilled）+ ADR-043 add-only Phase-39 Amendment（标 `console-api-rerank-forward` 重界定 fulfilled + `?rerank` per-request superseded）+ roadmap §3.21/§4 + adapter + defer marker 更新 |

**ADR**：ADR-044 (console-api-retrieval-signal-forward) 据 D1-D4 真实非合成验证 `Proposed → Accepted`（D1 console_data_plane proto add-only + 数据面 hybrid dispatch / D2 Go console-api hybrid 转发 + rerank provenance 可见性 / D3 rerank-forward 重界定：reranker 保持 env 驱动 + `?rerank` superseded / D4 默认 hybrid=false / reranker unset 字节等价 + 0 新 dep + proto add-only 既有契约不变）；ADR-025 (hybrid-scoring-fusion) add-only Phase-39 Amendment——标 `console-api-hybrid-forward` fulfilled，**NOT** 溯改 D-body（ADR-014 D5）；ADR-043 (embedding-remote-reranker-live) add-only Phase-39 Amendment——标 `console-api-rerank-forward` 按 provenance-visibility 口径 fulfilled + `?rerank` per-request superseded by D3，**NOT** 溯改 D-body（ADR-014 D5）；ADR-004/008/015/013 守线。**ADR-014 cross-validation gate — 第三十次激活**。

**Upgrade / Rollback**：默认行为 / proto 既有字段 / 既有契约不变（既有 v0.6–v0.31 client + 索引/数据无需改动），**0 新代码依赖、0 migration、默认 `hybrid=false` 字节等价、reranker 默认 unset 字节等价无 rerank**（ADR-004/008/015，proto 既有字段号 1-7 / 1-16 冻结）；改动均为 console_data_plane proto add-only（`hybrid=8` / `hybrid_score=17`）+ 数据面 hybrid dispatch 分支 + Go console-api add-only（`Hybrid` / `HybridScore` + `?hybrid` OR-merge + 转发/映射）+ smoke/test/docs；不设 `?hybrid` / body `hybrid` → 走既有 semantic / BM25 路径逐字节等价。linux/amd64（承 v0.21–v0.31；arm64 仍 deferred）；cosign keyless sign + SBOM + provenance（承 task-28.2）。Rollback：`git tag -d v0.32.0` + 删 Release / ghcr tag；与 v0.31.0 行为兼容（默认 `hybrid=false` byte-equivalent，console_data_plane 新字段对 v0.31.0 client 无害——未知字段忽略）。tag SHA `5a84a711a4b7a1fbc405b7565513e86b01fff9b9`（annotated tag obj `a2661ab7568d829a53c192171748e7ede9d13e93`）/ release run id `27065533515`（success）/ ghcr digest `sha256:d5c2b807d633110a3f8acfbe552f9e2ef9835ad84615b08587196f16d2be434e`（ghcr.io/tajiaoyezi/contextforge-daemon:v0.32.0 + :latest，linux/amd64）/ cosign Rekor tlog `1739987095`（sign）·`1739987849`（attest）（post-tag-push 回填，ADR-012/013）。

## v0.31.0 (2026-06-06) — embedding-remote-reranker-live (ADR-026/042 remote-reranker 维度兑现：首次对真实远程 reranker 端点端到端联调 + env-gated live rerank quality harness `core/tests/remote_rerank_recall.rs` + Go [reranker] config env-bridge `setRerankerEnv` + 首次数据面 opt-in `with_reranker` 接线 + 真实实测 rerank MRR=1.0000 / recall@1=1.0000 vs identity 基线 0.4762/0.0000 + ADR-043 ratified)

Phase 38 兑现 ADR-026/042 的 remote-reranker 维度——首次对一个**真实远程 reranker 端点**端到端联调 + 实测 rerank 质量，并把 Go `[reranker]` 配置段经跨进程 env-bridge 接通到 core，**首次**把 reranker 经 opt-in `with_reranker` 接入两条数据面热路径。`RemoteRerankerProvider`（ureq, feature-gated）于 task-38.1 新增（镜像 `CrossEncoderReranker` by-index map-back + `RemoteEmbeddingProvider` 纯函数 build/parse + ureq + `Debug` 永不打印 api_key），并加 `select_reranker` 工厂 + `reranker-remote` feature；v0.31.0 仅新增 env-gated harness（`core/tests/remote_rerank_recall.rs`，task-38.1）+ Go `RerankerConfig`（无 api-key 字段）+ `setRerankerEnv` 跨进程 env-bridge + Rust `reranker_from_env()` + 首次数据面 opt-in `with_reranker` 接线（task-38.2）。**0 新 dep**（ureq 自 task-22.3 起即 optional）+ **0 proto / 0 migration / 0 默认行为改动**，默认构建 0-network / 默认 reranker 未置位 → `None` → byte-equivalent 不 rerank（向后兼容）；feature-off/unknown → 显式 `Status::internal`（不静默回退，ADR-013）；**API key env-only 永不进 config.toml**。真实口径（主 agent 本机真实 run，SiliconFlow `https://api.siliconflow.cn/v1/rerank` + `Qwen/Qwen3-VL-Reranker-8B`，与 embedding 同 URL+key 不同 model，3 次 run，ADR-013 真实非预填）：作者手工标注集 **14 case（含刻意近义干扰 `config_save↔config_load`、`bm25↔hybrid`、`cjk_index↔cjk_vector`、`cosine↔vector_backend`、`cache↔chunk`）**，计 rerank **MRR + recall@1**（非 embedding recall@3）→ **remote（Qwen3-VL-Reranker-8B）MRR = 1.0000 / recall@1 = 1.0000（14/14 相关项排第 1，3 次 run 全稳定）** vs **identity（无语义基线）MRR = 0.4762 / recall@1 = 0.0000（uniform score → chunk_id tie-break，相关项从不字典序第一 → recall@1=0）**，**delta_MRR = +0.5238 / delta_recall@1 = +1.0000**；harness 护栏（floor `MRR_remote>=0.70` + `MRR_remote>MRR_identity`）每次 run 均过。诚实判读（ADR-013）：MRR=1.0/recall@1=1.0 = 真实 cross-encoder 把明显相关文档排在刻意近义干扰之上的**正确性证明**，**非大基准质量断言**；与 Phase 37 embedding（recall@1 跨 run 波动 0.8667–0.9333）不同，本 rerank **3 次 run 全稳定**（cross-encoder 联合打分在此小集上更决断）；大语料/标准基准 rerank 质量续 honest-defer（`[SPEC-DEFER:phase-future.reranker-large-corpus-quality]`）。**CI honest-defer（关键诚实差异）**：remote reranker 是**付费外部 API、无免费 service container**（与 qdrant 不同——qdrant 有免费 OSS service container 在每次 CI run 守护召回）；CI 无密钥时 harness 干净 honest-defer skip，质量**由本机已认证 run 实测**而非每次 CI run 守护，复用 `[SPEC-DEFER:phase-future.embedding-remote-ci-credential]`（不另造 reranker-ci-credential），据 ADR-013 据实记录、不夸大为「CI 每次 run 守护」。

| task | 交付 |
|---|---|
| 38.1 (#247) | remote-reranker-provider + live-rerank-harness：新增 `RemoteRerankerProvider`（镜像 `CrossEncoderReranker` by-index map-back + `RemoteEmbeddingProvider` 纯函数 build/parse + ureq + `Debug` impl 永不打印 api_key）+ `select_reranker` 工厂 + `reranker-remote` feature（**0 新 dep**——ureq 自 task-22.3 即 optional）+ env-gated `core/tests/remote_rerank_recall.rs`（`#![cfg(feature = "reranker-remote")]`，仅当 `CONTEXTFORGE_RERANKER_API_KEY` 置位时运行，否则诚实 honest-defer skip——默认构建 0-network 不变）→ 对真实远程 reranker 端点端到端联调：作者手工标注集 **14 case（含刻意近义干扰）**，计 rerank **MRR + recall@1**；真实实测（SiliconFlow `Qwen/Qwen3-VL-Reranker-8B`，3 次 run）：remote **MRR=1.0000 / recall@1=1.0000（14/14 稳定）** vs identity MRR=0.4762/recall@1=0.0000，delta_MRR=+0.5238/delta_recall@1=+1.0000；harness 护栏（floor `MRR_remote>=0.70` + `MRR_remote>MRR_identity`）每次 run 均过；de-risk 可行性探针（**非** harness MRR/recall）：query "how to save config to file" → `config_save` relevance_score=0.7356 排第 1 vs 近义干扰 `config_load`=0.0158（约 46x，前次 session）/ 本 session curl 探针 `config_load`=0.2196 排第 1，HTTP 200；honest-caveat：小型手工标注集 → 正确性证明非大基准质量断言（`[SPEC-DEFER:phase-future.reranker-large-corpus-quality]`）；CI honest-defer：remote 付费外部 API 无免费 service container（与 qdrant 异），复用 `[SPEC-DEFER:phase-future.embedding-remote-ci-credential]` |
| 38.2 (#248) | remote-reranker-config-bridge + data-plane-wiring：Go `RerankerConfig`（**无 api-key 字段**）+ `setRerankerEnv` 跨进程 env-bridge（镜像 `setRemoteEnv` task-37.2 先例；**ENV WINS**；**API key 永不桥接**）+ Rust `reranker_from_env()` + **首次数据面 opt-in `with_reranker` 接线** `server.rs`（hybrid+semantic）+ `data_plane/search.rs`（semantic）；默认未置位 → `None` → byte-equivalent 不 rerank（向后兼容）；feature-off/unknown → 显式 `Status::internal`（不静默回退）；Rust core 保持 **0 toml dep**；0 新 dep / 0 proto / 0 migration；TEST-38.2.*（config `[reranker]` round-trip + 无 api-key 字段 + setRerankerEnv export/env-wins + API key 永不桥接 + reranker_from_env feature-off 诚实 Err）；go test ./... + go vet clean |
| 38.3 (this) | v0.31.0 release docs + smoke v28 [47/47]（banner v27→v28）+ TestTask383（无回归）+ ADR-043 据 per-D ratify + ADR-026 + ADR-042 add-only Phase 38 Amendment（标记 remote-reranker 维度 **fulfilled**：真实远程 reranker 端点端到端联调 + 实测 rerank MRR/recall@1 + Go config bridge + 首次数据面 opt-in `with_reranker` 接线，不溯改 D-body ADR-014 D5）+ roadmap §3.20/§4 + phase-38 §6 闭合；honest-caveat 重申：大语料/标准基准 rerank 质量未压测（`[SPEC-DEFER:phase-future.reranker-large-corpus-quality]`）；CI honest-defer 重申（remote 付费 API 无免费 service container，质量由本机已认证 run 实测非每次 CI run 守护，复用 `[SPEC-DEFER:phase-future.embedding-remote-ci-credential]`，ADR-013 不夸大） |

**ADR**：ADR-043 (embedding-remote-reranker-live) 据 per-D 真实非合成验证 `Proposed → Accepted`（D1 `RemoteRerankerProvider` + `select_reranker` 工厂 + `reranker-remote` feature + env-gated live harness + 真实远程端点端到端联调 / D2 真实实测 remote MRR=1.0000 / recall@1=1.0000（14/14 稳定，3 次本机已认证 run）vs identity 0.4762/0.0000，delta_MRR=+0.5238 / D3 Go `[reranker]` config（无 api-key 字段）+ `setRerankerEnv` env-bridge + 首次数据面 opt-in `with_reranker` 接线（hybrid+semantic）+ 默认 byte-equivalent 不 rerank + feature-off/unknown 显式 Err 不静默回退 / D4 honest-caveat：小型手工标注集 → 正确性证明非大基准质量断言（`[SPEC-DEFER:phase-future.reranker-large-corpus-quality]`）+ CI honest-defer：remote 付费外部 API 无免费 service container（与 qdrant 异），质量由本机已认证 run 实测非每次 CI run 守护，复用 `[SPEC-DEFER:phase-future.embedding-remote-ci-credential]`，ADR-013 不夸大 / D5 default 0-network·0-远程-dep + 0 新 dep + 0 proto + 0 migration + 0 默认行为改动 baseline 不变 + API key env-only 永不进 config）；ADR-026 (cross-encoder-reranker 母 ADR) + ADR-042 (embedding-provider-remote-live 母 ADR) add-only Phase 38 Amendment——标记其 remote-reranker 维度 **fulfilled**（真实远程 reranker 端点端到端联调 + 实测 rerank MRR/recall@1 + Go config bridge + 首次数据面 opt-in `with_reranker` 接线），**NOT** 溯改 D-body（ADR-014 D5）；ADR-004/008/013 守线。**ADR-014 cross-validation gate — 第二十九次激活**。

**Upgrade / Rollback**：默认行为 / proto 既有字段 / 既有契约不变（既有 v0.6–v0.30 client + 索引/数据无需改动），**0 新代码依赖、0 proto、0 migration、0 默认行为改动、0 network 默认构建**（ADR-004/008，ureq 自 task-22.3 即 optional / `remote_rerank_recall.rs` env-gated 默认 honest-defer skip）；reranker-remote opt-in，默认构建 0-network / 0-远程-dep，默认 reranker 未置位 → `None` → byte-equivalent 不 rerank 不变；改动均 test/CI + Go config-bridge + Rust 数据面 opt-in 接线（live rerank harness env-gated + Go `[reranker]`/`setRerankerEnv` + `with_reranker` wiring），默认路径不改任何返回值/控制流/RPC 行为；feature-off/unknown reranker → 显式 `Status::internal`（不静默回退）；**API key env-only 永不进 config.toml**（仅经 `CONTEXTFORGE_RERANKER_API_KEY` 环境变量，永不桥接）。linux/amd64（承 v0.21–v0.30；arm64 仍 deferred）；cosign keyless sign + SBOM + provenance（承 task-28.2）。Rollback：`git tag -d v0.31.0` + 删 Release / ghcr tag；与 v0.30.0 行为兼容（harness env-gated 默认不运行，Go `[reranker]` 段对 v0.30.0 二进制无害——未知 section 忽略，默认 reranker 未置位 byte-equivalent）。tag SHA `5a84a711a4b7a1fbc405b7565513e86b01fff9b9`（annotated tag obj `a2661ab7568d829a53c192171748e7ede9d13e93`）/ release run id `27065533515`（success）/ ghcr digest `sha256:d5c2b807d633110a3f8acfbe552f9e2ef9835ad84615b08587196f16d2be434e`（ghcr.io/tajiaoyezi/contextforge-daemon:v0.31.0 + :latest，linux/amd64）/ cosign Rekor tlog `1739987095`（sign）·`1739987849`（attest）（post-tag-push 回填，ADR-012/013）。

## v0.30.0 (2026-06-06) — embedding-provider-remote-live (ADR-027 honest-defer 兑现：首次对真实远程 embedding 端点端到端联调 + env-gated live recall harness `core/tests/remote_embedding_recall.rs` + Go [remote] config env-bridge `setRemoteEnv` + 真实实测 recall@3=1.0000 / recall@1=0.8667–0.9333 vs deterministic 基线 + ADR-042 ratified)

Phase 37 兑现 ADR-027 honest-defer `[SPEC-DEFER:phase-future.embedding-provider-remote]`——首次对一个**真实远程 embedding 端点**端到端联调 + 实测语义召回，并把 Go `[remote]` 配置段经跨进程 env-bridge 接通到 core。`RemoteEmbeddingProvider`（ureq, feature-gated）自 Phase 22（task-22.3）起已完整实现；v0.30.0 仅新增 env-gated harness（`core/tests/remote_embedding_recall.rs`，task-37.1）+ Go `RemoteProviderConfig` add-only `Model` + `setRemoteEnv` 跨进程 env-bridge（task-37.2）。**0 backend 改动、0 新 dep**（ureq 自 task-22.3 起即 optional）+ 0 migration，默认构建 0-network / 0-远程-dep 不变（ADR-004/008）；**API key env-only 永不进 config.toml**。真实口径（主 agent 本机真实 run，SiliconFlow `https://api.siliconflow.cn/v1/embeddings` + `Qwen/Qwen3-Embedding-8B`，dim=1024，3 次 run，ADR-013 真实非预填）：作者手工标注集 **15 case / 16 doc（含近义干扰）**，real 远程模型 vs deterministic（model-free）基线走**同一 `BruteForceVectorBackend` 精确余弦路径** → **remote recall@3 = 1.0000（15/15，3 次 run 全稳定）/ recall@1 = 0.8667–0.9333（13–14/15，跨 run 波动——remote 模型/服务非完全确定）** vs deterministic recall@1=0.0000 / recall@3=0.0667（稳定），**delta@3 = +0.9333**；@1 的 1–2 个 miss 恰是故意埋的硬近义干扰（`config_save↔config_load`、`hybrid↔bm25`）的 top-1 让位。诚实判读（ADR-013）：小型手工标注集，recall@3=1.0 证明模型把明显语义对排在近义干扰之上、**非大基准质量断言**；大语料语义质量续 honest-defer（`[SPEC-DEFER:phase-future.embedding-large-corpus-recall]`）。**CI honest-defer（关键诚实差异）**：remote 是**付费外部 API、无免费 service container**（与 qdrant 不同——qdrant 有免费 OSS service container 在每次 CI run 守护召回）；CI 无密钥时 harness 干净 honest-defer skip，真实召回**由本机已认证 run 实测**而非每次 CI run 守护，据 ADR-013 据实记录、不夸大为「CI 每次 run 守护」。

| task | 交付 |
|---|---|
| 37.1 (#242) | remote-embedding-live-recall-harness：新增 env-gated `core/tests/remote_embedding_recall.rs`（仅当 `CONTEXTFORGE_REMOTE_API_KEY` 置位时运行，否则诚实 honest-defer skip——默认构建 0-network 不变）→ 对真实远程 embedding 端点端到端联调：作者手工标注集 **15 case / 16 doc（含近义干扰）**，real 远程模型 vs deterministic（model-free）基线走**同一 `BruteForceVectorBackend` 精确余弦路径**计 recall@1/@3；**0 backend 改动 / 0 新 dep**（ureq 自 task-22.3 即 optional，feature-gated）；真实实测（SiliconFlow `Qwen/Qwen3-Embedding-8B`，dim=1024，3 次 run）：remote **recall@3=1.0000（15/15 稳定）/ recall@1=0.8667–0.9333（跨 run 波动）** vs det recall@1=0.0000/@3=0.0667，delta@3=+0.9333；harness 护栏（floor `r3>=0.70` + `remote@1>det@1`）每次 run 均过；honest-caveat：小型手工标注集 → 正确性证明非大基准质量断言（`[SPEC-DEFER:phase-future.embedding-large-corpus-recall]`）；CI honest-defer：remote 付费外部 API 无免费 service container（与 qdrant 异），CI 无密钥 skip，真实召回由本机已认证 run 实测 |
| 37.2 (#243) | remote-embedding-config-bridge：Go `RemoteProviderConfig` 加 add-only `Model` 字段 + `setRemoteEnv` 跨进程 env-bridge（镜像 `setVectorEnv` task-34.2 先例）桥接 `[remote]` 段到 core embedding env；**ENV WINS**（显式 env 覆盖 config）；**API key env-only 永不进 config**（`config.toml` 仅 endpoint/model，密钥仅经 `CONTEXTFORGE_REMOTE_API_KEY` 环境变量）；Rust core 保持 **0 toml dep**；0 新 dep / 0 migration；TEST-37.2.*（config `[remote]` round-trip + Model add-only + setRemoteEnv export/env-wins + API key 永不进 config）；go test ./... + go vet clean |
| 37.3 (#244) | v0.30.0 release docs + smoke v27（banner v26→v27）+ 无回归 + ADR-042 据 per-D ratify + ADR-027 add-only Phase 37 Amendment（标记 `embedding-provider-remote` honest-defer **fulfilled**：真实远程端点端到端联调 + 实测召回 + Go config bridge 接通，不溯改 D-body ADR-014 D5）+ roadmap §3.19/§4 + phase-37 §6 闭合；honest-caveat 重申：大语料语义质量未压测（`[SPEC-DEFER:phase-future.embedding-large-corpus-recall]`）；CI honest-defer 重申（remote 付费 API 无免费 service container，真实召回由本机已认证 run 实测非每次 CI run 守护，ADR-013 不夸大） |

**ADR**：ADR-042 (embedding-provider-remote-live) 据 D1-D4 真实非合成验证 `Proposed → Accepted`（D1 env-gated live harness + 真实远程端点端到端联调 + real 模型 vs deterministic 基线同一精确余弦路径 / D2 真实实测 remote recall@3=1.0000（15/15 稳定）/ recall@1=0.8667–0.9333（跨 run 波动，remote 模型/服务非完全确定），delta@3=+0.9333，3 次本机已认证 run / D3 honest-caveat：小型手工标注集 → 正确性证明非大基准质量断言（`[SPEC-DEFER:phase-future.embedding-large-corpus-recall]`）+ CI honest-defer：remote 付费外部 API 无免费 service container（与 qdrant 异），真实召回由本机已认证 run 实测非每次 CI run 守护，ADR-013 不夸大 / D4 default 0-network·0-远程-dep + 0 backend 改动 + 0 新 dep + 0 migration baseline 不变 + API key env-only 永不进 config）；ADR-027 (embedding-provider-completion 母 ADR) add-only Phase 37 Amendment——标记其 `embedding-provider-remote` honest-defer **fulfilled**（真实远程端点端到端联调 + 实测召回 + Go config bridge 接通），**NOT** 溯改 D-body（ADR-014 D5）；ADR-004/008/013 守线。**ADR-014 cross-validation gate — 第二十八次激活**。

**Upgrade / Rollback**：默认行为 / proto 既有字段 / 既有契约不变（既有 v0.6–v0.29 client + 索引/数据无需改动），**0 backend 改动、0 新代码依赖、0 migration、0 network 默认构建**（ADR-004/008，ureq 自 task-22.3 即 optional / `remote_embedding_recall.rs` env-gated 默认 honest-defer skip）；embedding-remote opt-in，默认构建 0-network / 0-远程-dep / 0 远程 dep 不变；改动均 test/CI + Go config-bridge（live recall harness env-gated + Go `[remote]` Model/`setRemoteEnv`），不改任何返回值/控制流/RPC 行为；**API key env-only 永不进 config.toml**（仅经 `CONTEXTFORGE_REMOTE_API_KEY` 环境变量）。linux/amd64（承 v0.21–v0.29；arm64 仍 deferred）；cosign keyless sign + SBOM + provenance（承 task-28.2）。Rollback：`git tag -d v0.30.0` + 删 Release / ghcr tag；与 v0.29.0 行为兼容（harness env-gated 默认不运行，Go `[remote]` Model 字段对 v0.29.0 二进制无害——未知字段忽略）。tag SHA `b49f28803e73338997f04bc3ffad85e7d386edf5`（annotated tag obj `38bf3c2f86241ba25be8f64456c8258d3c5d12ff`）/ release run id `27050883547`（success）/ ghcr digest `sha256:ff1306bf088452df8cdc78d5f5f0c35bcda0e654258bcbfc0cbba5a4992fb95c`（ghcr.io/tajiaoyezi/contextforge-daemon:v0.30.0 + :latest，linux/amd64）/ cosign Rekor tlog `1738028951`（sign）·`1738031045`（attest）（post-tag-push 回填，ADR-012/013）。

## v0.29.0 (2026-06-04) — qdrant-live-vector-recall (ADR-034 D2 honest-defer 兑现：env-gated live qdrant KNN recall harness `core/tests/qdrant_live_recall.rs` + qdrant-recall CI service-container job 永久守护 + 真实 CI-measured recall@10=1.0000 vs BruteForce 精确 KNN ground truth + ADR-041 ratified)

Phase 36 兑现 ADR-034 D2 honest-defer `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`——对一个**真实运行的 qdrant 服务**测量 live KNN 召回，并经 CI service container 永久守护。qdrant backend（connect/health/ensure-create/upsert/KNN/delete）自 Phase 25/29 起已完整实现；v0.29.0 仅新增 env-gated harness（`core/tests/qdrant_live_recall.rs`，task-36.1）+ 带 qdrant service container 的 `qdrant-recall` CI job（task-36.2）+ closeout（task-36.3）。**0 backend 改动、0 新 dep**（qdrant-client 自 task-18.4 起即 optional），默认构建 0-vector-dep / 0-network 不变（ADR-004/008）。真实口径：**CI 实测 recall@10 = 1.0000**（N=2000 语料、dim=64、M=50 queries，qdrant LIVE KNN vs BruteForce 精确 KNN ground truth），于 CI run `26961084355`（`qdrant-recall` job，"qdrant ready after 1 attempt(s)"，"test result: ok. 2 passed; 0 failed"），并在本地 `qdrant/qdrant` docker container 复现。诚实 caveat（ADR-013）：recall=1.0 因 N=2000（低于 qdrant HNSW `indexing_threshold` 默认约 10000）qdrant 服务 **EXACT KNN**——故此为 live-KNN **正确性证明**（qdrant == brute-force 精确 ground truth），替换了合成 `eval_integration.rs` 0.7/0.85 fixture；压测 HNSW **近似**区间（大语料 > indexing_threshold + optimizer 构建索引）honest-defer（`[SPEC-DEFER:phase-future.vector-large-corpus-perf]`），**不宣称已压测 HNSW 近似**。

| task | 交付 |
|---|---|
| 36.1 (#236) | qdrant-live-recall harness：新增 env-gated `core/tests/qdrant_live_recall.rs`（仅当 `CONTEXTFORGE_QDRANT_URL` 置位时运行，否则诚实跳过——默认构建 0-network 不变）→ 对真实 qdrant 服务 connect/health/ensure-create/upsert（N=2000、dim=64）+ KNN（M=50 queries）+ delete 全生命周期；以 BruteForce 精确 KNN 为 ground truth 计算 recall@10；**0 backend 改动 / 0 新 dep**（qdrant-client 自 task-18.4 即 optional，feature-gated）；honest-caveat：N=2000 < HNSW `indexing_threshold` → qdrant 服务 EXACT KNN → 此为 live-KNN 正确性证明非 HNSW 近似压测（`[SPEC-DEFER:phase-future.vector-large-corpus-perf]`）；本地 `qdrant/qdrant` docker container 复现 |
| 36.2 (#237) | qdrant-recall CI service job：CI 新增 `qdrant-recall` job，以 `qdrant/qdrant` service container 起真实 qdrant + readiness 轮询（"qdrant ready after 1 attempt(s)"）→ 置 `CONTEXTFORGE_QDRANT_URL` 跑 `qdrant_live_recall.rs`（feature-gated `cargo test`）→ 永久守护 live KNN 召回；CI run `26961084355` 实测 **recall@10 = 1.0000**（"test result: ok. 2 passed; 0 failed"），替换合成 `eval_integration.rs` 0.7/0.85 fixture 为真实 live-KNN == brute-force 精确 ground truth correctness 守护；默认构建 0-network 不受影响（job-scoped service container） |
| 36.3 (this) | v0.29.0 release docs + smoke v26 [45/45]（banner v25→v26，staging cf-v28-cfg）+ TestTask363（无回归）+ ADR-041 据 per-D ratify + ADR-034 add-only Phase 36 Amendment（标记 D2 qdrant-server-lifecycle fulfilled：live KNN recall 已测 + CI 守护，不溯改 D-body ADR-014 D5）+ roadmap §3.18/§4 + phase-36 §6 闭合；honest-caveat 重申：HNSW 近似区间未压测（`[SPEC-DEFER:phase-future.vector-large-corpus-perf]`） |

**ADR**：ADR-041 (qdrant-live-vector-recall) 据 per-D 真实非合成验证 `Proposed → Accepted`（D1 env-gated live harness + connect/health/ensure-create/upsert/KNN/delete 全生命周期 / D2 真实 CI-measured recall@10=1.0000 vs BruteForce 精确 KNN ground truth，run `26961084355` / D3 honest-caveat：N=2000 < HNSW indexing_threshold → qdrant EXACT KNN → live-KNN 正确性证明非近似压测，HNSW 近似区间 honest-defer 不夸大 ADR-013 / D4 default 0-vector-dep·0-network + 0 backend 改动 + 0 新 dep baseline 不变）；ADR-034 (vector-store-abstraction-and-backends 母 ADR) add-only Phase 36 Amendment——标记其 D2 `qdrant-server-lifecycle` honest-defer **fulfilled**（live KNN recall 已测 + CI service-container 永久守护），**NOT** 溯改 D-body（ADR-014 D5）；ADR-004/008/013 守线。**ADR-014 cross-validation gate — 第二十七次激活**。

**Upgrade / Rollback**：默认行为 / proto 既有字段 / 既有契约不变（既有 v0.6–v0.28 client + 索引/数据无需改动），**0 backend 改动、0 新代码依赖、0 network 默认构建**（ADR-004/008，qdrant-client 自 task-18.4 即 optional / qdrant_live_recall.rs env-gated 默认跳过）；改动均 test/CI-only（live recall harness + qdrant-recall service-container job），不改任何返回值/控制流/RPC 行为。linux/amd64（承 v0.21–v0.28；arm64 仍 deferred）；cosign keyless sign + SBOM + provenance（承 task-28.2）。Rollback：`git tag -d v0.29.0` + 删 Release / ghcr tag；与 v0.28.0 行为兼容（harness env-gated 默认不运行，CI job job-scoped 不影响默认构建）。tag SHA `0a64c88dfdaf32d65d07a0639a98ef1000a6ade2`（annotated tag obj `e7062794719d6e39f1865f977e9642129ed52f09`）/ release run id `26962915074`（success）/ ghcr digest `sha256:2f0d3b82b738dd4cc0007d267a46f9149c73806e39928910d822f46fc72b87e3`（ghcr.io/tajiaoyezi/contextforge-daemon:v0.29.0 + :latest，linux/amd64）/ cosign Rekor tlog `1721985223`（sign）·`1721988377`（attest）（post-tag-push 回填，ADR-012/013）。

## v0.28.0 (2026-06-04) — observability-hardening (热路径静默错误显式化：index_session_backend store.append ×4 + retriever Tantivy/SQLite desync 经 eprintln! WARN + setVectorEnv config.Load/Setenv 经 fmt.Fprintf(os.Stderr) + 7→3-4 grounding 校正 + ADR-040 ratified)

Phase 35 承 Phase 31/33 治理债血脉，把热路径中**被静默吞掉的真实错误**显式化（surface genuinely-swallowed errors），镜像仓库既有 stderr 惯例（Rust `eprintln!` / Go `fmt.Fprintf(os.Stderr)`）。一个刻意保持**小**的版本（第三轮债清理性质、边际递减，honesty over padding，ADR-013；经 AskUserQuestion 2026-06-04 用户授权）。三条收口：(1) `index_session_backend.rs` **4 处** `store.append`（progress/index-error/commit-error/cancelled）`let _ =` → `if let Err(persist_err) { eprintln!("WARN indexing-event persist failed …: {persist_err}") }`（SQLite persist 失败=磁盘满/锁，不再无声吞掉）+ `retriever/mod.rs:415` `Err(_) => continue`（Tantivy/SQLite desync）→ `Err(e) => { eprintln!("WARN retriever: … desync …"); continue }`（skip 保留）；`eb.send` 保留 as-is（no-subscribers intentional）。(2) `setVectorEnv` `config.Load`/`os.Setenv` 失败 → `fmt.Fprintf(os.Stderr)`，`errors.Is(os.ErrNotExist)` 守护（missing 静默/malformed 报警）。(3) 7→3-4 grounding 校正：DROP/LEAVE 4 处已显式化/有意 site，不引新 metrics facility。**observability-only**——best-effort 契约不变（indexing 不阻断、query 续行、daemon 不阻断，**绝不**转 fail-fast，ADR-004），四门不退化。诚实口径：Rust eprintln! 输出仓库不断言 → guard/inspection（`[SPEC-DEFER:phase-future.rust-stderr-output-assertion]`）；`memstore.go:579` nil-sink = honest non-issue（生产 sink 总接线，`[SPEC-DEFER:phase-future.memstore-degraded-observability-warn]`）；core 无 metrics facility → 不引（`[SPEC-DEFER:phase-future.observability-metrics-facility]`）。

| task | 交付 |
|---|---|
| 35.1 (#229) | rust-silent-failure-surfacing：`index_session_backend.rs` **4 处** `store.append` `let _ =` → `if let Err(persist_err) { eprintln! }`（best-effort 不阻断 indexing；grounding 发现 4 处非 1，一致显式化）+ `retriever/mod.rs:415` `Err(_) => continue` → `Err(e) => { eprintln!; continue }`（skip 保留）；`eb.send` 保留 as-is（no-subscribers intentional）；镜像 `search.rs:108-113`；store 具体类型无 trait → 不引 trait 注入失败 double（scope creep），error 分支 inspection-verified；0 新 dep；TEST-35.1.1（真实 store best-effort 行为锁）+ TEST-35.1.2（删 chunks 行造 desync → search 优雅跳过）；lib 209→212 |
| 35.2 (#230) | go-silent-failure-surfacing：`setVectorEnv` `config.Load` 错误（`errors.Is(os.ErrNotExist)` 守护：missing 静默/malformed 报警，因 config.Load 对缺文件也 error）+ `os.Setenv` 失败 → `fmt.Fprintf(os.Stderr)`，镜像 `daemon/rest.go:110`，best-effort 保留（env-only 路径失败时不变）；grounding 校正 `memstore.go:579` nil-sink = honest non-issue（DROP，0 改动——`NewMemMemoryStore()` 唯一生产调用点 `console_api_serve.go:109` 紧随无条件 `SetEventSink`:112）；0 新 dep；TEST-35.2.1 stderr-capture(os.Pipe) 真 RED→GREEN（malformed→WARN / missing→no WARN / valid→no WARN）；go test ./... + go vet clean |
| 35.3 (this) | observability-hardening 7→3-4 grounding 校正如实记录（`search.rs:109` already-surfaced + core 无 metrics facility / `server.go:298` task-31.3 already-done / `allowlist.go:31` 有意 POSIX-only / `eb.send:193` 有意 no-subscribers DROP/LEAVE 不改代码，不引新 metrics facility）；smoke v25 [44/44]（banner v24→v25，staging cf-v27-cfg）+ TestTask353（无回归 [37/37]..[43/43]）+ v0.28.0 release docs + ADR-040 ratify + ADR-031 add-only Phase 35 Amendment + roadmap §3.17/§4 + phase-35 §6 闭合 |

**ADR**：ADR-040 (observability-hardening) 据 D1-D4 真实非合成验证 `Proposed → Accepted`（D1 rust surfacing Accepted + Rust eprintln! guard/inspection 不伪造 stderr-assert / D2 go surfacing Accepted + memstore nil-sink honest non-issue grounding 校正 / D3 7→3-4 grounding 校正诚实收敛不引新 metrics facility / D4 default 0-dep·0-network + 既有契约不变 best-effort 不转 fail-fast）；ADR-031 (observability-hardening v0.19.0 母 ADR) add-only Phase 35 Amendment（承其 stderr/best-effort surfacing 方向延伸热路径静默错误，不溯改正文 ADR-014 D5）；ADR-036（task-31.3 Go stderr audit-surfacing pattern 镜像源）/ ADR-004/008 守线。**ADR-014 cross-validation gate — 第二十六次激活**。

**Upgrade / Rollback**：默认行为 / proto 既有字段 / 既有契约不变（既有 v0.6–v0.27 client + 索引/数据无需改动），0 新代码依赖、0 network 默认构建（ADR-004/008，不引 log/tracing/metrics facility）；改动均 observability-only（仅 stderr 旁路 WARN，不改任何返回值/控制流/RPC 行为，best-effort 不转 fail-fast）；`memstore.go` 0 改动。linux/amd64（承 v0.21–v0.27；arm64 仍 deferred）；cosign keyless sign + SBOM + provenance（承 task-28.2）。Rollback：`git tag -d v0.28.0` + 删 Release / ghcr tag；与 v0.27.0 行为兼容（surfacing 仅新增 stderr WARN 旁路）。tag SHA `aa209ddd6d722ad2aecd5a72a0d6dbb467de7706`（annotated tag obj `550deb08aa3222375fa96d7a607e47d1a2ea74bd`）/ release run id `26934172978`（success）/ ghcr digest `sha256:ae5c4d0cdfbfc068b211df25c4afbd195711aacfda4d7491231cfe8d92b77257` / cosign Rekor tlog `1715917623`（sign）·`1715919741`（attest）（post-tag-push 回填，ADR-012）。

## v0.27.0 (2026-06-03) — vector-config-completeness (向量 dim 自动协商 negotiate_vector_dim + [vector] 配置段 Go→core env 桥接 + get_source_chunk workspace 隔离 verify-only grounding 校正 + ADR-039 ratified)

Phase 34 补完 Phase 32 起的 vector-backend 配置闭环（post-Phase 31/33 绿色 backlog 偏薄，honesty over padding，ADR-013）。三条收口：(1) `factory.rs` `select_vector_backend` 不再 `let _ = dim;` 静默丢弃 `CONTEXTFORGE_VECTOR_DIM` —— 构造 backend 后调新增纯函数 `negotiate_vector_dim(requested, declared)`（仿 `embedding::factory::negotiate_dim`）复用既有 `VectorError::DimMismatch`（types.rs:83）；`VectorBackend` trait 加 add-only 默认 `expected_dim()->Option<usize>` 返回 `None`（dim-agnostic），BruteForce 保持 `None`。(2) Go `config.Config` 加 add-only `[vector]` 段（Backend/Dim）+ `setVectorEnv(dataDir)` helper 桥接到 `CONTEXTFORGE_VECTOR_BACKEND`/`_DIM`，接线 doServe + doMCP。(3) `get_source_chunk` workspace 隔离经核 **已随 task-12.2 交付**（search.rs:421-423 已 scope candidates 到 `req.workspace_id`，survey 高估为缺口）→ VERIFY-ONLY guard 加真实测试。**默认行为 / proto 既有字段 / 既有契约不变**（ADR-004），四门不退化。诚实口径：**默认 BruteForce dim-agnostic** → 默认构建接受任意 dim 且 byte-equivalent（ADR-004），真实 dim enforcement 仅对声明 dim 的 feature backend 生效（`[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`）；Rust core 保持 **0 toml dep**（`grep -c ^toml core/Cargo.toml = 0`），配置经 Go 解析 + env 桥接（**ENV WINS**：显式 env 覆盖 config；无 `[vector]` 段 → 不导出 → unset → BruteForce byte-equivalent）。

| task | 交付 |
|---|---|
| 34.1 (#224) | vector-dim-auto-negotiation：`factory.rs` `select_vector_backend` 移除 `let _ = dim;`（静默丢弃 `CONTEXTFORGE_VECTOR_DIM`）→ 构造 backend 后调新增纯函数 `negotiate_vector_dim(requested, declared)`（仿 `embedding::factory::negotiate_dim`）复用既有 `VectorError::DimMismatch`（types.rs:83）；`VectorBackend` trait 加 add-only 默认 `expected_dim()->Option<usize>` 返回 `None`（dim-agnostic），BruteForce 保持 `None`；honest-caveat：默认 BruteForce dim-agnostic → 默认构建接受任意 dim 且 byte-equivalent（ADR-004），真实 enforcement 仅对声明 dim 的 feature backend 生效（`[SPEC-DEFER:phase-future.vector-dim-feature-enforce]`）；0 新 dep；TEST-34.1.1（negotiate 四路径）+ TEST-34.1.2（BruteForce any-dim byte-equiv）；lib 207→209 |
| 34.2 (#225) | vector-backend-config-file：Rust core 无 toml dep（0-dep 守线），Go `config.go` 已解析 `[collections]`/`[remote]`/`[embedding]` 且 spawn 的 core daemon 继承 Go env（`setDataDirEnv`/`CONTEXTFORGE_DATA_DIR`）→ Go `config.Config` 加 add-only `[vector]` 段（Backend/Dim，struct + encode/decode/assignVector）+ `setVectorEnv(dataDir)` helper（main.go）best-effort `config.Load` + 桥接 `[vector]` 到 `CONTEXTFORGE_VECTOR_BACKEND`/`_DIM`，接线 doServe + doMCP；**ENV WINS**（显式 env 覆盖 config）；无 `[vector]` 段 → 不导出 → unset → BruteForce byte-equivalent（ADR-004）；Rust core 保持 0 toml dep（`grep -c ^toml core/Cargo.toml = 0`）；0 新 dep；TEST-34.2.1（config `[vector]` round-trip + 既有段不受影响 + legacy zero value）+ TEST-34.2.2（setVectorEnv export/env-wins/empty）；go test ./... + go vet clean |
| 34.3 (this) | get_source_chunk workspace 隔离 VERIFY-ONLY guard（grounding 校正 —— 经核已随 task-12.2 交付，search.rs:421-423 已 scope candidates 到 `req.workspace_id`，survey 高估为缺口）：TEST-34.3.1 构真实 2-state 索引断言 workspace_id 设置 → 仅该 workspace / cross-workspace → not_found / 空 → aggregate；smoke v24 [43/43]（banner v23→v24，staging cf-v26-cfg）+ TestTask343（无回归 [37/37]..[42/42]）+ v0.27.0 release docs + ADR-039 ratify + ADR-037 add-only Phase 34 Amendment + roadmap §3.16/§4 + phase-34 §6 闭合 |

**ADR**：ADR-039 (vector-config-completeness) 据 D1-D4 真实非合成验证 `Proposed → Accepted`（D1 dim-negotiation Accepted + feature-enforce 🟡 honest-defer / D2 config-file Accepted / D3 get_source_chunk isolation 经核 already-present verify-only grounding-correction / D4 default 0-dep·0-network + 既有契约不变）；ADR-037 (vector-backend-config-plumbing-and-completeness) add-only Phase 34 Amendment（dim-negotiation + config-file 补完 Phase 32 env-plumbing，兑现其 `[SPEC-DEFER:phase-future.vector-backend-config-file]` Follow-up，不溯改正文 ADR-014 D5）；ADR-004 守线。**ADR-014 cross-validation gate — 第二十五次激活**。

**Upgrade / Rollback**：默认行为 / proto 既有字段 / 既有契约不变（既有 v0.6–v0.26 client + 索引/数据无需改动），0 新代码依赖、0 network 默认构建（ADR-004/008）；vector dim 自动协商（默认 BruteForce dim-agnostic → 接受任意 dim byte-equivalent）+ Go `[vector]` 配置段 → core env 桥接（ENV WINS；无段 → unset → BruteForce byte-equivalent）+ get_source_chunk workspace 隔离 verify-only（已随 v0.6.x task-12.2 在线）均 add-only / 不破协议；Rust core 保持 0 toml dep。linux/amd64（承 v0.21–v0.26；arm64 仍 deferred）；cosign keyless sign + SBOM + provenance（承 task-28.2）。Rollback：`git tag -d v0.27.0` + 删 Release / ghcr tag；与 v0.26.0 行为兼容（Go `[vector]` 段对 v0.26.0 二进制无害——未知 section 忽略）。tag SHA `a88e8f366e8c7789cee189c8b4315b2420dd7e60`（annotated tag obj `118e89b1`）/ release run id `26894328991`（success）/ ghcr digest `sha256:bf1119a37be3446cde891ab64598303f4e14a010da4e7dd55eb106b2e81020a2` / cosign Rekor tlog `1710499497`（sign）·`1710500590`（attest）（post-tag-push 回填，ADR-012）。

## v0.26.0 (2026-06-03) — governance-debt-cleanup-2 (L2 SQLite 缓存 rowid-FIFO 有界 + console memstore 访问序 LRU + hard-delete 无悬挂引用不变式 + indexing.* 事件持久化/重放 migration 0019 + TraceStore workspace 隔离 add-only proto + export --timeout + ADR-038 ratified)

Phase 33 接续 Phase 31 清理第二波跨 Phase 治理债：缓存有界化补完（embedding L2 SQLite 行数上限 + rowid-FIFO 淘汰）、console-api memstore chunk/trace 缓存由 FIFO 升级为访问序 LRU（读命中 + 既有键覆写均 move-to-front）、memory hard-delete 无悬挂引用不变式断言、indexing.* 索引事件持久化 + 重放映射（add-only migration `0019_indexing_events`，best-effort 旁挂既有 `eb.send` 广播）、TraceStore workspace 隔离（add-only proto `workspace_id` 字段，空值 = aggregate-all byte-equivalent）、export `--timeout` add-only flag。**默认行为 / proto 既有字段 / 既有契约不变**（ADR-004），三门 + lint 不退化；**0 新代码依赖**；新增 proto 字段与 migration 均 add-only / 不破协议。诚实校正：**drain-timeout 经核 Phase 26 已交付**（verify-only，0 代码改动，引 `TestDrainTimeoutFromEnv` 5 子例）；`with_sqlite_capacity` 无生产调用点（test-only，opt-in 纵深防御 NOT 已确认泄漏，honest-caveat）。

| task | 交付 |
|---|---|
| 33.1 (#218) | embedding L2 SQLite 缓存行数上限：`sqlite_put` 写入后 `DELETE ... WHERE provider=? AND dim=? AND rowid NOT IN (SELECT rowid ... ORDER BY rowid DESC LIMIT cap)` 隐式 rowid-FIFO 淘汰 + `DEFAULT_L2_EMBEDDING_CACHE_CAP=50_000` + add-only `with_sqlite_capacity` ctor；0 新 dep / 0 schema migration（复用隐式 rowid FIFO）；true-LRU honest-defer（`[SPEC-DEFER:phase-future.l2-cache-true-lru]`）；honest-caveat：`with_sqlite` 无生产调用点（test-only，opt-in 纵深防御非已确认 live leak，ADR-013 不夸大）；test_33_1_1（L2 cap evicts oldest FIFO）+ test_33_1_2（default cap 保留 modest workload，cache 7/7） |
| 33.2 (#219) | console-api memstore chunk/trace 缓存 FIFO → 访问序 LRU（读命中 `GetSourceChunk`/`GetSearchTrace` + 既有键覆写均经 `moveToMRU` move-to-front）+ memory hard-delete 无悬挂引用 INVARIANT 测试（schema introspection 断言 memory_id 仅在 memory_items + `get(id)=None` after hard_delete，cascade 为 non-issue honest-defer `[SPEC-DEFER:phase-future.memory-harddelete-cascade]`）；`handleMemoryPin` 依 ADR-022 D2 保持 lenient（诚实非改动，0 代码）；0 新 dep，fallback-mode only；TEST-33.2.1/.2（Go LRU + LRU_Trace）+ test_33_2_3（Rust hard_delete no dangling refs） |
| 33.3 (#220) | (A) indexing.* 事件持久化：add-only migration `0019_indexing_events` + `SqliteIndexingEventStore`（append/list，id ASC）+ `index_session_backend.rs` 4 emit 点（progress/error×2/cancelled）best-effort 持久化（旁挂既有 `eb.send` 广播不变）+ `events::indexing_rows_to_pb_events` 纯映射器（id ASC 重建 indexing.* PbEvent，真实 job_id/processed/total，deterministic `evt-idx-{id}`）；`server.rs` 生产接线。(B) TraceStore workspace 隔离：add-only `GetSearchTraceRequest.workspace_id=2` + `ListQueriesRequest.workspace_id=2`（buf generate；unrelated proto/grpc.pb.go EOL churn reverted 保持 surgical；Rust 经 tonic `include_proto!` 编译期生效）+ `search_persist` get/list/search_fts + in-mem TraceStore get/list + handlers WHERE workspace_id 过滤；**空 workspace_id = aggregate-all byte-equivalent**（ADR-004）。(C) drain-timeout VERIFY-ONLY（Phase 26 已交付 `drainTimeoutFromEnv`，0 代码改动，引 `TestDrainTimeoutFromEnv` 5 子例）；e2e honest-defer（`[SPEC-DEFER:phase-future.indexing-replay-e2e]` + `[SPEC-DEFER:phase-future.tracestore-multi-workspace-strict]`）；0 新 dep；test_33_3_1（mapper）/test_33_3_2（store round-trip + fixture emit persist）/test_33_3_3（persist ws filter）/test_33_3_4（handler ws passthrough） |
| 33.4 (this) | export `--timeout` add-only flag（`parseExportOpts` `fs.Duration("timeout", 60s)`，默认 60s **byte-equivalent** 于旧硬编码 `context.WithTimeout(60s)`，ADR-004，覆盖两次顺序 daemon spawn）+ TestParseExportOpts_Timeout；v0.26.0 release docs + smoke v23 step 42 + ADR-038 据 D1-D5 per-D ratify + ADR-031/027 add-only Amendment + roadmap §3.15/§4 + phase-33 §6 闭合；dropped-nits（诚实未实现）：`internal/cli/search.go:88` %v→%w 为 NON-BUG（terminal Fprintf 处 %w 是 vet-error，且 `err.Error()` 已携 grpc Status）/ tracestore-fts cross-version migration ALREADY-FIXED（`search_persist` open + `backfill_fts_if_empty`，TEST-26.1.4/.4b）/ datadir env→`daemon.Options.DataDir` REAL 但 🟡 `[SPEC-DEFER:phase-future.daemon-options-datadir]`；TestTask334 |

**ADR**：ADR-038 (governance-debt-cleanup-2) 据 D1-D5 真实非合成验证 `Proposed → Accepted`（D1 L2 cache rowid-FIFO 有界 + memstore 访问序 LRU + hard-delete 不变式 / D2 honest-caveat：with_sqlite test-only opt-in 纵深防御非已确认泄漏 / D3 indexing.* 持久化/重放 code-local 🟢 + indexing-replay-e2e/tracestore-isolation-e2e 🟡 honest-defer / D4 dropped-nits + datadir 🟡 honest-defer / D5 baseline 不变·proto·migration add-only，ADR-013 不伪造）；ADR-031 add-only Phase 33 Amendment（indexing.* 持久化 + 重放映射器 + drain-timeout verify-only，不溯改正文 ADR-014 D5）；ADR-027 add-only Phase 33 Amendment（L2 cache bound）；ADR-004 守线。**ADR-014 cross-validation gate — 第二十四次激活**。

**Upgrade / Rollback**：默认行为 / proto 既有字段 / 既有契约不变（既有 v0.6–v0.25 client + 索引/数据无需改动），0 新代码依赖、0 network 默认构建（ADR-004/008）；L2 cache rowid-FIFO 有界化（默认 cap 50_000）+ memstore 访问序 LRU（fallback-mode only）+ add-only migration `0019_indexing_events`（indexing.* 持久化旁挂既有广播）+ add-only proto `workspace_id`（空值 aggregate-all byte-equivalent）+ export `--timeout`（默认 60s byte-equivalent 旧硬编码）均 add-only / 不破协议。linux/amd64（承 v0.21–v0.25；arm64 仍 deferred）；cosign keyless sign + SBOM + provenance（承 task-28.2）。Rollback：`git tag -d v0.26.0` + 删 Release / ghcr tag；与 v0.25.0 行为兼容（add-only migration 0019 对 v0.25.0 代码无害）。tag SHA `6a98fd70233ffbaa161918e1b821173e60489e82`（annotated tag obj `360bc82b`）/ release run id `26888395968`（success）/ ghcr digest `sha256:8e924e25da66031e6c1b8d8e7ef7d6c8d4dae07aedc76d5cb25f718a6eb1caaf` / cosign Rekor tlog `1710104071`（sign）·`1710105166`（attest）（post-tag-push 回填，ADR-012）。

## v0.25.0 (2026-06-03) — vector-backend-config-plumbing-and-completeness (server.rs 热路径 env-config 注入 + sqlite-vec 工厂臂 + console vector_score 溯源 + retrieval-filter 契约诚实化 + ADR-037 ratified)

Phase 32 把 vector backend 打通到端到端：两条生产热路径（`server.rs` hybrid `:340` / semantic `:382`）经 env（`CONTEXTFORGE_VECTOR_BACKEND` + 可选 `CONTEXTFORGE_VECTOR_DIM`，仿 `resolve_data_dir`）选择 backend；`select_vector_backend` 新增 `"sqlite-vec"` 臂（feature `vector-sqlite` double-half cfg gating）；console search 携带 `vector_score` 溯源；`retriever/mod.rs:325` 误导 WARN 改为准确 no-op 契约。**默认行为 / proto 既有字段 / 既有契约不变**（ADR-004），三门 + lint 不退化。诚实口径：unset/空 backend → `BruteForce` byte-equivalent（默认不变）；unknown / feature-off → 工厂诚实 `Err`（不静默回退，ADR-013）；sqlite-vec in-process recall/latency matrix cell + 真实 chunk source_type/agent_scope 过滤 honest-defer，0 伪造数（ADR-013）。

| task | 交付 |
|---|---|
| 32.1 (#212) | `server.rs` hybrid (`:340`) + semantic (`:382`) 经 `resolve_vector_backend`/`parse_vector_backend` 选 backend（读 `CONTEXTFORGE_VECTOR_BACKEND` + 可选 `CONTEXTFORGE_VECTOR_DIM`，仿 `resolve_data_dir` env 模式，trim/blank→0）；unset/`""`→`BruteForce` byte-equivalent（默认不变）；unknown/feature-off→工厂诚实 `Err` surfaced as `Status::internal`（不静默回退，ADR-013）；0 新 dep；TEST-32.1.1（env name+dim parse + unknown 诚实 Err）+ TEST-32.1.2（unset → ("",0) → brute-force byte-equiv） |
| 32.2 (#213) | `select_vector_backend` 新增 `"sqlite-vec"` 臂（feature `vector-sqlite` double-half cfg gating，仿 qdrant/lancedb）：feat on → `SqliteVecBackend::new()`（name()="sqlite-vec"）/ feat off → 诚实 `Err` naming sqlite-vec + vector-sqlite；默认构建 TEST-32.2.1 feat-off honest-Err + TEST-32.2.2 selection-matrix wiring → factory 6/6；**真实 x86_64-pc-windows-msvc `cargo test --features vector-sqlite` 构建 PASSED**（臂 wiring 真实验证，非仅结构）；in-process recall/latency cell honest-defer（`[SPEC-DEFER:phase-future.sqlite-vec-inprocess-matrix]`）；0 新 dep（sqlite-vec 已 optional） |
| 32.3 (#214) | (A) `console_data_plane.proto` `SearchResultItem` add-only `vector_score=16`（parity v1 `RetrievalResult.vector_score=13`）→ buf generate proto 重生 Go binding（真实 rawDesc 0xdc→0xff，非 EOL churn；grpc/v1 reverted 保持 surgical）→ Rust producer 设值（vector hit cosine / BM25=0，仿 v1 `server.rs:407`）→ Go `grpcclient` 映射 `VectorScore` → add-only `contractv1.SearchResult.VectorScore`（ADR-015 add-only，仿 task-20.1 Semantic 先例；Console contractv1.go mirror 同字段为 cross-repo add-only signal，ADR-014 D4，向后兼容）。(B) `retriever/mod.rs:325` 误导 WARN → 准确 no-op 契约：FROZEN `chunks` 表无 `source_type`/`agent_scope` 列，真实 chunk filter 为 import-path feature → 新 backlog（`[SPEC-DEFER:phase-future.chunk-source-type-filter]` + `[SPEC-DEFER:phase-future.chunk-agent-scope-filter]`，ADR-013 不伪造）；TEST-32.3.1（grpcclient 携带 VectorScore：semantic 真值 / BM25=0）+ TEST-32.3.2（filter no-op：非空 source_type/agent_scope filter 结果 byte-for-byte 同 empty filter） |
| 32.4 (this) | v0.25.0 release docs + smoke v22 step 41 + ADR-037 据 D1-D5 per-D ratify（D2 sqlite-vec matrix cell honest-defer / D4 真实 chunk filter honest-defer 重申）+ ADR-034 add-only Amendment（sqlite-vec 工厂臂补全 backend coverage，不溯改 D-body·Phase 29 Ratification）；TEST-32.4 |

**ADR**：ADR-037 (vector-backend-config-plumbing-and-completeness) 据 D1-D5 真实非合成验证 `Proposed → Accepted`（D1 backend config plumbing TEST-32.1.*·默认 byte-equiv·0 新 dep / D2 sqlite-vec 工厂臂 + double-half gating + selection-matrix WIRING Accepted 含真实 MSVC feat-on 构建·matrix recall/latency CELL honest-defer / D3 console vector_score add-only 端到端 + filter WARN → 准确 no-op·真实 chunk filter honest-defer / D4 honest-defer 边界重申·受阻如实不伪造 ADR-013 / D5 default 0-vector-dep baseline·行为/proto/既有契约不变）；ADR-034 add-only Phase 32 Amendment（sqlite-vec 工厂臂补全 brute/qdrant/lancedb/sqlite-vec coverage，in-process matrix cell 仍 honest-defer，不溯改 D1-D5 body·Phase 29 Ratification，ADR-014 D5）；ADR-004 守线。**ADR-014 cross-validation gate — 第二十三次激活**。

**Upgrade / Rollback**：默认行为 / proto 既有字段 / 既有契约不变（既有 v0.6–v0.24 client + 索引/数据无需改动），0 新代码依赖、0 network（ADR-004/008）；vector backend env-config（unset/空→BruteForce byte-equiv）+ sqlite-vec 工厂臂（feature-gated）+ console `vector_score=16` add-only proto 字段 + retrieval-filter no-op 契约均 add-only / 不破协议。linux/amd64（承 v0.21–v0.24；arm64 仍 deferred）；cosign keyless sign + SBOM + provenance（承 task-28.2）。Rollback：`git tag -d v0.25.0` + 删 ghcr tag；与 v0.24.0 行为兼容。tag SHA `cb3bb08ec4f9eafc63805d89cd88c373a0cc2b26`（annotated tag obj `66299094cd31654ebed91e89ebfc2d72ec9014b8`）/ release run id `26874124924`（success）/ ghcr digest `sha256:d758f6760370553290362f8a2a36f325f993244512b93abd5bd74084973b97f4` / cosign Rekor tlog `1708460120`（sign）·`1708461746`（attest）（post-tag-push 回填，ADR-012）。

## v0.24.0 (2026-06-03) — governance-debt-cleanup (memstore-event parity + event-bus verify-only + cache LRU/cap + compose 硬化 + eval 子表 + exporter 全文 + 3 MCP nits + ADR-036 ratified)

Phase 31 清理跨 Phase 治理债：观测一致性（Go fallback memstore 发 `memory.*`）、缓存有界（embedding L1 LRU/cap + Go memstore cap 可配置）、部署硬化（compose 资源限 + 可选 TLS 反代）、eval 可查询（per-case 子表 + migration 0018）、exporter 全文保真（新 `ListAllChunks` RPC → 真实 content + ContentHash）、3 MCP nits。诚实校正：**event-bus partition/capacity 经核 Phase 26 已交付**（verify-only，不重复实现）。**默认行为 / proto 既有字段 / 既有契约不变**（ADR-004），三门 + lint 不退化。

| task | 交付 |
|---|---|
| 31.1 (#206) | `MemStore.EmitEvent` + `MemMemoryStore.SetEventSink`；Pin/Deprecate/SoftDelete/Unpin/HardDelete 发 `memory.*` 入 fallback ring（对齐 Rust `audit_op_to_event_type`，Pin/Unpin 共 memory.pin）+ console_api_serve 接线。event-bus partition/capacity verify-only（cargo events 6 passed，Phase 26 已交付）；TEST-31.1.* |
| 31.2 (#207) | `BoundedCache`（FIFO-on-insert，0 新 dep，cap 50_000）+ `with_capacity`；`resolveCacheCapacity`（env `CONTEXTFORGE_CONSOLEAPI_CACHE_CAP`）；compose mem_limit/cpus + `profiles:[tls]` caddy 反代 + Caddyfile（`docker compose config` + `--profile tls config` 实测 parse OK）；真实 cert honest-defer；TEST-31.2.* |
| 31.3 (#208) | migration 0018 `eval_case_results` 子表 + 双写 + query_case_results/case_pass_ratio SQL；`ListAllChunks` add-only RPC（proto+buf+Rust handler+daemon client+main.go+exporter ChunkLoader）→ exporter content 非空 + 真实 ContentHash；3 MCP nits（protocolVersion 日期解析 / audit err warn / allowlist mode warn）；C2/C3/C4 honest-defer 重申；TEST-31.3.* |
| 31.4 (this) | v0.24.0 release docs + smoke v21 step 40 + ADR-036 据 D1-D5 per-D ratify（D2 真实 cert / D4 native-runner·attestation honest-defer）+ ADR-021/027/029/033 add-only Amendment + roadmap §4 event-bus 更正 + phase-31 §6 闭合；TEST-31.4 |

**ADR**：ADR-036 (governance-debt-cleanup) 据 D1-D5 真实非合成验证 `Proposed → Accepted`（D1 memstore parity + event-bus verify-only / D2 cache·deploy code-local 达成·真实 cert honest-defer / D3 eval 子表·exporter 全文·MCP nits / D4 honest-defer 重申 / D5 baseline 不变，ADR-013 不伪造）；ADR-021/027/029/033 add-only Phase 31 Amendment（memstore parity·event-bus 更正 / cache LRU / case-results 子表 / native-runner·attestation defer 重申，不溯改正文 ADR-014 D5）；ADR-004 守线。**ADR-014 cross-validation gate — 第二十二次激活**。

**Upgrade / Rollback**：默认行为 / proto 既有字段 / 既有契约不变（既有 v0.6–v0.23 client + 索引/数据无需改动），0 新代码依赖；ListAllChunks add-only RPC + cache 有界化（默认 cap）+ memstore cap env-config + compose 限值/TLS profile + eval 子表 add-only migration + MCP nits 不破协议。Rollback：`git tag -d v0.24.0` + 删 Release / ghcr tag；与 v0.23.0 行为兼容。

## v0.23.0 (2026-06-03) — cjk-true-segmenter (jieba 真分词 analyzer feature-gated + 双站点注册 + reindex 迁移工具 + 真实 CJK recall delta + ADR-035 ratified)

Phase 30 把 Phase 24 的 0-dep 重叠 bigram CJK analyzer 升级为 **feature-gated 真分词器**（`cjk-segmenter`，jieba-rs，`配置加载`→`配置`/`加载`），bigram 保留作 0-dep fallback。真分词 analyzer 索引/查询**双站点对称注册**；新增 `IndexSession::reindex_with_tokenizer` 既有索引迁移工具。**默认构建 0 新 dep + 默认 tokenization 不变**（ADR-004 / ADR-035 D5），三门 + lint 不退化。诚实口径：**真分词相对 bigram file-level 召回 delta=+0.0000**（小语料持平，两者均完整召回 CJK case；真分词价值在 token 洁净非此规模召回，ADR-013 不伪造提升）。

| task | 交付 |
|---|---|
| 30.1 (#202) | `cjk-segmenter` feature（jieba-rs 0.7.4 optional，主 agent R7 chore + ADR-008 add-only，默认 off → 0 新 dep）+ `cjk_segmenter` analyzer（jieba.cut 真分词，`配置加载→配置/加载` 区别 bigram `配置/置加/加载`）+ index/query 双站点对称注册（漏注册→召回静默退化 task-24.1 R4）+ bigram `code_cjk` 0-dep fallback 保留；TEST-30.1.* |
| 30.2 (#203) | `IndexSession::reindex_with_tokenizer`（读 SQLite chunk + drop/重建 Tantivy 绑定 new analyzer + 重加，向后兼容）+ `RetrieverConfig.tokenizer` schema-driven 对称文档化（方案 B vestigial）+ phase24-harness 真分词 path + golden +5 CJK case（11→16，Go validator）；**实测 default 0.875 → bigram 1.0 → segmenter 1.0，delta(seg−bigram)=+0.0000 诚实零**；TEST-30.2.* |
| 30.3 (this) | v0.23.0 release docs + smoke v20 step 39 + ADR-035 据 D1-D5 per-D ratify（D3 default flip honest-defer，逐维如实）+ ADR-029 add-only Amendment + phase-30 §6 闭合；TEST-30.3 |

**ADR**：ADR-035 (cjk-true-segmenter-and-tokenizer-default) 据 D1-D5 真实非合成验证 `Proposed → Accepted`（D1 真分词 feature / D2 双站点对称 / D3 reindex 工具达成·default flip honest-defer / D4 真实 recall delta 含诚实零 / D5 baseline 不变，ADR-013 不伪造）；ADR-029 add-only Phase 30 Amendment（兑现 cjk-true-segmenter + tokenizer-default-on 部分，不溯改正文 ADR-014 D5）；ADR-008 jieba-rs add-only（optional，默认不编译）；ADR-004 守线。**ADR-014 cross-validation gate — 第二十一次激活**。

**Upgrade / Rollback**：默认运行时行为不变（默认 tokenization + 6-field schema 不变，既有 v0.6–v0.22 client + 既有默认索引无需改动），0 新代码依赖；新增 cjk-segmenter feature + jieba（optional）+ reindex 工具均 feature-gated/inward。Rollback：`git tag -d v0.23.0` + 删 Release / ghcr tag；与 v0.22.0 行为兼容。

## v0.22.0 (2026-06-03) — live-vector-recall (vector backend 工厂 + server.rs 热路径注入 + qdrant live KNN honest-defer + lancedb 真实 IVF_PQ/IVF_HNSW_SQ 索引 + compaction + 多 backend 选择矩阵 + ADR-034 ratified)

Phase 29 把 Phase 25 的 qdrant/lancedb 契约层 / 参数层兑现为**真实 live 向量召回**，并把真实 backend 工厂化注入生产热路径（`server.rs:302` hybrid / `:341` semantic 此前硬编码 `BruteForceVectorBackend`）。**默认构建 0-network / 0 新依赖 baseline 不变**（ADR-004 / ADR-023 D5），默认 semantic+hybrid 仍走 0-dep BruteForce，三门 + lint 不退化。诚实口径：**qdrant live KNN 无 server 时 honest-defer**（`health()==Unreachable` → exit 0，零伪造召回，ADR-013）；lancedb 真实 ANN 索引 feature-gated `--lib` scoped 实测。

| task | 交付 |
|---|---|
| 29.1 (#197) | `select_vector_backend(name, dim) -> Result<Arc<dyn VectorStore>, VectorError>` 工厂（仿 `select_provider`：`""`/`"brute"`→BruteForce 0-dep byte-equivalent、qdrant/lancedb feature-gated 否则诚实 Err）+ add-only 组合 trait `VectorStore: VectorIndexer + VectorSearcher`（三 base trait 签名不动）+ `server.rs:302/341` 经工厂注入；兑现 `[SPEC-DEFER:phase-future.vector-retrieval-integration]`；factory 4/4 + `cargo test --workspace` 191 lib + 全集成 0 failed；TEST-29.1.* |
| 29.2 (#198) | `core/examples/phase29_recall_via_qdrant.rs`（双 gate vector-qdrant+embedding-fastembed）`QdrantBackend::connect(from_env)` + `health()` 守门 → 无 server `Unreachable` 实测 eprintln + exit 0（零召回数、不伪造，ADR-013）；Ready 分支经 `Retriever::search_semantic` 量真实 recall；单节点部署基线文档化，集群/复制 `[SPEC-DEFER:phase-future.qdrant-deployment-topology]`；首次兑现 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 驱动维度；TEST-29.2.* |
| 29.3 (#199) | `LanceDbBackend::create_ann_index`（Lance `create_index` 真建 `Index::IvfPq`/`Index::IvfHnswSq`，兑现 `[SPEC-DEFER:phase-future.lancedb-index-tuning]`）+ `compact()`（`OptimizeAction::All`，兑现 `[SPEC-DEFER:phase-future.lancedb-schema-compaction]`，1536 行不丢）+ 真实多 backend 选择矩阵（n=1024 dim=384：IVF_HNSW_SQ recall@10≈0.90 / IVF_PQ≈0.44 / brute exact 最快）→ ADR-030/023 add-only Amendment；`--lib` scoped 4/4（规避 broad-test rustc 1.95.0 ICE）；TEST-29.3.* |
| 29.4 (this) | v0.22.0 release docs + smoke v19 step 38（live-vector-recall 状态）+ ADR-034 据 D1-D5 per-D ratify（D2 live-server honest-defer 部分 ratify，逐维如实）+ ADR-030/023 add-only Amendment + phase-29 §6 闭合；TEST-29.4 |

**ADR**：ADR-034 (production-vector-live-recall) 据 D1-D5 真实非合成验证 `Proposed → Accepted`（D1 工厂+热路径注入达成 / D2 qdrant live KNN wiring+honest-defer 部分·真实召回受阻如实 / D3 lancedb 真实索引+compaction 达成 / D4 选择矩阵真实测量 / D5 baseline 不变，ADR-013 不伪造）；ADR-030 + ADR-023 add-only Phase 29 Amendment（真实跨 backend 矩阵，不溯改 D 正文 ADR-014 D5）；ADR-004 守线（默认 0-dep + BruteForce 语义 baseline）。**ADR-014 cross-validation gate — 第二十次激活**。

**Upgrade / Rollback**：默认运行时行为不变（semantic+hybrid 默认仍走 0-dep BruteForce，server.rs 注入 byte-equivalent），无强制迁移、0 新代码依赖；新增工厂 + `VectorStore` add-only trait + qdrant/lancedb 真实索引均 feature-gated。Rollback：`git tag -d v0.22.0` + 删 Release / ghcr tag；与 v0.21.0 行为兼容。

## v0.21.0 (2026-06-02) — release-ci-hardening (anonymous-pull guard + cosign 签名/SBOM/provenance + CI strict-lint + multi-arch arm64 DEFERRED + ADR-033 ratified)

Phase 28 硬化发布 / CI 流水线。**全部改动为 `.github/workflows/*` + surgical clippy/gofmt 修复**；镜像运行时行为 + 默认构建 0-network / 0 新依赖 baseline **不变**（ADR-004），既有 cargo-test/go-test/spec-lint 三门不退化，0 新代码依赖。诚实口径：**arm64 multi-arch 延后**（QEMU emulation 实测不可行）、**真实 GHCR 签名于已授权 v0.21.0 release run 产生**（机制已 CI 端到端验证）。

| task | 交付 |
|---|---|
| 28.1 (#188) | `verify-image.yml` 未鉴权（`docker logout` 后）匿名 pull 守护（守 v0.10.0 GHCR-PRIVATE→403 回归，run 26788773926 ✅）；**multi-arch（arm64）实测不可行 DEFERRED**——run 26757640892 `platforms: amd64,arm64` `push:false` 构建 45min 超时（arm64 QEMU emulation 仍编译 Rust 依赖），`release.yml` 保持单架构 amd64 + arm64 `[SPEC-DEFER:phase-future.multi-arch-native-runner]`；TEST-28.1.* |
| 28.2 (#189) | `release.yml` cosign keyless sign（签 digest）+ `cosign attest` SPDX SBOM（syft）+ build-push `provenance:max` + `verify-image.yml` cosign verify + verify-attestation；GitHub 原生 attestation 私有仓库不可用（run 26789731232 failure）→ 改 cosign（ADR-033 §D2 原文，公共 Sigstore + GHCR OCI 工件与 repo 可见性无关）；全机制本地 registry run 26799480280 ✅ 端到端验证，真实 GHCR 签名于 v0.21.0 release run；TEST-28.2.* |
| 28.3 (#190) | `ci.yml` add-only `lint` job（clippy `-D warnings` + gofmt + go vet 三阻断，既有三门不退化）；先实测存量（CI/LF 权威：gofmt 15 真实 / go vet 0 / clippy ~33；本机 96 系 Windows CRLF 假阳性，初判误断 0 经 CI 纠正）→ 全修到全绿（gofmt -w+strip / clippy fix+手动+2 targeted allow），`cargo test` 187 passed；TEST-28.3.* |
| 28.4 (this) | v0.21.0 release docs + smoke v18 step 37（发布硬化状态）+ ADR-033 ratify（D1 arm64 DEFERRED / D2 cosign 机制验证·真签在 release / D3 lint 门绿，逐维如实）+ ADR-007 add-only Amendment + phase-28 §6 闭合；TEST-28.4 |

**ADR**：ADR-033 (release-ci-hardening) 据 D1-D4 真实 CI / release run 产物 `Proposed → Accepted`（D1 arm64 emulation 不可行 DEFERRED + anon-pull 达成、D2 cosign 机制验证·真实 GHCR 签名于 release run、D3 lint 门绿、D4 baseline 不变，逐维如实不伪造，ADR-013）；ADR-007 add-only Amendment（部署发布面扩展到 cosign 签名 OCI 镜像 + SBOM + provenance，arm64 延后，不溯改正文 D5）；ADR-004 守线（镜像运行时 + 默认 0-network/0-dep baseline 不变）。**ADR-014 cross-validation gate — 第十九次激活**。

**Upgrade / Rollback**：纯 CI/release 配置 + lint 修复，无运行时行为变更、无强制迁移。消费方可经 `cosign verify` + `cosign verify-attestation` 验 v0.21.0+ 镜像签名/SBOM（`--certificate-identity-regexp` 锚 release.yml + `--certificate-oidc-issuer` GitHub Actions）。Rollback：`git tag -d v0.21.0` + 删 Release / ghcr tag；与 v0.20.0 行为兼容（默认构建不变）。

## v0.20.0 (2026-06-01) — memory-ops-hardening (pin-actor + pinned-at-timestamp + Pin/Unpin split + hard-delete + is_pinned audit backfill + ADR-032 ratified)

Phase 27 硬化 Phase 13 / Phase 17 落地的 Memory pin / 生命周期语义，兑现 ADR-022 §Trade-offs 三条刻意缩范围延后的 marker。proto 全 add-only（既有 field 1-10 + 5 RPC + `Pin{bool pin}` 不动，proto-freeze guard 过）；默认构建恒 0 新依赖 / 0 network（ADR-004）；既有 5 Memory RPC + `confirmMiddleware` 不退化。

| task | 交付 |
|---|---|
| 27.1 (#181) | `MemoryItem` add-only proto `pinned_by`(field 11) + `pinned_at_unix`(field 12) + migration `0017`（`ensure_pin_actor_columns` 守护幂等 ALTER）+ `set_pinned_with_actor`（pin 写 actor+now / unpin 归 ''+0，`pinned_at` 独立于 `updated_at`，`set_pinned` 委托向后兼容）+ pin RPC 传 `"console-api"` + `memory_to_pb`/Go 投影；`console_message_fields` freeze guard；TEST-27.1.1-5（store 15/15 + data_plane 14/14 + proto_contract） |
| 27.2 (#183) | proto add-only `MemoryService.Unpin`/`HardDelete` RPC + 4 message + `store.hard_delete`（物理删除，0 行 NotFound）+ `AuditOperation::MemoryHardDelete`（event_type `memory.hard_delete`）+ console-api `POST /v1/memory/{id}/unpin`（204）+ `.../hard-delete`（confirmMiddleware gated 412→204→404）；TEST-27.2.1-5 |
| 27.3 (this) | is_pinned audit backfill `reconcile_is_pinned_from_audit`（last `memory_pin`/`memory_unpin` 事件胜，opt-in 一次性，仅修正 is_pinned 不臆造 actor/timestamp）+ smoke v17 step 36（REAL mode live round-trip）+ v0.20.0 release docs + ADR-032 ratify + ADR-022 add-only Amendment + phase-27 §6 闭合；TEST-27.3.1 |

**ADR**：ADR-032 (memory-ops-hardening) 据 D1-D4 真实非合成验证 `Proposed → Accepted`（actor 真实来源 + backfill 覆盖率据真实受限如实记录，不伪造，ADR-013）；ADR-022 add-only Amendment 兑现 §Trade-offs 三条 marker（`pin_actor`→`pinned_by` / `memory-pinned-at-timestamp`→`pinned_at_unix` / `is-pinned-backfill-from-audit`→`reconcile_is_pinned_from_audit`，不溯改正文 D1-D5）；ADR-017 X-Confirm 复用；ADR-015 全 add-only。

**Upgrade / Rollback**：默认行为不变，无强制迁移。`memory.db` boot 时经 `ensure_pin_actor_columns` 幂等 ALTER 加 `pinned_by`/`pinned_at_unix`（既有行缺省 backfill，add-only）。新 `Unpin`/`HardDelete` RPC + unpin/hard-delete 路由 add-only；is_pinned backfill 是 opt-in。Rollback：`git tag -d v0.20.0` + 删 Release / ghcr tag；与 v0.19.0 行为兼容。**hard-delete 物理删除不可恢复**（隐私基线设计意图，X-Confirm 兜底防误触）。

## v0.19.0 (2026-06-01) — observability-hardening (TraceStore FTS5 + VACUUM + events SSE/重放 + event-bus 配置 + ADR-031 ratified)

Phase 26 硬化 Phase 16 落地的两条可观测性信号路径（TraceStore 持久化 + events 实时面）。默认构建恒 0 新依赖 / 0 network（FTS5/VACUUM 复用 rusqlite bundled，SSE 用 Go stdlib `http.Flusher`，重放查既有 `audit_log`，event-bus 配置复用 `with_capacity` seam）；既有 long-poll endpoint + 22-endpoint 契约 + `put`/`get`/`list`/`load_warm` 签名不退化（add-only，ADR-015 D1）。

| task | 交付 |
|---|---|
| 26.1 (#178) | `SqliteTracePersist` FTS5 内容检索 `search_fts`（quoted-phrase MATCH，limit clamp 1..=100）+ 周期 `vacuum()` / `prune_older_than(cutoff)` + `open()` 旧 0015-only 库 boot 回填（`backfill_fts_if_empty`）+ `put()` FTS 同步；migration `0016_search_traces_fts.sql`（FTS5 影子虚表，`IF NOT EXISTS` 幂等）；既有签名语义不变；0 新依赖（rusqlite bundled）；TEST-26.1.1-5（10/10） |
| 26.2 (#179) | events SSE 实时推送 `GET /v1/observability/events/stream`（`text/event-stream` + `http.Flusher`，add-only 旁挂 long-poll）+ 从 audit log 重放漏失 memory state-op 事件（proto add-only `since_ts`/`last_event_id`，`replay_events_from_audit` id ASC + ADR-021 D3 映射，`evt-audit-{id}` 拼接边界去重）；SSE 帧契约 + 重放顺序 deterministic（不依赖墙钟）；真实 daemon SSE e2e 诚实延后 `[SPEC-DEFER:phase-future.sse-live-server-e2e]`；TEST-26.2.1-5（Rust 2/2 + Go SSE 4 契约） |
| 26.3 (this) | event-bus 容量/分区/drain 配置（`EventBus::from_config` + `CF_EVENT_BUS_CAPACITY`/`CF_EVENT_BUS_PARTITION` + `CONSOLE_EVENTS_DRAIN_TIMEOUT`，保守默认 1000/不分区/100ms 行为不变）+ smoke v16 step 35 + v0.19.0 release docs + ADR-031 ratify + ADR-021/015 add-only Amendment + phase-26 §6 闭合；TEST-26.3.1（events 6/6 + drain 5/5） |

**ADR**：ADR-031 (observability-hardening) 据 D1-D6 真实非合成验证 `Proposed → Accepted`（SSE live-server e2e 维度据 CI 无 running daemon **记录维持**，不强 ratify、不伪造，ADR-013）；ADR-021 add-only Amendment 兑现 events-replay-from-audit（`adr-021:115`）+ event-bus 容量/分区（Rollback path `adr-021:153`）；ADR-015 SSE endpoint add-only。

**Upgrade / Rollback**：默认行为不变，无强制迁移。旧 `search_traces.db`（0015-only）boot 时幂等创建 0016 FTS 表 + 回填（add-only，既有数据不损）。SSE endpoint + `?since_ts=` 重放 + event-bus 配置均 opt-in。Rollback：`git tag -d v0.19.0` + 删 Release / ghcr tag；与 v0.18.0 行为兼容（add-only / opt-in），旧库 0016 FTS 表对 v0.18.0 代码无害。

## v0.18.0 (2026-06-01) — production-vector-backend (qdrant 生命周期层 + lancedb 可构建性 🟢 + 生产 backend 选择矩阵 + ADR-030 ratified)

### 摘要

v0.18.0 minor release (Phase 25): pushes the two production-scale ANN backends that ADR-023 tiered — **qdrant** (hosted/scale-out) and **lancedb** (embedded-columnar) — from the Phase-18 spike state toward production. **qdrant** gains a server lifecycle layer (`QdrantConnConfig` validate + `health()` probe + `decide_ensure` collection ensure-create) that is contract-testable **without a live server**, replacing the spike's blind drop+create `open()` (task-25.1). **lancedb** gets a real dev-box buildability investigation — 🟢 `cargo build --features vector-lancedb` passes on `x86_64-pc-windows-msvc` (protoc supplied via the in-repo vendored binary, 0 new dependency) — plus an index-tuning parameter-validation layer (`LanceIndexTuning::validate`, IVF_PQ/HNSW + compaction threshold) (task-25.2). A **production backend selection matrix** (corpus-size × deployment-shape → hnsw / sqlite-vec / lancedb / qdrant + per-tier caveat) ships in the evidence doc (task-25.3).

**Honest scope (read this)**: the **default build is unchanged** — 0 vector dependency, BM25-only baseline (qdrant/lancedb are feature-gated, default unchanged). This is a **backend lifecycle / buildability-layer release with NO recall numbers**. qdrant's lifecycle layer is verified at the **contract layer without a live server** (config validation + health-probe unreachable shape + ensure-create decision); **real KNN over a live qdrant server is honestly deferred** (`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`, CI has no qdrant server). lancedb 🟢 builds on the Windows MSVC dev box, but the credential is **single-box** (protoc is still a hard prerequisite that must be explicitly provided; CI does not build the feature by default), and a broad `cargo test --features vector-lancedb` (compiling all integration-test targets) hits a rustc 1.95.0 ICE in vector-unrelated test targets — a toolchain limitation, not a regression (the `cargo build` + `--lib` tests pass). **lancedb real ANN index perf is deferred** (`[SPEC-DEFER:phase-future.lancedb-index-tuning]`). All recorded per ADR-013 (no faked live-server / cross-platform credentials).

### What shipped (Phase 25, tasks 25.1–25.3)

| task | delivery | PR |
|---|---|---|
| 25.1 | `core/src/retriever/vector/qdrant.rs` qdrant server lifecycle layer: `QdrantConnConfig` (url/timeout/api_key/tls + `from_env` + `validate`) + `QdrantHealth` + `CollectionDesc` + `EnsureAction` + `decide_ensure` pure fn + `QdrantBackend::connect`/`health()`; `open()` rewritten to ensure-create (reuse-if-matching, no silent drop) — TEST-25.1.1-4 contract-tested without a live server, 0 new dep | (merged) |
| 25.2 | `core/src/retriever/vector/lance_db.rs` `LanceAnnIndex` (IvfPq/Hnsw) + `LanceIndexTuning::validate(dim)` index-tuning param contract + `docs/spikes/phase-25-lancedb-buildability.md` real dev-box buildability (🟢 `cargo build --features vector-lancedb` exit 0 on x86_64-pc-windows-msvc, 1097 rlib hard evidence, 0 new dep) — TEST-25.2.1-5 | (merged) |
| 25.3 | Phase 25 closeout: production backend selection matrix (corpus-size × deployment-shape → hnsw/sqlite-vec/lancedb/qdrant + caveat) + smoke v15 step 34 + v0.18.0 release docs + ADR-030 ratify + ADR-023 D3/D4 add-only Amendment | this PR |

### ADR-030 ratified (Proposed → Accepted) + ADR-023 Amendment

**ADR-030 production-vector-backend** ratified on the **real non-synthetic** verification of D1–D4: D1 qdrant lifecycle contract layer (TEST-25.1.1-4, no live server), D2 lancedb 🟢 real dev-box build (`cargo build --features vector-lancedb` exit 0 + index-tuning param validation TEST-25.2.3-4), D3 production backend selection matrix, D4 default 0-vector-dep unchanged. **qdrant live-server KNN** (`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`) and **lancedb real ANN index perf** (`[SPEC-DEFER:phase-future.lancedb-index-tuning]`) are honestly deferred — not faked. **ADR-023** gets an add-only Amendment advancing the D3 (qdrant) + D4 (lancedb) tiers without rewriting D1-D6 (ADR-014 D5). **ADR-008** needs **no amendment** (qdrant-client/lancedb/arrow-array/futures are all pre-existing optional deps; 0 new direct dep). **ADR-014 — 16th activation.**

### Upgrade path

- Drop-in: default build behavior unchanged (0 vector dependency + BM25-only baseline). No forced migration. The production-scale backends are feature-gated: `--features vector-qdrant` (needs a live qdrant server) / `--features vector-lancedb` (needs a `protoc` prerequisite for the lance `build.rs`).
- qdrant `open()` upgraded from spike drop+create to **ensure-create** (reuse if the collection exists with matching dim/metric; error on mismatch instead of silently dropping data) — a safer semantic for existing collections under `--features vector-qdrant`.

### Rollback path

`git tag -d v0.18.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.17.0 (0 vector dependency + BM25 baseline), so a rollback is non-breaking (the qdrant/lancedb lifecycle/buildability work is all feature-gated and not enabled by default).

## v0.17.0 (2026-05-31) — code-and-cjk-tokenizer-and-eval-hardening (opt-in tokenizer + eval 校验器 + ADR-029 ratified)

### 摘要

v0.17.0 minor release (Phase 24): adds an **opt-in code/CJK tokenizer** and a **hardened eval ruler**. The Tantivy `content` field can now split code symbols (`camelCase`→`camel`+`case` keeping the original token, `snake_case`/`dotted.path`/`kebab-case`) and tokenize CJK text into bigrams (`配置加载`→`配置`/`置加`/`加载`) — opt-in via `RetrieverConfig.tokenizer="code_cjk"`. The eval golden dataset gains an independent validator (`ValidateGoldenSemantic`: schema well-formedness + duplicate detection + answer coverage) and a code/CJK-annotated `golden-semantic.jsonl`. A **real before/after recall delta** (default **0.9091 → code/CJK 1.0000**, +0.0909) is measured through the production `Retriever` BM25 path.

**Honest scope (read this)**: the **default build is unchanged** — 0 new dependency (pure std tokenizer), default tokenization unchanged (existing collections are not silently invalidated), eval gate thresholds unchanged. The tokenizer is **opt-in via config (not a feature flag)**; **adopting it requires a re-index** (it changes the inverted terms). The +0.0909 delta is on a **small dataset (11 queries / 12 files)**, driven by one real CJK-bigram case (`语义检索`); the other 10 queries are parity (full-symbol/full-phrase queries match in both analyzers). The tokenizer's sub-token discrimination is proven deterministically by the task-24.1 unit tests (not extrapolated). The **rust-native-eval-runner is honestly deferred** after a real evaluation (the Go harness stays the single source of truth). All recorded per ADR-013 (no faked numbers).

### What shipped (Phase 24, tasks 24.1–24.3)

| task | delivery | PR |
|---|---|---|
| 24.1 | `core/src/indexer/mod.rs` opt-in code/CJK `TextAnalyzer` (`CodeCjkTokenizer`: camelCase/snake_case/dotted.path/kebab-case split + 保留原 token + CJK bigram, pure std) + `build_tantivy_schema(tokenizer)` opt-in branch + `open_with_tokenizer` + retriever symmetric registration — 0 new dep, default tokenization unchanged | #173 |
| 24.2 | `internal/eval/eval.go` `ValidateGoldenSemantic` (add-only: schema + duplicate + coverage; `knownCategories` += code-symbol/cjk) + `test/fixtures/eval/golden-semantic.jsonl` (11 questions, code-symbol + CJK → real files) — zero Rust delta, gate thresholds unchanged | #174 |
| 24.3 | Phase 24 closeout: real before/after recall delta (`phase24_tokenizer_recall` example, +0.0909) + rust-native-eval-runner eval (honestly deferred) + smoke v14 step 33 + v0.17.0 release docs + ADR-029 ratify | this PR |

### ADR-029 ratified (Proposed → Accepted)

**ADR-029 code-and-cjk-tokenizer-and-eval-hardening** ratified on the **real non-synthetic** verification of D1–D3/D5: D1 code/CJK tokenizer (TEST-24.1.1-4 + real recall delta +0.0909), D2 eval validator (TEST-24.2.1-2), D3 code/CJK golden 扩充 (TEST-24.2.3), D5 default unchanged (0 new dep + default tokenization + gate thresholds). **D4 rust-native-eval-runner honestly deferred** (`[SPEC-DEFER:phase-future.rust-native-eval-runner]`) — not faked as implemented. **ADR-006** (recall gate thresholds) and **ADR-008** (library selection) need **no amendment** (gate unchanged; tokenizer is std-only, 0 new dep). **ADR-014 — 15th activation.**

### Upgrade path

- Drop-in: default build behavior unchanged (default tokenization + BM25 baseline + eval gate thresholds). No forced migration. The eval `ValidateGoldenSemantic` is add-only (existing `ValidateDataset` callers unaffected).
- The code/CJK tokenizer is opt-in via `RetrieverConfig.tokenizer="code_cjk"`. **Adopting opt-in requires a re-index of existing collections** (it changes the `content` inverted terms; the old index still works with the default analyzer but does not get code/CJK sub-token hits).

### Rollback path

`git tag -d v0.17.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.16.0 (default tokenization + BM25 + 0-dep), so a rollback is non-breaking (the opt-in tokenizer is not enabled by default).

## v0.16.0 (2026-05-31) — vector-persistence-and-cross-platform (hnsw 持久化 + sqlite-vec MSVC + ADR-028 ratified)

### 摘要

v0.16.0 minor release (Phase 23): makes the feature-gated vector backends **persistent + cross-platform** — **hnsw graph persistence** (`HnswBackend::save`/`load` to `VectorIndexConfig.persistence_path` + rebuild-on-load fallback, `vector-hnsw`), a **sqlite-vec Windows MSVC** investigation that **resolved the Phase 18 MSVC-build-blocked stop-condition** (real `cargo build` + run on `x86_64-pc-windows-msvc`), and a **vector incremental-index evaluation** (brute-force / sqlite-vec support row-level append; hnsw full-rebuild deferred).

**Honest scope (read this)**: the **default build stays local, 0-vector-dependency, and BM25-baseline-unchanged** — all persistence/cross-platform capability is behind the `vector-hnsw` / `vector-sqlite` features (ADR-023 D5). **This is a backend-layer release with no recall numbers.** The persisted graph is **not** wired into the `server.rs` semantic hot path yet (still rebuilds on demand — a future release). sqlite-vec MSVC evidence is from a single dev box (rustc 1.95.0); CI does not build the feature by default — honestly recorded (ADR-013), not faked.

### What shipped (Phase 23, tasks 23.1–23.3)

| task | delivery | PR |
|---|---|---|
| 23.1 | `core/src/retriever/vector/hnsw.rs` `HnswBackend::save`/`load` (path B: serialize `(normalized embedding, chunk_id)` inputs + load-rebuild; absent/corrupt/version-mismatch → rebuild-on-load) + `open` wires `persistence_path` — 0 new dep | #168 |
| 23.2 | sqlite-vec Windows MSVC investigation → 🟢 path (a) bundled amalgamation builds + runs on `x86_64-pc-windows-msvc` (resolves Phase 18 MSVC-blocked) + contract tests + `docs/spikes/phase-23-sqlite-vec-cross-platform.md` — 0 source/Cargo.toml change | #169 |
| 23.3 | Phase 23 closeout: incremental-index eval (brute-force/sqlite-vec row-level append; hnsw deferred) + smoke v13 step 32 + v0.16.0 release docs + ADR-028 ratify + ADR-023 add-only Amendment | this PR |

### ADR-028 ratified (Proposed → Accepted) + ADR-023 Amendment

**ADR-028 vector-persistence-strategy** ratified on the **real non-synthetic** verification of D1–D4: D1 hnsw persistence (path B roundtrip 3/3 PASS), D2 sqlite-vec MSVC (real build + run, resolves Phase 18 stop-condition), D3 incremental index (brute-force/sqlite-vec append; hnsw deferred), D4 default 0-vector-dep unchanged. **ADR-023** gets an add-only Amendment: its "hnsw rebuild-on-restart" disqualifier is resolved (task-23.1) and "sqlite-vec MSVC-blocked / dev-prod parity imperfect" is narrowed (task-23.2) — D1–D6 正文 not retro-edited (ADR-014 D5).

### Upgrade path

- Drop-in: default build behavior unchanged (BM25 + 0-vector-dep). No migration. `VectorIndexConfig.persistence_path` (existing field) is first consumed by `HnswBackend::open` (`None` → in-memory, byte-equivalent).
- Vector persistence (hnsw `save`/`load`) + sqlite-vec cross-platform are feature-gated (`vector-hnsw` / `vector-sqlite`) + explicit opt-in (not in the default image).

### Rollback path

`git tag -d v0.16.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.15.0 (BM25 + 0-dep deterministic semantic/hybrid path), so a rollback is non-breaking.

## v0.15.0 (2026-05-31) — embedding-provider-completion (provider 配置选择 + 缓存 + 远程骨架 + ADR-027 ratified)

### 摘要

v0.15.0 minor release (Phase 22): grows the Phase 19 embedding layer ("hardcoded `DeterministicEmbeddingProvider` default + a single feature-gated `FastEmbedProvider`") into a **configurable provider layer** — a runtime `select_provider` factory (deterministic / fastembed / remote) with **dim negotiation** (`DimMismatch`, no silent truncate/pad), a **content-hash embedding cache** (`CachingEmbeddingProvider`, memory L1 + optional SQLite L2), a **feature-gated remote provider skeleton** (`RemoteEmbeddingProvider`, OpenAI/Cohere HTTP via ureq rustls), and an **opt-in remote-reachability health probe**.

**Honest scope (read this)**: the **default build stays local, model-free, and 0-network-dep** — the deterministic identity provider is the default; fastembed (`embedding-fastembed`) and remote (`embedding-remote`) are **feature-gated + explicit opt-in** (ADR-004 local-first, the non-negotiable red line). The embedding cache + remote skeleton are verified at the **unit / contract layer** (no network in tests). **This is a provider-layer release with no recall numbers** — real remote-network 联调 / API keys / recall quality + the real remote health probe are honestly **deferred** (ADR-013 — CI has no credentials/network; not faked). `[embedding]` config is add-only — **no breaking contract bump**.

### What shipped (Phase 22, tasks 22.1–22.4)

| task | delivery | PR |
|---|---|---|
| 22.1 | `internal/config` add-only `[embedding]`(provider/dim) codec + `core/src/embedding/factory.rs` `select_provider` + `negotiate_dim`→`DimMismatch` + `server.rs` semantic path via factory (byte-equivalent default) | #163 |
| 22.2 | `core/src/embedding/cache.rs` `CachingEmbeddingProvider` (Sha256(text)→embedding; memory L1 + optional SQLite L2, ADR-002; f32 LE BLOB round-trip) — 0 new dep | #164 |
| 22.3 | `core/src/embedding/remote_provider.rs` `RemoteEmbeddingProvider` (`embedding-remote` feature, ureq rustls) + pure `build_request_body`/`parse_response` + contract tests (fixtures, no network); `ureq 2.12.1` R7 chore | #165 |
| 22.4 | Phase 22 closeout: `health.rs probe_embed` feature-gated opt-in remote probe (config-only default unchanged) + smoke v12 step 31 (`init` emits `[embedding]`) + v0.15.0 release docs + ADR-027 ratify | this PR |

### ADR-027 ratified (Proposed → Accepted)

**ADR-027 embedding-provider-abstraction** is ratified on the **real non-synthetic** verification of D1–D5: D1 config+factory (Go round-trip + Rust factory tests), D2 dim negotiation (`negotiate_dim`→`DimMismatch` + feature fastembed 384-mismatch, network-free), D3 cache (`CachingEmbeddingProvider` 4/4 deterministic), D4 remote skeleton (contract 4/4 fixtures, no network), D5 local-first (default build 0 network dep — `cargo tree | grep ureq` empty). The ratify scope is the provider **abstraction layer**; remote real-network integration quality is honestly deferred (ADR-013).

### Upgrade path

- Drop-in: default build behavior unchanged (deterministic default + 0 model/0 network dep). No migration. `[embedding]` is add-only (absent / `Provider=""` → deterministic; existing `[remote]`/`[[collections]]` unaffected).
- Embedding cache: a library decorator (`CachingEmbeddingProvider`) wrapping any provider. Remote provider: needs `--features embedding-remote` + explicit opt-in config + env API key (not in the default image; key never logged).

### Rollback path

`git tag -d v0.15.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.14.0 (BM25 + 0-dep deterministic semantic/hybrid path + config-only health probe), so a rollback is non-breaking.

## v0.14.0 (2026-05-31) — retrieval-quality (hybrid scoring + reranker + ADR-025/026 ratified)

### 摘要

v0.14.0 minor release (Phase 21): on top of the Phase 19/20 BM25 + semantic dual paths, it adds two **opt-in** ranking-quality enhancements — **hybrid scoring** (RRF fusion of the BM25 word-level + vector semantic scores, `retrieval_method = "hybrid"` + add-only `hybrid_score`) and a **reranker pipeline** (`Reranker` trait + deterministic `IdentityReranker` default + feature-gated real cross-encoder `CrossEncoderReranker`, wired via `Retriever::with_reranker`).

**Honest scope (read this)**: hybrid + rerank are both **opt-in** — default retrieval stays BM25. The **default build is unchanged and dependency-free** (0 new crate): its hybrid path uses the 0-dep `DeterministicEmbeddingProvider` + `BruteForceVectorBackend`, and the default reranker is the deterministic model-free `IdentityReranker`. The **real** embedding model (`FastEmbedProvider`, `all-MiniLM-L6-v2`) and **real** cross-encoder (`BGE-reranker-base`) are behind the `embedding-fastembed` / `reranker-fastembed` features; the real recall numbers below were measured with them. `SearchRequest.hybrid` is add-only — **no breaking contract bump**.

### What shipped (Phase 21, tasks 21.1–21.3)

| task | delivery | PR |
|---|---|---|
| 21.1 | `core/src/retriever/fusion.rs` RRF fusion (k=60) + `Retriever::search_hybrid` + proto `SearchRequest.hybrid=8` / `RetrievalResult.hybrid_score=15` (add-only, buf regen) + `server.rs` `req.hybrid` dispatch + `test_21_1×4` | #159 |
| 21.2 | `core/src/rerank/{mod,traits,identity,cross_encoder}.rs` `Reranker` trait + deterministic `IdentityReranker` (0 model dep) + feature-gated `CrossEncoderReranker` + `Retriever::with_reranker` seam + `docs/spikes/phase-21-reranker.md` | #160 |
| 21.3 | Phase 21 closeout: `internal/eval` Report hybrid/reranked columns + `internal/cli/eval.go --hybrid/--rerank` + smoke v11 step 30 multi-path assertion + real dogfood eval + v0.14.0 release docs + ADR-025/026 ratify | this PR |

### Real hybrid / reranked recall vs the BM25 baseline (dogfood, ADR-013 real run)

Real `FastEmbedProvider` (`all-MiniLM-L6-v2`, dim 384) + real `CrossEncoderReranker` (`BGE-reranker-base`), through the production `Retriever` over real ContextForge text (180 production chunks / 30 golden queries; `docs/spikes/phase-21-hybrid-recall.md`):

| method | recall@5 | recall@10 | top-1 | MRR | ADR-006 gate (≥0.70) |
|---|---|---|---|---|---|
| baseline BM25 | 0.9000 | 0.9667 | 0.0333 | 0.4095 | PASS |
| **hybrid RRF** | 0.9333 | 0.9667 | **0.6667** | **0.7881** | PASS |
| reranked cross-encoder | **0.9667** | 0.9667 | 0.3333 | 0.6306 | PASS |

- **Hybrid RRF is the decisive win**: top-1 +0.6334 / MRR +0.3786 over BM25 (BM25 finds the right file in top-10 but rarely ranks it first; fusing the vector signal fixes that) → ratifies **ADR-025 Accepted**.
- **Real cross-encoder ran** (ADR-026 D5 stop-condition not triggered): beats BM25 baseline (top-1 +0.30, MRR +0.22) + best recall@5. **Honest caveat (ADR-013)**: on this small code+ADR corpus, reranking the already-strong hybrid top-k does **not** beat hybrid on top-1/MRR (general-text BGE is weaker on code chunks; rerank only re-orders an already-good fusion) → rerank is recommended as a **domain-fit opt-in**, never a default. → ratifies **ADR-026 Accepted (with caveat)**.

### Upgrade path

- Drop-in: default build behavior unchanged (BM25-only, 0 new dependency). No migration.
- Hybrid: send `SearchRequest.hybrid=true` (or `contextforge eval run --hybrid`); unset → BM25. Reranking: wire `Retriever::with_reranker` (default `None` → no rerank). Real-model hybrid vector component needs `--features embedding-fastembed`; real cross-encoder rerank needs `--features reranker-fastembed` (not in the default image).
- Proto: `SearchRequest.hybrid` (8) + `RetrievalResult.hybrid_score` (15) are **add-only** — existing clients unaffected (Console Contract v1 shape unchanged, 22-endpoint conformance + proto-freeze guard PASS, unset → BM25). **No breaking contract bump.**

### Rollback path

`git tag -d v0.14.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.13.0 (BM25-only + 0-dep deterministic semantic/hybrid path + identity reranker seam), so a rollback is non-breaking.

### Contract

Console Contract v1 unchanged in shape; the new `SearchRequest.hybrid` / `RetrievalResult.hybrid_score` fields are add-only and default to BM25 behavior when unset. The reranker adds no proto field (builder seam).

### ADR

**ADR-025 hybrid-scoring-fusion → Accepted** + **ADR-026 reranker-provider → Accepted** — both ratified on real dogfood eval data (ADR-013: real `FastEmbedProvider` + `CrossEncoderReranker` run, no synthetic figures). ADR-025 confirms the default RRF strategy (decisive top-1/MRR uplift, resolving the "indistinguishable on synthetic data" open point). ADR-026 ratifies the reranker architecture (real model ran; uplift over baseline + best recall@5) with the honest caveat that it underperforms hybrid on this corpus → opt-in domain-fit enhancement. **ADR-014 cross-validation gate — 12th activation**.

详 [Phase 21 spec](docs/specs/phases/phase-21-retrieval-quality.md) + [ADR-025](docs/decisions/adr-025-hybrid-scoring-fusion.md) + [ADR-026](docs/decisions/adr-026-reranker-provider.md) + [v0.14.0 evidence](docs/releases/v0.14.0-evidence.md) + [v0.14.0 artifacts](docs/releases/v0.14.0-artifacts.md) + [hybrid/reranked recall evidence](docs/spikes/phase-21-hybrid-recall.md)。

## v0.13.0 (2026-05-31) — semantic-retrieval-throughline (语义检索贯通 console-api + 经 Retriever 真实召回 + ADR-024 ratified)

### 摘要

v0.13.0 minor release: carries the Phase 19 (v0.12.0) semantic path the last mile (Phase 20). It now engages **end-to-end through console-api** — `POST /v1/search?semantic=true` (or body `semantic`) routes through Go `handleSearch` → `grpcclient` → `console_data_plane` gRPC `SearchService.Query` semantic dispatch (Rust `SearchServer::query`, mirroring the core `CoreService` `server.rs`) → ranked vector hits — and real recall is now measured **through the production `Retriever::search_semantic` hot path** instead of the v0.12.0 standalone example. This closes the two caveats v0.12.0 honestly recorded (`docs/releases/v0.12.0-evidence.md` §3b / task-19.4 §10).

**Honest scope (read this)**: semantic retrieval is still **opt-in** — default retrieval stays BM25. The **default build is unchanged and dependency-free**: its semantic path (incl. via console-api) uses the **0-dependency `DeterministicEmbeddingProvider` + `BruteForceVectorBackend`** (proves wiring, not model quality). The **real** embedding provider (`FastEmbedProvider`, `all-MiniLM-L6-v2`) is behind the `embedding-fastembed` feature; the real recall numbers below were measured with it. The new `console_data_plane SearchRequest.semantic` field is **add-only** — no breaking contract bump.

### What shipped (Phase 20, tasks 20.1–20.3)

| task | delivery | PR |
|---|---|---|
| 20.1 | `console_data_plane.proto` `SearchRequest` add-only `bool semantic = 7` (buf regen) + Rust `SearchServer::query` semantic dispatch (mirrors core `CoreService` `server.rs`, `DeterministicEmbeddingProvider` + 0-dep `BruteForceVectorBackend`) + Go `contractv1.SearchRequest.Semantic` + `handleSearch` `?semantic=true` OR-merge + `grpcclient` passthrough | #155 |
| 20.2 | real recall through the production `Retriever::search_semantic` hot path (real fastembed) + deterministic `test_20_2` hot-path wiring + `docs/spikes/phase-20-recall-via-retriever.md` | #156 |
| 20.3 | Phase 20 closeout + smoke v10 step 29 console-api real semantic assertion + v0.13.0 release docs + ADR-024 ratify | this PR |

### Real-embedding recall, through the production Retriever (resolves the v0.12.0 example caveat)

Real `FastEmbedProvider` (`all-MiniLM-L6-v2`, dim 384) over real ContextForge text, exact cosine, routed through the production `Retriever::search_semantic` hot path (real scanner + chunker → **175 production chunks**; `docs/spikes/phase-20-recall-via-retriever.md`):

| metric | task-20.2 (production Retriever, 175 chunks) | task-19.5 (standalone, 40 capped chunks) baseline |
|---|---|---|
| SemanticRecall@5 | **0.9667** (29/30) | 0.8333 |
| SemanticRecall@10 | **1.0000** (30/30) | 0.9333 |
| top-1 accuracy | **0.7333** | 0.60 |
| MRR | **0.8367** | 0.70 |
| ADR-006 A1 gate (≥ 0.70) | **PASS** | PASS |

**Honest inflation caveat (ADR-013)**: @10 = 1.0 is **partly file-level chunk-count inflation** — the uncapped production chunker yields many chunks per file (175 across 11 files), making "any chunk from the expected file in top-K" mechanically easier; this is the same artifact task-19.5 deliberately suppressed with `MAX_CHUNKS_PER_FILE`. But the discriminating metrics rule out pure inflation: top-1 (0.7333) and MRR (0.8367) are not chunk-count-inflated and are **higher** than task-19.5's 0.60 / 0.70 — the production path genuinely ranks the right file first more often. The two numbers are **not directly comparable** (different corpora + chunking); both clear the gate. task-20.2 is the **representative** measurement (production pipeline), task-19.5 the **controlled** discrimination floor. Deterministic embeddings prove plumbing, not recall — every real number here is a real fastembed run, no synthetic/fabricated figures.

### Upgrade path

- Drop-in: default build behavior is unchanged (BM25-only retrieval, 0 new dependency). No migration.
- To use the semantic path via console-api: send `?semantic=true` on `POST /v1/search` (now forwarded end-to-end), or run `contextforge eval run --semantic`. The default-build semantic path uses the deterministic provider; for real-model semantic search build/deploy with `--features embedding-fastembed`.
- Proto: `console_data_plane SearchRequest.semantic` (7) is **add-only** — existing clients are unaffected (Console Contract v1 shape unchanged, 22-endpoint conformance + proto-freeze guard PASS, unset → BM25). **No breaking contract bump.**

### Rollback path

`git tag -d v0.13.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.12.0 (BM25-only + 0-dep deterministic semantic path), so a rollback is non-breaking.

### Contract

Console Contract v1 unchanged in shape; the new `console_data_plane SearchRequest.semantic` field is add-only and defaults to BM25 behavior when unset.

### ADR

**ADR-024 console-api-semantic-forward → Accepted** — ratified on task-20.1's real landing (Go contractv1/handleSearch/grpcclient tests + Rust `SearchServer::query` dispatch test `test_20_1` prove it; not synthetic, ADR-013). It records the `console_data_plane` add-only `semantic` field + the console-api↔daemon semantic-alignment口径 (mirroring the ADR-015/022 add-only pattern). **ADR-014 cross-validation gate — 11th activation**.

详 [Phase 20 spec](docs/specs/phases/phase-20-semantic-retrieval-throughline.md) + [ADR-024](docs/decisions/adr-024-console-api-semantic-forward.md) + [v0.13.0 evidence](docs/releases/v0.13.0-evidence.md) + [v0.13.0 artifacts](docs/releases/v0.13.0-artifacts.md) + [real recall evidence](docs/spikes/phase-20-recall-via-retriever.md)。

## v0.12.0 (2026-05-30) — vector-retrieval-integration (end-to-end semantic search + ADR-023 ratified)

### 摘要

v0.12.0 minor release: turns the Phase 18 vector-backend **infrastructure** into a **live, end-to-end semantic retrieval path** (Phase 19) and **ratifies ADR-023** on **real** embedding recall. A request can take the vector path through the whole stack (`POST /v1/search?semantic=true` → Go → Rust gRPC → `EmbeddingProvider` → vector backend → ranked hits), the eval CLI gains `contextforge eval run --semantic`, and the Phase 18 "synthetic recall is non-discriminating" caveat is resolved with measured real-embedding recall.

**Honest scope (read this)**: semantic retrieval is **opt-in** — default retrieval stays BM25. The **default build is unchanged and dependency-free**: its semantic path uses the **0-dependency `DeterministicEmbeddingProvider` + `BruteForceVectorBackend`** (proves wiring correctness, not model quality). The **real** embedding provider (`FastEmbedProvider`, `all-MiniLM-L6-v2`) is behind the `embedding-fastembed` feature; the real recall numbers below were measured with it. No model or vector dependency is compiled by default (ADR-023 D5).

### What shipped (Phase 19, tasks 19.1–19.7)

| task | delivery | PR |
|---|---|---|
| 19.1 | `EmbeddingProvider` trait + `DeterministicEmbeddingProvider` (0-dep default) + `FastEmbedProvider` (real, feature-gated) + spike evidence | #142 |
| 19.2 | default backend wired into `Retriever` (`with_embedder` + `with_vector_searcher` + `search_semantic`) | #143 |
| 19.3 | `/v1/search?semantic=true` Go→Rust gRPC semantic path + proto add-only (`semantic`, `vector_score`, `embedding_provider`) + 0-dep `BruteForceVectorBackend` | #144 |
| 19.4 | smoke v9 30-step (step 29 semantic REST + step 30 eval `--semantic`) + `contextforge eval run --semantic` dual-path CLI | #145 |
| 19.5 | **real** dogfood embedding `SemanticRecall@K` (fastembed) + `docs/spikes/phase-19-real-recall.md` | #146 |
| 19.6 | ADR-023 Proposed→**Accepted** + ADR-006 A1→**Active** + ADR-008 embedding-crate amendment | #147 |
| 19.7 | Phase 19 closeout (end-to-end semantic search) + v0.12.0 release docs | this PR |

### Real-embedding recall (resolves the Phase 18 non-discriminating caveat)

Real `FastEmbedProvider` (`all-MiniLM-L6-v2`, dim 384) over real ContextForge text, exact cosine (`docs/spikes/phase-19-real-recall.md`):

| metric | Phase 18 synthetic | Phase 19 real |
|---|---|---|
| SemanticRecall@5 | 1.0 (non-discriminating) | **0.8333** (25/30) |
| SemanticRecall@10 | 1.0 (non-discriminating) | **0.9333** (28/30) |
| top-1 / MRR | — | 0.60 / 0.70 |
| ADR-006 A1 gate (≥ 0.70) | aspirational | **PASS** |

ADR-013: every real number is a real fastembed run — no synthetic / deterministic / fabricated figures. The recall is exact-cosine (representative of any exact backend incl. the D1 `sqlite-vec` pick; upper bound for ANN).

### Upgrade path

- Drop-in: default build behavior is unchanged (BM25-only retrieval, 0 new dependency). No migration.
- To use the semantic path: send `?semantic=true` on `POST /v1/search`, or run `contextforge eval run --semantic`. The default-build semantic path uses the deterministic provider; for real-model semantic search build/deploy with `--features embedding-fastembed`.
- Proto: `SearchRequest.semantic` (7), `RetrievalResult.vector_score` (13) + `embedding_provider` (14) are **add-only** — existing clients are unaffected (22-endpoint conformance + proto-freeze guard PASS).

### Rollback path

`git tag -d v0.12.0` + delete the GitHub Release/ghcr tag. The default-build image is behavior-compatible with v0.11.0 (BM25-only), so a rollback is non-breaking.

### Contract

Console Contract v1 unchanged in shape; the three new proto fields are add-only and default to BM25 behavior when unset.

详 [Phase 19 spec](docs/specs/phases/phase-19-vector-retrieval-integration.md) + [ADR-023](docs/decisions/adr-023-vector-backend-default.md) + [v0.12.0 evidence](docs/releases/v0.12.0-evidence.md) + [v0.12.0 artifacts](docs/releases/v0.12.0-artifacts.md)。

## v0.11.0 (2026-05-30) — vector-backend-selection (infra + spike + ADR-023 Proposed)

### 摘要

v0.11.0 minor release: ships the **vector retrieval backend infrastructure + a data-driven backend selection** (Phase 18). It delivers the vector trait abstraction, a deterministic spike harness, **four real-data backend spikes measured on one Linux host**, the ADR-023 default-backend decision (**Proposed**), and the `SemanticRecall@K` eval metric + gate.

**Honest scope (read this)**: this is an **infrastructure + selection milestone**, *not* live semantic search. Production semantic retrieval and the ADR-023 ratification are **deferred** — the Phase 18 spike deliberately used deterministic seed vectors to avoid an ONNX/embedding dependency, so there is no real-distribution recall yet (all four backends score recall 1.0 on synthetic data — non-discriminating). Wiring a chosen backend into the production retriever + an embedding provider is a follow-on phase (`[SPEC-OWNER:phase-future.vector-retrieval-integration]`, ADR-023 D6). The `vector-*` features ship **off by default** — the default build is BM25-only and dependency-free.

### What shipped (Phase 18, tasks 18.1–18.9)

| task | delivery | PR |
|---|---|---|
| 18.1 | `Vector{Backend,Indexer,Searcher}` trait abstraction + `NoopVectorBackend` + retriever seam | #128 (+#129 review) |
| 18.2 | `bench/` spike harness — deterministic corpus + 5-dim measure + runner | #130 |
| 18.3 | **sqlite-vec** backend (`vec0`, stable 0.1.9) | #133 |
| 18.4 | **qdrant** backend (qdrant-client gRPC → local server) | #134 |
| 18.5 | **lancedb** backend (embedded Lance + Arrow) | #135 |
| 18.6 | **hnsw** backend (instant-distance, pure Rust) | #131 |
| 18.7 | **ADR-023 (Proposed)** default-backend decision + 4-backend comparison | #136 |
| 18.8 | `internal/eval` **SemanticRecall@K** metric + recall gate + ADR-006 Amendment A1 | #137 |
| 18.9 | Phase 18 closeout (honest scope) + v0.11.0 release docs | this PR |

### 4-backend comparison (real Linux data, n=100000 / dim=64)

| backend | recall@5/10 | P95 (ms) | index RSS (MB) | cold-start | model |
|---|---|---|---|---|---|
| sqlite-vec | 1.0 / 1.0 | 3.198 | 90.7 | 760 ms | embedded + disk, exact |
| hnsw | 1.0 / 1.0 | 0.871 | 180.0 | 28.4 s | in-mem ANN, pure Rust |
| qdrant | 1.0 / 1.0 | 0.947 | 91.6 (+~166 server) | 385 ms | external server ANN |
| lancedb | 1.0 / 1.0 | 10.893 | 90.8 | 50 ms | embedded + disk, flat |

recall is non-discriminating on synthetic vectors → the selection is driven by ContextForge's architecture (local-first, single-binary, SQLite-based per ADR-002, cross-platform incl. Windows MSVC), not recall. Full analysis: `docs/spikes/phase-18-comparison.md`.

### ADR-023 (Proposed) — tiered, feature-gated, default build BM25-only

- **D1** sqlite-vec = recommended embedded default (Linux prod, ADR-002-aligned) — **provisional** pending real-embedding recall.
- **D2** hnsw = cross-platform/dev fallback (pure Rust; but 28 s build + 180 MB at 100k → not the prod default at scale).
- **D3** qdrant = hosted/scale-out. **D4** lancedb = embedded-columnar alternative. **D5** default build ships no backend.

### Deferred to a follow-on phase (`[SPEC-OWNER:phase-future.vector-retrieval-integration]`)

- ADR-023 `Proposed → Accepted` ratification (needs real-embedding recall).
- Default backend wired into the production retriever hot path + smoke v9 `/v1/search?semantic=true`.
- An embedding provider (`[SPEC-DEFER:phase-future.embedding-provider-full]`).

### ADR-014 cross-validation gate — 9th activation

D1 mapping table + D2 spec-drift lint 0 hits + D3 verified-by + D4 main-agent autonomy + D5 no Phase 1–17 spec edits, across PRs #133/#134/#135/#136/#137 + this closeout. Each PR: default `cargo test --workspace` 0 failed + `go test ./...` ok + CI three-gate green before autonomous merge.

### Upgrade path (v0.10.0 → v0.11.0)

No migration, no breaking change. The `vector-*` features are off by default; the default build is byte-for-byte BM25-only behavior (NoopVectorBackend). Enabling a backend is a build-time feature choice (sqlite-vec needs Linux/gcc; lancedb needs protoc; qdrant needs a running server). `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.11.0` after the tag push.

### Rollback path

`git tag -d v0.11.0` + delete the GitHub release; the closeout is documentation + feature-gated code, so reverting is a no-op for the default build. No DB or schema change to undo.

## v0.10.0 (2026-05-28) — is-pinned-amendment (Console PR #91/#93 backlog 11/11 = 100% closed)

### 摘要

v0.10.0 minor release: closes the **final** ContextForge-Console PR #91/#93 backlog item (P2 #6 `MemoryItem.is_pinned`). Backlog is now **11/11 = 100% closed** — the full review feedback raised by the Console team in v0.7.x is addressed. **First successful activation of the ADR-015 D5 字段冻结 amendment path** via ADR-022 (`memory-is-pinned-field-amendment`, Proposed → Accepted in this closeout PR).

### Backlog item closed (1 final)

| Item | Backlog signal | Solution | PR |
|---|---|---|---|
| **P2 #6** | `MemoryItem.is_pinned` field missing — Console UI Memory list/detail could only infer pin state from `MemoryOperation.op_type=pin` history (fragile: unpin still leaves a pin record) | task-17.1 — proto field 10 + Rust `memory_to_pb` mapper + Go `contractv1.MemoryItem.IsPinned` + `grpcclient.protoToMemoryItem` + `MemMemoryStore.Pin(id, pin)` (no longer discards `_`) + fixture-1 preset `IsPinned: true` + `handleMemoryPin` JSON body parser (backward-compat empty-body default true) + 5 new tests + smoke v8 step 28 (4 sub-assertions: post-restart survive + explicit pin=false + explicit pin=true + empty-body backward-compat) | [#118](https://github.com/tajiaoyezi/contextforge/pull/118) |

Additional Phase 17 ship:
- **Phase 17 E1 scaffolding** (PR #116, post-v0.9.0): Phase 17 spec + task-17.1 spec + ADR-022 (Proposed) + adapter index — Status: Pending awaiting Console cross-repo amend trigger
- **Phase 17 closeout** (this PR): ADR-022 Status Proposed → Accepted + Phase 17 + task-17.1 spec final §10 fills + v0.10.0 release docs (README + RELEASE_NOTES + evidence + artifacts)

### Cross-repo coordination — first end-to-end exercise of ADR-022 D4/D5

Phase 17 is the first phase to use the ADR-022 D5 cross-repo `Pending → Ready → Done` protocol:

1. **2026-05-28 (Phase 17 scaffolding ship via PR #116)**: ContextForge ships Phase 17 spec + ADR-022 Proposed + task-17.1 with Status: Pending awaiting Console signal.
2. **2026-05-28T12:16:57Z (Console-first ship)**: ContextForge-Console PR [#101](https://github.com/tajiaoyezi/ContextForge-Console/pull/101) merges to Console master @ `415ee30fcd8effd7929806d196458ec6e60fb49f` — `MemoryItem.IsPinned bool` add-only field in `console-api/internal/coreadapter/contractv1/contractv1.go` (between `Status` and `Availability`, JSON tag `is_pinned`).
3. **2026-05-28 (User forwards SHA)**: User forwards Console PR #101 merge SHA `415ee30` to ContextForge main agent.
4. **2026-05-28 (Verification)**: ContextForge main agent verifies via `gh api repos/tajiaoyezi/ContextForge-Console/contents/console-api/internal/coreadapter/contractv1/contractv1.go?ref=415ee30` returns the expected field block; flips Phase 17 + task-17.1 Status: `Pending → Ready → Done` within PR #118 implementation PR.
5. **2026-05-28 (ContextForge ship via PR #118)**: PR #118 ships the proto + Rust + Go end-to-end + tests + smoke v8.
6. **2026-05-28 (this closeout PR)**: ADR-022 Status Proposed → Accepted; v0.10.0 release docs.

This pattern is now reusable for any future cross-repo schema evolution (`tags`, `pinned_at`, etc. — all `[SPEC-DEFER:phase-future.*]`).

### Spec drift discovery

The original task-17.1 §3 prescribed migration `0017_memory_items_add_is_pinned.sql` + PRAGMA gate + Rust `SqliteMemoryStore::set_pinned` implementation. Recon during PR #118 revealed task-13.1 (Phase 13) already shipped most of it forward-looking:

- **Migration 0017 NOT needed** — `is_pinned INTEGER NOT NULL DEFAULT 0` was already added in `core/migrations/0013_memory_items.sql:16` at task-13.1 ship (Phase 13). The comment in 0013 even read "9 columns 1:1 mirror contractv1.MemoryItem + orthogonal is_pinned flag". Creating 0017 would have errored with `duplicate column name` on existing v0.6+ DBs.
- **Rust `SqliteMemoryStore::set_pinned` + `MemoryServer.Pin` write-through wiring** already shipped at Phase 13. Only the proto wire propagation (via `memory_to_pb` mapper) and the Go-side surface needed update.
- **`handleMemoryPin` body parsing gap** — the original handler at `internal/consoleapi/handlers.go:524` hardcoded `deps.Memory.Pin(id, true)` and never read the request body. Task-17.1 spec §3 missed this gap; the new handler now parses `{"pin": bool}` with empty/malformed body defaulting to `true` (preserving v0.7-v0.9 backward-compat contract).

PR #118 commit body + task-17.1 §3 + this release notes capture the discovery for future readers.

### Schema additions (add-only per ADR-015 D1 + first ADR-015 D5 amendment via ADR-022)

- `proto/contextforge/console_data_plane/v1/console_data_plane.proto`: `MemoryItem.bool is_pinned = 10` (add-only field 10; next available after `string status = 9`)
- `internal/contractv1/contractv1.go::MemoryItem.IsPinned bool` (json tag `is_pinned`, position between `Status` and `Availability` — mirrors Console master @ `415ee30` exactly)
- No SQLite migration needed (column already at `0013:16`)
- 22-endpoint Console contract conformance unaffected (contract v1 not bumped)
- Forward/backward compat: legacy v0.7-v0.9 daemon responses lacking `is_pinned` key unmarshal to Go bool zero value (`false`) — Console v0.10+ client treats this as "memory item not currently pinned" fallback. New v0.10+ daemon responses carry the real state.

### 关键设计取舍

- **`bool` type, not `*bool`**: pin state is always defined (never "not applicable" — Memory items are either pinned or not). Pointer + `omitempty` would let Console UI render ambiguously. ADR-022 D1 locks this.
- **`handleMemoryPin` empty-body defaults to `pin=true`**: preserves v0.7-v0.9 callers that POST without body. Pointer-typed body (`*bool`) cleanly distinguishes "absent" (default true) from "explicit false". Malformed JSON also falls back to `true` rather than 400 — lenient contract preserved.
- **No `pinned_at` / `pin_actor` / `tags` / `priority` fields in this amendment**: explicitly `[SPEC-DEFER:phase-future.*]`. ADR-022 §Trade-offs locks this — future amendments can follow the same D4/D5 protocol established here.
- **MemMemoryStore fixture-1 preset `IsPinned: true`**: ADR-022 D3 stipulates at least one pinned fixture so Console UI fallback mode (`CONSOLE_API_FALLBACK_INMEM=1`) renders a pinned row when verifying the new field. ADR-018 deny default keeps this off in production.
- **Smoke v8 step 28 gated on `MODE=real && sqlite3`**: the runtime end-to-end check needs both the Rust daemon (for SQLite persistence) and `sqlite3` CLI (for fixture seeding via `test/fixtures/memory-seed/seed.sql`). LOCAL_ONLY/docker modes verify via `internal/consoleapi/memstore_test.go` unit tests instead.

### ADR-014 cross-validation gate 第八次激活

- D1 mapping table: PR #118 body contains the Phase §6 ↔ task-17.1 §6 AC mapping (7-row table including the deferred AC7 → resolved in this closeout PR)
- D2 lint `--touched origin/master`: 0 unannotated hits across PR #118 + this closeout PR
- D3 verified-by: every Phase 17 §6 AC and task-17.1 §6 AC carries an explicit `verified by <test>` clause
- D4 governance: 主 agent 自治 §2A Ready review + R6 merge decision; user as single driver forwards the Console SHA but does not edit ContextForge code
- D5 历史不溯改: Phase 1-16 specs untouched (verified via `git diff origin/master` scoping)

### Tests (cumulative Phase 17)

- `cargo test --workspace`: 41 tests across crates (lib + integration); PR #118 adds 1 new lib test (`test_list_returns_is_pinned_column`) + 2 new gRPC integration tests (`test_is_pinned_propagates_via_grpc_list_and_get`, `test_pin_rpc_unpin_reverses_state`). Existing `test_set_pinned_persists` from Phase 13 covers the SqliteMemoryStore toggle path.
- `go test ./...`: 21 packages all PASS. PR #118 adds 2 new unit tests in `internal/consoleapi/memstore_test.go` (`TestMemMemoryStore_Pin_TogglesIsPinned`, `TestMemMemoryStore_List_ReturnsIsPinned`) + 1 new test in `internal/contractv1/types_test.go` (`TestMemoryItemForwardBackwardCompat`) + extended `TestJSONRoundtrip` with `MemoryItem_pinned` case.
- `test/conformance` 22-endpoint Console contract: unchanged (contract v1 not bumped).
- `bash scripts/console_smoke.sh` v8 28-step bash syntax verified; runtime gated `MODE=real && sqlite3` per step 28.
- `bash scripts/spec_drift_lint.sh --touched origin/master`: 0 unannotated hits across PR #118 + this closeout PR.

### Upgrade path (v0.9.0 → v0.10.0)

- **SQLite DB users**: no migration required (column already in 0013 from v0.6 ship). After upgrade to v0.10.0, `is_pinned` field begins surfacing on `GET /v1/memory[/<id>]` responses with the actual persisted value.
- **Console UI clients (v0.7-v0.9)**: existing client code reading the v0.10.0 response silently ignores the new `is_pinned` key (Go JSON unmarshal ignores unknown fields). No client-side change required.
- **Console UI clients (v0.10+ adapted)**: client can now sort/render based on `MemoryItem.IsPinned`. Existing Console PR #101 ships the field type; rendering UI is the next user-driven Console PR (visual closure, outside this autonomous flow).
- **Docker users**: `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.10.0` after tag push (release.yml handles ghcr build/push on `v*` tag).
- **No BREAKING** — purely additive schema. Backward compatible in both directions per ADR-015 D1 + ADR-022 D4.

### Rollback path

If v0.10.0 ship reveals an unexpected issue:

1. `git revert <v0.10.0 merge SHA>` to roll back to v0.9.0 (master HEAD `cfcdbd4` post-PR-#118 but pre-this-closeout)
2. Ship v0.10.0.1 patch tagging the specific concern
3. No DB rollback needed — `is_pinned` column has always been in 0013 (Phase 13); rolling back the proto/contractv1 field doesn't drop the column
4. ADR-022 stays Accepted (the decision path is sound even if implementation needs patching)

### Cross-repo follow-up — **COMPLETED 2026-05-29** 🎉

User-forwarded after this closeout PR merge + v0.10.0 tag push:
- ✅ Notified Console team of v0.10.0 release ship via GitHub Release page URL (2026-05-28)
- ✅ **Console UI visual closure SHIPPED end-to-end** to Console master @ `c1c4609744a9c34201e3fd87cba4ab1596be4fd4`:
  - PR [#102](https://github.com/tajiaoyezi/ContextForge-Console/pull/102) `30aeff4` — pin 排序 + 列表 icon + 详情 "已置顶" badge (UI 主体)
  - PR [#103](https://github.com/tajiaoyezi/ContextForge-Console/pull/103) `14f9ce0` — v0.10.0 ack: mock 落真 is_pinned + docker-compose 切 GHCR pull + 联调清单文档 + apiFetch typecheck 潜伏 bug 修
  - PR [#104](https://github.com/tajiaoyezi/ContextForge-Console/pull/104) `c1c4609` — pin-sort util 抽函数 + 混合 pinned/unpinned 数组排序单测
- ✅ E2E daemon-level verification (Console-reported): `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.10.0` → http stack → daemon fixtures → daemon → console-api(http) → BFF → web → 详情页 "已置顶" badge 实拍坐实
- 🎉 **ContextForge-Console PR #91/#93 review backlog end-to-end 100% closed** (backend protocol via cumulative Phase 13/15/16/17 + UI visual surface via Console PRs #102/103/104)
- Feedback acknowledged: GHCR package v0.10.0 / :latest was initially shipped as PRIVATE (anonymous pull 403, observed by Console team); owner has since flipped to public. Future enhancement to add anonymous-pull verify step `[SPEC-DEFER:phase-future.verify-image-anonymous-pull]`.

ContextForge agent has no further obligation on this backlog.

---

## v0.9.0 (2026-05-28) — v0.9.0-backlog-completion (10/11 closed) + release infra

### 摘要

v0.9.0 minor release：closes 4 of the remaining 5 Console PR #91/#93 backlog items (P3 + P4) + ships production release infrastructure (GHCR image push CI + docker-compose.production.yml + verify-image.yml workflow). Backlog status now **10/11 = 91% closed**; only `MemoryItem.is_pinned` (P2 #6) remains for Phase 17 cross-repo coord. **No new ADR in v0.9.0 itself** — 4 Phase 16 tasks all extend existing ADR-013/015/016/017/018.

### Backlog items closed (4 more)

| Item | Backlog signal | Solution | PR |
|---|---|---|---|
| **P4 #10** | TraceStore daemon-restart 即丢历史 | task-16.1 — migration 0015_search_traces.sql + SqliteTracePersist + TraceStore write-through + warm restore | #110 |
| **P4 #11** | events `?wait=` 等价 batch polling | task-16.2 — handleEvents 真传 wait + EventsClient.Recent(limit, wait) + 两阶段 long-poll (phase 1 block + phase 2 100ms drain) | #111 |
| **P3 #8** | ghcr.io image push 缺 CI/CD | task-16.3 — `.github/workflows/release.yml` (tag push → docker build + push ghcr) + `ci.yml` (PR/push → cargo+go+lint 3 parallel jobs) | #112 |
| **P3 #9** | production-ready docker-compose 缺示例 | task-16.4 — `deploy/docker-compose.production.yml` 双容器 (contextforge-core + console-api-serve, ADR-018 fallback deny 沿用, 卷持久化, healthcheck) + `.env.production.example` + `docs/deploy/production.md` + smoke v7 27-step | #113 |

Additional Phase 16 ship:
- **Phase 16 E6 closeout** (PR #114): Status → Done + §10 Completion Notes + adapter sync
- **Phase 16 E7 release-verify** (PR #115): `.github/workflows/verify-image.yml` GHA pull+run+/v1/health verification workflow

Remaining (deferred to Phase 17 / v0.10.0):
- P2 #6 `MemoryItem.is_pinned` (ADR-015 D5 amendment via ADR-022 Proposed) — Phase 17 + task-17.1 scaffolded in PR #116 with Status: Pending awaiting Console contractv1.go cross-repo amend trigger

### v0.9.0 不引入新 ADR

Phase 16 4 task 全部是既有 ADR 的延伸实施：
- task-16.1 ↔ ADR-013 (CLI data plane gRPC bridge) + ADR-015 D1 (add-only schema)
- task-16.2 ↔ ADR-017 D4 (long-poll v1.0 lock — 不引入 SSE)
- task-16.3 ↔ ops practice (CI/CD pipeline 不构成 architectural decision)
- task-16.4 ↔ ADR-018 (fallback deny default 沿用)

**ADR-022 (memory-is-pinned-field-amendment)** 在 v0.9.0 ship 后作为 Phase 17 scaffolding PR #116 单独 ship — Status: Proposed；属 Phase 17 不属 v0.9.0 release。

### Schema additions (all add-only, ADR-015 D1)

- `core/migrations/0015_search_traces.sql`: 新建 `search_traces` 表 (query_id PK / trace_json TEXT / workspace_id TEXT / ts_unix INTEGER / created_at TEXT) + `idx_search_traces_ts_desc` 索引 (IF NOT EXISTS 幂等)
- `core/src/data_plane/search_persist.rs`: 新模块 `SqliteTracePersist` (open + put + get + list + load_warm)
- `internal/consoleapi/types.go`: `EventsClient.Recent(limit int)` → `Recent(limit int, wait time.Duration)` (signature extension; 所有 callers 同步更新)
- 既有 `RetrievalTrace` / `QueryRecord` / `MemoryItem` / `CoreHealth` 等 contract v1 message **完全不动** (ADR-015 D1 freeze 维持)

### 关键设计取舍

- **task-16.1 write-through dual-write**: 内存 LRU cap=1000 保留作 hot cache (低延迟读) + SQLite SoT best-effort 双写 (持久化保证)；SQLite write 失败 swallow 不阻塞 RPC 返回
- **task-16.1 SQLite trace_json 序列化**: prost-encoded bytes → base64 → store as TEXT (与 PbRetrievalTrace prost-derive 一致；非 serde_json — 避免 schema drift)
- **task-16.1 cap-by-LRU 内存 + cap-by-DELETE 留 future**: 内存 LRU cap=1000 同 v0.8；SQLite 端无 LRU eviction → 长时间运行后表可能数百万行；留 SPEC-DEFER:phase-future.tracestore-sqlite-vacuum
- **task-16.2 两阶段 long-poll**: phase 1 block 等首 event ≤ wait；phase 2 短 drainTimeout=100ms drain immediately-available events；避免单 event 触发后立即返就只带 1 个 event 浪费 RTT
- **task-16.4 CONTEXTFORGE_ALLOW_WILDCARD_BIND=1 env opt-in**: ADR-004 安全基线下 daemon 默认 127.0.0.1 bind；docker compose-prod 需 0.0.0.0 跨容器；引入 env opt-in 显式解锁 (PR #113 review fix c21315b) — 非默认行为 + 用户感知
- **task-16.4 ADR-018 deny 默认沿用**: compose-prod 不注入 `CONSOLE_API_FALLBACK_INMEM=1` → 真 grpcclient 不可达时 503 (与 v0.7.2 deny 默认一致)

### ADR-014 cross-validation gate 第七次激活

- D1 closeout PR (#114) body 含 Phase §6 ↔ Task §6 mapping 表 (6 行)
- D2 lint `--touched origin/master`: 0 unannotated hits in PR-changed lines
- D3 phase-16 §6 每条 AC 含 verified-by owner 显式
- D4 governance: 主 agent 自治 §2A Ready review + R6 merge decision
- D5 历史不溯改: Phase 1-15 spec 内容未触

### Tests (cumulative Phase 16 E1-E7)

- `cargo test --workspace`: full PASS (Phase 11-15 既有 + Phase 16 task-16.1 新增 TraceStore SQLite persist tests + memory_persist_integration tests 不退化)
- `go test ./...`: 22 packages 全 PASS (含 task-16.2 handlers_test.go::TestHandleEvents_Wait5s_Blocks_When_NoEvent + TestHandleEvents_Returns_Early_OnEvent + grpcclient 4 unit tests + e2e_grpc Step 11b real long-poll 不退化)
- `test/conformance`: 22-endpoint Console contract conformance 不退化
- `bash -n scripts/console_smoke.sh`: syntax OK; v7 27-step (v6 24 + step 25 `?wait=2s` + step 26 TraceStore restart roundtrip + step 27 compose-prod stack health gated `COMPOSE_PROD_SMOKE=1`)
- `gh workflow run verify-image.yml -f tag=v0.9.0-rc1`: GHA run 26555768957 GREEN in 18s (pull + run + /v1/health probe + `?detailed=true` 5-component breakdown)
- `gh workflow run verify-image.yml -f tag=v0.9.0`: GHA run 26556137023 GREEN in 11s (post-release verify)

### Upgrade path (v0.8.0 → v0.9.0)

**Console UI / SDK 用户** (v0.7.x-v0.8.x clients 继续工作):
- 旧 client 解析 v0.9 JSON 自动忽略未知字段 (`Events.wait` semantic 仅 server-side 生效) → zero migration
- Console UI Dashboard 历史查询面板自动 survive daemon restart (无 client 改动)
- Memory 操作历史 events stream 现在真 long-poll (≤ wait latency)

**ContextForge daemon 升级**:
- 二进制升级 v0.8.0 → v0.9.0 不破坏既有部署 (无 BREAKING)
- SQLite migration 0015 自动应用 (IF NOT EXISTS 幂等)；既有 in-memory traces 不迁移 (重启时空 cap=1000 LRU + 后续 search 累积新 trace)
- Docker users: `docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0` (替代 v0.8 本地 `docker build`)

**新功能 opt-in 试用**:

```bash
# 1. GHCR image pull (replaces local docker build)
docker pull ghcr.io/tajiaoyezi/contextforge-daemon:v0.9.0
docker pull ghcr.io/tajiaoyezi/contextforge-daemon:latest  # always points to latest release

# 2. Real long-poll events
curl 'http://localhost:48181/v1/observability/events?wait=5s'
# now truly blocks 5s when no events (vs prior batch polling)

# 3. Trace persistence — survive daemon restart
curl -X POST -H "X-Confirm: yes" http://localhost:48181/v1/search \
  -d '{"query":"foo","limit":5}'
# (note query_id)
docker restart contextforge
curl 'http://localhost:48181/v1/queries?limit=10'    # 仍有历史
curl http://localhost:48181/v1/search/{query_id}/trace  # 仍 200

# 4. Production-ready compose stack
git clone https://github.com/tajiaoyezi/contextforge && cd contextforge
cp deploy/.env.production.example deploy/.env.production
# edit deploy/.env.production for your tokens
docker compose -f deploy/docker-compose.production.yml up -d
curl http://localhost:48181/v1/health   # expect: {"status":"healthy", ...}
```

### Rollback path

若 v0.9.0 ship 后发现非预期问题：
1. `git revert <v0.9.0 merge SHA>` 回退到 v0.8.0 (master HEAD `622155b` 或 v0.8.0 tag)
2. ship v0.9.0.1 patch + 标具体 task 16.x Reverted
3. SQLite migration 0015 不撤回 (新表无 backward break — 既有 v0.8 binary 不读 search_traces 表)
4. 不撤回 v0.8.0 ADR-020 / ADR-021 / v0.7.2 ADR-018 / v0.7.0 ADR-017 (跨版本独立)

Cross-repo follow-up:
- 通知 Console 团队 v0.9.0 ship → Console UI 验证 Dashboard 历史查询面板跨重启 + events real long-poll latency 提升
- **Phase 17 启动信号**: 用户人工转发本 release page → Console 主 Agent 启动 contractv1.go IsPinned add-only field amend PR (ADR-022 D4 第 1 步) → 完成后回报触发 ContextForge Phase 17 Pending → Ready

详 [docs/releases/v0.9.0-evidence.md](docs/releases/v0.9.0-evidence.md) + [v0.9.0-artifacts.md](docs/releases/v0.9.0-artifacts.md)。

---

## v0.8.0 (2026-05-26) — Console functional gap closure (6/11 backlog)

### 摘要

v0.8.0 minor release：closes 6 of 11 items raised in the ContextForge-Console PR #91/#93 backlog (P0 + P1 + P2#7). New Dashboard backend endpoints (chunks stats / eval-runs list / queries history), 5-link health detail (db / index / embed / retriever / eval), MemStore fallback drill-down fix, and the long-standing memory.* → EventBus bridge (Phase 13 [SPEC-DEFER:phase-future.memory-event-bus-bridge] lifted). Two new ADRs (020 / 021) promoted to Accepted.

### Backlog items closed (6/11)

| Item | Backlog signal | Solution | PR |
|---|---|---|---|
| **P0 #1** | MemStore inmem-fallback 503 on drill-down | task-15.1 — chunkCache + traceCache (FIFO cap=256) | #99 |
| **P0 #2** | `memory.*` event 桥接 缺失 | task-15.2 (ADR-021) — `emit_audit` 同步追加 `EventBus.send` | #100 |
| **P1 #3** | Dashboard "已索引块" 缺 backend | task-15.3 — `GET /v1/stats/chunks` (Tantivy `num_docs` + SQLite COUNT today) | #101 |
| **P1 #4** | Eval 列表 缺 endpoint | task-15.4 — `GET /v1/eval-runs?workspace_id=&status=&limit=N` (ORDER DESC) | #102 |
| **P1 #5** | Dashboard "最近查询" 缺 backend | task-15.5 — `GET /v1/queries?limit=N` (TraceStore.list wrapper) | #103 |
| **P2 #7** | CoreHealthCard 5 链路 缺 | task-15.6 (ADR-020) — `GET /v1/health?detailed=true` (5 probes opt-in) | #104 |

Remaining (deferred to Phase 16 / v0.9.0):
- P2 #6 `MemoryItem.is_pinned` (needs ADR-015 D5 amendment — BREAKING window required)
- P3 #8 ghcr.io image push — CI/CD pipeline work
- P3 #9 docker-compose.production.yml example
- P4 #10 TraceStore SQLite persistence (currently in-memory ring buffer)
- P4 #11 `?wait=` real long-poll (currently batch polling — v0.7.2 cleanup already documented this)

### 新增 ADR

- **ADR-020 health-component-breakdown** (Accepted 2026-05-26): D1-D5 spelling out the 5 probes (db SQLite ping / index Tantivy open / embed config check / retriever top_k=1 / eval store open), add-only ComponentHealth schema, opt-in `?detailed=true`, aggregation rule (any unreachable → 503; any degraded → 200 + degraded), Console cross-repo coord.
- **ADR-021 memory-event-bus-bridge** (Accepted 2026-05-26): D1-D4 — `emit_audit_and_event` shared path (no new channel), 3 new event_type string values (`memory.pin` / `memory.deprecate` / `memory.soft_delete`; pin/unpin share via payload `op`), field contract (severity=info, source=contextforge-core, trace_id/job_id None), best-effort emit with SendError swallowed.

### Schema additions (all add-only, ADR-015 D1)

- proto `console_data_plane.proto`:
  - `SearchService.GetChunksStats` + `GetChunksStatsRequest` + `ChunksStats{total, today_delta}`
  - `SearchService.ListQueries` + `ListQueriesRequest` + `ListQueriesResponse` + `QueryRecord{query_id, query, ts_unix, workspace_id}`
  - `EvalService.List` + `ListEvalRunsRequest` + `ListEvalRunsResponse`
  - new `HealthService.GetDetailed` + `ComponentHealth` + `DetailedHealthRequest` + `DetailedHealthResponse`
- `internal/contractv1`:
  - `ChunksStats`, `QueryRecord`, `ListEvalRunsFilter`, `ComponentHealth` Go structs
  - `CoreHealth.Components map[string]ComponentHealth` (omitempty) + `CoreHealth.TotalLatencyMs *int64` (omitempty)
- 既有 `RetrievalTrace` / `EvalRun` / `MemoryItem` 消息**完全不动** (ADR-015 D1 字段冻结保留)

### 关键设计取舍

- **task-15.5 TraceRecord wrapper**: 保留 `RetrievalTrace` 不动 (ADR-015 D1 freeze)，workspace_id + ts_unix 仅作 Rust-side metadata 储存在 `TraceStore.put` 内部；新 `QueryRecord` message 是这俩元数据的真承载
- **task-15.6 synthesize fallback for nil HealthClient**: handleHealth 在 fallback / degraded 模式下 synthesize 5-component 全 healthy / 全 degraded，让 Console UI CoreHealthCard 永远拿到完整 5 key shape
- **task-15.3 today_delta lexicographic SQLite compare**: 复用既有 `chunks.indexed_at TEXT NOT NULL` 列；`seconds_to_iso` (Howard Hinnant 算法，无 chrono dep) 生成 `YYYY-MM-DD HH:MM:SS` 格式 — lexicographic >= 与时序一致
- **task-15.2 memory.pin / memory.unpin 合并 event_type**: payload_json `op` 区分；event_type 命名空间紧凑

### ADR-014 cross-validation gate 第六次激活

- D1 closeout PR (#105) body 含 Phase §6 ↔ Task §6 mapping 表 (7 行)
- D2 lint `--touched origin/master`: 0 unannotated hits in PR-changed lines (Python equivalent 实测；bash 在 Windows 太慢)
- D3 phase-15 §6 每条 AC 含 verified-by owner 显式
- D4 governance: 主 agent 自治 §2A Ready review + R6 merge decision (cross-repo 字段仅 add-only)
- D5 历史不溯改: Phase 1-14 spec 内容未触

### Tests (cumulative E2-E7)

- `cargo test --workspace`: 121 lib + 17 integration test files 全 PASS (Phase 11-14 既有不退化)
- `go test ./...`: 22 packages 全 PASS (含 `test/conformance` 22-endpoint Console contract conformance 不退化)
- `bash -n scripts/console_smoke.sh`: syntax OK; v6 24-step (既有 20 + 4 new for chunks-stats / eval-runs / queries / health-detail)
- Smoke daemon-level CONSOLE_REAL_SMOKE_EXIT=0 留 v0.8.0 ship 前 manual / CI 实测

### Upgrade path (v0.7.x → v0.8.0)

**Console UI / SDK 用户** (v0.7.x 客户端继续工作):
- 旧 client 解析 v0.8 JSON 自动忽略未知字段 (`Components` / `TotalLatencyMs` / new endpoint shapes) → zero migration
- Console UI 启动 standby PR 后切到 v1.x：Dashboard 3 KPI / CoreHealthCard 5 链路 / Memory 操作历史 自动有数据

**ContextForge daemon 升级**:
- 二进制升级 v0.7.2 → v0.8.0 不破坏既有部署 (无 BREAKING)
- Docker users: `docker pull contextforge-daemon:v0.8.0` — fallback 默认行为不变 (ADR-018 v0.7.2 决定继承)

**新 endpoints opt-in 试用**:
```bash
# Dashboard 已索引块
curl http://localhost:48181/v1/stats/chunks

# Eval 最近评测
curl 'http://localhost:48181/v1/eval-runs?limit=10'

# Dashboard 最近查询
curl 'http://localhost:48181/v1/queries?limit=20'

# CoreHealthCard 5 链路
curl 'http://localhost:48181/v1/health?detailed=true' | jq .components
```

### Rollback path

若 v0.8.0 ship 后发现非预期问题：
1. `git revert <v0.8.0 merge SHA>` 回退到 v0.7.2 (master HEAD `c3e6698^` 前一版本 `5264fd6`)
2. ship v0.8.0.1 patch + 标 ADR-020 / ADR-021 status Superseded 或 Reverted
3. 不撤回 v0.7.2 ADR-018 / v0.7.0 ADR-017 (跨版本独立)

Cross-repo follow-up: 通知 Console 团队 v0.8.0 ship → Console UI standby PR (Dashboard 3 KPI 真接 + CoreHealthCard 5 链路 + Memory 操作历史)。

详 [docs/releases/v0.8.0-evidence.md](docs/releases/v0.8.0-evidence.md) + [v0.8.0-artifacts.md](docs/releases/v0.8.0-artifacts.md)。

---

## v0.7.2 (2026-05-26) — fallback-inmem default reversal ⚠️ BREAKING

### 摘要

v0.7.2 patch release：按 v0.7.1 pre-announce 反转 single-image deployment 默认行为，消除 in-mem fallback 的 silent footgun（HTTP 200 healthcheck 掩盖容器重启数据失风险）。代码无改动，仅 Dockerfile 删 ENV 行 + ADR-018 spec lock。

### 变更点

详 [ADR-018: fallback-inmem-default-reversal](docs/decisions/adr-018-fallback-inmem-default-reversal.md)（D1-D4 共 4 决策）。

#### 1. Dockerfile 删 `ENV CONSOLE_API_FALLBACK_INMEM=1`
- v0.7.1 行为：`docker run contextforge-daemon:v0.7.1` → 默认 fallback-inmem，`/v1/health` 返 200（degraded），容器重启数据失
- **v0.7.2 行为**：`docker run contextforge-daemon:v0.7.2` → 默认 fallback **deny**，gRPC core 不可达时 `/v1/health` 返 **503**，docker healthcheck 立即报 unhealthy

#### 2. Binary code 无变更
- `internal/cli/console_api_serve.go` binary default 一直是 `false`，v0.7.1 是 Dockerfile ENV 单方面强制 set 成 true
- v0.7.2 删 ENV 行后，binary default 自然生效，container 内外行为统一

#### 3. ADR-018 ratification test
- 新增 `TestADR018_BinaryDefaultIsFallbackDeny` 锚定意图（`internal/cli/console_api_serve_test.go`）
- 现有 `TestBuildDeps_DegradedWhenNoDaemon` + `TestRouter_HealthDegraded_503` 已覆盖默认 deny 路径，本 patch 无 logic change

### ⚠️ BREAKING change call-out

**v0.7.1 → v0.7.2 升级前请 review 您的部署方式**：

| 部署方式 | v0.7.1 默认 | v0.7.2 默认 | 升级动作 |
|---|---|---|---|
| `docker run` single-image | inmem-fallback (200) | **fallback deny (503)** | 保留旧行为需 `-e CONSOLE_API_FALLBACK_INMEM=1` opt-in |
| docker-compose single-service | inmem-fallback (200) | **fallback deny (503)** | docker-compose.yml `environment` 加 `CONSOLE_API_FALLBACK_INMEM=1` opt-in |
| docker-compose multi-process (核 + proxy) | 已 opt-out via `=0` | 无变更 | 无需动 |
| k8s Deployment | inmem-fallback (200) | **fallback deny (503)** | manifest env 加 `CONSOLE_API_FALLBACK_INMEM=1` opt-in 或切真 multi-process |
| 纯 binary (非 docker) | fallback deny | fallback deny | **无影响** |

### Upgrade path (v0.7.1 → v0.7.2)

```bash
# 1. 切到新 image (拉 v0.7.2 tag)
docker pull contextforge-daemon:v0.7.2

# 2. 验证默认 deny 行为
docker run -d -p 48181:48181 --name v072 contextforge-daemon:v0.7.2
sleep 5
curl -o /dev/null -w '%{http_code}\n' localhost:48181/v1/health
# expect: 503 (v0.7.1 是 200)

# 3. 保留旧行为 (in-mem fallback) → 显式 opt-in
docker rm -f v072
docker run -d -p 48181:48181 -e CONSOLE_API_FALLBACK_INMEM=1 --name v072-optin contextforge-daemon:v0.7.2
sleep 5
curl -o /dev/null -w '%{http_code}\n' localhost:48181/v1/health
# expect: 200 + status=degraded
```

### Trade-offs / Conscious decisions

- **env 名保留 `CONSOLE_API_FALLBACK_INMEM`**（不改 `ALLOW_INMEM`）— v0.7.x patch series 不引入 dual-name + deprecate 包袱；改名留 v0.8/v1.0
- **不加 startup banner WARN** — (a) 方案的 503 healthcheck 已是 ops 链路最强信号，banner WARN 易被 multi-container log 掩盖
- **不变更 contractv1.go / proto / Rust core code** — 仅 Dockerfile + 单元测试 + spec docs
- **Console 端 standby chore PR 已准备好**（ContextForge-Console PR #91 §6.5 F1 列出动作清单）— v0.7.2 ship 后 Console 团队同步 ship docker-compose.yml + .env.example 更新

### Tests

- `cargo test -p contextforge-core`: 94 lib + 5 integration suites all PASS (无 logic change，不退化)
- `go test ./...`: 43 packages PASS + 新增 1 个 `TestADR018_BinaryDefaultIsFallbackDeny`
- Docker container 实测 (manual verify on PR review)：
  - 默认 `docker run contextforge-daemon:v0.7.2` → `/v1/health` 503 + healthcheck unhealthy
  - `-e CONSOLE_API_FALLBACK_INMEM=1` → `/v1/health` 200 + status=degraded + healthcheck healthy

### Console (cross-repo) sync state

- Console 主仓 master `3370a92` (PR #91) checklist §6.5 F1 已 standby
- v0.7.2 ship 后 Console 端启动 chore PR：docker-compose.yml + .env.example 加 `CONSOLE_API_FALLBACK_INMEM=1` opt-in；checklist §6.5 F1 标 ✅
- 跨仓 break change 双向 coordinate path：ContextForge → 用户转达 → Console 主 Agent 启动 standby PR

### Rollback path

若 v0.7.2 ship 后发现 (a) 方案不可接受（Console standby PR 延迟 / 其它用户 ops 链路无法适配）：
1. `git revert <v0.7.2 commit>` 反转
2. ship v0.7.3 patch + ADR-018 status 改 "Reverted"
3. 重新 design：可能切到 (b) startup-banner WARN 双重防御，或等 v0.8 ship 2 进程 image 一起解决
4. 跨仓通知 Console 团队 v0.7.3 ship + standby PR 撤回

---

## v0.7.1 (2026-05-26) — Dockerfile + single-image deployment fix

### 摘要

v0.7.1 patch release：收齐 v0.7.0 Dockerfile 4 处 stale，single-image docker
deployment ready。ContextForge-Console 团队联调期发现，本 patch 一次性 ship。

### 4 处 fix (PR #94, master `233ced5`)

#### 1. Rust 1.82-bullseye → 1.93-slim-bookworm
- 现象：cargo build fail，`cpufeatures-0.3.0 Cargo.toml: feature edition2024 is required`
- 根因：transitive deps `darling@0.23` / `tantivy@0.26` / `time@0.3.47` 要 rustc >= 1.88
- Fix：升 `rust:1.93-slim-bookworm`（保稳定 + 300 MB 小镜像；bullseye Go 1.26 dropped）

#### 2. Go 1.22-bullseye → 1.26-bookworm
- 现象：`go: go.mod requires go >= 1.26 (running go 1.22.12)`
- Fix：升 `golang:1.26-bookworm`（Go 1.26 dropped bullseye）

#### 3. 加 ENV CONSOLE_API_FALLBACK_INMEM=1（single-image default 模式）
- 现象：v0.7.0 image 起来后 daemon 只跑 REST proxy 不起 Rust gRPC core 进程
  → `/v1/health` 返 503 → docker healthcheck `curl -fsS` 永远不过
- Fix：single-image deployment 默认 in-memory MemStore 模式（ADR-016 §D4）
  - 默认：`docker run contextforge-daemon:v0.7.1` → backend=inmem-fallback → 200
  - 多进程：`docker run -e CONSOLE_API_FALLBACK_INMEM=0 ...` 关闭 fallback +
    另起 contextforge-core daemon 实现真持久化

#### 4. 加 .dockerignore（build context 瘦身）
- 现象：v0.7.0 build context 含 `target/` 9.3 GB cargo cache 全 transfer →
  build 5+ min 才到 cargo 阶段
- Fix：新加 `.dockerignore` 排除 `target/` / `.git/` / `_dispatch/` / `docs/` /
  `test/` 等，build context 从 GB 级降到 ~50 MB

### Behavior change call-out

- **Single-image deployment 默认 `inmem-fallback` 模式 → 容器重启数据全失**
- Multi-process 部署用户需 `docker run -e CONSOLE_API_FALLBACK_INMEM=0` 显式 opt-out
- PR #94 reviewer 与 ContextForge-Console 团队已独立 flag 该默认是 silent
  footgun 风险（telemetry 充分但 HTTP 200 healthcheck 掩盖）→
  **v0.7.2 将反转该默认行为**（详 §"v0.7.2 pre-announce"）

### Verify

```bash
docker build -t contextforge-daemon:v0.7.1 .
# 默认：should be healthy (fallback-inmem)
docker run -d --name v071 -p 48181:48181 contextforge-daemon:v0.7.1
curl localhost:48181/v1/health
# 200 + status="degraded" + error_reason="...in-memory fallback store active..."

# Override：should be 503 (no gRPC core)
docker run -d --name v071-strict -e CONSOLE_API_FALLBACK_INMEM=0 -p 48182:48181 contextforge-daemon:v0.7.1
curl localhost:48182/v1/health
# 503
```

### v0.7.2 pre-announce — fallback default 反转 ⚠️ BREAKING

为消除 single-image silent footgun（HTTP 200 healthcheck 掩盖 in-mem
fallback 风险），v0.7.2 将反转默认行为：

- Daemon default 改为 `CONSOLE_API_FALLBACK_INMEM=0`（强制 opt-in）
- gRPC core 不可达时 → `/v1/health` 返 **503**，docker healthcheck 立刻报 unhealthy
- 旧 v0.7.1 行为兼容：用户显式设 `CONSOLE_API_FALLBACK_INMEM=1` 即可保留
- **Console 团队 standby**：docker-compose.yml 已准备好加 `CONSOLE_API_FALLBACK_INMEM=1`
  env 显式 opt-in；ContextForge-Console 端 chore PR standby 待 v0.7.2 ship

详 v0.7.2 ship 时 ADR-018。

### Console (cross-repo) sync state

- ContextForge-Console 联调期发现本 PR 4 项 stale，cross-repo notify → ship 同步
- Console master `3370a92` (PR #91) 已更新 checklist §6.3 / §6.5 反映 v0.7.1 ship
- Console docker-compose.yml `CONSOLE_API_FALLBACK_INMEM=1` env 当前作显式声明保留，
  v0.7.2 ship 后转为必需 opt-in

---

## v0.7.0 (2026-05-24) — Console 22-endpoint conformance 100% PASS 🎉

### 摘要

ContextForge v0.7.0 完成 **Phase 14 eval-rest-surface** 收口 + **ADR-017
Proposed → Accepted** 6-D-clause 一次性 promote。Console HTTPAdapter v1.0
conformance 从 18/22 提升到 22/22 (100%)。**ContextForge v0.4-v0.7 ship 全
22 Console contract v1 endpoint**; Console UI HTTPAdapter 端到端调用代码
已 cross-repo ship — 双方握手成功 standardized signal landed.

### 主要改进

- **task-14.1 Rust SoT** (PR #89):
  - `core/migrations/0014_eval_runs.sql` (10 columns + 3 indexes + status CHECK)
  - `core/src/eval/store.rs` `SqliteEvalStore` (5 methods: create / get /
    update_metrics / update_case_results / mark_finished) + 7 unit tests
  - `core/src/eval/runner.rs` `EvalRunner` stub (real triggering Go side per task-14.2)
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` add-only
    `EvalService` 3 RPC + 5 messages (CaseResult / EvalRun / CreateEvalRunRequest /
    GetEvalRunRequest / UpdateEvalRunProgressRequest+Response)
  - `core/src/data_plane/eval.rs` `EvalServer` impl 3 RPC + 3 unit tests;
    JSON roundtrip verified (HashMap<String,f64> + Vec<CaseResult>)
  - `core/src/data_plane/mod.rs` `DataPlaneStores` 加 Option<eval>; `with_eval()`
    构造函数; `full()` takes 8 params; `register_services` + `server_with_services`
    都加 6th EvalServiceServer
  - `core/src/server.rs` `serve_full` 实例化 SqliteEvalStore 真接到 daemon
  - 2 integration tests via tonic client + EvalServiceClient
- **task-14.2 Go REST + runEvalAsync goroutine** (PR #90):
  - `internal/consoleapi/types.go` `EvalClient` interface (Create/Get/UpdateProgress)
    + `Deps.Eval` field
  - `internal/consoleapi/router.go` 2 new routes (non-destructive — no confirm gate)
  - `internal/consoleapi/handlers.go` `handleCreateEvalRun` (spawn goroutine + 200 + running)
    + `handleGetEvalRun` (200 / 404)
  - `internal/consoleapi/eval_runner.go` `runEvalAsync` goroutine:
    - 5min context timeout
    - Light-weight recall harness using `BuiltinGoldenQuestions` + mock pass-all
    - Computes `recall@5` / `recall@10` / `precision@5` metrics
    - Builds `case_results` array with `case_id` / `query` / `expected_chunks` /
      `actual_chunks` / `score` / `passed`
    - Defer-recover panic → status=failed + error_message="panic: ..."
    - Calls `deps.Eval.UpdateProgress(...)` to reverse-update Rust store on terminal
  - `internal/consoleapi/memstore.go` `MemEvalStore` (in-memory) + 2s timer
    auto-advance to succeeded with mock metrics (`recall@5: 0.7` 等)
  - `internal/consoleapi/grpcclient/grpcclient.go` `evalClient` 3 method wrappers
    + `protoToEvalRun` helper; `Client.Eval()` accessor; Create generates
    `eval-{nanos}` id Go-side per task-14.1 contract
  - `internal/cli/console_api_serve.go` buildDeps wires Eval in both inmem +
    gRPC modes; degradedDeps adds Eval
  - e2e_grpc Step 9e: real Rust daemon EvalService end-to-end PASS
- **scripts/console_smoke.sh v5** (PR #90):
  - Header v4 → v5; subtitle "Phase 14 console-22-endpoint complete"
  - 18 → 20 endpoint flow; renumber `[1/20]..[20/20]`
  - New Step 19/20: POST /v1/eval-runs → 200 + status=running
  - New Step 20/20: poll GET /v1/eval-runs/<id> 30s for terminal + verify metrics
    contains `recall@5` + 404 on unknown id
  - REAL mode: `CONSOLE_REAL_SMOKE_EXIT=0` 20/20 PASS (eval terminal at attempt 1!)
- **治理 / spec 同步** (PR #91):
  - Phase 14 spec / adapter §Phase 14 / task-14.{1,2} 全 `Status: Done`
  - **ADR-017 Status: Proposed → Accepted** (one-shot promotion, 6 D-clauses
    spanning v0.5/v0.6/v0.7 3 phase)
  - ADR-014 D1 mapping 表 / D2 lint 0 violation / D3 verified-by

### ADR-017 D-clauses (all landed by v0.7.0)

| D | Clause | Where shipped |
|---|---|---|
| D1 | 22-endpoint roadmap (Wave 1+2+3+4) | task-12.{1,2,3} + task-13.{1,2} + task-14.{1,2} |
| D2 | X-Confirm OR ?confirm=true → 412 | `confirmMiddleware` on PATCH config + memory deprecate + soft-delete |
| D3 | cancel 200 → 204 | handlers.go handleCancelJob StatusNoContent |
| D4 | Long-poll v1.0 lock (no SSE) | retained from v0.4 task-11.4 |
| D5 | RFC3339Nano kept | Go time.Time JSON unchanged |
| D6 | ADR-016 sub | Rust SoT + Go thin proxy preserved across all 13 new endpoints |
| D7 | ADR-014 cross-validation gate 3rd/4th/5th activation | Phase 12+13+14 closeout PRs each shipped D1 mapping + D2 lint verified |

### Trade-offs / Conscious limitations

- **Light-weight recall harness in runEvalAsync** [SPEC-DEFER:phase-future.real-recall-via-retriever]:
  v0.7 ship 用 BuiltinGoldenQuestions + mock pass-all 计算 metrics；future v1.x
  接 retriever-backed recall (RetrievalResult dispatch + EvaluateQuestion)
- **5min ctx timeout** in runEvalAsync (大 dataset 可能超时；future ?timeout query param)
- **Eval orphan reaper** not implemented [SPEC-DEFER:phase-15.eval-orphan-reaper]:
  console-api-serve crash 时 in-flight eval 状态卡 running；future 加 Rust 侧
  orphan reaper 扫描 status=running 超时 → mark failed
- **Eval cancel REST** 不实施 [SPEC-DEFER:console-eval-cancel] (Console 22 endpoint contract 不含)
- **Pin state not in contractv1.MemoryItem** (carried from v0.6)

### Migration notes (v0.6.0 → v0.7.0)

- **daemon 重启后 eval_runs 表自动创建** (migration 0014 IF NOT EXISTS 幂等);
  既有 v0.6 data_dir 兼容
- **新 2 endpoint** (POST /v1/eval-runs + GET /v1/eval-runs/{id}): client 按 OpenAPI/contractv1 v1 spec 调用
- contractv1.go 字段集合不变 (ADR-015 D5)
- 新 proto RPC + message add-only (ADR-013 D2)

### Tests (Phase 14 全程)

- **Rust**: 94 lib (含 10 new task-14.1: 7 store + 3 server) + 2 eval_integration
  + 既有 phase 1-13 测试不退化 (含 3 memory_integration / 5 indexjob_real /
  4 search_real / 5 data_plane_integration 等)
- **Go**: 43 packages PASS (含 e2e_grpc Step 9e 真接 Rust daemon eval-runs +
  既有 task-12.x/13.x 不退化)
- **smoke**: `bash scripts/console_smoke.sh` REAL mode 20/20 PASS;
  eval terminal at attempt 1: status=succeeded; metrics contains recall@5 ✅
- **conformance**: v0.4-v0.6 既有 endpoints 不退化

### Console (cross-repo) sync state

- ContextForge-Console contractv1.go (Workspace + IndexJob + SourceChunk +
  Search + Memory + EvalRun + CaseResult + ObservabilityEvent 等 全套 22-endpoint
  types) cross-repo 已 ship (v0.3 锁定不动)
- Console UI HTTPAdapter v1.0 端到端 22-endpoint 调用代码已 cross-repo ship
- ContextForge v0.7 ship 后 Console UI 可切到 production HTTPAdapter mode
  (关闭 MockAdapter)

### Verification commands

```bash
cargo test -p contextforge-core   # expect all PASS (94 lib + integration tests)
go test ./...                     # expect 43 packages PASS
bash scripts/console_smoke.sh     # expects CONSOLE_REAL_SMOKE_EXIT=0 20/20
RELEASE_SMOKE_CONSOLE=1 bash scripts/release_smoke.sh   # PHASE_RELEASE_SMOKE_EXIT=0
```

---

## v0.6.0 (2026-05-24)

### 摘要

ContextForge v0.6.0 完成 **Phase 13 memory-rest-surface** 收口：ADR-017 D1
Wave 3 共 5 个 memory REST endpoint 落地，把 Console HTTPAdapter conformance
从 13/22 提升到 18/22（82% coverage）。新增 SQLite 表 + `MemoryService` 5 gRPC
RPC + 4 个 AuditOperation 变体 + Go REST 5 handler。ADR-014 cross-validation
gate **第四次完整激活** 跨 4 phase 验证制度稳定性。

### 主要改进

- **task-13.1 Rust SoT** (PR #84):
  - `core/migrations/0013_memory_items.sql` (10 columns + 3 indexes + status CHECK constraint)
  - `core/src/memory/store.rs` `SqliteMemoryStore` (5 methods + 9 unit tests)
  - `proto/contextforge/console_data_plane/v1/console_data_plane.proto` add-only
    `MemoryItem` + 5 request/response messages + `MemoryService` 5 RPC
  - `core/src/data_plane/memory.rs` `MemoryServer` impl (5 RPC + 5 unit tests)
  - `core/src/memoryops/audit.rs` `AuditOperation` 加 4 variants
    (MemoryPin / MemoryUnpin / MemoryDeprecate / MemorySoftDelete)
  - Pin / Deprecate / SoftDelete 各 emit 一条 audit event
  - `core/src/data_plane/mod.rs` `DataPlaneStores` 加 Option<memory> + Option<audit>;
    新 `with_memory()` + `full()` 构造函数; `register_services` 加 5th MemoryServiceServer
  - `core/src/server.rs` `serve_full` 实例化 SqliteMemoryStore + AuditSink 真接到 daemon
  - 3 integration tests via tonic client + MemoryServiceClient
- **task-13.2 Go REST** (PR #85):
  - `internal/consoleapi/types.go` `MemoryClient` interface + `MemoryListFilter` + `Deps.Memory`
  - `internal/consoleapi/router.go` 5 new routes; deprecate + soft-delete
    confirmMiddleware-gated (ADR-017 D2 OR-semantics)
  - `internal/consoleapi/handlers.go` 5 new handlers (Pin/Deprecate/SoftDelete
    each return 204 No Content); `deps.Memory == nil → 503` graceful degrade
  - `internal/consoleapi/memstore.go` `MemMemoryStore` + `SeedFixtures()` (5 hard-coded)
    for `CONSOLE_API_FALLBACK_INMEM=1` mode
  - `internal/consoleapi/grpcclient/grpcclient.go` `memoryClient` 5 wrappers +
    `protoToMemoryItem` helper; `Client.Memory()` accessor
  - `internal/cli/console_api_serve.go` `buildDeps` wires Memory in both modes;
    `degradedDeps()` adds `degradedMemory{}`
  - 7 new router_test + e2e_grpc Step 9d (real Rust daemon 404/412 invariants)
- **scripts/console_smoke.sh v4** (PR #85):
  - Header v3 → v4; subtitle "Phase 13 memory-rest-surface"
  - 13 → 18 endpoint flow; renumber [1/18]..[18/18]
  - 新 Step 13/18: sqlite3 seed (gracefully skips if sqlite3 unavailable)
  - 新 Step 14-18/18: memory list / get / pin 204 / deprecate 412+204 / soft-delete 412+204
  - REAL mode: `CONSOLE_REAL_SMOKE_EXIT=0` 18/18 PASS
- **test/fixtures/memory-seed/seed.sql** (新增): 5 rows + agent_scope 分布
- **治理 / spec 同步** (PR #86):
  - Phase 13 spec / adapter §Phase 13 / task-13.{1,2} 全 `Status: Done`
  - ADR-017 Status: Proposed (full Accepted 推到 Phase 14 closeout 一次性)
  - ADR-014 D1 mapping 表 / D2 lint 0 violation

### Trade-offs / Conscious limitations

- **is_pinned 列设计**：选 `is_pinned bool` 列 + `status` 三态独立；pin state
  存在 Rust SqliteMemoryStore 但**不在 contractv1.MemoryItem 暴露** (ADR-015 D5
  字段锁定)；Console UI 显示 Pin 按钮但 pinned visual indicator 需通过
  future contractv1 amendment 或 inferred via 单独 Get-by-id 调用
- **importer 写入 memory_items 路径** `[SPEC-DEFER:phase-15.import-to-memory-items]`
  留 v0.6.x；v0.6.0 ship 后 Console UI 看 0 条 memory items（fresh install）→
  Console UI 端 graceful degrade
- **memory hard delete** 不实施（Console PRD 显式只支持 soft-delete）
- **POST /unpin separate endpoint** 不实施（Console v1.0 contract 只有 `/pin`；
  `Pin(id, false)` API 端已支持 unpin 语义；如 Console 需要 separate route →
  cross-repo amendment `[SPEC-DEFER:console-memory-unpin]`)

### Migration notes (v0.5.0 → v0.6.0)

- **daemon 重启后 memory_items 表自动创建**（schema migration 0013_memory_items.sql
  在 SqliteMemoryStore.open 内 execute_batch IF NOT EXISTS）；v0.5 用户重启
  daemon 后 `<data_dir>/memory.db` 自动 ready
- **新 5 endpoint**（Memory CRUD + Pin/Deprecate/SoftDelete）— 无 v0.5 baseline;
  client 按 OpenAPI/contractv1 v1 spec 调用
- **destructive endpoints** (deprecate + soft-delete) 需要 X-Confirm: yes header
  或 ?confirm=true query；Console BFF 自动注入；ops curl 用户须显式加
- contractv1.go 字段集合不变 (ADR-015 D5)
- 新 proto RPC + message add-only (ADR-013 D2)

### Tests (Phase 13 全程)

- **Rust**: 84 lib tests (含 14 new memory: 9 store + 5 server) + 3 memory_integration
  + 既有 phase 1-12 测试不退化 = 17 test groups all PASS
- **Go**: 43 packages PASS (含 7 new memory router_test + e2e_grpc Step 9d
  real Rust daemon + grpcclient_test 不退化)
- **conformance**: v0.4/v0.5 既有 endpoints 不退化
- **smoke**: `bash scripts/console_smoke.sh` REAL mode 18/18 PASS

### Verification commands

```bash
cargo test -p contextforge-core   # expect all PASS (17 test groups)
go test ./...                     # expect 43 packages PASS
bash scripts/console_smoke.sh     # expects CONSOLE_REAL_SMOKE_EXIT=0
RELEASE_SMOKE_CONSOLE=1 bash scripts/release_smoke.sh   # PHASE_RELEASE_SMOKE_EXIT=0
```

---

## v0.5.0 (2026-05-24)

### 摘要

ContextForge v0.5.0 完成 **Phase 12 console-contract-completion** 收口：把
ADR-017 D1 Wave 1（quick win 4 个 endpoint）+ Wave 2（mid scope 2 个 endpoint）
共 5 个新 endpoint + 1 个 behavior 切换（cancel 200→204）一次性 ship，把 Console
HTTPAdapter conformance 从 9/22 提升到 13/22（route inventory 9→14 含 PATCH
config）。ADR-014 cross-validation gate **第三次完整激活** 验证制度稳定性。

### 主要改进

- **task-12.1 Wave 1 quick win** (PR #78):
  - `PATCH /v1/workspaces/{id}/config` 走 gRPC `WorkspaceService.UpdateConfig`
    (proto add-only `UpdateWorkspaceConfigRequest`)；body `{allowlist, denylist}`
    覆盖式更新；SqliteWorkspaceStore.update_config 真持久化 + updated_at_unix 推进
  - `GET /v1/index-jobs?status=active` 走 gRPC `JobService.List` + status_filter
    (proto add-only `ListJobsRequest{status_filter, workspace_id}` + `ListJobsResponse`)；
    Rust 端 `list_active()` 包装 + Go 端 missing-filter → 400
  - `POST /v1/index-jobs/{id}/cancel` 返 **204 No Content** (ADR-017 D3)
  - `confirmMiddleware` 服务端 X-Confirm 兜底 (ADR-017 D2): 破坏性 endpoint
    必须 `X-Confirm: yes` header **或** `?confirm=true` query (OR-semantics);
    缺失 → 412 PRECONDITION_FAILED + ErrorBody `{code:"PRECONDITION_FAILED",...}`
- **task-12.2 source-chunk-by-id** (PR #79):
  - `GET /v1/source-chunks/{id}` 走 gRPC `SearchService.GetSourceChunk` (proto
    add-only `GetSourceChunkRequest{chunk_id, workspace_id(optional)}`)
  - Rust impl 复用既存 `Retriever::get_chunk(chunk_id)` (task-6.2 ship 的 SQL
    fast-path)；workspace_id 缺失时枚举 SqliteWorkspaceStore.list() 真试每个
    workspace 寻 chunk (chunk_id 全局唯一 SqliteChunkStore 假设
    `[SPEC-DEFER:phase-15.multi-workspace-strict]`)
  - chunk_offset_start/end = 0 占位 `[SPEC-DEFER:chunk-byte-offsets]` (current
    schema 不存 byte offsets; Console UI 用 line_start/end)
- **task-12.3 search-trace-by-query-id** (PR #80):
  - `GET /v1/search/{query_id}/trace` 走 gRPC `SearchService.GetSearchTrace`
    (proto add-only `GetSearchTraceRequest{query_id}`)
  - 自研 `TraceStore { HashMap, VecDeque, cap=1000 }` ~30 行 LRU/FIFO eviction
    (避免 `lru` crate R7 风险)；`std::sync::Mutex` 包裹 read-heavy 场景足够
  - `SearchService.Query` 内统一生成 `qry-{nanos}` 唯一 query_id 字段
    (task-11.4 既存返 empty query_id 字段被替换)；每次 Query 自动 put trace
    到 trace_store
- **scripts/console_smoke.sh v3** (PR #80):
  - Header bump v2 → v3；subtitle "Phase 12 console-contract-completion"
  - 9 → 13 endpoint flow；renumber [1/13]..[13/13]
  - 新 Step 9/13: task-12.1 PATCH workspace/config (412→200×2)
  - 新 Step 10/13: task-12.1 GET active jobs + missing-status 400
  - 新 Step 11/13: task-12.2 GET source-chunks/{id} (uses chunk_id from search)
  - 新 Step 12/13: task-12.3 GET search/{query_id}/trace + unknown 404
  - REAL mode 真接 daemon: `CONSOLE_REAL_SMOKE_EXIT=0` 13/13 PASS
- **治理 / spec 同步** (PR #81):
  - Phase 12 spec / adapter §Phase 12 / task-12.{1,2,3} 全 `Status: Done`
  - ADR-017 Status: Proposed (full Accepted 推到 Phase 14 closeout 一次性)
  - ADR-014 D1 mapping 表 / D2 lint 0 violation / D3 verified-by 显式

### Trade-offs / Conscious limitations

- **task-12.2 §10**: chunk_offset_start/end = 0 占位
  `[SPEC-DEFER:chunk-byte-offsets]` — current SqliteChunkStore schema 不存
  byte offsets; Console UI 用 line_start/end 显示足够；future schema migration
  填充字节偏移留 v0.5.x
- **task-12.2 §10**: workspace_id 全局唯一假设
  `[SPEC-DEFER:phase-15.multi-workspace-strict]` — multi-workspace strict
  isolation 留 v1.x
- **task-12.3 §10**: trace_store 重启即丢 `[SPEC-DEFER:task-future.search-trace-sqlite-persistence]`
  — SQLite 持久化跨 daemon 重启留 v0.5.x；Console UI 端 graceful degrade 承接
- **task-12.3 §10**: trace_store cap=1000 硬编码 — env var 参数化留 v0.5.x

### Migration notes (v0.4.0 → v0.5.0)

- **`POST /v1/index-jobs/{id}/cancel` 改 204 No Content** — Console HTTPAdapter
  v1.0 已 200/204 双 check (cross-repo 验证)，应不出现 break；如发现 strict
  200 only 的旧 client → rollback path 是把 handlers.go handleCancelJob 回退
  到 `StatusOK`
- **PATCH /v1/workspaces/{id}/config + 新破坏性 endpoint** 现在强制
  X-Confirm/?confirm=true — Console BFF 自动注入；ops curl 用户须显式加
- **新 4 endpoint (PATCH config + active filter + source-chunks + trace)**
  无 v0.4 baseline; client 端按 OpenAPI/contractv1 v1 spec 调用
- contractv1.go 字段集合不变 (ADR-015 D5 字段镜像约束沿用)
- 新 RPC 全 proto add-only (ADR-013 D2)，既有 RPC 字段编号不动

### Tests (Phase 12 全程)

- **Rust**: 70 lib tests (含 4 new task-12.1 workspace UpdateConfig/job List + 3
  new task-12.2 GetSourceChunk + 4 new task-12.3 GetSearchTrace+TraceStore +
  既有 phase 1-11 测试不退化)
- **Go**: 43 packages PASS (含 task-12.1 7 new router_test + 4 new grpcclient_test
  + task-12.2 2 new + task-12.3 1 new + degraded fallback impls + e2e_grpc with
  real Rust daemon Step 8a/8b/9/9b/9c PASS)
- **conformance**: `test/conformance/console_contractv1_test.go` v0.4 9 endpoint
  不退化
- **smoke**: `bash scripts/console_smoke.sh` REAL mode 13/13 endpoint PASS
  with `CONSOLE_REAL_SMOKE_EXIT=0` final marker

### Verification commands

```bash
# Rust workspace
cargo test -p contextforge-core --lib   # expect 70/70 PASS

# Go full
go test ./...   # expect 43 packages PASS

# Phase 12 console real smoke v3 (default REAL mode)
bash scripts/console_smoke.sh   # expects CONSOLE_REAL_SMOKE_EXIT=0

# Release smoke (§5 enables console smoke via env)
RELEASE_SMOKE_CONSOLE=1 bash scripts/release_smoke.sh   # PHASE_RELEASE_SMOKE_EXIT=0

# ADR-014 D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master   # 0 violation
```

---

## v0.4.0 (2026-05-25)

### 摘要

ContextForge v0.4.0 完成 **Phase 11 console-real-data-plane** 收口：把 Phase 10
task-10.4 §10 显式记录的两个 Trade-off (`[SPEC-DEFER:task-future.cross-process-
sqlite-sharing]` 与 JobRunner 不真索引) 一次性 resolve。通过新 ADR-016
**cross-process-rust-go-via-grpc-bridge** 实施 4 个新 Rust gRPC service
(Workspace / Job / Search / Events)，Go console-api-serve 重构为 **thin REST→gRPC
translator**；console UI 期望的 Workspace 持久化跨 daemon 重启 + IndexJob 真触发
Rust 索引 + Search 真返回 indexed chunks + Events 真接 JobRunner progress 全部
端到端落地。ADR-014 cross-validation gate **第二次完整激活** 验制度稳定性。

### 主要改进

- **ADR-016 cross-process Rust ↔ Go gRPC bridge** (Proposed → Accepted): 6 个 D
  条款落地。D1 Rust 持 SoT (Go 不写 SQLite); D2 4 gRPC service in
  `proto/contextforge/console_data_plane/v1/console_data_plane.proto` (snake_case
  1:1 镜像 Go contractv1 JSON tag); D3 Go console-api-serve thin proxy
  (`internal/consoleapi/grpcclient/`); D4 in-memory MemStore 降级为 env-gated
  fallback (`CONSOLE_API_FALLBACK_INMEM=1`); D5 schema 单 owner = Rust; D6 沿用
  ADR-014 cross-validation gate.
- **Rust data plane gRPC services** (`core/src/data_plane/`): 4 tonic service
  trait impls (`WorkspaceServer` / `JobServer` / `SearchServer` / `EventsServer`)
  + `register_services` helper + `serve_full(addr, svc, data_dir)` 把 Phase 9
  ContextService + Phase 11 4 service 注册到同一 tonic Server.
- **Real JobRunner wiring** (task-11.3): `IndexSessionBackend` impl
  `IndexerBackend` 包 `IndexSession::index_path_cancellable` (add-only API
  extension; cancel_token at file boundaries); `JobService.Enqueue` 真
  `tokio::spawn(JobRunner.run_one)`; `orphan_reaper` 在 `serve_full` 启动早期
  清理上一 boot 留下的 running 行 (mark failed + error_message="job lost: daemon
  restart"); JobRunner.run_one 改 per-file cancel-check (heartbeat 仍 throttled
  100files/5s) 让小 fixture 也能在 5s 内观察 cancel.
- **Real SearchService + EventBus** (task-11.4): `SearchService.Query` 真接
  `core/src/retriever/Retriever::search` (Tantivy + SQLite chunks);
  `RetrievalTrace.retrieved_chunks` 真填 (chunk_id + score + source_file +
  `chunk_text_preview` ≤200 chars via `utf8_safe_truncate` UTF-8 boundary safe);
  `EventBus` (broadcast::Sender 容量 1000) 接 `EventsService.Subscribe` server
  stream; `JobRunner` progress callback emit `indexing.progress` /
  `indexing.cancelled` / `indexing.error` events.
- **Go grpcclient** (`internal/consoleapi/grpcclient/`): `Client.Workspace/Job/
  Search/Events()` 4 wrapper impl `consoleapi.{Workspace,Job,Search,Events}Client`;
  `mapGrpcErr` maps gRPC status → consoleapi sentinel (NotFound → ErrNotFound /
  FailedPrecondition → ErrJobTerminal / Unavailable → ErrDataPlaneUnavailable).
- **console-api-serve 新 flags**: `--grpc-addr 127.0.0.1:50551` (default; alias
  to Rust DEFAULT_LISTEN) + `--fallback-inmem` (alias env
  `CONSOLE_API_FALLBACK_INMEM=1`). `BackendKind`-aware `/v1/health`: grpc → 200
  healthy; inmem-fallback → 200 degraded + ErrorReason; degraded → 503 + missing=
  ["data_plane"].
- **Long-poll wait/limit** (`/v1/observability/events`): `?wait=<duration>`
  (default 30s, clamped [1s, 60s]) + `?limit=<int>` (default 100, clamped [1, 500])
  query params; grpcclient.eventsClient.Recent uses ctx 30s timeout to drive
  long-poll behaviour at the gRPC layer.
- **scripts/console_smoke.sh v2** (REAL mode default): spawns both contextforge-
  core daemon and console-api-serve, drives the 9 endpoint flow + real index
  job against `test/fixtures/index-job-real/` (5 markdown files). Final marker:
  `CONSOLE_REAL_SMOKE_EXIT=0`. v0.3 inmem mode retained as `LOCAL_ONLY=1`.
- **release_smoke.sh §5 updated** for REAL mode; final
  `phase11_console_real=ok` marker.
- **ADR-014 D1-D5 second activation pass**: D1 mapping (in closeout PR body);
  D2 lint `bash scripts/spec_drift_lint.sh --touched <base>` 0 violation (with
  proper [SPEC-OWNER]/[SPEC-DEFER] tags throughout phase-11 + 4 task spec);
  D3 each phase §6 AC verified by explicit owner; D4 main-agent self-merge
  via /goal autonomy; D5 historical Phase 1-10 unchanged.
- **治理 / spec 同步**: ADR-016 Proposed → Accepted; Phase 11 / Task 11.1-11.4
  全 Done; PRD §Implementation Phases Phase 11 + §Open Questions O14 partially
  resolved by ADR-016 (business plane wiring; endpoint expansion [SPEC-DEFER:
  console-endpoint-expansion]); adapter §Phase / §Tasks / §ADRs / §BDD synced.

### Trade-offs / Conscious limitations

- **task-11.2 §10 T2** `--grpc-addr` default `127.0.0.1:50551` (与 Rust
  `DEFAULT_LISTEN` 对齐); playbook 文档曾写 `:48180` 是 ADR-013 概念预留, 实施
  按 Rust 既有 default 落地 (无 spec drift — gRPC 字段集合才是契约, 端口可配).
- **task-11.3 §10 T1** cancel co-operative only (file-boundary granularity);
  hard kill cancel [SPEC-DEFER:task-future.hard-cancel].
- **task-11.4 §10 T1** EventBus volatile broadcast (daemon 重启即丢历史
  events); persistent event ring buffer [SPEC-DEFER:task-future.event-persistence].
- **task-11.2 §10 T1** v0.3 in-memory MemStore retained as env-gated fallback
  (not deleted) for conformance test backward compat + degraded mode demo.
- Multi-instance daemon leader election [SPEC-DEFER:task-future.multi-daemon-leader-election].

### Migration notes (v0.3.0 → v0.4.0)

- `console-api-serve` 默认 backend 从 in-memory MemStore 切到 gRPC. v0.3 用户
  若需 inmem 行为, 设 `CONSOLE_API_FALLBACK_INMEM=1` (CLI flag `--fallback-inmem`).
- v0.3 console_smoke.sh 默认 local mode → v0.4 默认 REAL mode (需 cargo build
  Rust binary). 兼容 v0.3 行为: `LOCAL_ONLY=1 bash scripts/console_smoke.sh`.
- Console contract v1 字段集合不变 (ADR-015 D5 字段镜像约束沿用); Console UI
  端无任何改动 — v0.4 仅 ContextForge 单仓内业务面真接通.
- 新 deploy 形态: `contextforge-core <listen> <data_dir> &` 后 `contextforge
  console-api-serve --addr ... --grpc-addr ...`. 双进程 deploy 可用 systemd /
  docker compose / 脚本管理.

### Tests (Phase 11 全程)

- Rust: 60 lib + 5 indexjob_real_runner + 4 search_real_retriever + 5
  data_plane_integration + 既有 phase 1-10 测试不退化.
- Go: 9 grpcclient + 6 cli + 1 e2e gRPC backed E2E (TestRESTEndpoints_E2E_
  GrpcBacked spawns Rust daemon + 9 endpoint flow + workspace 持久化跨 daemon
  restart) + 既有 consoleapi v0.3 + conformance test 不退化.

### Verification commands

```bash
# Rust full workspace
cargo test --workspace

# Go full
go test ./...

# Phase 11 console real smoke (default REAL mode)
bash scripts/console_smoke.sh   # expects CONSOLE_REAL_SMOKE_EXIT=0

# Release smoke (§5 enables console smoke via env)
RELEASE_SMOKE_CONSOLE=1 bash scripts/release_smoke.sh   # PHASE_RELEASE_SMOKE_EXIT=0

# ADR-014 D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master   # 0 violation
```

---

## v0.3.0 (2026-05-24)

### 摘要

ContextForge v0.3.0 完成 **Phase 10 console-contract-v1** 收口：实现 ContextForge ↔
**ContextForge-Console** v1.0 (已 ship) **Contract v1 兼容层** —— 17 个 Go 类型
1:1 镜像 Console `contractv1.go` + Rust workspace/jobs 资源模型 + 9 个对齐 Console
HTTPAdapter 期望的 REST 端点 + cross-repo conformance test + docker compose 集成
smoke。同时 ADR-014 cross-validation gate (D1 mapping / D2 lint / D3 verified-by /
D4 自治补丁 / D5 历史不溯改) 首次完整激活。

### 主要改进

- **internal/contractv1/ Go 类型镜像**：1:1 复刻 Console
  `console-api/internal/coreadapter/contractv1/contractv1.go` 17 个类型 +
  `ContractVersion = "v1"` 常量 + `FieldAvailability` helper；env
  `CONSOLE_REPO=$path` 设时 reflect 反射跑 Console parity 校验。
- **Rust Workspace + IndexJob 资源**：`core/src/workspace/` (CRUD + 1:1
  collection 映射) + `core/src/jobs/` (异步 lifecycle queued/running/
  succeeded/failed/cancelled + heartbeat + co-operative cancel) +
  SQLite migration `0010_workspaces.sql` + `0011_index_jobs.sql`。
- **9 Console Contract v1 REST endpoint** (新增 `internal/consoleapi/`)：
  `GET /v1/health` + `POST/GET/GET /v1/workspaces*` +
  `POST/GET/POST /v1/index-jobs*[/cancel]` + `POST /v1/search` (nested
  `{result, trace}`) + `GET /v1/observability/events` (long-poll, 非 SSE)；
  路径 / shape / 错误码 严格对齐 Console HTTPAdapter；bearer auth +
  OpenAPI 3.0 yaml (`docs/consoleapi/openapi.yaml`)。
- **新 CLI 子命令** `contextforge console-api-serve --addr ...` 启动
  consoleapi router (in-memory MemStore v0.3；cross-process SQLite 共享留
  v0.4 task-future)。
- **Cross-repo conformance test** (`test/conformance/`)：env-based skip
  机制 + Console-style 9 endpoint flow + FieldAvailability.Complete() +
  Console sentinel error mapping (404→ErrNotFound / 409→ErrConflict)。
- **Docker compose stack**：`deploy/console-stack.yml` 含 5 service
  (postgres + redis + contextforge + console-api + console-web)；profile
  `console` gates the optional Console UI services。
- **多阶段 `Dockerfile`**：rust:1.82 + golang:1.22 → debian:bookworm-slim，
  CMD `contextforge console-api-serve --addr 0.0.0.0:48181`。
- **新 smoke**：`scripts/console_smoke.sh` 默认本地 mode (build + spawn
  + 9 endpoint curl); env DOCKER_SMOKE=1 触发 docker compose 模式。
- **release_smoke.sh 第 5 段**：env `RELEASE_SMOKE_CONSOLE=1` 启用 (默认 SKIP
  避 CI 强依赖 docker)。
- **ADR-014 cross-validation gate 全程激活**：D2 lint `scripts/spec_drift_lint.sh
  --touched origin/master` 0 violation；D3 每条 phase §6 AC + task §6 AC 含
  `verified by ...` 显式 owner；D1 closeout PR body mapping 表。
- **治理 / spec 同步**：ADR-015 Proposed → Accepted；Phase 10 / Task
  10.1-10.6 全 Done；PRD §Implementation Phases Phase 10 + §Open Questions
  O12 (Resolved by ADR-014) + O13 (新增 Console 集成)；adapter §Phase /
  Task / ADR / BDD 索引同步。

### v0.3 trade-offs (§Implementation Notes)

- **Cross-process SQLite 共享 Rust ↔ Go (task-10.4 §10 #1)**：v0.3 Go 端 REST
  用 in-memory MemStore；Rust 端 workspace/jobs 用 SQLite。两者各自独立，
  Console UI POST 创建的 workspace 不进 Rust JobRunner。**Why**：保守
  优先级 backward compat > spec literal > minimal change；避新增 sqlite Go
  driver (mattn/go-sqlite3 CGO 或 modernc/sqlite 纯 Go) — playbook v0.3 不
  预期新 dep。**v0.4 follow-up**：[SPEC-DEFER:task-future.cross-process-sqlite-sharing]。
- **时间字段 Unix epoch i64 (workspace/jobs)**：避新增 chrono dep；Go REST
  序列化时 `time.Unix(sec, 0).UTC()` 转 RFC3339 喂 Console wire。
- **Console UI integration smoke 在 docker compose 默认 SKIP**：Console v1.0
  docker image 公网未发布；console_smoke.sh 默认 local mode (ContextForge
  daemon only)；DOCKER_SMOKE=1 + CONSOLE_API_IMAGE / CONSOLE_WEB_IMAGE 三
  env 同时设才跑 full Console UI 集成。

### 限制（继承 v0.1 + v0.2 + Phase 10 新增）

- v0.3 Console 集成是 spec/REST 契约层 conformance；Console UI 真返回
  workspace 列表（非 Mock）已通过 console_smoke.sh 在 ContextForge daemon
  端验证。**Console docker image 公网拉取 + UI 真渲染**留 v0.4 (依赖 Console
  仓库发布 image)。
- v0.3 in-memory MemStore 不持久化 — daemon 重启后数据丢失。Cross-process
  SQLite 共享 / 持久化 IndexJob 留 v0.4。
- 其它 10+ Console endpoint (`/v1/memory*` / `/v1/eval-runs*` /
  `/v1/source-chunks/:id` / `/v1/search/:query_id/trace` /
  `/v1/workspaces/:id/config` PATCH) — Console Mock Adapter 覆盖到 v0.4。

### Migration notes (from v0.2.0)

- `internal/cli` 新增 `console-api-serve` 子命令 — 现有子命令行为不变。
- `internal/daemon/rest.go` v0.2 既有 5 endpoint (`/v1/search`, `/v1/chunks/{id}`,
  `/v1/collections`, `/v1/import`, `/v1/eval/run`) 不变；Console Contract v1
  9 endpoint 在独立 `internal/consoleapi/` 包内，通过 `console-api-serve` 子
  命令暴露 (不与 `serve` 子命令的 daemon REST 冲突)。
- `scripts/release_smoke.sh` 增第 5 段 (env RELEASE_SMOKE_CONSOLE=1 启用)；
  `PHASE_RELEASE_SMOKE_EXIT` 退出码兼容 v0.2。

---

## v0.2.0 (2026-05-24)

### 摘要

ContextForge v0.2.0 完成 Phase 9 cli-pipeline 收口：补齐 v0.1 ship 后实测的
CLI 数据通路 spec drift —— `contextforge index` / `contextforge import` 在
v0.1 是 stub，v0.2 通过 ADR-013 add-only 扩 `rpc Index` server-stream 真接通
Go↔Rust gRPC + 真扫描 + 真写 SQLite/Tantivy。README Quick Start 现可复制粘贴
跑通。

### 主要改进

- **CLI 数据通路打通**：`proto/contextforge/v1/service.proto` 新增 `rpc Index(IndexRequest) returns (stream IndexProgress)`；Rust `CoreService::index`
  wire `IndexSession::index_path_with_progress` 按文件粒度上报进度；Go
  `Daemon.Index` + `internal/cli/index.go` 真实 stream consume + human/JSONL render。
- **`contextforge import` 三子命令真实**：hermes / openclaw / agent-rules 现产
  YAML-frontmatter Markdown 到 `<data-dir>/imports/<source>/`；`contextforge index --source <output_dir>` 把它灌入。
- **README Quick Start 可复制粘贴**：新增 `examples/quickstart/` fixture +
  `scripts/quickstart_smoke.sh` 一键 7 步端到端；README 重写 manual steps + 注释 flag 顺序陷阱。
- **Release smoke 真端到端**：删除 `internal/release/release_test.go` 三个
  fake-evidence 测试（`TestTask83_AC2/AC4/AC5`），重写 `TestTask83_AC1` 用真
  `go build` + `cargo build`，新增 `TestPhase9ReleaseSmoke_EndToEnd` 7-step
  CLI binary 真跑；`scripts/release_smoke.sh` 加 phase 9 段 + 重命名
  `PHASE_RELEASE_SMOKE_EXIT`（去 v0.1-only PHASE8 前缀）。
- **治理 / spec 同步**：ADR-013 Proposed → Accepted；Phase 9 / Task 9.1-9.6 全
  Done；PRD §Implementation Phases Phase 9 + §Open Questions O12 同步；
  adapter §Phase 状态索引 / Task 索引 / ADR 索引 / BDD 索引同步。

### 验证证据

最终 `master` 上执行：

```bash
bash -lc 'source docs/s2v/scripts/lib/preflight.sh; source docs/s2v/scripts/lib/verify.sh; s2v_baseline_green "cmd/contextforge internal core/src core/tests"'
```

结果：`FINAL_HEAD_BASELINE_EXIT=0`。

```bash
bash scripts/release_smoke.sh
```

结果：`PHASE_RELEASE_SMOKE_EXIT=0`（4 段：go release harness / task-8 reliability/eval / Rust gRPC search smoke / phase 9 CLI e2e）。

```bash
bash scripts/quickstart_smoke.sh
```

结果：`QUICKSTART_SMOKE_EXIT=0`（7 步：build / init / import hermes / index records / index source / search / eval）。

完整证据见 [`docs/releases/v0.2.0-evidence.md`](docs/releases/v0.2.0-evidence.md)；产物清单见 [`docs/releases/v0.2.0-artifacts.md`](docs/releases/v0.2.0-artifacts.md)。

### 发布边界

- 继承 v0.1 限制：Linux x86_64 / WSL2 官方目标；macOS 应能跑（bash + cargo + go）；Windows 走 Git Bash / WSL；macOS / Windows 官方 tarball 仍延后。
- `LICENSE` 继续 all-rights-reserved（占位于明确 OSI 许可证前）。
- 真实 GitHub Release 上传、checksum / signing、CI release job 仍需外部发布流水线执行。

### v0.1.0 → v0.2.0 迁移

无 schema 变更（schema_version 仍 `0.1`，proto add-only `rpc Index` 不破坏现有 wire 兼容）。脚本端：`PHASE8_RELEASE_SMOKE_EXIT` 重命名为 `PHASE_RELEASE_SMOKE_EXIT` — 任何依赖此标记的外部 CI 步骤需相应更新。

---

## v0.1.0 (2026-05-23)

### 摘要

ContextForge v0.1.0 完成本地优先的双二进制基础闭环：Go 控制面 `contextforge` + Rust 数据面 `contextforge-core`，覆盖初始化、索引核心、检索解释、REST/MCP/export、recall eval、可靠性 guard 与 release smoke gate。

### 主要能力

- S2V 治理：ADR-012 放宽主 agent 自治决策，同时保留 R3 分支校验、R6 PR-only、worktree 隔离和合入 gate。
- Eval：`contextforge eval run` 具备 30 条内置 golden questions、Top-5/Top-10 strong hit rate、miss cases 与 latency p95 输出。
- Reliability：长任务 resume manifest、资源预算 gate、secret/export/audit safety regression guard。
- Release：新增 `internal/release` tarball contract、七步 smoke evidence、10 万 chunk P95 benchmark gate，以及 `scripts/release_smoke.sh` Phase 8 smoke 入口。
- Distribution docs：新增 `README.md`、`LICENSE`、`contextforge.example.toml` 和 ADR-007 产物清单。

### 验证

最终 `master` 上通过：

```bash
bash -lc 'source docs/s2v/scripts/lib/preflight.sh; source docs/s2v/scripts/lib/verify.sh; s2v_baseline_green "cmd/contextforge internal core/src core/tests"'
```

结果：`FINAL_HEAD_BASELINE_EXIT=0`。

最终 `master` 上通过：

```bash
bash scripts/release_smoke.sh
```

结果：`PHASE8_RELEASE_SMOKE_EXIT=0`（v0.1 版本；v0.2 已重命名为 PHASE_RELEASE_SMOKE_EXIT）。

完整证据见 `docs/releases/v0.1-evidence.md`。

### 发布边界

- 本 tag 提供 release contract gate 与产物清单；真实 GitHub Release 上传、checksum/signing 与 CI release job 仍需在发布流水线中执行。
- v0.1 官方目标平台为 Linux x86_64 / WSL2；macOS / Windows 官方 tarball 延后。
- `LICENSE` 当前为 all-rights-reserved，占位于明确开源许可证之前。
