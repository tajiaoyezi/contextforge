# Task `28.3`: `ci-strict-lint — 先实测 clippy/gofmt/go vet 存量（区分 Windows CRLF 假阳性）→ 修 clippy ~33 到 -D warnings 全绿 → ci.yml 加 lint job（clippy + gofmt + go vet 三项阻断）`

**Status**: Done

**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 28 (release-ci-hardening)
**Dependencies**: 既有 `.github/workflows/ci.yml`（三 job cargo-test / go-test / spec-lint）/ ADR-033（release-ci-hardening §D3——CI 强 lint 先测存量再定卡红时机）/ ADR-013（禁伪造凭据红线，真实存量计数）/ ADR-014 D1-D5（第十九次激活）/ PRD:524（clippy/gofmt 卡红 `[SPEC-DEFER:phase-future.ci-strict-lint]`）/ roadmap §3.9（明确告诫先评估存量避免一次性大面积变红）

## 1. Background

`.github/workflows/ci.yml` 现仅三 job：`cargo-test`（`cargo test --workspace`）/ `go-test`（`go test ./...`）/ `spec-lint`（`spec_drift_lint.sh --touched`）。**无任何 clippy / rustfmt / gofmt / golangci-lint 静态质量门**（仓内无 `.golangci.*` / `clippy.toml` / `rustfmt.toml`）。代码风格 / lint 退化无门禁（`[SPEC-DEFER:phase-future.ci-strict-lint]`，PRD:524）。roadmap §3.9 明确告诫：**先评估存量 clippy/gofmt 告警量再决定卡红时机，避免一次性大面积变红**。

## 2. Goal

先**实测真实存量**（ADR-013 非合成），区分 Windows `core.autocrlf` 假阳性；据存量（实测小）决定**卡红**：把 clippy 存量修到 `-D warnings` 全绿，`ci.yml` 加 `lint` job（clippy `-D warnings` + gofmt + go vet 三项阻断）。既有三门 + 全测试不退化；0 行为变更。

## 3. Scope

### In Scope（实际交付）

- **实测存量**（本机 + 推断 CI/Linux）：`cargo clippy --workspace --all-targets` / `gofmt -l .` / `go vet ./...`。
- **修 clippy 存量**到 `cargo clippy --workspace --all-targets -- -D warnings` 全绿：`cargo clippy --fix`（机械可修）+ 手动收尾（`field_reassign_with_default` 结构字面量 / `doc_lazy_continuation` 文档缩进 / `ptr_arg` `&PathBuf`→`&Path` / `while_let_loop` / `slice::from_ref` / `vec_init_then_push`）+ 2 处 targeted allow（生成代码 + `result_large_err`）。涉及 `core/src/{chunker,indexer,health,retriever,memory,jobs,parser,data_plane,contract,server}` + `core/examples` + `core/tests` + `bench/src` + `core/src/lib.rs`。
- 修改 `.github/workflows/ci.yml`——加 `lint` job：Rust toolchain 1.93 + clippy component → `cargo clippy --workspace --all-targets -- -D warnings`；Go 1.26 → gofmt check（`gofmt -l .` 非空即 fail）+ `go vet ./...`。三项阻断。

### 范围外（[SPEC-DEFER] / [SPEC-OWNER]）

- rustfmt 卡红（`cargo fmt --check`）[SPEC-DEFER:phase-future.rustfmt-gate]——本 task 落 clippy + gofmt + go vet；rustfmt 风格门后续。
- golangci-lint（更全 Go linter 集）[SPEC-DEFER:phase-future.golangci-lint]——本 task 用 stdlib gofmt + go vet。
- v0.21.0 closeout（smoke v18 / release docs / ADR-033 ratify）[SPEC-OWNER:task-28.4-closeout-v0.21.0]

## 4. Actors

- 主 agent（ADR-012 自治）
- `.github/workflows/ci.yml`（新 `lint` job）
- `cargo clippy` / `gofmt` / `go vet`（质量门工具，toolchain 自带）
- `contextforge-core` / `contextforge-bench` Rust crate（clippy 修复落点）

## 5. Behavior Contract

### 5.1 Required Reading

- `.github/workflows/ci.yml:46-54`（spec-lint job，lint job 加于其后）+ `:13-31`（cargo-test job 的 Rust toolchain + cache pattern 复用）
- `docs/decisions/adr-033-release-ci-hardening.md §D3`（先测存量再定卡红）
- roadmap §3.9（先评估存量避免大面积变红的告诫）

### 5.2 关键设计 — 存量实测 + 卡红决策

