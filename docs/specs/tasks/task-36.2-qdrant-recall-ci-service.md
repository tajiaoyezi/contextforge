# Task `36.2`: `qdrant-recall-ci-service — 向 .github/workflows/ci.yml 新增 qdrant-recall job，用 qdrant SERVICE CONTAINER（services: qdrant: image qdrant/qdrant，端口 6334:6334 + 6333:6333）+ Rust 1.93 + install protoc，每次 CI run 跑 QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture，对 live service container 实测 task-36.1 recall harness——CI 从此 HAS a live server，永久兑现并关闭 [SPEC-DEFER:phase-future.qdrant-server-lifecycle]（ADR-034 D2）；CI-only / add-only / 默认 build + 默认行为不变（vector-qdrant 仍 opt-in，0 新 dep，ADR-004/008）；验证实证 = 本 PR 自身 live CI run（据实记录，ADR-013）`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 36 (qdrant-live-vector-recall)
**Dependencies**: task-36.1（`core/tests/qdrant_live_recall.rs` env-gated live recall harness——`#![cfg(feature = "vector-qdrant")]`，读 `QdrantConnConfig::from_env()`（`QDRANT_URL` + 可选 `QDRANT_API_KEY`，TLS 由 https scheme 推断）；`health() != Ready` 时 eprintln skip notice + return（honest-defer 干净退出，非 fail）；`health() == Ready` 时把同一 deterministic 语料同时索引进 `QdrantBackend`（ensure-create open + `index_batch`）与 `BruteForceVectorBackend`，对 M 条 deterministic query 量 recall@k = mean(|qdrant_topk ∩ exact_topk| / k) 并断言 >= floor + eprintln 实测数；本 task 给该 harness 接 live service container 令其每次 CI run 跑出真实数）/ 既有 `.github/workflows/ci.yml`（task-28.3 / BUILD-4 审计后既有 5 job：`cargo-test` / `go-test` / `spec-lint` / `lint` / `feature-build` 矩阵——`feature-build` 已含 `vector-qdrant` 的 `cargo build` build-check + `Install protoc` 步骤；本 task add-only 第 6 个 `qdrant-recall` job，既有 5 job 不改）/ `core/src/retriever/vector/qdrant.rs`（`QdrantBackend::connect` / `health()` / ensure-create `open` / `index_batch` / `search`，task-25.1/18.4 落地，本 task 不改）/ ADR-041（qdrant-live-vector-recall；本 task = 其 §D3 CI service-container 集成原文实现，ratify @ task-36.3 closeout）/ ADR-034 D2（`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` honest-defer——本 task 关闭 CI-no-server 这一前提，令该 defer 永久兑现，但 NOT retro-edit ADR-034 D-body，per ADR-014 D5）/ ADR-004（local-first-privacy-baseline，默认 build 0-vector-dep / 默认行为不变）/ ADR-008（dep add-only，本 task = 0 新 dep）/ ADR-013（禁伪造红线——验证实证 = 本 PR 自身 live CI run，据实记录，不预填 run-id / 召回数）/ ADR-012（main-agent-governance-autonomy）/ ADR-014 D1-D5（第二十七次激活）

## 1. Background

Phase 36（qdrant-live-vector-recall）的主题是**为已实现但从未在 CI live server 上端到端跑过的 qdrant KNN 召回补上永久护栏**。`core/src/retriever/vector/qdrant.rs` 的 `QdrantBackend` 自 Phase 25/29 起已完整实现 connect / health / open（ensure-create via `decide_ensure`）/ `index_batch`（upsert）/ `search`（KNN，cosine）/ delete；但真实 live 端到端 KNN 召回经 ADR-034 D2 honest-defer 标记为 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`——理由是 **CI 无在跑的 qdrant server**。仓库内仅有的召回数（`core/tests/eval_integration.rs` 的 0.7 / 0.85）是 **synthetic fixture**，非真实 ANN 召回。task-36.1 已建好 env-gated live recall harness（qdrant HNSW ANN recall@k vs BruteForce exact KNN，deterministic 可复现语料，无 server 时 honest-defer 干净跳过）。本 task 是 Phase 36 关闭该 defer 的**第二步、也是把临时实证升格为永久护栏的关键一步**：把 harness 接进 CI 的 qdrant SERVICE CONTAINER，令它**每次 CI run** 都对一个真实 qdrant server 跑出真实召回——CI 从此 HAS a live server，`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 永久兑现并关闭。

