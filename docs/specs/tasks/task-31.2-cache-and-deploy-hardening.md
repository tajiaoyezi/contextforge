# Task `31.2`: `cache-and-deploy-hardening — embedding-cache LRU/cap（cache.rs 无界 HashMap）+ Go memstore cache cap 可配置（memstore.go 硬编码 256）+ production compose 资源限（mem_limit/cpus）+ 可选 TLS 终止反代`

**Status**: Draft

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 31 (governance-debt-cleanup)
**Dependencies**: 既有 `core/src/embedding/cache.rs`（task-22.2 CachingEmbeddingProvider，Phase 22 已交付）/ `internal/consoleapi/memstore.go`（task-15.1 fallback chunk/trace 缓存 FIFO，Phase 15 已交付）/ `deploy/docker-compose.production.yml` + `docs/deploy/production.md`（task-16.4 production compose，Phase 16 已交付）/ ADR-027（embedding-provider-completion，cache LRU add-only Amendment @ task-31.4）/ ADR-004（local-first-privacy-baseline，默认行为 + 既有契约不变）/ ADR-016（Go thin proxy + Rust SoT 两进程布局，compose 加固不改拓扑）/ ADR-012（main-agent-governance-autonomy，真实 cert 须域名 / outward-facing 须用户授权）/ ADR-013（禁伪造凭据红线——真实 TLS cert / 实测内存上界不伪造）/ ADR-014 D1-D5（第二十二次激活）

## 1. Background

跨 Phase 累积的缓存 / 部署治理债，本 task 聚焦四项 code-local + compose-config 加固：

- **B1 embedding-cache 无界**：`core/src/embedding/cache.rs` 的 `CachingEmbeddingProvider.mem`（`Mutex<HashMap<String, Vec<f32>>>`，`:23`）为无界 L1——`embed` 在 L1 miss + inner 回填时 `mem.lock().unwrap().insert(...)`（`:154` L2 promote、`:170` inner write-through）只增不减；L2 SQLite `INSERT OR REPLACE`（`:99-104`）同样无界。长跑 daemon（console-api / 索引热路径）对大量不同文本 embed 会令进程内存随唯一文本数线性无上界增长。
- **B2 Go memstore cache cap 硬编码**：`internal/consoleapi/memstore.go` 的 `const memStoreCacheDefaultCapacity = 256`（`:49`，doc comment `:46-48` 已带 `[SPEC-DEFER:phase-future.cache-cap-configurable]`）；`cacheCapacity` 字段（`:41`）在 `NewMemStore`（`:57`）赋值，FIFO 驱逐在 `cacheChunkUnlocked`（`:73-77`）/ `cacheTraceUnlocked`（`:93-97`）强制。容量不可经 config / env 调整，运维无法据主机内存放大/缩小 fallback 缓存。
- **B3 production compose 无资源限**：`deploy/docker-compose.production.yml` 两服务（`contextforge-core` `:20-43`、`console-api-serve` `:45-68`）均未设 `mem_limit` / `cpus` / `deploy.resources`；多租户 / 共享主机下单服务可吃满宿主内存。`docs/deploy/production.md:383-391` 已以 `[SPEC-DEFER:phase-future.compose-resource-limits]` 记此缺口并给样例。
- **B4 compose 无 TLS 终止**：`console-api-serve` 明文绑定 `0.0.0.0:48181`（`:50-58`），栈本身不做 TLS；`docs/deploy/production.md:165-167` 仅以散文建议「由 nginx / traefik / caddy 反代前置鉴权」，compose 无配套反代服务。

经核 Phase 26 已交付 event-bus partition/capacity（`core/src/data_plane/events.rs`），与本 task 无关；本 task 范围内的 B1/B2 为 code-local 🟢 可单测，B3 compose-config parse 🟢，B4 中 compose-config parse 🟢 而真实 cert 签发须域名 🟡 honest-defer。

## 2. Goal

