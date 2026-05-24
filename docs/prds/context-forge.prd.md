# ContextForge · 产品需求文档（PRD）

> 本 PRD 由 `/s2v-prd` 生成，并基于后续审查意见完成修订。
>
> ⚠️ **不要手工重命名章节标题**（`/s2v-init` 按"中文｜English"双语锚点解析）。修改章节内容随时可以；改章节名会让 init 漏读字段。
>
> 解析逻辑：`## Vision` 或 `## 愿景` 任一命中即认；但 `## 产品愿景` / `## Product Vision` 会失败（不在双语模板内）。

**生成日期**：2026-05-17  
**作者**：tajiaoyezi  
**版本**：v0.1

---

## Vision｜愿景

三个月后，ContextForge 让一个同时使用 OpenClaw、Hermes、Claude Code、Cursor、Zed 的重度 AI Agent 开发者，在切换工具或新开会话时不再反复手工喂上下文，而是通过一个统一、本地优先、可解释、可评测的 Context Hub 在 3-5 分钟内恢复项目上下文。

它不替代各 Agent 自带 memory，而是位于其下的统一中枢，把分散在不同 Agent、项目、文件和记忆系统中的上下文统一管理、索引、评测、迁移与复用。**关键差异**：ContextForge 聚焦的是现有 memory provider、Agent 框架、MCP server 和向量数据库通常不会作为默认重点覆盖的中立治理层：当多 Agent 与多上下文源并存时，如何统一治理、解释、评测、迁移与复用上下文。用户因为「多 Agent 中立 + 可解释 / 可治理 / 可评测 MemoryOps」而选它，而不是只用单一 memory provider。

---

## Problem Statement｜问题陈述

**谁有这个问题**：

同时使用 OpenClaw、Hermes、Claude Code、Cursor、Zed 等多个工具的个人/独立开发者与 AI 工具链探索者；3-8 人的小型 AI 工具链 / AI 应用开发团队（维护多仓库、多 Agent 配置、多 prompt 版本、多 memory provider）；以及不愿把全部代码库、日志、内部文档上云的本地优先 / 隐私敏感开发者与小团队。痛点不是「没有 AI 工具」，而是「AI 工具太多，上下文被割裂」。

**痛点**：

1. **换 Agent 上下文丢失**：同一项目今天 Claude Code、明天 OpenClaw、后天 Hermes，每次切换或新会话需 10-30 分钟重新解释项目背景、目录、历史决策和踩坑记录，前 3-5 轮回答不稳定，容易重复犯已经解决过的错。
2. **知识分散无法统一管理**：同一条规则在 OpenClaw memory、Hermes `MEMORY.md` / `USER.md`、Claude Code 上下文、Cursor / Zed 规则文件、本地 Markdown 中重复存 3-5 份，更新不同步，开发者不知道哪个 Agent 拿的是最新上下文，导致回答互相矛盾。
3. **检索黑盒**：memory / RAG 只返回「相关内容」，不说来自哪个文件、哪几行、关键词还是向量命中、分数多少、是否过期，错误召回难排查，调参无依据。
4. **Memory 缺治理**：过期配置仍被召回，同一事实重复写入，新旧规则冲突，临时调试信息长期保存，memory 从「增强 Agent」变成「污染 Agent」。
5. **效果无法量化**：换 embedding、reranker、provider 后只能凭感觉，无召回率回归测试，一次升级可能悄悄破坏原本可用的检索效果。

**现状**（现存方案 + 失败原因）：

- **各 Agent 自带 memory**（Hermes / OpenClaw / Claude Code / Cursor / Zed）：格式、存储、更新机制各异，难迁移、难统一评测、难统一去重、过期和冲突检测；只服务单 Agent，解决不了「多 Agent 如何共享、治理、评测、迁移上下文」。
- **手工复制粘贴 prompt / README / 日志**：重复劳动、易遗漏、不结构化、无法持续更新、无法追踪引用来源，只适合临时场景。
- **普通 RAG 框架**（LangChain / LlamaIndex / 向量库方案）：偏「上传文档问答」，不天然理解多 Agent memory，缺各 Agent 适配，MemoryOps、可解释召回、召回评测、跨 Agent 迁移需大量自研。
- **MCP Server**：偏工具接入协议，能让 Agent 调工具，但不解决 memory 治理、上下文评测、跨 Agent 迁移、检索质量分析；可作 ContextForge 接入方式之一，非替代品。
- **云端 memory / 外部 provider**：对隐私敏感代码、日志、内部文档不友好，弱联网或离线不可用，长期成本不可控，本地代码库级深度索引不一定理想；可作可选增强，不应成为前提。

**为什么是现在**：

1. 多 Agent 并存已成常态，问题从「有没有 AI 助手」变成「多个 Agent 如何共享同一套可靠上下文」。
2. Agent memory 已普及但仍停留在存事实、摘要、语义检索、注入，下一阶段需要的去重、过期、冲突检测、迁移、质量评测、可解释召回，即从 Memory 进入 MemoryOps，尚不成熟。
3. 本地 embedding、reranker、代码检索、轻量模型已越来越现实，本地优先上下文基础设施具备可行性。
4. MCP 等工具 / 多 Agent 协议生态正在成熟，需要 Agent 与本地上下文之间的统一中枢。
5. 开发者已开始遇到「多 Agent 上下文碎片化」，正是切入这一早期窗口的时机。

---

## Users & Context｜用户与场景

**主要用户**：

- **多 Agent 重度个人 / 独立开发者**：一个项目里 Claude Code 写代码、Cursor / Zed 编辑、OpenClaw / Hermes 做多 Agent 调度，本地文档、日志、配置作长期上下文；有多个长期项目，每个都有自己的代码库、配置、历史问题、调试记录和 Agent memory。
- **3-8 人小型 AI 工具链 / 应用团队**：在构建 Agent、RAG、内部代码助手、自动化开发工作流；同时维护多仓库、多 Agent 配置、多 prompt 版本、多 memory provider、多模型供应商；需要知道哪个 Agent 读了什么上下文、某次任务为什么失败、换 provider 后效果是否变好、成员上下文能否复用。
- **本地优先 / 隐私敏感开发者与小团队**：不希望把全部代码、日志、内部文档上云；需要代码、文档、日志、memory 默认留本地，远程模型、Agent、云服务仅可选接入，离线或弱联网仍可用。

**v0.1 不优先服务的用户**：

- 只使用单一 Agent 的轻度用户。
- 希望开箱即用云端 memory SaaS 的用户。
- 需要企业多租户、统一权限、集中部署和审计合规的团队。
- 需要完整 GraphRAG / 知识图谱推理的用户。
- 希望 ContextForge 自动写回或接管第三方 Agent memory 的用户。

说明：v0.1 可以被小团队成员以个人本地模式各自使用，但不提供共享团队服务、统一权限和集中部署。

**次要用户 / 利益相关者**：

- **团队平台 / DevOps 负责人**：不直接用，但被影响——担心多一个需部署、监控、升级的本地服务，与已有 RAG / observability 系统重叠，个人工具演变为团队不可控依赖。
- **各 Agent / memory provider 厂商**：被影响方——ContextForge 的中立层会降低对单一 Agent memory 的锁定。对外定位需明确为「兼容 / 治理 / 评测层」，而非「替代 memory」。

**关键使用场景**：

1. **跨工具恢复项目上下文**：开发者从 Claude Code 切到 OpenClaw 接手同一项目，无需重新解释背景，执行一次 `contextforge search` 或 import 即可恢复目录结构、历史决策和踩坑记录。
2. **可解释排障**：Agent 引用了错误上下文导致后续推理跑偏，开发者用检索结果里的 `file_path / line / score / retrieval_method / reason` 定位错误召回来源，而不是盲目换 embedding 模型。
3. **provider / 参数横向评测**：开发者想知道 OpenClaw Builtin vs OpenClaw + QMD vs Hermes built-in vs ContextForge BM25 / hybrid 哪个更适合当前项目，跑 `contextforge eval run` 看 Top-5 / Top-10 命中率与延迟对比。
4. **跨 Agent 上下文迁移**：把 OpenClaw workspace 上下文导出为 canonical JSONL / Markdown bundle / agent draft，供 Hermes、Claude Code、Cursor、Zed 经人工导入或 MCP 查询复用，不被某 Agent memory 格式锁定。

---

## Core Capabilities｜核心能力

> ≤ 5 条。多于 5 条说明范围还没收敛 — 拆 v1.0 / v1.1 / v2.0。