- **B1 既有 harness 只在「有 server 时」才量真实召回（task-36.1）**：task-36.1 的 `core/tests/qdrant_live_recall.rs` 在 `health() != Ready` 时 eprintln skip notice + return（honest-defer：本机 / CI 无 server 时干净跳过，NOT fail）。这是诚实的——但意味着 **CI 默认环境（无 server）下 harness 永远走 skip 分支**，真实 recall 数永远跑不出来、永远无护栏。要让 harness 真跑、recall 永久被验证，CI 必须**提供一个 live qdrant server**。
- **B2 GitHub Actions service container = CI 提供 live server 的标准手段**：GitHub Actions 的 `services:` 块在 job 运行前拉起一个容器（这里 `qdrant/qdrant`）并把其端口映射到 runner localhost，job 步骤可经 `localhost:<port>` 连接。这正是「让 CI HAS a live server」的标准、可复现手段——无需自建 runner、无需手动 musl 二进制（spike 曾用）、无需 dev-box。qdrant 监听 gRPC `6334`（`QdrantBackend` 用 `Qdrant::from_url(...:6334)`）+ REST `6333`，本 task `services: qdrant` 同时映射两端口（gRPC 主用，REST 备查）。
- **B3 DE-RISK 已由 lead 验证（真实 round-trip 通过）**：本 task 的可行性已被 de-risk——lead 已实测真实 qdrant + `qdrant-client` 1.18 端到端 round-trip 通过、KNN 正确：query `[1,0,0,0]` 返回 `[(a, 1.0), (c, 0.994)]`，cosine 排序正确。即「真实 qdrant server + 本仓库 client + KNN 语义」这条链路本身已被证实跑通；本 task 是把这条已证链路接进 CI service container 并令 harness 自动跑，不是从零探索可行性。
- **B4 既有 feature-build job 已含 vector-qdrant build-check + protoc 步骤（复用形态）**：`.github/workflows/ci.yml` 的 `feature-build` 矩阵 job（task-28.3 / BUILD-4 审计）已对 `vector-qdrant` 跑 `cargo build -p contextforge-core --features vector-qdrant`（仅 compile/link，NO tests），并已有 `Install protoc（lancedb build.rs needs a system protoc）` 步骤范式。本 task 新增的 `qdrant-recall` job 镜像该形态（checkout + Rust 1.93 + install protoc + cargo cache），区别在于：(a) 附带 `services: qdrant`；(b) 跑的是 `cargo test ... --test qdrant_live_recall -- --nocapture`（真实测试 + recall），而非 `cargo build`（build-check）。两者职责不同：`feature-build` 守 vector-qdrant 可编译，`qdrant-recall` 守 vector-qdrant 真实召回——add-only、互补、不返工既有 job。

本 task 为 **CI 配置改动**（`.github/workflows/ci.yml` add-only 一个 job），0 新 dep（vector-qdrant 自 task-18.4 起即 optional）/ 0 proto / 0 schema migration / 0 源码改动 / 默认 build + 默认行为不变；验证实证 = **本 PR 自身的 live CI run**（recall job 跑过 harness 且绿 = 真实证据，run-id / 真实召回数真实跑出后据实记录，ADR-013 不预填、不伪造）。

## 2. Goal

