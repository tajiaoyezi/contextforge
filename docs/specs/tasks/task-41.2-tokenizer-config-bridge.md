# Task `41.2`: `tokenizer-config-bridge — internal/config/config.go add-only RetrievalConfig{Tokenizer} + Config.Retrieval + [retrieval] 段 encode/decode round-trip（镜像 VectorConfig/[vector]）+ cmd/contextforge/main.go setTokenizerEnv（镜像 setVectorEnv：[retrieval] tokenizer 非空且 CONTEXTFORGE_TOKENIZER 未设 → 导出，env-wins，无段/空值不导出 → Rust resolve_tokenizer 默认 code_cjk）接线 doServe/doMCP；tokenizer 非密钥；Rust core 0 toml dep`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 41 (tokenizer-default-on)
**Dependencies**: 既有 `internal/config/config.go`（`VectorConfig` :96-99 / `Config.Vector` :40 / `[vector]` encode :238-240 / decode `case "[vector]"` :269 + `assignVector` :435-451，task-34.2 add-only 段范式已在；`RerankerConfig` :73-78 / `setRerankerEnv` 镜像 task-38.2）/ 既有 `cmd/contextforge/main.go`（`setVectorEnv` :304-346 env-bridge 镜像源 + `setRemoteEnv` :356 + `setRerankerEnv` :404 + doServe :108-118 / doMCP :150-160 接线点，task-34.2/37.2/38.2 已在）/ 既有 `internal/config/config_test.go`（vector/reranker round-trip test 范式）/ task-41.1（`CONTEXTFORGE_TOKENIZER` 消费方 `resolve_tokenizer`，本 task 桥的 env 由其消费）/ ADR-046（tokenizer-default-on，本 task 即其 D2 config-bridge 原文实现）/ ADR-004（opt-out 通道，无段=默认 code_cjk）/ ADR-008（dep add-only，Rust 0 toml dep）/ ADR-013（禁伪造红线——env-wins / 无段不导出据实）/ ADR-012 / ADR-014 D1-D5（第三十二次激活）

## 1. Background

task-41.1 令 `CONTEXTFORGE_TOKENIZER` env 可控生产默认 tokenizer（unset → `code_cjk` 翻默认 / `"default"` → opt-out 回 `TEXT`），但 Go config.toml 尚无对应段——用户只能经 env 而非声明式 config 文件 opt-out / override。本 task 补 config 桥：

- **B1 既有 config 段范式（真实）**：`internal/config/config.go` 已有 `VectorConfig{Backend,Dim}`（:96-99）/ `RerankerConfig`（:73-78）/ `EmbeddingConfig`（:84-87）/ `RemoteProviderConfig`（:53-62）四个 add-only 段，各有 `Config.X` 字段（:38-41）+ `encodeTOML` `[x]` 段（:225-240）+ `decodeTOML` `case line == "[x]"`（:260-271）+ `assignX`（:356-451）+ round-trip test。
- **B2 既有 env-bridge 范式（真实）**：`cmd/contextforge/main.go` 已有 `setVectorEnv`（:304-346）/ `setRemoteEnv`（:356）/ `setRerankerEnv`（:404）三个跨进程 env-bridge：best-effort load config、`setIfAbsent(key,val)`（env-wins：env 已设则不覆盖、val 空则不导出）、missing config 静默 / 真 parse-err stderr WARN、返回 restore closure、doServe/doMCP 接线（:108-118 / :150-160）。
- **B3 桥语义（设计定性）**：翻默认在 **Rust 默认**（task-41.1 `resolve_tokenizer` unset → `code_cjk`）；Go config 仅作 **opt-out / override 通道**——无 `[retrieval]` 段 / 空值 → 不导出 `CONTEXTFORGE_TOKENIZER` → Rust 默认 `code_cjk`（翻默认生效）；显式 `[retrieval] tokenizer = "default"` → 导出 → Rust opt-out 回 `TEXT`。与 vector/reranker 桥同构（Rust env 是消费方、Go 桥 config→env、env-wins）。tokenizer 非密钥（不涉 API key 安全 baseline，与 remote/reranker 桥不同——无 key 排除字段）。

