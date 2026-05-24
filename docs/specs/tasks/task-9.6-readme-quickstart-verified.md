# Task `9.6`: `readme-quickstart-verified — examples/quickstart/ fixture + scripts/quickstart_smoke.sh + README rewrite + v0.2.0 release docs`

> Status=Draft；主 agent 待用户 §2A Ready review 后推进。本 task 是 Phase 9 收口 task — §6 phase smoke gate 在此触发。

**Status**: Draft

**Priority**: P0
**Owner**: main agent（ADR-012 自治）
**Related Phase**: Phase 9 (cli-pipeline)
**Dependencies**: 9.5 (release-smoke-real)

## 1. Background

v0.1 实测：README quick start 的命令序列
```bash
contextforge init --root "$HOME/.contextforge"
contextforge index --source ./example --data-dir "$HOME/.contextforge" --collection default --resume
contextforge search "configuration" --collections default --top-k 5 --explain
contextforge eval run --collection default
```
照抄会在 `index` 步静默失败 + `search` 步报 `collection not found`，因 `./example` 不存在 + 即使存在 index 也是 stub。详见 [ADR-013](../../decisions/adr-013-cli-data-plane-grpc-bridge.md) §Context #1。

task-9.3 / 9.4 完成后 CLI 真实可跑，但 README 字面命令仍依赖用户自备 `./example` 目录；本 task 提供仓库内 `examples/quickstart/` fixture + 一键 smoke 脚本，让 README 命令真实可复制粘贴运行（任何用户在 cloned repo 内）。

本 task 同时是 Phase 9 收口（v0.2 release）：填实 phase-9 spec §6 端到端 smoke + 推进 ADR-013 状态 Proposed → Accepted + 写 v0.2.0 RELEASE_NOTES / evidence / artifacts manifest。

## 2. Goal

`examples/quickstart/` 目录含 minimal sample 项目（≥5 .md + 1 .env + 1 secret-redacted .yaml）+ 1 Hermes MEMORY.md fixture；`scripts/quickstart_smoke.sh` 一键跑完 README quick start 命令序列在临时 data_dir 中（CI 可跑 + 本地一键验证）；README 改成基于 examples/quickstart/ fixture 的可复制粘贴命令；`RELEASE_NOTES.md` 加 v0.2.0 章节；`docs/releases/v0.2.0-evidence.md` + `docs/releases/v0.2.0-artifacts.md` 按 ADR-007 模板 + task-8.3 v0.1-artifacts.md 先例。

## 3. Scope

### In Scope

- **新增 `examples/quickstart/`** 目录布局：
  ```
  examples/quickstart/
    sample-project/
      README.md          # 项目说明（用作 search query target）
      docs/config.md     # 含 "configuration" keyword (search target)
      docs/setup.md
      src/main.go        # 简单代码示例
      logs/app.log       # 普通日志
      .env               # 含 fake secret → 被 denylist 跳过
      config.yaml        # 含 fake AWS key → 被 redact
    hermes-memory/
      MEMORY.md          # Hermes 项目 memory 样例
      USER.md            # Hermes 用户 memory 样例
    README.md            # 解释 quickstart 目录用途 + 引导回主 README
  ```
- **新增 `scripts/quickstart_smoke.sh`**（一键跑 README quick start）：
  ```sh
  #!/usr/bin/env bash
  set -euo pipefail
  ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
  cd "$ROOT"
  
  STAGING="$(mktemp -d -t cfg-quickstart-XXXXXX)"
  trap "rm -rf $STAGING" EXIT
  
  echo "[1/7] build binaries"
  go build -o "$STAGING/contextforge" ./cmd/contextforge
  cargo build -p contextforge-core --release
  cp target/release/contextforge-core "$STAGING/"
  export PATH="$STAGING:$PATH"
  
  echo "[2/7] init"
  contextforge init --root "$STAGING/data"
  
  echo "[3/7] import hermes"
  contextforge import hermes "$ROOT/examples/quickstart/hermes-memory" \
    --collection demo --data-dir "$STAGING/data"
  
  echo "[4/7] index hermes records"
  contextforge index --source "$STAGING/data/imports/hermes" \
    --collection demo --data-dir "$STAGING/data"
  
  echo "[5/7] index sample project"
  contextforge index --source "$ROOT/examples/quickstart/sample-project" \
    --collection demo --data-dir "$STAGING/data"
  
  echo "[6/7] search 'configuration'"
  export CONTEXTFORGE_DATA_DIR="$STAGING/data"
  contextforge search "configuration" --collections demo --top-k 5 --explain
  
  echo "[7/7] eval run"
  contextforge eval run --collection demo
  
  echo "QUICKSTART_SMOKE_EXIT=0"
  ```