(1) **B1/B2**：向 `.github/workflows/ci.yml` add-only 一个 `qdrant-recall` job——`runs-on: ubuntu-22.04`，`services: qdrant: { image: qdrant/qdrant, ports: ["6334:6334", "6333:6333"] }`，步骤镜像 `feature-build` 形态：`actions/checkout@v4` + `dtolnay/rust-toolchain@stable`（`toolchain: '1.93'`）+ `Install protoc`（`sudo apt-get update && sudo apt-get install -y protobuf-compiler`）+ `actions/cache@v4`（cargo），最后跑 `QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`。该 job 令 task-36.1 harness 在每次 CI run 对 live service container 跑出真实 recall@k 并经 floor 断言守护。(2) **关闭 defer（B3）**：service container 提供 live server 后，`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（ADR-034 D2）的「CI 无 live server」前提被解除——defer 永久兑现并关闭；在 task-36.3 / ADR-034 add-only Phase-36 Amendment 据实标记其 D2 fulfilled（live KNN recall measured + CI-guarded），**不** retro-edit ADR-034 D-body（ADR-014 D5）。(3) **add-only / 默认不变（B4）**：既有 5 job（`cargo-test` / `go-test` / `spec-lint` / `lint` / `feature-build`）逐字符不改；`qdrant-recall` 是第 6 个 add-only job；默认 build 仍 0-vector-dep（vector-qdrant 仅在本 job 内 `--features` 开），默认行为 + 默认 build dep 集不变（ADR-004/008）。(4) **honest 验证（ADR-013）**：本 task 是 CI 配置改动，验证实证 = 本 PR 自身的 live CI run（`qdrant-recall` job 跑 harness 且绿）；run-id / 真实召回数 **真实跑出后**据实回填（§10 + v0.29.0 evidence），绝不预填、不伪造 live-server 通过。

pass bar：`.github/workflows/ci.yml` 新增 `qdrant-recall` job（`services: qdrant/qdrant` + 6334/6333 端口 + Rust 1.93 + install protoc + `QDRANT_URL=http://localhost:6334 cargo test ... --features vector-qdrant --test qdrant_live_recall -- --nocapture`）；本 PR 自身 CI 的 `qdrant-recall` job 绿（harness 对 live service container 真跑、recall@k >= floor、`--nocapture` 打印真实数）= TEST-36.2.1 的真实实证；既有 5 job 不退化（默认 build + 默认行为不变，0 新 dep）；`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 经 service container 永久兑现（task-36.3 / ADR-034 Amendment 据实标 D2 fulfilled）；ADR-014 D2 lint PR 触及行 0 未标注命中。

## 3. Scope

### In Scope（计划交付）

- 改 `.github/workflows/ci.yml`——add-only 一个 `qdrant-recall` job：
  - `runs-on: ubuntu-22.04` + `timeout-minutes`（与既有 job 量级一致，如 30）。
  - `services: qdrant: { image: qdrant/qdrant, ports: ["6334:6334", "6333:6333"] }`——CI 拉起 live qdrant service container（gRPC 6334 主用 + REST 6333 备查），端口映射到 runner localhost。
  - 步骤镜像 `feature-build` job 形态：`actions/checkout@v4` → `dtolnay/rust-toolchain@stable`（`toolchain: '1.93'`）→ `Install protoc`（`sudo apt-get update && sudo apt-get install -y protobuf-compiler`）→ `actions/cache@v4`（cargo registry/git/target，key 含 `qdrant-recall` 区分）。
  - 最后一步跑 harness：`QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`——令 task-36.1 harness 检测到 `health() == Ready`（service container 在跑）走真实召回分支，量 recall@k >= floor 并 `--nocapture` 打印真实数。
- 既有 5 job（`cargo-test` / `go-test` / `spec-lint` / `lint` / `feature-build`）**逐字符不改**（add-only，§5.3 不变量）。
- 验证 = 本 PR 自身的 live CI run：`qdrant-recall` job 在本 PR CI 上跑 harness 且绿（TEST-36.2.1，验证实证 = 真实 CI run，ADR-013）。
- 真实 recall@k 数 + 本 PR `qdrant-recall` run-id / run link **真实跑出后**据实回填 §10 + v0.29.0 evidence（绝不预填，ADR-013）。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- task-36.1 harness 本体（`core/tests/qdrant_live_recall.rs` 的 deterministic 语料生成 / recall@k 度量 / floor 断言 / honest-defer 逻辑）[SPEC-OWNER:task-36.1-qdrant-live-recall-harness]——本 task 仅给该 harness 接 CI live service container，不改 harness 源码。
- qdrant 集群 / 复制 / sharding / 多节点部署拓扑 [SPEC-DEFER:phase-future.qdrant-deployment-topology]——本 task service container 是单节点 CI 验证形态；生产多节点拓扑承 `docs/spikes/phase-18-qdrant.md` Follow-up，诚实延后。
- 把 qdrant 作为生产 backend 跨多后端常驻部署矩阵 [SPEC-DEFER:phase-future.multi-backend-production]——本 task 仅在 CI service container 内验证 recall，生产多后端常驻运维诚实延后。
- recall vs golden semantic labels（需真实 embedding model 的语义金标召回）[SPEC-DEFER:phase-future.qdrant-semantic-golden-recall]——ADR-041 A3：qdrant-vs-exact-KNN 是 model-free + 可复现的干净主指标，语义金标召回诚实延后（不在本 task / Phase 36 范围）。
- 把 service container 形态推广到其他 vector backend（lancedb / sqlite-vec 的 live CI 服务）[SPEC-DEFER:phase-future.multi-backend-production]——本 task 仅 qdrant；其余后端 live CI 服务诚实延后。
- 真实 release tag / run-id / digest（v0.29.0）[SPEC-OWNER:task-36.3-closeout]（ADR-012 用户授权后回填）。