本 task add-only `RetrievalConfig` + `[retrieval]` 段 + `setTokenizerEnv`，🟢 可单测（config round-trip + env-bridge），Rust core 0 toml dep（复用既有跨进程 env-bridge）。

## 2. Goal

(1) **B1**：`internal/config/config.go` add-only `RetrievalConfig struct { Tokenizer string }`（toml `tokenizer`）+ `Config.Retrieval RetrievalConfig`（镜像 `VectorConfig`/`Config.Vector`）+ `encodeTOML` `[retrieval]` 段 + `decodeTOML` `case line == "[retrieval]"` + `assignRetrieval`（镜像 `assignVector`），Save/Load round-trip 等价。(2) **B2/B3**：`cmd/contextforge/main.go` add `setTokenizerEnv(dataDir string) func()`（镜像 `setVectorEnv`：load config best-effort、`setIfAbsent("CONTEXTFORGE_TOKENIZER", cfg.Retrieval.Tokenizer)`、env-wins、missing config 静默 / 真 parse-err stderr WARN、返回 restore）+ doServe/doMCP 接线（`restoreTok := setTokenizerEnv(opts.DataDir)` + defer）。无段 / 空值 → 不导出 → Rust `resolve_tokenizer` 默认 `code_cjk`。tokenizer 非密钥（无 key 字段）。Rust core 0 toml dep。

pass bar：`[retrieval] tokenizer` Save→Load round-trip 等价（🟢）；`setTokenizerEnv` env-wins（`CONTEXTFORGE_TOKENIZER` 已设 → 不覆盖）/ 无段 / 空值不导出 / 非空导出 `CONTEXTFORGE_TOKENIZER`（🟢，镜像 setVectorEnv test 形态）；既有 vector/reranker/embedding config round-trip + setVectorEnv/setRerankerEnv 不退化；Rust core 0 toml dep（ADR-008）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `internal/config/config.go`——add-only `RetrievalConfig struct { Tokenizer string }`（doc-comment：tokenizer 选择，bridge 到 spawned core daemon 的 `CONTEXTFORGE_TOKENIZER`，无段 → 不导出 → Rust 默认 `code_cjk`，env-wins，非密钥）+ `Config.Retrieval RetrievalConfig`（:38-41 段加字段）+ `encodeTOML` `b.WriteString("\n[retrieval]\n")` + `fmt.Fprintf(&b, "tokenizer = %s\n", tomlQuote(c.Retrieval.Tokenizer))`（镜像 `[vector]` :238-240）+ `decodeTOML` `case line == "[retrieval]": section = "retrieval"`（镜像 :269）+ `case "retrieval": assignRetrieval(...)`（镜像 :305-308）+ `func assignRetrieval(r *RetrievalConfig, key, raw string) error`（`case "tokenizer"` parseTOMLString，镜像 `assignVector` :435-451）
- 改 `cmd/contextforge/main.go`——add `func setTokenizerEnv(dataDir string) func()`（镜像 `setVectorEnv` :304-346：load config best-effort、`setIfAbsent` env-wins、`setIfAbsent("CONTEXTFORGE_TOKENIZER", cfg.Retrieval.Tokenizer)`、missing config 静默 / 真 parse-err stderr WARN、返回 restore closure）+ doServe（:108-118）`restoreTok := setTokenizerEnv(opts.DataDir)` + `defer restoreTok()` + doMCP（:150-160）同接线（镜像 setVectorEnv/setRerankerEnv 接线）
- 改 `internal/config/config_test.go`——`[retrieval] tokenizer` Save→Load round-trip 断言（镜像既有 vector/reranker round-trip test）
- 同源测试：`main_test.go` `setTokenizerEnv` env-wins / 无段不导出 / 非空导出 `CONTEXTFORGE_TOKENIZER`（镜像既有 `setVectorEnv` test 形态）

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- task-41.1 Rust `resolve_tokenizer` 消费逻辑（本 task 只桥 config→env，消费在 task-41.1）
- `[retrieval]` 段其它字段（如 hybrid / rerank toggle——本 task 仅 `tokenizer`）[SPEC-DEFER:phase-future.retrieval-config-more-fields]
- API key 处理（tokenizer 非密钥，无 key 字段——与 remote/reranker 桥不同）
- 真实 release tag / run-id / digest（v0.34.0）[SPEC-OWNER:task-41.3-closeout]（ADR-012 用户授权后回填）