1. **多 Agent 中立的本地上下文统一接入与索引**：把 OpenClaw workspace、Hermes `MEMORY.md` / `USER.md`、本地代码、文档、日志统一收进一个本地 collection。例：`contextforge import openclaw <ws> && contextforge index ./project` 后所有源进同一可搜索索引。
2. **可解释检索（一等公民）**：每条结果带 `file_path / line_start-end / score / retrieval_method / last_modified / reason / agent_scope`。例：`contextforge search "QMD reranker 为什么 Vulkan OOM"` 能回答「这段来自哪个文件第几行、是关键词还是向量命中、分数多少」。
3. **MemoryOps 治理**：导入即治理——对 memory / context record 做去重、过期标记、冲突检测、来源追踪、审计与版本化管理；并接入安全基线的 secret redaction，确保敏感内容默认不以明文进入索引或导出。核心价值是避免 memory 从「增强 Agent」变成「污染 Agent」。
4. **召回评测**：内建 golden questions + recall eval。例：`contextforge eval run` 输出 Top-5 / Top-10 命中率、延迟、错误召回样例，用于回归与 provider 横向对比，使 ContextForge 成为「上下文质量实验台」而非仅一个检索系统。
5. **跨 Agent 上下文迁移**：canonical record 导入/导出，上下文不被某 Agent memory 格式锁定。例：OpenClaw workspace 导出为 canonical JSONL / Markdown bundle / Agent rule draft，供 Hermes、Claude Code、Cursor、Zed、OpenClaw 经人工导入、配置引用、MCP 查询或后续适配器复用，v0.1 结构化字段保留 ≥ 80%。

**v0.1 范围切分**：

**v0.1 P0 必须完成**：

- `contextforge init`
- 本地文件 / Markdown / 代码 / 日志索引
- Hermes `MEMORY.md` / `USER.md` 只读导入
- OpenClaw workspace 通用 file / markdown / config / log 导入
- BM25 / metadata / filter 检索
- 可解释检索结果：source、file_path、line_start、line_end、score、retrieval_method、last_modified、reason、agent_scope
- 基础 secret redaction
- canonical JSONL 导出
- Markdown bundle 导出
- golden questions recall eval 最小版
- Linux / WSL2 本地运行闭环

**v0.1 P1 / Stretch Goal**：

- OpenClaw schema-aware importer
- Claude Code / Cursor / Zed 规则文件深度适配
- Agent rule draft 导出
- 冲突检测
- 版本化 memory record
- MCP adapter 完整工具集
- provider 横向对比
- embedding / reranker / hybrid search

**v0.1 MemoryOps 能力边界**：

- 去重：仅做 normalized content hash / source hash / exact duplicate 去重。
- provenance 合并：相同内容 hash 的记录合并来源链，不丢失原始来源。
- 过期标记：支持 `expires_at`、source deleted、source modified 后 stale 标记。
- 冲突检测：v0.1 仅检测同一 key / path / tag 下的明显冲突，不做 LLM 语义冲突判断。
- 版本化：记录 import batch version 与 content_hash 变化，不做完整 event sourcing。
- 审计：记录 import、search、export、redaction、delete 等关键事件。

**明确不做（Out of Scope，至少列 3 项）**：

- **不替代各 Agent 自带 memory**：v0.1 不自动写回 OpenClaw / Hermes memory backend（只读导入 + 导出 draft）。
- **不做 Web Dashboard / Desktop / Mobile**：v0.1 仅 CLI + Daemon + MCP/API（Web Dashboard 推到 v0.2+）。
- **不做多用户 / 多租户 / 团队版部署**：v0.1 本地单用户，多用户权限留待团队版。
- **不强依赖云端模型 / 外部 provider**：远程 embedding、reranker、LLM 为显式 opt-in，非运行前提。
- **不追求 GDPR / SOC2 / HIPAA 正式合规认证**：仅工程化隐私基线。
- **不做完整知识图谱 / GraphRAG**：v0.1 不构建实体关系图谱（与 Cognee 类方案显式划清）。
- **不自动修改用户源文件或第三方 Agent 配置**：v0.1 不默认写回 `MEMORY.md`、`USER.md`、`CLAUDE.md`、`AGENTS.md`、Cursor / Zed rules、OpenClaw workspace；导出一律 draft / bundle，需用户确认后手动应用。

---

## User Flow｜用户流程

**主流程（happy path）**：

1. `contextforge init` → 生成本地配置 + SQLite / 索引目录（本地优先，不联网）。
2. `contextforge import openclaw <ws>` / `contextforge import hermes <path>` / `contextforge index ./project` → scanner 过滤 denylist + secret redact → chunker 切片 → indexer 写本地索引。
3. `contextforge search "..."` 或经 MCP `context_search` → retriever 返回带 provenance + score + reason 的可解释结果。
4. `contextforge export --format markdown-bundle/jsonl/agent-draft` → 将选定 collection 或 search result 导出为可复用上下文包。
5. 终态：用户 3-5 分钟内恢复项目上下文，无需手工复制 README、日志、配置；Agent 经 MCP 获取一致、可追溯上下文；用户可将上下文以 draft / bundle 方式迁移到其他 Agent 工作流中。

**异常流（≥ 2 项）**：

- **索引中断**（大仓库 10 万文件）：进入长任务模式，进度显示、可中断、`index --resume` 断点续传，不重复全量。
- **secret 命中**（结果含 token / key）：默认 redaction，结果不展示完整 secret，export 二次扫描。
- **导入源 schema 不识别**（OpenClaw 版本差异）：降级为通用 file / markdown 处理 + 警告，不中断整个导入。
- **远程 provider 不可用 / 离线**：自动降级为本地 BM25 / 全文检索，不阻塞核心检索。

**secret 命中时的 CLI 输出示例**：

```text
Found 3 potential secrets:
- .env:12 [REDACTED:API_KEY]
- config.yml:8 [REDACTED:BEARER_TOKEN]
- logs/app.log:391 [REDACTED:SESSION_COOKIE]

Indexed with redaction.
Original files were not modified.
Run `contextforge scan --dry-run <path>` to inspect before indexing.
```

要求：

- 不在 CLI 输出中展示完整 secret。
- redaction 结果保留类型标签，例如 `[REDACTED:GITHUB_TOKEN]`。
- 用户 override redaction 时必须写入 audit log。

**边界场景（≥ 1 项）**：

- 同一事实跨多 Agent source 重复：MemoryOps 去重，保留 provenance 链，不返回冗余多份。
- 超大单文件（100MB 日志）：流式分块 + 大小上限保护，内存不爆。
- 100 万 chunk 规模检索：性能降级目标 P95 < 1500ms（需压测），不崩溃。

---

## Technical Approach｜技术方案

- **项目类型**：Infrastructure（本地优先的 AI Agent Context / MemoryOps 基础设施）—— 主形态 = Local CLI + Local Daemon/API Server + MCP Server/Agent Adapter + Rust Context Indexing & Retrieval Core；次要形态（v0.2+）= Web Dashboard / Library SDK；不做 Desktop / Mobile。
- **技术栈**：Go + Rust 混合。Go = 控制面（CLI = cobra / 配置 = viper·koanf / REST = net/http + chi / 日志 = slog·zap / gRPC client = grpc-go），负责 CLI、daemon、REST、MCP adapter、agent adapter、provider 配置、import / export、eval 调度；Rust = 数据面（async = tokio / gRPC = tonic / 错误 = anyhow + thiserror / 序列化 = serde / 扫描 = ignore·walkdir / 监听 = notify / 全文 = tantivy / 代码解析 = tree-sitter / Markdown = pulldown-cmark / 存储 = SQLite via rusqlite 优先，sqlx async-heavy 再评估 / trace = tracing），负责扫描、过滤、解析、chunking、metadata、检索、可解释 trace、provenance。embedding / reranker 为 P1 增强（QMD 本地 / OpenAI-compatible / SiliconFlow），P0 不强依赖。
- **关键模块边界**（≥ 3 个，越具体越好）：
  - `cli`（Go）：CLI 入口——命令解析、配置加载、子命令编排（init / import / index / search / serve / mcp / eval / export）
  - `daemon`（Go）：本地 REST API server + 长任务调度（索引 / eval run）
  - `mcp-adapter`（Go）：MCP server——暴露 `context_search` / `context_read` / `context_explain` / `context_collections`
  - `agent-importer`（Go）：Agent 适配编排——openclaw-workspace / hermes-memory / agent-rules importer，产出 canonical record
  - `config`（Go）：TOML 配置、denylist / allowlist、provider 配置、collection / agent scope 管理
  - `scanner`（Rust）：文件扫描 + denylist / allowlist 过滤 + secret 扫描
  - `parser`（Rust）：代码（tree-sitter）/ Markdown（pulldown-cmark）/ 日志解析
  - `chunker`（Rust）：文档 / 代码 chunking + metadata 抽取 + provenance 维护
  - `indexer`（Rust）：Tantivy 全文索引 + SQLite metadata / chunk 存储 + 增量更新
  - `retriever`（Rust）：BM25 / metadata / filter 检索 + explainable retrieval trace
  - `memoryops`（Go+Rust）：去重 / 冲突检测 / 过期标记 / provenance 合并 / 审计事件
  - `exporter`（Go）：canonical JSONL / Markdown bundle / Agent rule draft / eval dataset 导出
  - `eval`（Go+Rust）：golden questions 加载、检索调用、命中率 / 延迟 / 错误召回统计