## 4. Actors

- 主 agent（ADR-012 自治）
- `.github/workflows/ci.yml`（既有 5 job：`cargo-test` / `go-test` / `spec-lint` / `lint` / `feature-build`；本 task add-only 第 6 个 `qdrant-recall` job，既有 5 job 不改）
- `feature-build` job（`.github/workflows/ci.yml:106-138`，task-28.3 / BUILD-4——既有 `vector-qdrant` `cargo build` build-check + `Install protoc` 步骤范式；本 task `qdrant-recall` job 镜像其 checkout + Rust 1.93 + protoc + cache 形态，区别在附 `services: qdrant` + 跑 `cargo test --test qdrant_live_recall` 而非 `cargo build`）
- `services: qdrant`（GitHub Actions service container，`image: qdrant/qdrant`，gRPC 6334 + REST 6333 映射到 runner localhost——CI 提供的 live qdrant server）
- `core/tests/qdrant_live_recall.rs`（task-36.1 env-gated live recall harness——本 job 经 `QDRANT_URL=http://localhost:6334` 令其检测 `health() == Ready` 走真实召回分支；本 task 不改 harness 源码）
- `QdrantBackend`（`core/src/retriever/vector/qdrant.rs`，connect / health / ensure-create open / index_batch / search——harness 真实驱动，本 task 不改）
- 本 PR 自身的 CI run（验证实证：`qdrant-recall` job 跑 harness 且绿 = 真实证据，run-id / 召回数据实回填，ADR-013）

## 5. Behavior Contract

### 5.1 Required Reading

- `.github/workflows/ci.yml:106-138`（`feature-build` 矩阵 job——`:112-120` `matrix.feature` 含 `vector-qdrant` / `:123-126` `dtolnay/rust-toolchain@stable` `toolchain: '1.93'` / `:127-128` `Install protoc`（`sudo apt-get update && sudo apt-get install -y protobuf-compiler`）/ `:129-136` `actions/cache@v4` cargo / `:137-138` `cargo build -p contextforge-core --features ${{ matrix.feature }}`——本 task `qdrant-recall` job 镜像 checkout + Rust 1.93 + protoc + cache 形态，区别：附 `services: qdrant` + 跑 `cargo test --features vector-qdrant --test qdrant_live_recall -- --nocapture`）
- `.github/workflows/ci.yml:1-54`（既有 `on:` 触发（PR + push to master）+ `permissions: contents: read` + `cargo-test` / `go-test` / `spec-lint` 三 job——本 task add-only 第 6 job，既有不改；`qdrant-recall` 与既有 job 同级 sibling）
- `core/tests/qdrant_live_recall.rs`（task-36.1 harness——`#![cfg(feature = "vector-qdrant")]` / `QdrantConnConfig::from_env()` 读 `QDRANT_URL` / `health() != Ready` → eprintln skip + return（honest-defer）/ `health() == Ready` → deterministic 语料同时索引 Qdrant + BruteForce → recall@k >= floor 断言 + eprintln 实测数——本 job 令其在 service container 下走 `Ready` 真实召回分支，本 task 不改其源码）
- `core/src/retriever/vector/qdrant.rs:160-189`（`QdrantBackend::connect`（懒连接 client + tokio runtime）/ `health()`（live→`QdrantHealth::Ready`，无 server→`Unreachable`）——harness 经 `from_env()` + `connect` + `health()` gate，service container 在跑 → `Ready`，本 task 不改）
- `docs/decisions/adr-041-qdrant-live-vector-recall.md §D3`（本 task 即其 CI service-container 集成原文实现）+ `§D2`（real measured recall 待 task-36.2 run 回填）+ ADR-034 D2（`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` honest-defer——本 task 关闭其 CI-no-server 前提，Amendment 标 D2 fulfilled，NOT retro-edit D-body per ADR-014 D5）+ ADR-004（默认 build 0-vector-dep / 默认行为不变）+ ADR-008（0 新 dep）+ ADR-013（验证实证 = 本 PR 自身 live CI run，据实记录不预填）
- GitHub Actions `services:` 文档（service container 在 job 前拉起 + 端口映射到 runner localhost——核实 `image` / `ports` 形态 + service container 经 localhost 暴露端口的语义）+ `qdrant/qdrant` 镜像（gRPC `6334` + REST `6333` 监听端口核实）