## 4. Actors

- 主 agent（ADR-012 自治）
- `RetrievalConfig` / `Config.Retrieval`（`internal/config/config.go`，本 task 新增 add-only 段，镜像 `VectorConfig`/`Config.Vector`）
- `setTokenizerEnv`（`cmd/contextforge/main.go`，本 task 新增 env-bridge，镜像 `setVectorEnv` :304-346）
- `resolve_tokenizer`（`core/src/server.rs`，task-41.1 交付，消费本 task 桥的 `CONTEXTFORGE_TOKENIZER`）
- 用户 / 运维（经 config.toml `[retrieval] tokenizer = "default"` 声明式 opt-out 回 legacy `TEXT` / `"cjk_segmenter"` 升 jieba；env 仍 wins）

## 5. Behavior Contract

### 5.1 Required Reading

- `internal/config/config.go:96-99`（`VectorConfig{Backend,Dim}` add-only 段范式）+ `:40`（`Config.Vector` 字段）+ `:238-240`（encode `[vector]`）+ `:269`（decode `case "[vector]"`）+ `:305-308`（decode dispatch `case "vector"`）+ `:435-451`（`assignVector`）+ `:73-78`（`RerankerConfig` 无 key 字段范式）
- `cmd/contextforge/main.go:304-346`（`setVectorEnv` env-bridge 镜像源：load best-effort + setIfAbsent env-wins + missing 静默 / parse-err WARN + restore）+ `:404-441`（`setRerankerEnv` 无 key 镜像）+ `:108-118`（doServe 接线）+ `:150-160`（doMCP 接线）
- `core/src/server.rs`（task-41.1 `resolve_tokenizer` 消费 `CONTEXTFORGE_TOKENIZER`——本 task 桥的 env 由其消费）
- `docs/decisions/adr-046-tokenizer-default-on.md §D2`（本 task 即其 config-bridge 原文实现）+ `adr-004`（opt-out 通道，无段=默认 code_cjk）

### 5.2 关键设计 — config→env 桥（env-wins / 无段默认 code_cjk / 0 Rust toml dep）

- **B1 add-only `[retrieval]` 段**：`RetrievalConfig{Tokenizer string}` + `Config.Retrieval` + encode/decode round-trip，完全镜像 `VectorConfig`/`[vector]`（add-only，既有段 / 字段不动，向后兼容——既有 config.toml 无 `[retrieval]` 段 → 解码零值 `Tokenizer=""`）。
- **B2 setTokenizerEnv env-bridge**：镜像 `setVectorEnv`——`config.Load(dataDir)` best-effort（missing config = 正常默认、静默；真 parse/read err → stderr WARN）；`setIfAbsent("CONTEXTFORGE_TOKENIZER", cfg.Retrieval.Tokenizer)`（val 空 → 不导出；env 已设 → 不覆盖 = env-wins；导出成功 → 记 restore）；返回 restore closure；doServe/doMCP 接线 + defer restore。
- **B3 无段默认 code_cjk（翻默认生效）**：无 `[retrieval]` 段 / `tokenizer=""` → 不导出 `CONTEXTFORGE_TOKENIZER` → task-41.1 `resolve_tokenizer` unset → 默认 `code_cjk`（翻默认在 Rust 默认生效，Go 桥不阻碍）。显式 `[retrieval] tokenizer = "default"` → 导出 → resolve_tokenizer opt-out 回 `TEXT`。
- **非密钥（无 key 排除）**：tokenizer 非密钥，`RetrievalConfig` 无 api-key 字段（与 `RemoteProviderConfig`/`RerankerConfig` 的「API key env-only 不进 config」约束不同——tokenizer 本就可入 config）。
- **Rust core 0 toml dep**：复用既有跨进程 env-bridge（Go config→env，Rust 读 env），Rust core 不引 toml 解析 dep。