- **重写 `README.md`**：
  - 顶部仍是 product 介绍 + dual binary 解释
  - "Quick Start" 段改写：
    - 第一选项："One-shot smoke (Linux/WSL2)" → 直接跑 `bash scripts/quickstart_smoke.sh`
    - 第二选项："Manual steps" → 列出 build → init → import → index → search → eval 七步，命令显式带 `--data-dir` `--collection` 等参数（不再省略），sample data 引用 `examples/quickstart/sample-project` 和 `examples/quickstart/hermes-memory`
  - 加 "Expected output" 段示意 init / index / search 输出片段
  - 加 "v0.2 limitations" 段（PRD §Constraints 平台 / §发布边界）：Linux x86_64 / WSL2 only, no GitHub Release tarball yet, LICENSE all-rights-reserved 占位
  - 移除 / 注解任何 v0.1 之前不可跑的命令
- **新增 `RELEASE_NOTES.md` v0.2.0 章节**（追加在 v0.1.0 章节之前）：
  - 标题 + 日期 + 摘要
  - 主要改进：
    - CLI 数据通路打通（rpc Index stream + import 三子命令真实）
    - README quick start 可复制粘贴运行
    - Release smoke 升级为真端到端（删除 task-8.3 假证据测试）
    - ADR-013 cli-data-plane-grpc-bridge accepted
    - Phase 9 cli-pipeline 6 task 全 Done
  - 验证证据（同 v0.1.0 RELEASE_NOTES 格式）：
    - `bash scripts/release_smoke.sh` 输出 `PHASE_RELEASE_SMOKE_EXIT=0`
    - `bash scripts/quickstart_smoke.sh` 输出 `QUICKSTART_SMOKE_EXIT=0`
  - 发布边界（继承 v0.1 限制）
  - v0.1.0 → v0.2.0 migration（无 — 加法变更不改 schema）
- **新增 `docs/releases/v0.2.0-evidence.md`**（按 v0.1-evidence.md 模板）：
  - 日期 / 主干 / 当前 HEAD（待 chore PR 合后填实 commit SHA）
  - Phase 9 合入记录（PR 列表）
  - S2V 状态（phase-9 spec Status / task-9.X 6 task Status / ADR-013 Status）
  - 验证证据（s2v_baseline_green + release_smoke + quickstart_smoke 三段）
  - Release 边界（继承 v0.1 + Phase 9 新增 CLI 数据通路）
- **新增 `docs/releases/v0.2.0-artifacts.md`**（按 ADR-007 模板 + v0.1.0-artifacts.md 先例）：
  - tarball name: `contextforge-linux-amd64-v0.2.0.tar.gz`
  - 必含 entries: contextforge / contextforge-core / contextforge.example.toml / README.md / LICENSE
  - checksum 算法 + 占位（实际 release job 填）
  - 平台支持矩阵（Linux x86_64 / WSL2 only）
- **修改 `internal/release/release_test.go`**（如本 task 引入 RELEASE_NOTES.md 长度变化触发现有 test fail）：minor adjust assertions
- **修改 `.gitignore`** (nit fix discovered during task-9.3 testing):
  - 第 27 行 `/contextforge` 后加 `/contextforge.exe` + `/contextforge-core.exe`（Windows build artifact 防误 commit）
- 文件锚点：
  - `examples/quickstart/sample-project/*`（新增 ≥5 文件）
  - `examples/quickstart/hermes-memory/MEMORY.md` + `USER.md`（新增）
  - `examples/quickstart/README.md`（新增）
  - `scripts/quickstart_smoke.sh`（新增可执行）
  - `README.md`（rewrite Quick Start 段）
  - `RELEASE_NOTES.md`（追加 v0.2.0 章节）
  - `docs/releases/v0.2.0-evidence.md`（新增）
  - `docs/releases/v0.2.0-artifacts.md`（新增）
  - `.gitignore`（修改 — Windows .exe）

### Out Of Scope