### 5.2 关键设计 — qdrant-recall job add-only（service container + harness live run / CI-only / 0 源码改动 / 0 新 dep）

- **B1 service container 提供 live server（令 harness 走真实分支）**：`qdrant-recall` job 的 `services:` 块在 job 步骤前拉起 `qdrant/qdrant` 容器并映射端口；harness 经 `QDRANT_URL=http://localhost:6334` 连到它，`health()` 返 `Ready` → 走真实召回分支（deterministic 语料同时索引 Qdrant + BruteForce → recall@k >= floor）。形态示意（最终以 §3 In Scope 为准）：
  ```yaml
  qdrant-recall:
    runs-on: ubuntu-22.04
    timeout-minutes: 30
    services:
      qdrant:
        image: qdrant/qdrant
        ports:
          - 6334:6334
          - 6333:6333
    steps:
      - uses: actions/checkout@v4
      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: '1.93'
      - name: Install protoc
        run: sudo apt-get update && sudo apt-get install -y protobuf-compiler
      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-qdrant-recall-${{ hashFiles('**/Cargo.lock') }}
      - name: qdrant live recall harness
        run: QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture
  ```
  端口映射 `6334:6334`（gRPC，`QdrantBackend` 主用）+ `6333:6333`（REST，备查）；`Install protoc` 镜像 `feature-build` 既有步骤（qdrant-client 1.18 携预生成 protobuf，但与 `feature-build` 形态一致保险起见保留 protoc，最终以实测为准）；`--nocapture` 令 harness 的 eprintln 实测 recall 数出现在 CI log。