### 5.3 不变量

- 既有 config 段不变（ADR-004）：`[remote]`/`[reranker]`/`[embedding]`/`[vector]`/`[[collections]]` 段 + 顶层标量编解码不变；既有 config.toml（无 `[retrieval]` 段）解码 → `Config.Retrieval` 零值（`Tokenizer=""`）→ setTokenizerEnv 不导出 → Rust 默认 code_cjk。
- env-wins：`CONTEXTFORGE_TOKENIZER` 已在环境中 → setTokenizerEnv 不覆盖（显式 env 覆盖 config，向后兼容）。
- 无段默认 code_cjk：无 `[retrieval]` 段 / 空值 → 不导出 → Rust `resolve_tokenizer` 默认 `code_cjk`（翻默认生效）。
- 既有 env-bridge 不退化：`setVectorEnv`/`setRemoteEnv`/`setRerankerEnv`/`setDataDirEnv` 行为不变；doServe/doMCP 既有接线不破。
- Rust core 0 toml dep（ADR-008）：Go config→env 桥、Rust 读 env，无 Rust toml dep 增量。
- tokenizer 非密钥：`RetrievalConfig` 无 api-key 字段（tokenizer 可入 config，与 remote/reranker key-exclusion 不同）。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（config `[retrieval] tokenizer` round-trip 🟢）: `internal/config/config.go` add-only `RetrievalConfig{Tokenizer string}` + `Config.Retrieval` + `[retrieval]` 段 encode/decode；Save→Load round-trip 等价（含 `tokenizer = "code_cjk"` / `"default"` / `""`）；既有段 / 字段不变（既有 config.toml 无 `[retrieval]` → 零值 `Tokenizer=""`） — verified by **TEST-41.2.1**
- [x] **AC2**（setTokenizerEnv env-bridge 🟢）: `cmd/contextforge/main.go` `setTokenizerEnv`（镜像 `setVectorEnv`）：`[retrieval] tokenizer` 非空且 `CONTEXTFORGE_TOKENIZER` 未设 → 导出（env-wins：已设不覆盖）；无段 / 空值 → 不导出（→ Rust 默认 code_cjk）；missing config 静默 / 真 parse-err stderr WARN；doServe/doMCP 接线 + restore；既有 setVectorEnv/setRerankerEnv 不退化；Rust core 0 toml dep — verified by **TEST-41.2.2**
- [x] **AC3**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-41.2.3**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-41.2.1 | config `[retrieval] tokenizer` Save→Load round-trip 等价（`code_cjk` / `default` / 空）；既有段不变（无 `[retrieval]` → 零值） | `internal/config/config_test.go` | Done |
| TEST-41.2.2 | `setTokenizerEnv` env-wins（已设不覆盖）/ 无段 / 空值不导出（→ Rust 默认 code_cjk）/ 非空导出 `CONTEXTFORGE_TOKENIZER` / missing config 静默 / restore（镜像 setVectorEnv test）；既有 setVectorEnv/setRerankerEnv 不退化 | `cmd/contextforge/main_test.go` | Done |
| TEST-41.2.3 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）add-only 段破既有 config round-trip**：加 `[retrieval]` 段若编解码不对称或顺序错则破既有 config Save/Load。
  - **缓解**：完全镜像 `VectorConfig`/`[vector]`（add-only，既有段不动）；既有 config.toml 无 `[retrieval]` → 解码零值；TEST-41.2.1 断言 round-trip + 既有段不变。stop-condition：既有 config round-trip 退化则 AC1 不标 `[x]`。
- **R2（中）setTokenizerEnv 破 env-wins / 误覆盖**：若 setTokenizerEnv 覆盖已设 env 则破 env-wins、不一致于 setVectorEnv。
  - **缓解**：镜像 `setVectorEnv` 的 `setIfAbsent`（env 已设 → 不覆盖）；TEST-41.2.2 断言 env-wins。stop-condition：env-wins 破 / 既有 setVectorEnv 退化则 AC2 不标 `[x]`。