(1) **B1**：为 `CachingEmbeddingProvider` 的 L1（并可选 L2）加容量上界 + LRU（或 capacity-capped）驱逐策略——插入超 cap 后最旧 key 被逐出，被逐 key 再次 embed 时 inner 被重新调用（miss）。默认行为对未触上界的工作流不变（同文本仍命中）。(2) **B2**：把 Go memstore cache cap 经 config / env 暴露（替代硬编码 256），未设时保留默认 256。(3) **B3**：为 production compose 两服务加（文档化 / 可选）`mem_limit` + `cpus`。(4) **B4**：加一个可选 TLS 终止反代服务（caddy / traefik）或文档化 cert-mount，`docs/deploy/production.md:165-167` 散文升级为可落地 compose 片段；真实 cert 签发须域名 🟡 honest-defer。

pass bar：B1/B2 经确定性单测验证（🟢）；B3/B4 经 `docker compose config` parse 验证 compose 合法（🟢），真实 cert 签发 🟡 据真实跑出后回填 / `[SPEC-DEFER:phase-future.compose-tls-auto-cert]`；默认行为 / proto / 既有契约不变（ADR-004）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `core/src/embedding/cache.rs`——`CachingEmbeddingProvider` L1（`mem` 字段 `:23`）由无界 `HashMap` 改为容量上界的 LRU（或 capacity-capped）结构；`new` / `with_sqlite` 接受（或经默认常量）cap；`embed` 回填路径（`:154` L2 promote、`:170` inner write-through）在插入时执行 LRU 驱逐；可选对 L2 SQLite（`:99-104`）加行数上界（add-only，按需）。默认 cap 取合理值（不破现有命中行为）。
- 改 `internal/consoleapi/memstore.go`——cache cap 经 config / env（如 `CONTEXTFORGE_CONSOLEAPI_CACHE_CAP`）可配置，替代硬编码 `memStoreCacheDefaultCapacity = 256`（`:49`）；`NewMemStore`（`:57`）读取配置，未设时回落 256；`cacheCapacity` 字段（`:41`）+ FIFO 驱逐（`:73-77` / `:93-97`）逻辑沿用。`:46-48` doc comment 的 `[SPEC-DEFER:phase-future.cache-cap-configurable]` 标注随实现移除（兑现）。
- 改 `deploy/docker-compose.production.yml`——两服务（`:20-43` / `:45-68`）加（可选 / 文档化）`mem_limit` + `cpus`；加一个可选 TLS 终止反代服务（caddy / traefik）section 或 cert-mount 注释。既有 image / command / healthcheck / depends_on / restart / volumes 拓扑不动（ADR-016）。
- 改 `docs/deploy/production.md`——`:383-391` 资源限 `[SPEC-DEFER]` 散文升级为已落地配置说明；`:165-167` TLS 散文升级为可落地反代 compose 片段说明（真实 cert 签发段保留 honest-defer 标注）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- 真实域名 + 真实 TLS cert 自动签发（Let's Encrypt / ACME 实跑）[SPEC-DEFER:phase-future.compose-tls-auto-cert]——须真实域名 + 可达 80/443，🟡 live-env；本 task 验 compose-config parse + cert-mount 路径，真实签发待实测回填。
- multi-arch（arm64）原生 runner [SPEC-DEFER:phase-future.multi-arch-native-runner]（task-28.1 已据实延后；task-31.3 §3 重申）
- embedding-cache 跨进程 / 分布式共享（Redis 等外部缓存）[SPEC-DEFER:phase-future.distributed-embedding-cache]
- L2 SQLite 缓存 TTL / 主动过期（本 task 仅按需加行数上界，时间维度过期延后）[SPEC-DEFER:phase-future.l2-cache-ttl]
- Go memstore 由 FIFO 升级为真 LRU（access-order）[SPEC-DEFER:phase-future.memstore-true-lru]——本 task 为 cap 可配置，驱逐策略沿用 FIFO
- 真实 release tag / run-id / digest（v0.24.0）[SPEC-OWNER:task-31.4-closeout]（ADR-012 用户授权后回填）

## 4. Actors