- **B2 关闭 `[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（CI 从此 HAS a live server）**：ADR-034 D2 的 defer 理由是「CI 无 live server」；service container 解除该前提——harness 每次 CI run 对 live server 跑真实 recall，defer 永久兑现并关闭。task-36.3 在 ADR-034 add-only Phase-36 Amendment 据实标其 D2 fulfilled（live KNN recall measured + CI-guarded），**NOT retro-edit ADR-034 D-body**（ADR-014 D5——既有 ADR D-body 冻结，新事实经 Amendment add-only 记录，不溯改正文）。
- **B3 CI-only / add-only / 0 源码 + 0 新 dep**：本 task 改的只是 `.github/workflows/ci.yml`（add-only 一个 job），不改任何 `core/` / `internal/` 源码、不改 `Cargo.toml` / `Cargo.lock`（vector-qdrant 自 task-18.4 起即 optional，`--features` 开它不增 dep）、不改 proto / migration；既有 5 job 逐字符不改；默认 build（不带 `--features vector-qdrant` 的 `cargo-test` job）仍 0-vector-dep、默认行为 + dep 集不变（ADR-004/008）。
- **B4 验证实证 = 本 PR 自身 live CI run（ADR-013 不预填）**：CI 配置改动的真实验证只能是「这个 job 在真实 CI 上跑过且绿」——本 PR 自身的 `qdrant-recall` job 跑 harness 对 live service container 量真实 recall@k >= floor + `--nocapture` 打印真实数，即 TEST-36.2.1 的真实实证。run-id / run link / 真实召回数 **真实跑出后**据实回填（§10 + v0.29.0 evidence），绝不预填、不伪造 live-server 通过（ADR-013）。

### 5.3 不变量

- 默认 build 0-vector-dep + 默认行为不变（ADR-004/008）：`vector-qdrant` 仍 opt-in（仅在 `qdrant-recall` job 的 `--features vector-qdrant` 内开）；默认 `cargo-test` job（`cargo test --workspace`，不带 vector feature）行为逐字节不变；默认 build dep 集不变；0 新 dep（qdrant-client 1.18 自 task-18.4 起 optional，`--features` 开它不增依赖）。
- 既有 5 job 不退化：`cargo-test` / `go-test` / `spec-lint` / `lint` / `feature-build` 逐字符不改（add-only 第 6 job）；既有 4 门验证语义不变。
- CI-only / 0 源码改动：本 task 改的只是 `.github/workflows/ci.yml`（add-only），不改任何源码 / proto / migration / `Cargo.toml` / `Cargo.lock`；harness 源码（task-36.1）不改（本 job 只经 env + service container 驱动它）。
- honest 验证边界（ADR-013）：本 task 是 CI 配置改动，验证实证 = 本 PR 自身的 live CI run（`qdrant-recall` job 绿 + harness 真跑）；run-id / 真实召回数据实回填，**不**预填、**不**伪造 live-server 通过；service container 若因镜像 / 网络 / 端口在 CI 偶发不可用，harness 仍走 honest-defer skip 分支（task-36.1，NOT fail）——但本 task pass bar 要求本 PR CI 上 service container 真起、harness 真走 `Ready` 分支（据实记录实际结果）。
- 关闭 defer 不溯改 ADR D-body（ADR-014 D5）：`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]` 经 service container 永久兑现，标记 fulfilled 经 ADR-034 add-only Phase-36 Amendment（task-36.3），不 retro-edit ADR-034 D-body。

## 6. Acceptance Criteria（Draft 阶段未勾选，实施后逐条置 `[x]`）

- [x] **AC1**（qdrant-recall service-container job add-only + 本 PR live CI run 绿）: `.github/workflows/ci.yml` 新增 `qdrant-recall` job——`services: qdrant: { image: qdrant/qdrant, ports: ["6334:6334", "6333:6333"] }` + `dtolnay/rust-toolchain@stable` `toolchain: '1.93'` + `Install protoc` + cargo cache + `QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`；既有 5 job 逐字符不改；默认 build 0-vector-dep + 默认行为不变 + 0 新 dep；本 PR 自身 CI 的 `qdrant-recall` job 绿（harness 对 live service container 真跑、`health() == Ready` 走真实召回分支、recall@k >= floor、`--nocapture` 打印真实数）= 真实实证（run-id / 真实召回数真实跑出后据实回填，ADR-013 不预填）— verified by **TEST-36.2.1**（本 PR 自身 live CI run = real evidence）
- [x] **AC2**（ADR-014 D2 lint）: `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-36.2.2**（= LAST）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-36.2.1 | `qdrant-recall` CI job 用 service container（`qdrant/qdrant` + 6334/6333）+ Rust 1.93 + protoc 对 live server 跑 task-36.1 harness（`QDRANT_URL=http://localhost:6334 cargo test ... --features vector-qdrant --test qdrant_live_recall -- --nocapture`）且绿——`health() == Ready` 走真实召回分支、recall@k >= floor、`--nocapture` 打印真实数；既有 5 job 不退化、默认 build + 默认行为不变、0 新 dep。验证实证 = 本 PR 自身 live CI run（run-id / 真实召回数真实跑出后据实回填，ADR-013 不预填） | `.github/workflows/ci.yml` + 本 PR 自身 CI run | Done |
| TEST-36.2.2 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威）（= LAST） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（中）service container 在 CI 启动慢 / 端口未就绪致 harness 误走 honest-defer skip 分支**：`qdrant/qdrant` 容器拉起 + readiness 需若干秒，若 harness 在 service 就绪前连接，`health()` 可能返 `Unreachable` → harness honest-defer skip（NOT fail）→ recall 没真跑、defer 没真兑现。
  - **缓解**：GitHub Actions service container 在 job 步骤启动前已拉起并默认等容器进入运行（service container 生命周期由 runner 管理）；harness `health()` 经 `connect` 懒连接 + `health_check`，service 就绪后即 `Ready`；本 PR CI run 据实核验 `qdrant-recall` job log 中 harness 走的是 `Ready` 真实召回分支（`--nocapture` 打印 recall 数）而非 skip notice——若 log 显示 skip 则 AC1 不标 `[x]`，需加 service health/readiness 等待。stop-condition：本 PR CI log 未见真实 recall 数（走了 skip）则 AC1 不标 `[x]`。