- **Windows CRLF 假阳性区分（关键）**：本机 `core.autocrlf=true`，工作区 Go 文件为 CRLF；`gofmt -l .` 本机报 **96 个文件**（≈ 全部 Go 文件），但 `gofmt -d` 诊断显示**每行仅差 `^M`**（CRLF↔LF）——repo 存 LF，**CI（Linux/LF）gofmt 实为 0**。真实 gofmt 存量 = 0（非 96）。**不把 Windows CRLF 噪声当真实存量报**（ADR-013）。
- **实测存量**：gofmt **0**（CI/LF）/ go vet **0** / clippy **~33**（唯一位置 ~39 含 1 生成代码 → allow；类型：`field_reassign_with_default`×8 / `doc_lazy_continuation`×8 / 无用 cast×5 / `ptr_arg`×3 / 其余零星）。均小 lint。
- **卡红决策**：存量小（gofmt/go vet 已 0；clippy ~33 多数 `cargo clippy --fix` 可自动修）→ 据 roadmap §3.9，**修完即卡红**（非 warn-first）；三项全阻断。
- **生成代码**：`core/src/lib.rs` 的 `pb`/`pb_console`（`tonic::include_proto!`）生成代码 clippy 不可改 → 模块加 `#[allow(clippy::all)]`（标准做法）。
- **result_large_err（1）**：`EventBus::send` 返回 tokio `broadcast::SendError`，boxing 会改公共签名 + ripple 全调用方 → targeted `#[allow(clippy::result_large_err)]` + 注释说明（非 boxing 刻意取舍）。

### 5.3 不变量

- 0 行为变更（clippy 修均 surgical 等价改写；cargo test --workspace 全过不退化）。
- 0 新依赖（toolchain 自带 clippy/gofmt/go vet；无 Cargo / go.mod 改动）。
- 既有 `cargo-test` / `go-test` / `spec-lint` 三 job 不退化（lint 为 add-only 第四 job）。
- 不 blanket 抑制（仅生成代码 + 1 result_large_err targeted allow；其余真实修，非 #[allow]）。

## 6. Acceptance Criteria

- [x] **AC1**（实测存量 + CRLF 区分）: `cargo clippy --workspace --all-targets` / `gofmt -l` / `go vet` 真实存量实测——gofmt 0（CI/LF；本机 96 系 `autocrlf` CRLF 假阳性，`gofmt -d` 仅 `^M` 差异）/ go vet 0 / clippy ~33 小 lint；不把 CRLF 噪声当真实存量（ADR-013）— verified by **TEST-28.3.1** + §10 实测
- [x] **AC2**（修 clippy 到全绿）: clippy ~33 经 `cargo clippy --fix` + 手动收尾 + 2 targeted allow（生成代码 `clippy::all` + `result_large_err`）修到 `cargo clippy --workspace --all-targets -- -D warnings` exit 0；`cargo test --workspace` 0 failed 不退化 — verified by **TEST-28.3.2** + §10 实测
- [x] **AC3**（lint job 卡红 + 不退化）: `ci.yml` add-only `lint` job（clippy `-D warnings` + gofmt check + go vet，三项阻断）；既有 cargo-test/go-test/spec-lint 不退化 — verified by **TEST-28.3.3**（PR CI lint job PASS + §10）
- [x] **AC4**: ADR-014 D2 lint — `bash scripts/spec_drift_lint.sh --touched origin/master` PR 触及行 0 未标注命中 — verified by **TEST-28.3.4** + §10 记录（CI spec-lint 权威）

## 7. 追踪表

| TEST-ID | 描述 | 落地文件 | Status |
|---|---|---|---|
| TEST-28.3.1 | 实测存量 gofmt 0(CI/LF)/go vet 0/clippy ~33；Windows CRLF 假阳性（gofmt -d 仅 ^M）如实区分 | 实测命令 + §10 | Done |
| TEST-28.3.2 | clippy ~33 修到 `-D warnings` exit 0（fix+手动+2 targeted allow）+ cargo test --workspace 0 failed | `core/src/*` + `bench/src` + `core/src/lib.rs` | Done |
| TEST-28.3.3 | `ci.yml` lint job（clippy -D warnings + gofmt + go vet 三阻断）+ 既有三门不退化 | `.github/workflows/ci.yml` | Done（PR CI 权威） |
| TEST-28.3.4 | D2 lint `--touched origin/master` 0 未标注命中（CI spec-lint 权威） | `scripts/spec_drift_lint.sh` | Done |

## 8. Risks

- **R1（高）ci-strict-lint 存量一次性大面积变红 — 已规避**：roadmap §3.9 告诫。
  - **处置（已执行）**：先实测存量（非直接 `-D warnings` 卡红）——gofmt/go vet 已 0，clippy 仅 ~33 小 lint（多数可自动修）；修完才加卡红，无大面积变红。若存量曾过大则会改 warn-first + `[SPEC-DEFER:phase-future.lint-backlog-cleanup]`（本次存量小，无需）。
- **R2（中）clippy --fix / 手动修引入行为变更或破测试**：自动 + 手动改 ~15 文件。
  - **缓解**：所有修 surgical 等价改写；`cargo test --workspace` 全过复验（core lib 187 passed 等 0 failed）。stop-condition：任何修破测试则回退该修。
- **R3（低）clippy 版本漂移致 CI 与本机存量不一致**：未来新 clippy 版本可能引入新 lint。
  - **缓解**：`ci.yml` lint job 用 `dtolnay/rust-toolchain@stable` toolchain 1.93（与 cargo-test job 同），与本机 1.95 略差但 lint 集稳定；新 lint 出现时后续 task 处理。