- **架构风格**：本地双进程模块化架构——Go 控制面进程（`contextforge`）+ Rust 数据面进程（`contextforge-core`），经 local gRPC 解耦；模块化单体式部署（单机多二进制），非微服务、非云，v0.1 不用 FFI / cgo。
- **数据流（如适用）**：外部源（本地代码库 / 文档 / 日志 / OpenClaw · Hermes · agent-rules）→ `scanner`（denylist / allowlist 过滤 + secret redact）→ `parser` → `chunker`（+ provenance）→ `memoryops`（去重 / 冲突 / 过期）→ `indexer`（Tantivy 全文索引 + SQLite metadata / chunk 存储，本地落盘）→ `retriever`（BM25 / metadata / filter + explainable trace）→ 经 CLI / 本地 REST / MCP 输出，或 `exporter` 导出 canonical JSONL / Markdown bundle / agent draft。

**Canonical Record v0.1 最小 schema**：

ContextForge v0.1 至少区分四类对象：

- `SourceRecord`：原始来源，例如文件、memory 文件、日志文件、规则文件。
- `ContextRecord`：治理后的上下文记录，用于迁移、审计、导出。
- `Chunk`：检索用切片，用于 Tantivy / metadata search。
- `RetrievalResult`：返回给 CLI / REST / MCP 的可解释检索结果。

`ContextRecord` v0.1 最小字段：

```json
{
  "id": "ctx_...",
  "schema_version": "0.1",
  "collection_id": "project_x",
  "source_type": "file|memory|log|agent_rule|config",
  "source_provider": "local|openclaw|hermes|claude_code|cursor|zed|generic",
  "source_uri": "file:///path/to/source",
  "agent_scope": ["openclaw", "hermes"],
  "title": "optional",
  "content": "...",
  "content_hash": "sha256...",
  "redaction_status": "none|partial|full",
  "language": "markdown|go|rust|json|yaml|log|text",
  "file_path": "...",
  "line_start": 10,
  "line_end": 25,
  "tags": ["config", "memory"],
  "provenance": [
    {
      "importer": "hermes-memory",
      "original_path": "...",
      "imported_at": "...",
      "source_modified_at": "..."
    }
  ],
  "security_labels": ["local_only", "redacted"],
  "created_at": "...",
  "updated_at": "...",
  "expires_at": null,
  "version": 1,
  "metadata": {
    "extra": {}
  }
}
```

v0.1 允许未识别字段进入 `metadata.extra`，但不得影响核心检索、导出和审计字段。

**REST / MCP 最小接口契约草案**：

`POST /v1/search` 请求：

```json
{
  "query": "memorySearch 配置在哪里",
  "collections": ["silijian"],
  "agent_scope": ["openclaw"],
  "top_k": 10,
  "filters": {
    "source_type": ["file", "memory"],
    "language": ["markdown", "json"]
  },
  "explain": true
}
```

`POST /v1/search` 响应：

```json
{
  "results": [
    {
      "chunk_id": "chk_...",
      "context_id": "ctx_...",
      "source_type": "memory",
      "file_path": "...",
      "line_start": 10,
      "line_end": 18,
      "score": 12.4,
      "retrieval_method": "bm25",
      "reason": "matched terms: memorySearch, config",
      "agent_scope": ["openclaw"],
      "redaction_status": "none",
      "provenance": []
    }
  ]
}
```

MCP tools v0.1 至少包括：

- `context_search`
- `context_read`
- `context_explain`
- `context_collections`

MCP tool 的返回字段必须与 REST search result 的可解释字段保持一致。

**本地数据目录结构 v0.1**：

默认数据目录：

```text
~/.contextforge/
  config.toml
  collections/
    <collection_id>/
      metadata.sqlite
      tantivy/
      audit.log
      exports/
      eval/
  logs/
  runtime/
    contextforge.token
    contextforge-core.pid
```

要求：

- `config.toml` 与 token 文件权限应尽量限制为当前用户可读写。
- 删除 collection 时必须清理对应 `metadata.sqlite`、`tantivy/`、`exports/` 和 `eval/`。
- `audit.log` 不记录完整 secret 和完整导出内容。

---

## Constraints｜约束

- **运行时**：Go toolchain（建议 Go 1.22+）+ Rust stable（建议 1.75+，cargo 构建），产出 `contextforge` + `contextforge-core` 两个本地二进制；无 JVM / Node 运行时依赖；CPU-only 环境必须能完成基础索引与检索（GPU 仅外部 embedding / reranker provider 可选利用，非 core 硬依赖）。
- **平台**：
  - **v0.1 P0**：Linux x86_64（Ubuntu 22.04 / 24.04 / 26.04 / WSL2 Ubuntu）。
  - **v0.1 Nice-to-have**：macOS arm64 / macOS x86_64 可源码构建运行，但不承诺官方 tarball。
  - **v0.2**：macOS arm64 / macOS x86_64 官方 tarball + Homebrew tap。
  - **v0.3**：Windows native preview（路径权限、文件监听、符号链接、shell 差异、MCP 启动方式需调研）。
  - **Nice-to-have**：Docker 环境。
- **性能**：检索（已完成本地索引、未调用 embedding / reranker / 远程 provider）—— 10 万 chunk 内 BM25 / metadata / filter P95 < 500ms、chunk 原文读取 P95 < 100ms；100 万 chunk P95 目标 < 1500ms（`TBD - 需压测`）；本地 hybrid search P95 目标 < 2s（reranker 延迟单独记录，远程 API 延迟不计入 core retrieval）。索引——1 万源码 / Markdown / 配置文件扫描 + chunking + metadata + Tantivy < 10 分钟，10 万文件进长任务模式（进度 / 可中断 / 可恢复，embedding 生成不计入）。增量——单文件变更 < 5s、100 文件 < 60s、大规模自动降级后台任务。资源——daemon idle 内存 < 300MB、基础索引 < 2GB、单次搜索额外 < 200MB、磁盘索引 ≤ 可索引文本 1.5x-3x（本地 embedding / reranker / 外部向量库不计入 core 内存指标）。
- **安全**：处理高度敏感本地数据（源码 / 配置 / 日志 / Agent memory / 调试记录 / shell 记录 / 文档，可能含 token / key / password / cookie / 内部 URL）。v0.1 不追求 GDPR / SOC2 / HIPAA 正式合规，承诺工程化隐私基线：默认本地存储不上传、远程 provider 显式 opt-in 才启用、collection allowlist 路径导入、默认 denylist 敏感路径（`.env`、`.env.*`、`*.pem`、`*.key`、`*.p12`、`*.pfx`、`id_rsa`、`id_ed25519`、`.ssh/`、`.git/objects/`、`node_modules/`、`target/`、`dist/`、`build/`、`.cache/`、`vendor/`）、secret pattern 检测 + redaction 后入索引（不改原文件）、检索结果默认不展示完整 secret、export 二次扫描、agent_scope / collection_scope / operation_scope 隔离、检索与导出写本地 audit log、支持一键删除 collection 索引 / 彻底清空本地数据库与索引。

**Local service security baseline**：

- daemon 默认只监听 `127.0.0.1` 或 Unix domain socket。
- v0.1 禁止默认绑定 `0.0.0.0`。
- REST API 默认启用本地随机 token。
- token 文件权限应为 `0600`。
- MCP client 需要显式 allowlist。
- audit log 不记录完整 secret，不记录完整导出内容。
- audit log 默认记录 operation、collection、source、result_count、redaction_count、timestamp，不默认记录完整 query content。
- 远程 provider opt-in 时，CLI/API 必须显示将发送的数据类型、目标 provider 和是否包含原文。

- **兼容性**：v0.1 必须支持的 P0 导入源 = 本地文件系统 / Markdown / 代码文件（`.go`、`.rs`、`.py`、`.ts`、`.tsx`、`.js`、`.jsx`、`.md`、`.txt`、`.json`、`.yaml`、`.yml`、`.toml`）/ 普通日志（`.log`、`.jsonl`、`.txt`）/ Hermes `MEMORY.md` / `USER.md` / OpenClaw workspace / memory 目录 / `AGENTS.md` / `CLAUDE.md` / 项目规则类文件；导出 = canonical JSONL / Markdown bundle / Search result JSON / Eval dataset JSONL（可选：Hermes-style `MEMORY.md` / `USER.md` draft、`AGENTS.md` draft、`CLAUDE.md` draft）。只读导入 + 导出 draft / bundle，不自动写回各 Agent 原生 memory。OpenClaw 具体 memory schema、Claude Code / Cursor / Zed 规则文件路径与格式、MCP 协议版本均 `TBD - 需基于实测版本与样本适配`（见 Open Questions）。
- **发布**：v0.1 = GitHub Release Linux x86_64 tarball（含 `contextforge` + `contextforge-core` + `contextforge.example.toml` + README + LICENSE）+ 源码 self-host（`git clone && make dev`）+ Docker Compose（`docker compose up`）；不优先 pip / pipx / npm / cargo install / go install / Homebrew / Windows installer / `.dmg`（Go+Rust 混合产物无法用单一包管理器干净分发，v0.1 优先验证价值闭环）。回滚策略：tarball 版本化，出问题回退上一 release tag + README 标注已知问题。后续路线：v0.2 macOS tarball + Homebrew tap；v0.3 Windows native preview + 一键安装脚本 + MCP server 独立包；v1.0 多平台 release + 签名校验 + 自动更新 + 企业部署文档。