- **R2（中）service container 镜像 tag / 端口 / 网络在 CI 偶发不可用**：`qdrant/qdrant`（无显式 tag = latest）镜像拉取 / 端口映射 / 容器网络在 CI 偶发失败，致 job 红或 service 不可用。
  - **缓解**：端口映射 `6334:6334` + `6333:6333` 同时暴露（gRPC 主用、REST 备查）；harness 经 `localhost:6334` 连接（service container 端口映射到 runner localhost 的标准语义）；偶发拉取失败属 CI flake，按既有 flake 处理（rerun）；本 task pass bar 以本 PR CI 上 service 真起、harness 真走 `Ready` 为准（据实记录，ADR-013）。stop-condition：service 持续不可用（非偶发 flake）则 AC1 不标 `[x]`，需排查镜像/端口配置。
- **R3（低）误改既有 5 job（非 add-only）**：在 `ci.yml` 加 job 时易顺手「改进」既有 job（缩进 / 步骤 / 缓存 key）破 add-only 契约。
  - **缓解**：本 task scope 明确**只** add-only 第 6 个 `qdrant-recall` job；既有 5 job（`cargo-test` / `go-test` / `spec-lint` / `lint` / `feature-build`）逐字符不改（§5.3 不变量）；review diff 确认既有 job 0 行改动。stop-condition：既有 job 被改动则 review 退回。
- **R4（低）被误读为「已永久关闭 defer + 已有真实召回数」而预填（伪造）**：CI 配置写好但本 PR CI 未真跑出 recall 前，易被夸大为「defer 已兑现 + recall = X」预填数值。
  - **缓解**：spec §1 B4 / §2(4) / §5.2 B4 / §5.3 + AC1 + ADR-041 D2 据实记「验证实证 = 本 PR 自身 live CI run，run-id / 真实召回数真实跑出后回填，绝不预填」；defer fulfilled 标记经 task-36.3 / ADR-034 Amendment（real numbers 回填后）据实落，不预断（ADR-013）。stop-condition：在本 PR CI 真跑出前回填任何 run-id / 召回数则违 ADR-013。

## 9. Verification Plan