- **真 100k chunk benchmark 跑 quickstart_smoke.sh**：fixture 限 ≤10 文件保 CI 快；benchmark 留 task-9.5 §3 OOS 提到的 nightly task
- **GitHub Release 真上传 / tag push**：留 manual release flow；本 task 仅产 release notes + evidence + artifacts manifest 文档（task-8.3 §3 OOS 沿用）
- **macOS / Windows quickstart_smoke.sh 支持**：bash 脚本 Linux/WSL2 优先；macOS 应能跑（cargo target 路径 + bash 兼容）但不在 v0.2 §6 AC 内；Windows skip
- **LICENSE 决策**：保持 all-rights-reserved 占位（PRD 未决）；如未来定 MIT/Apache 走独立 chore PR
- **修改 `core/src/` / `internal/` 业务代码**（本 task 纯 docs + scripts + fixture）
- **修改 `Cargo.toml` / `go.mod`**（R7）；脚本用 stdlib bash
- **`contextforge.example.toml` 内容改进**（task-8.3 已写，不动）

## 4. Users / Actors

- **新用户**（首次 git clone）：本 task 后跑 `bash scripts/quickstart_smoke.sh` 一键验证 v0.2.0 真实可用
- **README quick start 复制粘贴用户**：本 task 后 README 命令真实可跑
- **CI / GitHub Actions**：可加 quickstart_smoke.sh 到 release job 作为 pre-tag gate
- **v0.2.0 release 负责人**：本 task 产出 RELEASE_NOTES + evidence + artifacts manifest 作为 release 文档
- **Phase 9 / Phase 8 spec drift 回顾读者**（间接）：ADR-013 + v0.2.0 evidence 提供 spec drift retrospective 给未来 phase planning 参考

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§User Flow 主流程 / §Constraints 发布 / §Decisions Log D7）
- `docs/specs/phases/phase-9-cli-pipeline.md`
- `docs/specs/tasks/task-9.3-go-cli-index.md`
- `docs/specs/tasks/task-9.4-go-cli-import.md`
- `docs/specs/tasks/task-9.5-release-smoke-real.md`
- `docs/decisions/adr-007-minimal-tarball-distribution.md`
- `docs/decisions/adr-013-cli-data-plane-grpc-bridge.md`
- `docs/releases/v0.1-evidence.md`（v0.1 evidence 模板）
- `docs/releases/v0.1.0-artifacts.md`（v0.1 artifacts 模板）
- `RELEASE_NOTES.md`（v0.1.0 章节格式）
- `README.md`（当前内容）
- `scripts/release_smoke.sh`（task-9.5 改动后版本）

### 5.2 Imports

- **shell**：bash (sh-compatible) + `go build` + `cargo build` + 标 unix utilities (cp / mktemp / trap)
- **Markdown**：README + RELEASE_NOTES + evidence + artifacts 文档纯 markdown
- **fixture 文件**：Markdown / YAML / Go / Log / .env 文本（无 binary）
- **不引入**：R7 — 无新依赖；bash 标准 shell；脚本 sh-compatible

### 5.3 函数签名

> 本 task 纯文档 + shell script + fixture，无 Go / Rust 函数；§5.3 改为脚本接口约定：

```sh
# scripts/quickstart_smoke.sh
#   - 无 args (一键模式)
#   - 退出码: 0 全部七步成功 / 非 0 任一步失败 (set -e propagate)
#   - 输出: 每步 [N/7] echo + 命令输出 + 最末 QUICKSTART_SMOKE_EXIT=0
#   - 不接受环境变量配置 (默认 build dir / fixture path 写死)
#   - trap cleanup STAGING tempdir
```

```sh
# scripts/release_smoke.sh (task-9.5 已改)
#   - 多段调用，本 task 不动
```

- SCEN/TEST-9.6.1 → `examples/quickstart/sample-project/` 含 ≥5 .md + 1 .env + 1 secret-redacted .yaml + 1 .go + 1 .log 文件（AC1）
- SCEN/TEST-9.6.2 → `examples/quickstart/hermes-memory/MEMORY.md` + `USER.md` 存在 + 内容符合 Hermes importer detect 规则（AC2）
- SCEN/TEST-9.6.3 → `bash scripts/quickstart_smoke.sh` 退出码 0 + 最末输出 `QUICKSTART_SMOKE_EXIT=0`（AC3）
- SCEN/TEST-9.6.4 → README "Quick Start" 段命令包含 `examples/quickstart/` 引用 + 列出 build → init → import → index → search → eval 七步（AC4）
- SCEN/TEST-9.6.5 → `RELEASE_NOTES.md` 含 v0.2.0 章节 + `docs/releases/v0.2.0-evidence.md` + `docs/releases/v0.2.0-artifacts.md` 存在 + 各文件符合既有 v0.1 模板格式（AC5）

## 6. Acceptance Criteria