---

## Implementation Phases｜实施阶段

> `/s2v-init` 会读这张表批量生成 phase spec 和 task spec。要求：
> - `description` 列写「完成后能做什么」，不写 TODO 风格
> - `scope` 列要列出**具体模块名 / 文件名**（不写「全部代码」）
> - `depends_on` 用 phase 编号；零依赖写 `-`
> - `parallel` 标「是 / 否」；写「是」时必须说明「可与谁并行」

| # | Phase 名称（kebab） | 描述（完成后能做什么） | 范围（涉及模块 / 文件） | 依赖 | 可并行 |
|---|---|---|---|---|---|
| 1 | foundation | `contextforge init` 跑通；Go CLI ↔ Rust core 双二进制经 local gRPC 打通；canonical record schema + denylist/allowlist 配置定型 | `cli` + `config` + `daemon`(skeleton) + `contextforge-core`(skeleton) + gRPC/canonical-record proto | - | 否（基础设施，所有 phase 依赖契约） |
| 2 | index-core | `contextforge index ./project` 建立本地 Tantivy + SQLite 索引；denylist/allowlist 与 secret redaction 生效；支持基础增量索引，完整长任务恢复在 Phase 8 硬化 | `scanner` + `parser` + `chunker` + `indexer`（Rust） | 1 | 是（可与 phase 3 并行） |
| 3 | agent-importers | `contextforge import openclaw/hermes/agent-rules` 把外部源转为 canonical record（与 phase 2 集成后端到端入索引） | `agent-importer`（Go）+ canonical record 映射 | 1 | 是（可与 phase 2 并行） |
| 4 | retrieval-explain | 检索链路跑通；可通过内部 gRPC Search API / `contextforge search` 调试入口返回带 file_path/line/score/retrieval_method/last_modified/reason/agent_scope 的可解释结果 | `retriever`（Rust）+ explainable result schema | 2 | 是（可在 phase 2+3 完成后与 phase 5 并行） |
| 5 | memoryops | 基于 content hash / source hash 完成重复记录去重并保留 provenance 链；支持 stale 标记；完成基础冲突提示；生成审计事件与 audit log | `memoryops`（Go+Rust） | 2, 3 | 是，可在 Phase 2 + Phase 3 完成后与 Phase 4 并行 |
| 6 | cli-api-export | `contextforge search` / REST `/v1/search` / `contextforge export` 可用；导出 canonical JSONL / Markdown bundle / agent draft，迁移字段保真 ≥ 80% | `cli`(search/export) + `daemon`(REST API) + `exporter` | 4, 5 | 否 |
| 7 | mcp-adapter | Agent 经 MCP 获取一致、可追溯上下文（context_search/context_read/context_explain/context_collections） | `mcp-adapter`（Go） | 6 | 否 |
| 8 | eval-and-reliability | `contextforge eval run` 输出 Top-5/Top-10 命中率、延迟、错误召回报告；v0.1 七项技术闭环在 Linux/WSL2 端到端跑通；完成长任务/中断恢复/资源占用/secret redaction/export 的可靠性硬化；产出可安装的 Linux x86_64 release 包并通过 smoke test | `eval`（Go+Rust）+ 全链路集成测试 | 6, 7 | 否 |
| 9 | cli-pipeline | （v0.2 收口）补齐 v0.1 CLI 数据通路 spec drift：proto add-only `rpc Index` stream + Rust `CoreService::index` wire + Go CLI `index` / `import` 真实接通 + task-8.3 假证据测试取代为真集成 + README quick start 可复现；详见 ADR-013 | proto/contextforge/v1/*.proto + core/src/server.rs + internal/cli/index.go·import.go + internal/daemon/index.go + internal/release/release_test.go + scripts/{release_smoke,quickstart_smoke}.sh + examples/quickstart/ + docs/releases/v0.2.0-*.md | 8 | 否 |
| 10 | console-contract-v1 | （v0.3 收口）实现 ContextForge ↔ ContextForge-Console v1.0 Contract v1 兼容层：internal/contractv1/ Go 镜像 + core/src/{workspace,jobs}/ Rust 资源模型 + SQLite migration 0010/0011 + internal/consoleapi/ 9 REST endpoint + OpenAPI + cross-repo conformance test + docker compose 端到端联调；详见 ADR-015 | internal/contractv1/ + core/src/workspace/ + core/src/jobs/ + core/migrations/0010_workspaces.sql + 0011_index_jobs.sql + internal/consoleapi/ + docs/consoleapi/openapi.yaml + test/conformance/console_contractv1_test.go + scripts/console_smoke.sh + deploy/console-stack.yml + Dockerfile + docs/releases/v0.3.0-*.md | 9 | 否 |

**Phase Exit Criteria｜阶段验收标准**：

**Phase 1 foundation**

- `contextforge init` 能生成默认配置与本地数据目录。
- `contextforge-core` 能由 daemon 启动。
- Go daemon 能通过 local gRPC health check Rust core。
- canonical record schema v0.1 与 proto 契约冻结。
- denylist / allowlist 默认配置可被 CLI 读取。

**Phase 2 index-core**

- `contextforge index ./sample_project` 能索引 ≥ 1000 个文件。
- `.env`、`.ssh/`、`.git/objects/`、`node_modules/`、`target/` 默认跳过。
- secret fixture 能被 redacted。
- SQLite 中可查询 chunk metadata。
- Tantivy 中可搜索到基础结果。
- 单文件变更能触发基础增量更新。

**Phase 3 agent-importers**

- Hermes `MEMORY.md` / `USER.md` 能导入为 canonical record。
- OpenClaw workspace 至少能按通用 file / markdown / config / log 方式导入。
- `AGENTS.md` / `CLAUDE.md` 能作为 agent_rule source 导入。
- 不识别的 schema 会降级为通用文件导入并提示 warning，不中断导入。

**Phase 4 retrieval-explain**

- `contextforge search` 能返回 Top-K 结果。
- 每条结果至少包含 file_path、line_start、line_end、score、retrieval_method、reason、agent_scope。
- 错误 query 返回空结果，不 panic。
- 返回结果能定位回原始文件和行号。

**Phase 5 memoryops**

- exact duplicate 能被去重。
- provenance 链能合并并保留多个来源。
- redaction 状态能写入 ContextRecord。
- stale 标记可被设置和检索。
- import / search / export / redact 事件能写入 audit log。

**Phase 6 cli-api-export**

- `contextforge search` 可用。
- REST `/v1/search` 可用。
- `contextforge export --format jsonl` 可导出 canonical JSONL。
- `contextforge export --format markdown-bundle` 可导出 Markdown bundle。
- export 前执行二次 secret scan。
- 迁移字段保真率可通过 fixture 计算。

**Phase 7 mcp-adapter**

- MCP `context_search` 可返回可解释结果。
- MCP `context_read` 可读取指定 chunk / context。
- MCP `context_explain` 可返回召回理由和 provenance。
- MCP `context_collections` 可列出可用 collection。
- MCP client 未被 allowlist 时拒绝访问。

**Phase 8 eval-and-reliability**

- `contextforge eval run` 输出 Top-5 / Top-10、latency、miss cases。
- Linux / WSL2 release smoke test 通过。
- 10 万 chunk 内 BM25 / metadata / filter 检索 P95 < 500ms。
- secret redaction / export / audit log 回归测试通过。
- 大仓库长任务中断后可恢复或安全重建。

**Phase 9 cli-pipeline**（v0.2 收口；v0.1 spec drift 补齐，详见 ADR-013）

- `proto/contextforge/v1/service.proto` 含 `rpc Index(IndexRequest) returns (stream IndexProgress);`（schema_version 仍 `0.1`，add-only 演进）。
- `contextforge import hermes|openclaw|agent-rules <path> --collection X` 真实写出 canonical record 文件到 `<data_dir>/imports/<source>/`（D1 两步式：import 离线产物 → index 灌入）。
- `contextforge index --source <path> --data-dir <root> --collection X` 真实调 Rust gRPC `CoreService::index` 索引（取代 v0.1 manifest 存根）；进度按文件粒度上报；`--resume` + reliability manifest 保留。
- `contextforge search` / `eval run` 在已索引 collection 上真实返回结果（不再 `collection not found`）。
- `scripts/release_smoke.sh` 含 phase 9 CLI 端到端段；`internal/release/release_test.go` 删除 fake-evidence 测试（v0.1 task-8.3 假 AC2/AC4），改为真集成。
- `scripts/quickstart_smoke.sh` 一键跑 README quick start 七步；`examples/quickstart/` 提供可复制粘贴 fixture。
- ADR-013 状态推进 Proposed → Accepted；adapter §Phase 索引 Phase 9 → Done；v0.2.0 RELEASE_NOTES + evidence + artifacts 落盘。

**Phase 10 console-contract-v1**（v0.3 收口；ContextForge ↔ ContextForge-Console v1.0 集成；详见 ADR-015）

- `internal/contractv1/` 含 17 Contract v1 类型 Go 镜像（1:1 对齐 Console `console-api/internal/coreadapter/contractv1/contractv1.go`）+ `ContractVersion = "v1"` 常量 + FieldAvailability helper。
- `core/src/workspace/` + `core/migrations/0010_workspaces.sql` 实现 Workspace 资源 CRUD + workspace_id ↔ collection_id 1:1 映射。
- `core/src/jobs/` + `core/migrations/0011_index_jobs.sql` 实现 IndexJob 异步 lifecycle（queued/running/succeeded/failed/cancelled）+ heartbeat + co-operative cancel。
- `internal/consoleapi/` 含 9 REST endpoint：`GET /v1/health` + `POST/GET/GET /v1/workspaces*` + `POST/GET/POST /v1/index-jobs*[/cancel]` + `POST /v1/search`（嵌套响应 `{result, trace}`）+ `GET /v1/observability/events` long-poll；路径/shape/错误码严格对齐 Console HTTPAdapter 期望；`docs/consoleapi/openapi.yaml` 落 OpenAPI 3.0。
- `test/conformance/console_contractv1_test.go` 反向取 Console fakehttpserver oracle 跑过端到端（env `$CONSOLE_REPO` 设时跑全套，未设 SKIP）。
- `scripts/console_smoke.sh` + `deploy/console-stack.yml` + `Dockerfile` 启动 docker compose stack（Console v1.0 + ContextForge daemon + Postgres + Redis）+ curl Console UI 真返回 workspace 列表（非 Mock）+ `CONSOLE_SMOKE_EXIT=0`。
- ADR-015 状态推进 Proposed → Accepted；adapter §Phase 索引 Phase 10 → Done；§Open Questions O13 标记 resolved by ADR-015；v0.3.0 RELEASE_NOTES + evidence + artifacts 落盘。
- ADR-014 cross-validation gate（D1 mapping 表 + D2 lint 0 violation + D3 phase §6 每条 AC verified by 显式 + D4 主 agent 自治补丁）首次完整激活并跑通。

**Phase 11 console-real-data-plane**（v0.4 收口；把 Phase 10 task-10.4 §10 Trade-off #1 + #2 一次性 resolve；详见 ADR-016）

- `core/proto/console_data_plane.proto` 含 4 service × 14 RPC + 11 message 类型（`WorkspaceService` / `JobService` / `SearchService` / `EventsService`），字段命名 snake_case 与 Go `internal/contractv1/contractv1.go` JSON tag 1:1；包声明 `contextforge.console_data_plane.v1`，与 Phase 9 `proto/contextforge/v1/service.proto` (Index gRPC) 分离演进。
- `core/src/data_plane/` Rust module（`mod.rs` + `workspace.rs` + `job.rs` + `search.rs` + `events.rs`）含 4 个 tonic service trait 实现；复用 task-10.2 `SqliteWorkspaceStore` + task-10.3 `SqliteJobStore` + `JobRunner` 框架；daemon `serve` 子命令启动时把 4 service `add_service` 到现有 `:48180` tonic Server（与 Phase 9 cli-data-plane gRPC 共存）。
- `internal/consoleapi/grpcclient/` Go 包 + `Deps` 接口 gRPC-backed 4 wrapper 实现；`internal/consoleapi/handlers.go` 重构为 thin protocol translator（不引入字段映射代码 + 不引入业务逻辑）；`internal/consoleapi/memstore.go` 降级为 env-gated fallback (`CONSOLE_API_FALLBACK_INMEM=1`)；console-api-serve 新增 `--grpc-addr` 默认 `127.0.0.1:48180` + `--fallback-inmem` flag。
- `JobService.Enqueue` 真触发 `JobRunner.spawn_blocking(IndexSession::index_path_with_progress)` + heartbeat 每 100 files 或 5s 持久化 + co-operative cancel via `CancelToken Arc<AtomicBool>` + `JobOutcome` 写回 succeeded/failed/cancelled + error_message；orphan reaper 在 daemon `serve` 启动早期标 running → failed。
- `SearchService.Query` 真接 existing retriever (Tantivy + SQLite chunks) + `RetrievalTrace.retrieved_chunks` 真填 (score + source_file + content snippet ≤200 字 UTF-8 boundary safe)；`EventsService.Subscribe` 真接 tokio broadcast channel-backed `EventBus` (容量 1000，Lagged log warning + continue)；Go `/v1/observability/events` 改 long-poll wrap (30s timeout / 100 evt batch)。
- `scripts/console_smoke.sh` v2 REAL mode 默认 + `CONSOLE_REAL_SMOKE_EXIT=0` final marker；local-only mode 保留为 `LOCAL_ONLY=1` env-gated；`scripts/release_smoke.sh` 第 5 段更新为 REAL 模式 + `PHASE_RELEASE_SMOKE_EXIT=0`。
- ADR-016 状态推进 Proposed → Accepted；adapter §Phase 索引 Phase 11 → Done；§Open Questions O14 标记 resolved by ADR-016 (business plane wiring) + endpoint expansion `[SPEC-DEFER:console-endpoint-expansion]`；v0.4.0 RELEASE_NOTES + evidence + artifacts 落盘。
- ADR-014 cross-validation gate（D1 mapping 表 + D2 lint 0 violation + D3 phase §6 每条 AC verified by 显式 + D4 主 agent 自治补丁）第二次完整激活验证制度稳定性。

**Phase 12 console-contract-completion**（v0.5 收口；Console 22-endpoint Wave 1+2 落地 9→15 endpoint；详见 ADR-017 D1 Wave 1+2）

- Wave 1 quick win（task-12.1）：`PATCH /v1/workspaces/{id}/config` 走 gRPC WorkspaceService.Update（task-11.1 已 ship 复用）+ `GET /v1/index-jobs?status=active` 走 JobService.List filter + `POST /v1/index-jobs/{id}/cancel` 改 204 No Content（ADR-017 D3）+ `confirmMiddleware` 服务端 X-Confirm 兜底（ADR-017 D2，破坏性 endpoint 缺 X-Confirm:yes header 或 ?confirm=true query → 412 Precondition Failed）。
- Wave 2 mid scope（task-12.2 + task-12.3）：`GET /v1/source-chunks/{id}` 走新 SearchService.GetSourceChunk RPC + retriever 加 `get_chunk_by_id` 接口（add-only proto 演进）；`GET /v1/search/{query_id}/trace` 走新 SearchService.GetSearchTrace RPC + Rust 端 SearchService.Query 执行时把 RetrievalTrace 持久化 to in-memory LRU store 容量 1000（SQLite 持久化 [SPEC-DEFER:task-future.search-trace-sqlite-persistence] 留 v0.5.x）。
- `scripts/console_smoke.sh` v3 升级 15 endpoint flow；v0.4 既有 9 endpoint test 不退化；conformance test (TestConsoleContractV1Conformance) 不退化。
- ADR-017 Status: Proposed（Accepted 在 Phase 14 closeout 时回填；6 D-clauses 跨 3 phase）；adapter §Phase 索引 Phase 12 → Done；§Open Questions O15 标记 partially resolved (RFC3339Nano kept; Console Zod relax follow-up) + O18 partially resolved (Wave 1+2 ship)。
- ADR-014 cross-validation gate 第三次完整激活验证制度稳定性。

**Phase 13 memory-rest-surface**（v0.6 收口；Console 22-endpoint Wave 3 落地 15→20 endpoint；详见 ADR-017 D1 Wave 3）

- task-13.1：新增 SQLite migration `0013_memory_items.sql` 表 `memory_items`（9 列 1:1 镜像 contractv1.MemoryItem + status enum {active/deprecated/soft_deleted} + is_pinned bool + 3 索引）；新增 `core/src/memory/store.rs` `SqliteMemoryStore` CRUD + 3 state ops；新增 `proto/contextforge/console_data_plane/v1/console_data_plane.proto` `MemoryService` 5 RPC + 5 message；新增 `core/src/data_plane/memory.rs` `MemoryServer` impl + pin/deprecate/soft-delete 每 emit 一条 audit event 到既存 task-5.3 AuditSink 框架。
- task-13.2：Go `internal/consoleapi/grpcclient/grpcclient.go` 加 `MemoryClient` 5 method wrapper；Go REST 5 handler — `GET /v1/memory?agent_id=&scope=&namespace=` + `GET /v1/memory/{id}` + `POST /v1/memory/{id}/pin` (204，非破坏性) + `POST /v1/memory/{id}/deprecate` (走 confirmMiddleware) + `POST /v1/memory/{id}/soft-delete` (走 confirmMiddleware)；MemStore fallback 模式下 list/get 用 in-memory 5 fixture items；smoke v4 升级 20 endpoint flow + sqlite3 CLI seed memory fixture step。
- **重要 scope**：本 phase **不实施 importer 写入 memory_items 路径**（[SPEC-DEFER:phase-15.import-to-memory-items] 留 v0.6.x）；Console UI 在 v0.6.0 ship 后 list 端可能返空数组 → Console UI 端 graceful degrade 显示「No memory items yet; import via CLI」。
- ADR-017 Status: 仍 Proposed；adapter §Phase 索引 Phase 13 → Done；§Open Questions O16 partially resolved (REST 表面 ship; importer 写入路径留 v0.6.x)。
- ADR-014 cross-validation gate 第四次完整激活验证制度稳定性。

**Phase 14 eval-rest-surface**（v0.7 收口 + **Console 22-endpoint conformance 100% PASS**；详见 ADR-017 D1 Wave 4）

- task-14.1：新增 SQLite migration `0014_eval_runs.sql` 表 `eval_runs`（10 列含 config_snapshot_json / metrics_json / case_results_json + status enum {running/succeeded/failed/cancelled} + nullable finished_at + dataset_ref + error_message）；新增 `core/src/eval/store.rs` `SqliteEvalStore` CRUD + update_metrics/update_case_results/mark_finished；新增 `EvalService` 3 RPC (Create / Get / UpdateProgress) + Rust `EvalServer` impl；既存 `proto/contextforge/v1/eval.proto` (Phase 8 recall-only) **不动**，console_data_plane v1/EvalService 独立演进。
- task-14.2：Go `grpcclient.EvalClient` 3 method wrapper；Go REST 2 handler — `POST /v1/eval-runs` (非破坏性，不走 confirmMiddleware；返 200 + EvalRun status="running" + spawn goroutine 异步触发 recall harness) + `GET /v1/eval-runs/{id}` (返 EvalRun 含 metrics + case_results)；**Go-side `runEvalAsync` goroutine 调既存 Phase 8 `internal/eval/eval.go` recall harness → 完成时调 EvalService.UpdateProgress 反向 update Rust SqliteEvalStore**（trade-off：Rust spawn_blocking 调 Go binary as subprocess [SPEC-DEFER:phase-future.rust-native-eval-runner] 留 v1.x）；smoke v5 升级 22 endpoint flow + POST eval-run + poll 60s 等 terminal step。
- **Console 22-endpoint conformance suite 全 PASS = ContextForge ↔ Console v1.0 集成完整闭环**（v0.7.0 release ship 标志）。
- **ADR-017 状态推进 Proposed → Accepted**（Phase 14 closeout PR；6 D-clauses 完整覆盖 v0.5/v0.6/v0.7 3 phase 一次推进）；adapter §Phase 索引 Phase 14 → Done；§Open Questions O15/O16/O17 全 fully resolved；v0.7.0 RELEASE_NOTES + evidence + artifacts 落盘 + cross-repo follow-up 通知 Console 团队切到 production HTTPAdapter mode。
- ADR-014 cross-validation gate 第五次完整激活验证制度稳定性。

---

## Decisions Log｜决策日志

> `/s2v-init` 阶段 9.1 会把每条决策转成一份 ADR（默认 Status=Accepted）。
> 至少 3 条；至少覆盖 S2V 8 类决策中的任 3 类。
> 完整 8 类决策见 S2V `full-standard.md` §16.1。
> **`类别`列取值约束**：从 full-standard.md §16.1「8 类决策类别（唯一权威）」表的 8 个字面值中选其一 —— `架构` / `依赖` / `数据持久化` / `协议接口` / `安全` / `测试工具链` / `部署发布` / `兼容性`（**逐字照抄、勿用同义词**；下游 `/s2v-init` 渲染 ADR `Category` + 做「8 类是否都覆盖」审计按字符串相等匹配，写法不一致会让已覆盖类别被误判为未覆盖）。

| ID (D1, D2...) | 类别 | 决策（一句话） | 选择 | 候选方案 | 拒绝候选的理由 |
|---|---|---|---|---|---|
| D1 | 架构 | 控制面/数据面分离的双二进制架构 | Go 控制面（CLI/daemon/REST/MCP/编排）+ Rust 数据面（scan/parse/chunk/index/retrieve），经 local gRPC 通信，不用 FFI/cgo | 纯 Go 单体 / 纯 Rust 单体 / Go+Rust FFI/cgo | 纯 Go：tree-sitter·tantivy 级检索解析生态与性能弱于 Rust；纯 Rust：CLI/MCP/配置编排生态不如 Go 成熟、迭代慢；FFI/cgo：引入内存归属/panic 边界/构建复杂度，v0.1 不值得用这复杂度换那点性能 |
| D2 | 数据持久化 | 分层本地存储，向量后端仅抽象不强依赖 | SQLite 存 metadata/chunk/provenance + Tantivy 全文索引；向量后端做 provider 抽象，v0.1 不强依赖 | 纯向量库（Qdrant/LanceDB）起步 / Elasticsearch·OpenSearch / 纯 SQLite FTS | 纯向量库起步：过早把 v0.1 绑定到 embedding/vector pipeline，增加模型/向量维度/重建索引/provider 选择复杂度，而 v0.1 P0 目标是可解释 BM25/metadata baseline，不应让向量检索成为启动前提；ES/OpenSearch：JVM + 部署重，与单机本地优先冲突；纯 SQLite FTS：解释性与打分能力弱于 Tantivy |
| D3 | 协议接口 | 三对外接口 + 一内部 RPC | 对外 CLI / 本地 REST `/v1/*` / MCP tools（context_search/read/explain/collections）；内部 Go↔Rust 用 local gRPC | 仅 CLI / 仅 MCP / 内部用 stdin·stdout JSON-RPC 代替 gRPC | 仅 CLI：无法服务 Agent 程序化调用；仅 MCP：不能脚本化/调试；stdin·stdout JSON-RPC：长任务/流式进度/并发语义不如 gRPC 清晰 |
| D4 | 安全 | 本地优先隐私基线 + 默认脱敏 + 远程显式 opt-in | 默认本地不上传；denylist 敏感路径；secret 检测 + redaction 后入索引（不改原文件）；远程 provider 显式 opt-in；检索/导出写 audit log | 不做 secret 处理靠用户自管 / 索引前强制全量人工审查 / 全库加密 | 靠用户自管：目标用户处理含 key 的代码/日志，必泄露；强制人工审查：破坏「3-5 分钟恢复上下文」体验；全库加密：v0.1 作为默认能力过重（密钥管理/恢复/备份/性能复杂度），可作 v0.2+ 增强安全能力评估，v0.1 先采用 denylist+redaction+audit+explicit remote opt-in 隐私基线 |
| D5 | 兼容性 | 只读导入 + 导出 draft/bundle，不写回第三方 Agent | 导入 OpenClaw workspace / Hermes MEMORY.md·USER.md / agent-rules → canonical record；导出 canonical JSONL / Markdown bundle / agent draft；不自动写回 | 双向同步写回各 Agent 原生 memory / 仅支持单一 Agent 格式 / 私有格式不做 canonical | 双向写回：悄改用户 Agent memory 风险高且各 Agent schema 不稳定；仅单 Agent：违背「多 Agent 中立」核心定位；私有格式：锁死用户，违背「上下文不被锁定」价值主张 |
| D6 | 测试工具链 | recall eval 作为 PRD 级一等验收门 | Go `go test` + Rust `cargo test`；内建 `contextforge eval run`（golden questions → Top-5/10 命中率/延迟/错误召回）作为 PRD 级验收 | 仅单元测试不做 recall eval / 用外部 RAG eval 框架（ragas 等）/ 纯人工抽检 | 仅单测：无法回答「换 provider/embedding 后召回是否退化」（核心价值）；外部 RAG eval 框架：多为 Python 生态，增加运行时/工程复杂度，且评测对象是多 Agent 上下文召回/provenance/迁移保真/本地索引质量，通用框架不能完全覆盖，v0.1 内建轻量 recall eval 后续可导出数据兼容外部工具；人工抽检：不可回归 |
| D7 | 部署发布 | v0.1 极简分发，不追多渠道 | GitHub Release Linux x86_64 tarball（contextforge + contextforge-core + example.toml）+ 源码 self-host + Docker Compose | 单一语言包管理器（cargo/go/npm）分发 / 立即多平台+签名+自动更新 / 仅 Docker | 单一包管理器：Go+Rust 混合产物无法干净分发；多平台/签名/自更新：价值未验证前过早；仅 Docker：对本地 CLI/MCP 工作流不便 |
| D8 | 依赖 | 核心库选成熟生态、避免重复造轮子 | Rust：tantivy + tree-sitter + pulldown-cmark + tokio + tonic + rusqlite/sqlx(SQLite，rusqlite 优先，async-heavy 再评估 sqlx)；Go：cobra + chi + grpc-go + slog | 自研全文索引/分词 / sled·RocksDB 替 SQLite / Go 侧用 gin·echo 替 chi | 自研索引/分词：重复造轮子且质量不可控；sled/RocksDB：结构化 metadata 查询不如 SQLite 直观，且 SQLite 单文件可移植契合本地优先；gin/echo：中间件偏重，chi 轻量贴近 net/http 已足够 v0.1 |

---

## Success Metrics｜成功指标

**主要指标**（Primary，≥ 1 个，必须可测量）：

- **上下文重建时间**：开发者在新 Agent / 新会话恢复项目上下文从 15-30 分钟降到 ≤ 3-5 分钟（记录用户手动准备上下文耗时，对比使用前后新会话启动成本，观察是否减少复制 README、日志、配置频率）。

**上下文重建时间定义**：

从用户在一个已完成索引的 collection 中发起第一次 `contextforge search` / MCP `context_search` 开始，到用户或 Agent 获得足以继续任务的引用上下文为止。

**不包含**：

- 首次全量索引时间
- 安装时间
- 远程 embedding / reranker / LLM provider 调用时间

**包含**：

- 搜索
- 读取结果
- 必要的 `context_read`
- 导出或注入 Agent 工作流的时间

**补充工程指标**：

- TTFC（Time To First Useful Context）：已索引 collection 下，首次有效上下文返回 ≤ 30 秒。

- **Golden questions 命中率**：Top-5 ≥ 75%、Top-10 ≥ 85%。

**Eval Measurement Protocol｜评测口径**：

v0.1 recall eval 使用 golden questions 数据集进行评测。

**数据集最小规模**：

- v0.1 最少 30 条 golden questions。
- 每类至少 5 条：
  - 配置定位
  - 错误复现
  - 历史决策
  - 日志排查
  - Agent memory / rule 检索
  - 代码位置 / 函数定位

**每条 question 必须包含**：

- `query`
- `expected_sources`
- `expected_file_path`
- `expected_line_range` 或 `expected_chunk_id`
- `category`
- `notes`

**命中规则**：

- `Strong hit`：Top-K 结果中包含目标文件，并命中正确 chunk 或合理 line range。
- `Weak hit`：Top-K 结果中包含正确文件，但 line range 或 chunk 偏差较大。
- `Miss`：Top-K 中没有返回正确文件、正确片段或正确历史记录。

**主指标计算方式**：

- Top-5 / Top-10 主命中率只统计 `Strong hit`。
- `Weak hit` 单独报告，不计入主命中率。
- 延迟指标不包含 embedding、reranker、远程 provider API 调用时间。

**次要指标**（Secondary，≥ 2 个）：

- **可解释性覆盖率**：≥ 90% 检索结果包含 source、file_path、line_start、line_end、chunk_id、score、retrieval_method、last_modified、reason，用户可判断「为什么这段被召回」并快速定位原文。
- **跨 Agent 迁移保真**：至少支持 2 种 Agent / 上下文源导入 + 1 种标准化导出格式，迁移后结构化字段保留 ≥ 80%。
- **真实接入度**：≥ 3 个真实本地项目、≥ 1000 文件或 ≥ 10000 chunk、≥ 50 次真实检索、≥ 20 条 memory / context 治理记录、≥ 1 套 recall eval dataset。
- **检索性能**：已完成本地索引、未调用 embedding / reranker / 远程 provider 时，10 万 chunk 内 BM25 / metadata / filter P95 < 500ms。

**反指标**（Anti-metrics — 优化主指标时不能牺牲的，≥ 1 项）：

- 不能为提升命中率牺牲可解释性（禁止返回无 provenance 的「黑盒高分」结果）。
- 不能为索引速度牺牲 secret redaction（denylist / secret scan 不可被性能优化绕过）。
- 不能为多 Agent 兼容牺牲本地优先（不得默认把原文 / 索引上传云端换兼容性）。

---

## Open Questions｜开放问题

> ≥ 1 项。零 open question 通常是危险信号 — 说明思考还没到位。

- [ ] **O1 反对者抵制缓解策略**（源自 Q8，业务 / 产品层）：单 Agent 用户、极简主义者、隐私敏感者、团队平台负责人、Agent / memory provider 厂商、高级检索用户的抵制 → 需产品负责人在 v0.1 早期做 PM / UR 调研，定位首批种子用户画像与对外话术（「兼容 / 治理 / 评测层」而非「替代 memory」）。
- [ ] **O2 向量后端最终选型**（源自 D2 / 技术 TBD）：SQLite vec ext / Qdrant local / LanceDB / 内嵌 HNSW，需核心开发在 Phase 5-6 期间做 spike 压测后定。
- [ ] **O3 OpenClaw / Hermes / Cursor / Zed 实际 memory schema 与路径**（技术 TBD）：需在 Phase 3 开始前基于实测版本与真实工作区样本收集 fixture 确定适配范围。
- [x] **O4 MCP 协议 / SDK 目标版本**（技术 TBD）：需在 Phase 7 启动前跟随当时主流 spec / SDK 锁定。**Resolved by task-7.1 §2A Decision E**：锁 MCP spec `2025-06-18`（current per modelcontextprotocol.io 是 `2025-11-25` — 选熟成版本 trade-off；2025-11-25 newer features 接入留 future SPEC-DRIFT-task-7.1.spec-bump；client 高版本 → MCP initialize handshake 原生 negotiate down）。
- [ ] **O5 canonical record schema 无损承载边界**：迁移保真 ≥ 80% 字段，但哪些字段属「结构化必保留」、哪些可降级 free-text，需 Phase 3 实测样本后定标。
- [ ] **O6 golden questions 数据集构建与维护**：谁标注、覆盖哪些场景（配置定位 / 错误复现 / 历史决策 / 日志排查 / 跨 Agent memory 检索）、如何防过拟合，需在 Phase 4 前确定。
- [ ] **O7 v0.1 威胁模型边界**：本地单用户模式下哪些风险由 ContextForge 负责、哪些交给 OS / 用户环境（本地磁盘被其他进程读取、用户主动导出敏感 bundle、远程 provider opt-in 后数据外发、MCP client 权限边界），需在安全设计文档中明确。
- [ ] **O8 v0.1 P0 / P1 功能切分**：哪些能力必须进入首个可用版本，哪些仅作为 stretch goal？尤其是 MCP 完整能力、conflict detection、agent draft export、provider 对比评测是否全部进入 P0。
- [ ] **O9 canonical record 最小 schema**：`SourceRecord` / `ContextRecord` / `Chunk` / `RetrievalResult` 的边界、字段、版本号和兼容策略如何最终冻结？
- [ ] **O10 本地 API / MCP 安全边界**：daemon 监听地址、token、client allowlist、audit log 内容、远程 provider opt-in 数据外发提示如何设计？
- [ ] **O11 中英文与代码符号检索策略**：Tantivy tokenizer、CJK 处理、代码符号字段、路径 boost、exact match 如何实现和评测？
- [x] **O12 Phase 1-8 spec drift 击鼓传花机制如何在治理层提前发现**（ADR-013 §Follow-ups 新增）：v0.1 CLI 数据通路 spec drift 跨 Phase 1 / 2 / 6 / 8 击鼓传花（每 phase 把 CLI wire 推给下一 phase，到 task-8.3 §3 OOS 终点声明"历史 gap"但 AC2 仍勾选通过）。Phase 9 实施完成后产 governance retrospective：是否需要 ADR-014 引入"Phase 顶层 Exit Criteria 与 task 收口 AC 必须 cross-validation"机制？主 agent 自治在 spec-drift 检测层面的能力边界（ADR-012 把 §2A / merge / Waive 交给主 agent；spec drift 检测需跨 phase / 跨 task 视角，单 task 视角的主 agent 容易漏）。**Resolved by ADR-014** (cross-phase-exit-criteria-validation，Status=Accepted 2026-05-24)：D1 closeout mapping 表 + D2 `scripts/spec_drift_lint.sh` + D3 phase §6 verified by 显式 + D4 主 agent 自治补丁 + D5 历史不溯改；Phase 10 首次完整激活。
- [ ] **O13 ContextForge ↔ ContextForge-Console Contract v1 集成机制**（Phase 10 启动前提出）：Console v1.0 已 ship 但 HTTPAdapter 期望 ContextForge 端实现 9-19 REST endpoint + Workspace/IndexJob 资源模型，v0.2 ContextForge 端尚不提供；Console / ContextForge 双仓库 cross-repo 字段对齐如何 verifiable？v0.3 Phase 10 console-contract-v1 收口；详见 ADR-015。
- [ ] **O14 Console Real Data Plane gRPC bridge 演进路径**（Phase 11 启动前提出）：v0.3 Phase 10 task-10.4 §10 Trade-off #1 + #2 显式记录 Console business plane 仍 in-memory MemStore 模拟 + JobRunner 不真索引；v0.4 Phase 11 通过 ADR-016 cross-process Rust ↔ Go gRPC bridge 把 Workspace / IndexJob / Search / Events 业务面真接通（4 个新 gRPC service + JobRunner 真触发 IndexSession + retriever 真返回 indexed 分块 + EventBus 真接 progress）。v0.4.x Memory / Eval / source-chunks / search trace / workspace PATCH 等 endpoint 仍 `[SPEC-DEFER:console-endpoint-expansion]`；多实例 daemon leader election `[SPEC-DEFER:task-future.multi-daemon-leader-election]`。**Partially resolved by ADR-016** (cross-process-rust-go-via-grpc-bridge，Status=Proposed → Accepted at Phase 11 closeout)：D1 Rust 持 SoT + D2 4 gRPC service + D3 Go thin proxy + D4 MemStore env-gated fallback + D5 schema 单 owner = Rust + D6 沿用 ADR-014 cross-validation gate (第二次激活)。Endpoint expansion 留 v0.4.x。**Fully resolved by ADR-017 + Phase 12/13/14**（v0.5/v0.6/v0.7 ship 后 Console 22 endpoint 全 PASS）。
- [ ] **O15 Console v1 RFC3339Nano vs strict RFC3339 timestamp 字段对齐**（Phase 12 启动前提出 by ADR-017 D5）：Go `encoding/json` 对 `time.Time` 默认输出 RFC3339Nano (含纳秒)；Console v1 前端 Zod schema 若用 strict RFC3339 校验（不接受 nano）会 reject ContextForge 输出；ContextForge 端**不**改 marshal（避免自写 truncate）；Console 端建议改 Zod 为 `z.string().datetime({offset:true, precision:9})` 接受 nano。**Resolved by ADR-017 D5** (Status=Proposed → Accepted at Phase 14 closeout)：服务端保持 RFC3339Nano + cross-repo follow-up 通知 Console 团队 Zod relax。
- [ ] **O16 Memory v0.6 REST gating + importer 写入路径**（Phase 13 启动前提出 by ADR-017 D1 Wave 3）：Phase 5 memoryops dedup/lifecycle/audit Go-side 已 ship 但是「纯 transform 逻辑无持久化」（`internal/memoryops/lifecycle/lifecycle.go` 文件头注释「Phase 6 daemon 决定 in-memory cache / SQLite 持久化层归宿」）；Phase 13 task-13.1 一次建立 `memory_items` SQLite schema + SqliteMemoryStore + MemoryService gRPC + audit hooks；但 **importer 写入 memory_items 路径不在 Phase 13 scope**（importers Hermes / OpenClaw / agent-rules 当前不写本表）→ v0.6.0 ship 后 fresh install GET /v1/memory 可能返空数组 → Console UI 端 graceful degrade 显示「No memory items yet; import via CLI」。**Partially resolved by ADR-017 + Phase 13** (REST 表面 ship)；importer 写入路径 [SPEC-DEFER:phase-15.import-to-memory-items] 留 v0.6.x。
- [ ] **O17 Eval v0.7 EvalRun schema strict alignment + recall harness orchestration 路径**（Phase 14 启动前提出 by ADR-017 D1 Wave 4）：Phase 8 task-8.1 ship CLI `contextforge eval run` + `internal/eval/eval.go` recall harness；既存 `proto/contextforge/v1/eval.proto` 是 recall-only 二参 schema (EvalRequest{collection_id, golden_path} → EvalResponse{total, strong_hits, weak_hits, misses})，**不够 Console contractv1.EvalRun 9 字段**（含 status lifecycle / config_snapshot / case_results / metrics 等）。Phase 14 task-14.1 不动 v1/eval.proto，在 `console_data_plane v1/EvalService` 独立 ship 3 RPC + 完整 EvalRun schema；**EvalRunner spawn 路径选「Go console-api-serve 进程内 spawn goroutine + 通过 gRPC 反向 UpdateProgress」**（task-14.2）而非「Rust spawn_blocking 调 Go binary as subprocess」（OS process 管理复杂，留 v1.x）。**Resolved by ADR-017 D1 Wave 4 + Phase 14** (Status=Proposed → Accepted at Phase 14 closeout)；Rust native EvalRunner [SPEC-DEFER:phase-future.rust-native-eval-runner] 留 v1.x。
- [ ] **O18 Console contract 4 行为 trade-off 锁定**（Phase 12 启动前提出 by ADR-017 D2/D3/D4）：v0.5.0 ship 前必须锁定的 4 处行为 trade-off：(a) X-Confirm OR 语义服务端兜底 412 (D2)；(b) cancel 200→204 切换 (D3, Console HTTPAdapter v1.0 已 200/204 双 check)；(c) Observability events long-poll only, SSE defer v1.x (D4)；(d) RFC3339Nano kept, Console Zod relax (D5)。**Resolved by ADR-017 D2/D3/D4/D5 + Phase 12** (Status=Proposed → Accepted at Phase 14 closeout)；4 行为锁定，避免 v0.5/v0.6/v0.7 3 phase 内行为漂移。

---

## Technical Risks｜技术风险

> ≥ 3 项。

| # | 风险 | 概率 | 影响 | 缓解策略 |
|---|---|---|---|---|
| R1 | Go↔Rust local gRPC 边界复杂度：契约演进 / 版本错配 / 进程生命周期（daemon 起停、core 崩溃恢复） | 中 | 高 | Phase 1 先冻结 canonical record + gRPC proto 契约并版本化；core 崩溃 daemon 自动重启 + 健康检查；契约变更走 proto 兼容规则（仅加字段、不删不改 tag） |
| R2 | 向量后端选型悬而未决（TBD）：4 候选选错导致 P1 hybrid search 返工或性能不达标 | 中 | 中 | v0.1 用 provider 抽象隔离，P0 不依赖向量；Phase 5-6 期间做 1 周 spike，对 SQLite vec ext / Qdrant local / LanceDB / 内嵌 HNSW 在真实 10 万 chunk 数据集上压测后定；抽象层保证换后端不动检索 API |
| R3 | 检索召回率达不到 Top-5 ≥ 75% / Top-10 ≥ 85%：纯 BM25 / metadata 在代码 + 日志 + memory 混合语料上可能不足 | 中 | 高 | Phase 4 起持续跑 recall eval 监控；golden questions 按配置定位 / 错误复现 / 历史决策 / 日志排查 / Agent memory 检索分组统计，先分场景达标再看总分；chunking 策略可配置并对 code / markdown / log 分别调参；不达标时先优化 BM25 / metadata / filter，再启用 P1 本地 embedding；golden questions 早建早回归 |
| R4 | secret redaction 漏检或误报 → 敏感信息进索引 / 导出，或有效上下文被过度脱敏：pattern 覆盖不全可能漏掉自定义 token / base64 凭证 / URL 内嵌 credential；pattern 过宽可能误伤普通配置、错误日志和代码片段 | 中 | 高 | denylist 路径优先作第一道防线（不读 `.env` / `.ssh` / `*.key`）；pattern 可扩展 + export 二次扫描 + 结果默认不展示完整 secret；redaction 结果保留占位符和类型标签如 `[REDACTED:GITHUB_TOKEN]`；提供 `scan --dry-run` 预检和 allow override（override 必须写入 audit log）；audit log 可追溯 |
| R5 | 外部 Agent schema 不稳定（OpenClaw / Hermes / Cursor / Zed 版本漂移）：适配器对实测样本编码，上游一变即失效 | 高 | 中 | importer 分层：通用 file / markdown fallback 永远可用，schema-aware 解析为增量增强；不识别降级 + 警告不中断；每 importer 带版本探测 + 样本 fixture 回归；canonical record 与 importer 解耦 |
| R6 | 大仓库索引性能 / 资源不达标：1 万文件基础索引 <10min、10 万文件进入长任务模式、idle <300MB、基础索引 <2GB 在真实 monorepo + 大日志下可能不达标 | 中 | 中 | Phase 2 起以真实大仓库为基准持续测；流式分块 + 单文件大小上限 + 默认排除 node_modules / .git / target；超阈值自动降级后台长任务；Phase 8 专项性能 / 资源回归 |
| R7 | MCP 协议 / SDK 漂移（TBD）：spec 与主流 SDK 仍演进，Phase 7 对接版本可能在 v0.1 周期内变 | 中 | 中 | mcp-adapter 与核心检索解耦（adapter 仅做协议翻译）；锁定一个已发布 spec 版本并标注兼容范围；协议变更只动 adapter 层不动 retriever / daemon |
| R8 | 中文 / 英文 / 代码符号混合检索质量不稳定：中文 query、英文日志、代码符号、路径、错误码、snake_case / camelCase 混合语料可能导致默认 tokenizer 效果不足 | 中 | 高 | v0.1 支持 configurable tokenizer；路径、文件名、扩展名、symbol 单独建 field 并 boost；支持 exact phrase / exact symbol search；中文 Markdown 使用 CJK-aware tokenizer 或 n-gram fallback；eval dataset 必须覆盖中文 query、英文 query、代码符号 query |
| R9 | 本地 daemon / MCP 暴露面风险：REST / MCP 若监听范围、token、client allowlist 设计不当，可能让本地其他进程读取敏感上下文 | 中 | 高 | daemon 默认只监听 127.0.0.1 或 Unix socket；禁止默认 0.0.0.0；REST API 使用本地随机 token；MCP client 显式 allowlist；audit log 对 query / export 做脱敏；远程 provider opt-in 时显示外发数据类型 |

---

## Next Steps｜后续步骤

1. **审本 PRD 内容**（重点：阶段表 / 决策日志 / 风险 / Exit Criteria）
2. **后续路径**：参见 S2V SKILL.md 的「项目识别流程」，按当前项目状态决定接 `/s2v-init`，或用 `/s2v-add` 逐项补增量产物

> ⚠️ task spec 实施完后留在原地不归档（SDD 单一事实源核心要求）。