- **R3（中）无段误导出 / 空值导出抵消翻默认**：若无 `[retrieval]` 段或空值仍导出 `CONTEXTFORGE_TOKENIZER`（即便空），可能干扰 Rust resolve_tokenizer 默认。
  - **缓解**：`setIfAbsent` val 空 → 不导出；TEST-41.2.2 断言无段 / 空值不导出（→ Rust 默认 code_cjk）。stop-condition：无段 / 空值误导出则 AC2 不标 `[x]`。
- **R4（低）gofmt 对齐 / 跨平台 CRLF**：Go add-only 字段 / 函数若 gofmt 不齐则 CI gofmt 卡红（Windows autocrlf 本机 gofmt 不可靠）。
  - **缓解**：以 `dos2unix | gofmt -d`（LF）预检、CI gofmt 权威；TEST-41.2.3 D2 lint + CI gofmt gate。stop-condition：CI gofmt 红则不合并。

## 9. Verification Plan

```bash
# 1. AC1 — config [retrieval] tokenizer round-trip
go test ./internal/config/...

# 2. AC2 — setTokenizerEnv env-bridge（env-wins / 无段不导出 / 非空导出）
go test ./cmd/contextforge/...

# 3. 不退化（全量）
go test ./...
go vet ./...
gofmt -l .   # 以 CI(LF) 为权威；本机 autocrlf 用 dos2unix|gofmt -d 预检

# 4. AC3 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.tokenizer-config-bridge-defer-note]：本 task 交付 Go `[retrieval] tokenizer` config 桥（add-only 段 round-trip + `setTokenizerEnv` env-bridge env-wins）+ doServe/doMCP 接线，🟢 可单测，Rust core 0 toml dep。`[retrieval]` 段其它字段 `[SPEC-DEFER:phase-future.retrieval-config-more-fields]` 不在本 task 范围；翻默认语义在 Rust 默认（task-41.1），Go 桥仅 opt-out / override 通道（无段 → Rust 默认 code_cjk）；tokenizer 非密钥（无 key 字段，与 remote/reranker 桥不同）；实测产物（v0.34.0）真实跑出后回填。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification（PR #263，master @ `2cead8b`，真实证据）**：
- AC1：`go test ./internal/config/... -run TestTask412RetrievalConfig` —— PASS（`[retrieval] tokenizer` Save→Load round-trip code_cjk / default / cjk_segmenter 保真 + 既有 `[vector]`/`[reranker]`/`[[collections]]` 段不受影响 + 旧 config 无 `[retrieval]` 段向后兼容 zero value）。
- AC2：`go test ./cmd/contextforge/... -run TestSetTokenizerEnv` —— PASS（`setTokenizerEnv` 非空导出 `CONTEXTFORGE_TOKENIZER` + restore unset / env-wins 已设不覆盖 / 空段不导出 → core 默认 code_cjk）；既有 `setVectorEnv`/`setRerankerEnv`/config round-trip 不退化（`go test ./...` 全过）。
- AC3：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威，PR #263 spec-lint pass）。
- Rust core 0 toml dep（ADR-008）/ 无段默认 code_cjk / env-wins / tokenizer 非密钥（无 api-key 字段）；`go vet ./...` clean；`gofmt` 0 diff（4 文件 LF-normalized）。

**实际改动文件**（PR #263）：
- `internal/config/config.go`——add-only `RetrievalConfig{Tokenizer}` + `Config.Retrieval` + `[retrieval]` encode/decode（镜像 `VectorConfig`/`[vector]`）+ `assignRetrieval`。
- `cmd/contextforge/main.go`——add `setTokenizerEnv`（镜像 `setVectorEnv`）+ doServe/doMCP 接线。
- `internal/config/config_test.go` + `cmd/contextforge/main_test.go`——round-trip + env-bridge test。