## 9. Verification Plan

```bash
# 1. AC1 — 实测存量（区分 CRLF 假阳性）
cargo clippy --workspace --all-targets 2>&1 | grep -c "^warning"   # 修前 ~33（去重）
gofmt -l . | wc -l            # 本机 96（CRLF 假阳性）；gofmt -d <file> 验仅 ^M
go vet ./...                  # exit 0（0）

# 2. AC2 — clippy 修到全绿 + 测试不退化
cargo clippy --workspace --all-targets -- -D warnings   # exit 0（全绿）
cargo test --workspace                                  # 0 failed

# 3. AC3 — lint job（PR CI 权威跑 clippy + gofmt + go vet）
#    本机 gofmt 因 CRLF 不可直接验；CI Linux/LF 权威
go vet ./...

# 4. AC4 — D2 lint
bash scripts/spec_drift_lint.sh --touched origin/master
```

> **说明**：本 task 纯 CI 门 + clippy 源码修，无 outward-facing 操作（不碰 GHCR / 不 tag）。lint job 真实生效由 PR CI 的 `lint` job PASS 权威确认。

## 10. Completion Notes (s2v 6 项标准)

- **Status**: Done（2026-06-02）。
- **完成日期**：2026-06-02。
- **改动文件**：
  - `.github/workflows/ci.yml`——add-only `lint` job（Rust 1.93 + clippy → `cargo clippy --workspace --all-targets -- -D warnings`；Go 1.26 → gofmt check + go vet）。
  - clippy 修复（surgical 等价）：`core/src/lib.rs`（`pb`/`pb_console` `#[allow(clippy::all)]` 生成代码）、`core/src/chunker/mod.rs` / `indexer/mod.rs` / `core/examples/phase24_tokenizer_recall.rs`（`field_reassign_with_default`→结构字面量）、`core/src/health.rs`（`vec_init_then_push`→`vec!`）、`core/src/retriever/mod.rs`（`slice::from_ref`）、`core/src/data_plane/memory.rs`（`while let`）、`core/src/jobs/index_session_backend.rs` / `core/tests/indexjob_real_runner.rs` / `bench/src/measure.rs`（`ptr_arg` `&PathBuf`/`&mut Vec`→slice）、`core/src/parser/mod.rs` / `core/src/memory/store.rs` / `core/tests/search_real_retriever.rs`（`doc_lazy_continuation` 文档缩进）、`core/src/data_plane/events.rs`（`result_large_err` targeted allow + 注释）；+ `cargo clippy --fix` 机械修 `core/src/contract.rs` / `data_plane/search.rs` / `server.rs` / `retriever/mod.rs`。
- **commit 列表**：`docs(spec): task-28.3` + `chore(lint): clippy 存量修到 -D warnings 全绿` + `ci: ci.yml 加 lint job（clippy + gofmt + go vet 卡红）`（合于一 PR）。
- **§9 Verification 实测结果（ADR-013 真实非合成）**：
  - **存量实测**：gofmt **0**（CI/LF；本机 `gofmt -l` 96 全系 CRLF 假阳性，`gofmt -d internal/config/config.go` 显示每行仅 `^M` 差异，`core.autocrlf=true`）；go vet **exit 0**（0）；clippy 修前 ~33 唯一小 lint（`field_reassign_with_default`×8 / `doc_lazy_continuation`×8 / 无用 cast×5 / `ptr_arg`×3 / 其余）。
  - **修后**：`cargo clippy --workspace --all-targets -- -D warnings` **exit 0**（独立复验）；`cargo test --workspace` 全过（`contextforge-core` lib **187 passed**、各 integration test 0 failed、`contextforge-bench` 7 passed）。
  - **lint job**：`ci.yml` 第四 job，PR CI 权威跑 clippy -D warnings + gofmt check + go vet 三阻断。
- **设计取舍**：(1) **先实测再卡红**（roadmap §3.9）——区分 Windows CRLF 假阳性（gofmt 真实 0 非 96）避免误判存量大；存量实测小 → 修完即卡红非 warn-first。(2) **2 处 targeted allow**——生成代码（`pb`/`pb_console` `clippy::all`，不可改）+ `result_large_err`（`EventBus::send` 返 tokio broadcast SendError，boxing ripple 公共签名 → 刻意不 box + 注释）；其余 ~30 全真实修，不 blanket 抑制（对照用户否决的「存量 #[allow]」选项）。(3) clippy `--all-targets`（含 tests/bench/examples，门更全）。
- **剩余风险 + 下游影响**：rustfmt 卡红 `[SPEC-DEFER:phase-future.rustfmt-gate]` + golangci-lint `[SPEC-DEFER:phase-future.golangci-lint]` 后续；clippy toolchain 版本漂移新 lint 后续处理；task-28.4 closeout（smoke v18 + ADR-033 ratify，含 §D1 multi-arch 延后 / §D2 cosign 真签 / §D3 lint 门 三维据真实结果）。