- 主 agent（ADR-012 自治；真实 cert / outward-facing 须用户授权）
- `CachingEmbeddingProvider`（`core/src/embedding/cache.rs`，L1/L2 缓存 LRU 驱逐）
- `MemStore`（`internal/consoleapi/memstore.go`，fallback chunk/trace 缓存 cap 可配置）
- `deploy/docker-compose.production.yml`（资源限 + 可选 TLS 反代服务）
- `docker compose config`（compose 合法性 parse 校验）
- 运维 / 部署者（据主机内存调 cap + 据域名挂 cert）

## 5. Behavior Contract

### 5.1 Required Reading

- `core/src/embedding/cache.rs:23`（`mem: Mutex<HashMap<String, Vec<f32>>>` 无界 L1）+ `:99-104`（L2 `INSERT OR REPLACE` 无界）+ `:154`（L2 hit promote 入 L1）+ `:170`（inner miss write-through 入 L1）
- `internal/consoleapi/memstore.go:48-49`（`memStoreCacheDefaultCapacity = 256` const + doc `[SPEC-DEFER]`）+ `:73-77`（chunk FIFO 驱逐）+ `:93-97`（trace FIFO 驱逐）+ `:41`（`cacheCapacity` 字段）+ `:57`（`NewMemStore` 赋值点）
- `deploy/docker-compose.production.yml:20-68`（两服务 section——`contextforge-core` `:20-43`、`console-api-serve` `:45-68`，均无资源限 / TLS）
- `docs/deploy/production.md:165-167`（TLS 散文「由反代前置」）+ `:383-391`（资源限 `[SPEC-DEFER]` + 样例片段）
- `docs/decisions/adr-027-*.md`（embedding-provider；cache LRU add-only Amendment 落点 @ task-31.4）+ `docs/decisions/adr-036-governance-debt-cleanup.md §D2`（本 task 即其原文实现）

### 5.2 关键设计 — 缓存有界 + compose 加固（默认行为不变）

- **B1 LRU 容量上界**：L1 由 `HashMap` 改为容量上界结构（LRU 链或 capacity-capped map + 访问/插入序列）。pass bar 测试：cap=N，连续 embed N+1 个互异文本 → 最旧（首插入）key 被驱逐 → 对被逐文本再 embed 触发 inner **重新调用**（断言 inner 调用计数增加 = miss）；对仍在缓存内的文本 embed **不**触发 inner（仍命中）。0 新 dep 优先（手写 LRU 或既有 `std` 结构）；若引入 LRU crate 须在 §10 记 ADR-008 Amendment 必要性（优先 0 新 dep）。
- **B2 cap 可配置**：`NewMemStore` 读 env / config（`CONTEXTFORGE_CONSOLEAPI_CACHE_CAP`，解析失败 / 未设 → 256），写入 `cacheCapacity`；FIFO 驱逐阈值（`:73-77` / `:93-97`）用该字段（已是字段引用，无须改逻辑）。pass bar：设 env=2 → 插 3 个 chunk → 缓存仅留 2（最旧逐出）；未设 → 默认 256 行为不变。
- **B3 资源限**：两服务加 `mem_limit` + `cpus`（compose v2 顶层键，非 `deploy.resources`——后者仅 swarm 生效）；值经 env 可覆盖（如 `${CORE_MEM_LIMIT:-2g}`），默认值文档化。pass bar：`docker compose config` parse 通过且渲染含 `mem_limit`。
- **B4 可选 TLS 反代**：加一个 `profiles: [tls]` 下的 caddy（或 traefik）服务，反代 `console-api-serve:48181`，监听 443，cert 经 volume mount（或 caddy ACME 自动）；默认 profile 不启该服务（不破现有明文部署）。pass bar：`docker compose --profile tls config` parse 通过；真实 cert 签发须真实域名 🟡 `[SPEC-DEFER:phase-future.compose-tls-auto-cert]`，compose-config parse 🟢 本 task 验。

### 5.3 不变量