```bash
# 1. 本地 — ci.yml 语法 / job 结构核验（add-only qdrant-recall，既有 5 job 不改）
#    （CI 配置真实验证只能在 GitHub Actions 上跑——见下方 honest-defer 边界）
git diff .github/workflows/ci.yml   # 期望仅 add-only qdrant-recall job，既有 5 job 0 改动

# 2. 本地 — harness 在有 live qdrant 时真跑（可选 dev-box 预检，与 CI service container 同形态）
#    需本机/容器起 qdrant 监听 6334；无 server 时 harness honest-defer skip（task-36.1）
QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture

# 3. 默认 build 不退化（无 vector feature，0-vector-dep / 默认行为不变）
cargo test --workspace

# 4. AC1 真实实证 — 本 PR 自身 CI 的 qdrant-recall job 绿（service container 真起、harness 真走 Ready 分支、recall@k >= floor、--nocapture 打印真实数）
gh pr checks   # 期望 qdrant-recall job PASS

# 5. AC2 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **honest-defer 边界** [SPEC-DEFER:phase-future.qdrant-recall-ci-service-defer-note]：本 task 仅向 `.github/workflows/ci.yml` add-only 一个 `qdrant-recall` job（`services: qdrant/qdrant` + 6334/6333 端口 + Rust 1.93 + install protoc + `QDRANT_URL=http://localhost:6334 cargo test ... --features vector-qdrant --test qdrant_live_recall -- --nocapture`），令 task-36.1 harness 每次 CI run 对 live service container 跑真实 recall——CI 从此 HAS a live server，`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（ADR-034 D2）经此永久兑现并关闭（task-36.3 / ADR-034 add-only Phase-36 Amendment 据实标 D2 fulfilled，NOT retro-edit D-body per ADR-014 D5）。本 task 是 **CI 配置改动**——验证实证只能是本 PR 自身的 live CI run（`qdrant-recall` job 绿 + harness 真走 `Ready` 真实召回分支），run-id / 真实召回数 **真实跑出后**据实回填（§10 + v0.29.0 evidence），绝不预填、不伪造 live-server 通过（ADR-013）。task-36.1 harness 本体（deterministic 语料 / recall@k 度量 / floor / honest-defer skip）[SPEC-OWNER:task-36.1-qdrant-live-recall-harness] 不在本 task 改；qdrant 集群/复制/部署拓扑 [SPEC-DEFER:phase-future.qdrant-deployment-topology]、多后端生产常驻 / 其他 backend live CI 服务 [SPEC-DEFER:phase-future.multi-backend-production]、语义金标召回 [SPEC-DEFER:phase-future.qdrant-semantic-golden-recall] 均诚实延后；真实 release tag / run-id / digest（v0.29.0）[SPEC-OWNER:task-36.3-closeout] 实施授权后回填。

## 10. Completion Notes (s2v 6 项标准)

**Status**: Done

**§9 Verification 实证**（real evidence，本 PR 自身 live CI run）：
- **AC1 真实 live CI 召回**：本 PR `qdrant-recall` CI job（run **26961084355**）service container `qdrant/qdrant` 真起（log `qdrant ready after 1 attempt(s)`），harness 走 `health()==Ready` 真实召回分支（**非** honest-defer skip），CI 实测 `PHASE36 qdrant LIVE recall@10 vs brute-force exact KNN | N=2000 dim=64 M=50 @ http://localhost:6334 => recall@10=1.0000`，`test result: ok. 2 passed; 0 failed`。`[SPEC-DEFER:phase-future.qdrant-server-lifecycle]`（ADR-034 D2）从此每次 CI run 被验证——结构性「CI 无 live server」约束**永久解除**（task-36.3 / ADR-034 add-only Phase-36 Amendment 据真实数标 D2 fulfilled）。
- 既有 6 job（cargo-test / go-test / spec-lint / lint / feature-build×8）不退化、默认 build 0-vector-dep + 默认行为不变、0 新 dep（全门绿，run 26961084355）。
- AC2：D2 lint `--touched origin/master`（CI spec-lint 权威，绿）。

**诚实判读（ADR-013）**：CI 实测 recall@10=**1.0000** 与本地一致——qdrant 在 N=2000（低于其 HNSW indexing_threshold）即时服务精确 KNN，故为 live KNN **正确性**真实证明（取代合成 fixture 0.7/0.85）；HNSW 近似域大语料真实 ANN recall 压测留 `[SPEC-DEFER:phase-future.vector-large-corpus-perf]`，不夸大。验证证据为 **PR 自身 live CI run（run 26961084355）**，非预填（ADR-013）。

**grounding 校正**：「Wait for qdrant to be ready」步（curl `/readyz` retry）实测 `ready after 1 attempt(s)` 即成——防 harness 在 server 就绪前误走 honest-defer skip（确保 recall 真测而非 no-op）；spec §8 R1 stop-condition（CI log 须见真实 recall 数非 skip）经 log 据实满足。

**实际改动文件**：
- `.github/workflows/ci.yml`——add-only 第 6 个 job `qdrant-recall`（`services: qdrant/qdrant` 6334+6333 + Rust 1.93 + protoc + cargo cache + Wait-for-ready + `QDRANT_URL=http://localhost:6334 cargo test -p contextforge-core --features vector-qdrant --test qdrant_live_recall -- --nocapture`）；既有 5 job 逐字符不改。
- 0 backend 改动 / 0 新 dep / 0 默认构建变更。ADR-041 D3（CI service-container 集成）ratify 依据（@ task-36.3 closeout）。