- [ ] **AC1** (本 task 新增 / Phase 9 §6 端到端 smoke fixture): `examples/quickstart/sample-project/` 含 ≥5 .md + 1 .env (denylist test) + 1 secret-redacted .yaml (redaction test) + 1 .go (代码示例) + 1 .log (日志示例) 文件；total 大小 ≤100KB
- [ ] **AC2** (本 task 新增 / hermes fixture for import smoke): `examples/quickstart/hermes-memory/MEMORY.md` + `USER.md` 存在 + 内容符合 Hermes importer (task-3.2) detect 规则（task §2A 时主 agent verify hermes 包 Detect() return ok=true）
- [ ] **AC3** (本 task 新增 / Phase 9 §6 端到端 smoke 入口): `bash scripts/quickstart_smoke.sh` 退出码 0 + 最末输出 `QUICKSTART_SMOKE_EXIT=0` + 七步全成功（init / import hermes / index records / index source / search / eval）
- [ ] **AC4** (PRD §User Flow 主流程 README 可复制粘贴): `README.md` "Quick Start" 段重写：含 one-shot `bash scripts/quickstart_smoke.sh` 选项 + manual 七步命令引用 `examples/quickstart/`；移除/注解 v0.1 误导命令
- [ ] **AC5** (ADR-013 §Decision #6 v0.2 release docs): `RELEASE_NOTES.md` 含 v0.2.0 章节（追加在 v0.1.0 之前）+ `docs/releases/v0.2.0-evidence.md` 按 v0.1 模板 + `docs/releases/v0.2.0-artifacts.md` 按 ADR-007 模板；ADR-013 状态 Proposed → Accepted（在本 task §10 Completion Notes 推进 + chore phase-9-closeout PR 中实际改）

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 sample-project fixture | SCEN-9.6.1 | TEST-9.6.1 (`ls examples/quickstart/sample-project/` count assert) | - | unit-test (script) | - |
| AC2 hermes-memory fixture | SCEN-9.6.2 | TEST-9.6.2 (`ls examples/quickstart/hermes-memory/` + hermes Detect()) | - | unit-test | - |
| AC3 quickstart_smoke.sh 端到端 | SCEN-9.6.3 | - | TEST-9.6.3 (bash scripts/quickstart_smoke.sh) | runtime-smoke (manual or CI) | - |
| AC4 README rewrite | SCEN-9.6.4 | TEST-9.6.4 (grep README for examples/quickstart/) | - | unit-test (script grep) | - |
| AC5 v0.2.0 release docs | SCEN-9.6.5 | TEST-9.6.5 (file exists + format match) | - | unit-test (script) | - |

## 8. Risks

- 关联 PRD §Technical Risks **R1**（Go↔Rust gRPC 边界）：quickstart_smoke.sh 是 task-9.3 / 9.4 / 9.5 CLI 路径的最高层 e2e；如 task-9.3 / 9.4 实现有 regression，本 task smoke 会失败 — 本 task 不修 regression 而是阻塞 Phase 9 closeout（fall back 走 §8 卡住协议）。
- 关联 **R6**（大仓库性能）：fixture ≤10 文件保 quickstart_smoke.sh < 30s + cargo build 60s cold cache；CI fast loop 用 `-short` 跳 release_smoke_e2e（task-9.5），quickstart_smoke 仍跑（本 task 是 v0.2 收口必跑）。
- 关联 **R9**（本地 daemon 暴露面）：脚本中多次起 daemon，每次自动选 loopback port；predict 无 conflict；如有 → 同 task-9.5 风险。
- 风险次：v0.2.0 evidence 中 HEAD commit SHA 在 chore PR 合后才能填实 — 本 task §10 实施时先写占位 `<待 chore PR 合后填>`，phase-9 closeout PR 时主 agent 替换。
- 风险次：LICENSE 仍 all-rights-reserved — README 段落需明确"v0.2 内部 development release / not for redistribution"；如未来定 OSI license 走独立 chore PR。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->
- **Runtime smoke**: bash scripts/quickstart_smoke.sh  <!-- AC3 端到端 -->

> 本 task §6 AC3 是 Phase 9 §6 phase smoke gate 入口；team §4 Gate 3 必须跑 `bash scripts/quickstart_smoke.sh` 全过才允许合本 task PR；亦触发 Phase 9 closeout 流程（adapter §Phase 状态索引 Phase 9 → Done + ADR-013 Proposed → Accepted）。

## 10. Completion Notes

> 待 task 完成后回填。