- 默认行为不变（ADR-004）：未触缓存上界的工作流命中行为与改前一致；Go memstore 未设 env 时默认 256；compose 默认 profile（无 `--profile tls`）拓扑 + 明文端口不变。
- 既有契约不变：`EmbeddingProvider` trait / `CachingEmbeddingProvider` 公共构造签名兼容（cap 经默认常量或可选参数，调用方不破）；compose 既有 image / command / healthcheck / depends_on / restart / volumes 不动（ADR-016）。
- 缓存内存有界：L1 改后任意 embed 序列下 L1 条目数 ≤ cap（实测上界 / 内存上界 待实测回填，不伪造数值）。
- 0 新代码依赖优先（手写 LRU / 既有 `std`）；compose / docs 改动无 Cargo / go.mod 依赖增量。
- 真实 cert / 真实域名 outward-facing 不可逆 → 不自行签发（ADR-012）。

## 6. Acceptance Criteria

- [ ] AC1（embedding-cache LRU/cap）: `CachingEmbeddingProvider` L1（`cache.rs:23`）改为容量上界 LRU——插入超 cap 后最旧 key 驱逐，被逐 key 再 embed 时 inner 重新调用（miss）、仍在缓存内的 key 不触发 inner；默认 cap 不破现有命中；0 新 dep 优先 — verified by TEST-31.2.1
- [ ] AC2（Go memstore cache cap 可配置）: `memstore.go` cache cap 经 config / env 可配置（替代硬编码 256，`:49`），未设时默认 256 行为不变；`:46-48` `[SPEC-DEFER:phase-future.cache-cap-configurable]` 标注随实现移除 — verified by TEST-31.2.2
- [ ] AC3（compose 资源限 + 可选 TLS proxy）: production compose 两服务（`:20-68`）加 `mem_limit` + `cpus`；加 `profiles: [tls]` 可选 TLS 终止反代服务；`docker compose config` 与 `docker compose --profile tls config` parse 通过；`production.md:165-167` / `:383-391` 散文升级为可落地说明；真实 cert 签发 🟡 `[SPEC-DEFER:phase-future.compose-tls-auto-cert]`（待实测回填） — verified by TEST-31.2.3
- [ ] AC4（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by TEST-31.2.4

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-31.2.1 | embedding-cache LRU/cap：插入超 cap → 最旧 key 驱逐 → 被逐 key 再 embed inner 重新调用（miss），仍在缓存内 key 不触发 inner；默认 cap 不破命中 | `core/src/embedding/cache.rs`（+ 单测模块） | Planned |
| TEST-31.2.2 | Go memstore cache cap 经 config/env 可配置（非硬编码 256）；设 env=2 → 缓存仅留 2；未设 → 默认 256 | `internal/consoleapi/memstore.go`（+ `*_test.go`） | Planned |
| TEST-31.2.3 | compose mem_limit/cpus + 可选 TLS proxy service；`docker compose config` + `--profile tls config` parse 通过；真实 cert deferred `[SPEC-DEFER:phase-future.compose-tls-auto-cert]` | `deploy/docker-compose.production.yml` + `docs/deploy/production.md` | Planned |
| TEST-31.2.4 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Planned |

## 8. Risks

- **R1（中）embedding-cache LRU 引入 0 新 dep 的可行性**：手写 LRU（链表 + map）须维护正确性 + 锁内序列更新。
  - **缓解**：优先 capacity-capped + 简单插入序列（FIFO-on-insert，与 Go memstore 同策略，确定性强）；若需真 access-order LRU 评估 0-dep 手写 vs LRU crate，引 crate 须 §10 记 ADR-008 Amendment 必要性（优先 0 新 dep）。stop-condition：正确性单测不过则 AC1 不标 `[x]`。
- **R2（中→🟡）真实 TLS cert 须真实域名**：ACME / Let's Encrypt 实跑须域名 + 可达 80/443，本环境无。
  - **缓解**：本 task 验 `docker compose --profile tls config` parse + cert-mount 路径合法（🟢）；真实 cert 签发 `[SPEC-DEFER:phase-future.compose-tls-auto-cert]`，据 ADR-013 不伪造签发结果，真实跑出后回填。stop-condition：真实签发未实跑则 AC3 仅就 compose-config parse 维度标记 + honest-defer 记录。
- **R3（低）默认行为回归**：cap 默认值过小致原本命中的工作流变 miss；compose 资源限过低致 OOM。
  - **缓解**：默认 cap 取不破现有命中的合理值（B1）+ 未设 env 默认 256（B2）+ 资源限默认值文档化且经 env 可覆盖（B3）；单测断言默认路径命中不变。
- **R4（低）compose v2 `mem_limit` vs `deploy.resources` 语义**：`deploy.resources` 仅 swarm 生效，单机 compose 须顶层 `mem_limit` / `cpus`。
  - **缓解**：用顶层键 + `docker compose config` parse 验证渲染正确。

## 9. Verification Plan

```bash
# 1. AC1 — embedding-cache LRU/cap（确定性单测：cap 上界 + 驱逐 + miss 重算）
cargo test -p contextforge-core embedding::cache

# 2. AC2 — Go memstore cache cap 可配置（env=2 驱逐 + 未设默认 256）
go test ./internal/consoleapi/ -run TestMemStoreCacheCap

# 3. AC3 — compose parse（默认 + tls profile）；真实 cert 🟡 deferred
docker compose -f deploy/docker-compose.production.yml config
docker compose -f deploy/docker-compose.production.yml --profile tls config
#    真实 TLS cert 签发须真实域名 [SPEC-DEFER:phase-future.compose-tls-auto-cert]（待实测回填）

# 4. 不退化（全量）
cargo test --workspace
go test ./...

# 5. AC4 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界**：B4 真实 TLS cert 签发须真实域名 + 可达 80/443（🟡 live-env），本环境不具备 → compose-config parse（🟢）本 task 验，真实签发 `[SPEC-DEFER:phase-future.compose-tls-auto-cert]`，据 ADR-013 不伪造签发凭据 / 结果。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Draft（待实施）

**计划改动文件**：
- `core/src/embedding/cache.rs`——L1 `mem`（`:23`）改容量上界 LRU；回填路径（`:154` / `:170`）插入时驱逐；可选 L2（`:99-104`）行数上界（add-only）。+ 单测模块（cap 上界 + 驱逐 + miss 重算）。
- `internal/consoleapi/memstore.go`——cache cap 经 config/env 可配置（替代 `:49` 硬编码 256）；`NewMemStore`（`:57`）读取 + 默认回落；`:46-48` doc `[SPEC-DEFER]` 标注随实现移除。+ `*_test.go`（env=2 驱逐 + 默认 256）。
- `deploy/docker-compose.production.yml`——两服务（`:20-68`）加 `mem_limit` + `cpus`；加 `profiles: [tls]` 可选 TLS 反代服务（caddy / traefik）。
- `docs/deploy/production.md`——`:383-391` 资源限 + `:165-167` TLS 散文升级为可落地说明（真实 cert 段保留 honest-defer 标注）。
- `docs/decisions/adr-027-*.md` cache LRU add-only Amendment 落点在 task-31.4 closeout（非本 task body）。

**§9 Verification 计划** (will record real evidence at impl)：
- AC1：`cargo test -p contextforge-core embedding::cache` —— cap 上界 + 最旧驱逐 + 被逐 key 再 embed inner 重算（miss）+ 仍在缓存 key 命中（inner 不调用）；实测内存上界 / 命中率 真实跑出后回填（ADR-013，不伪造数值）。
- AC2：`go test ./internal/consoleapi/` —— env=2 → 缓存仅留 2（最旧逐出）+ 未设 → 默认 256 行为不变。
- AC3：`docker compose config` + `docker compose --profile tls config` parse 通过（🟢）；真实 TLS cert 签发 🟡 `[SPEC-DEFER:phase-future.compose-tls-auto-cert]`，待实测回填（不伪造签发结果）。
- AC4：`bash scripts/spec_drift_lint.sh --touched origin/master` 0 未标注命中（CI spec-lint 权威）。
- 0 新 dep / 默认行为不变 / 既有契约不变 真实结果待实施回填。
