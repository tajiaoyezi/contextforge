# Task `48.1`: `v1.0.1-patch — P0 CLI version ldflags 注入 + P1-P3 文档残留清理 + v1.0.1 closeout`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 48 (v1.0.1-patch)
**Dependencies**: v1.0.0（已 ship）/ v1.0 收口审查残留清单 / ADR-050（Accepted，不动）/ ADR-014（第三十九次激活）

## 1. Background
v1.0.0 ship 后全面审查发现 4 个残留：(P0) CLI version 字符串过时——`internal/cli/cli.go` Version 默认值停在 Phase 45 的 dev 串 + Dockerfile/release.yml 无 ldflags 注入 → v1.0.0 镜像 `contextforge version` 报旧 dev 串（D2 API/CLI 冻结缺陷）；(P1) docs/decisions/README.md ADR-050 漏更新（仍 Proposed）；(P2) README Latest 段描述措辞过时；(P3) example.toml header 版本过时。

## 2. Goal
(1) P0：cli.go Version 默认值 `"1.0.1-dev"` + Dockerfile ARG VERSION + ldflags 注入 + release.yml build-args VERSION 传 tag。
(2) P1-P3：docs/decisions/README.md ADR-050 Accepted + README Latest 段 v1.0 描述 + example.toml header v1.0.1。
(3) smoke v37→v38[57/57] + TestTask481 + release docs + roadmap/adapter。

## 3. Scope
- 改 `internal/cli/cli.go`：`var Version = "0.38.0-dev"` → `var Version = "1.0.1-dev"`（默认值兜底；release 时 Dockerfile ldflags 注入真实 tag）
- 改 `Dockerfile`：go-build stage 加 `ARG VERSION=1.0.1-dev` + go build 加 `-ldflags "-X github.com/tajiaoyezi/contextforge/internal/cli.Version=${VERSION}"`
- 改 `.github/workflows/release.yml`：docker/build-push-action 加 `build-args: VERSION=${{ steps.ref.outputs.tag }}`
- 改 `docs/decisions/README.md`：ADR-050 行 `Proposed (partial D1/D2 ratified)` → `Accepted (full D1/D2/D3/D4)`
- 改 `README.md`：Latest 段描述 `(v1.0 收口冲刺第二步 — ...)` → `(v1.0 收口终点 — ADR-050 完整 ratify Accepted + maturity label flip)`
- 改 `contextforge.example.toml`：header `(v0.38.0)` → `(v1.0.1)`
- 改 `scripts/console_smoke.sh`：v37→v38，step [56/56]→[57/57]
- 加 `internal/cli/smoke_syntax_test.go`：`TestTask481`（cli.go Version 默认值 + Dockerfile ldflags + release.yml build-args + 文档残留 + no-regression denominator）
- 新增 `docs/releases/v1.0.1-evidence.md` + `v1.0.1-artifacts.md`
- 改 `docs/roadmap.md`（§3.30）+ `docs/s2v-adapter.md`（Phase 48 行）+ `RELEASE_NOTES.md`（v1.0.1 段）+ `CHANGELOG.md`（[v1.0.1] 段）
- 新增 `test/features/phase-48-v1.0.1-patch.feature`

## 6. AC
- [x] **AC1**（P0 CLI version ldflags）: cli.go Version `"1.0.1-dev"` + Dockerfile ARG VERSION + ldflags `-X .../cli.Version` + release.yml build-args VERSION — verified by **TEST-48.1.1**
- [x] **AC2**（P1-P3 文档残留）: docs/decisions/README.md ADR-050 Accepted + README Latest 段 v1.0 + example.toml header v1.0.1 — verified by **TEST-48.1.2**
- [x] **AC3**（v1.0.1 closeout）: smoke v38[57/57] + release docs + roadmap/adapter — verified by **TEST-48.1.3**
- [x] **AC4**（ADR-014 cross-validation gate）: D1-D5（第三十九次激活） — verified by PR body + LAST TEST

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-48.1.1 | cli.go Version 1.0.1-dev + Dockerfile ldflags + release.yml build-args | grep | Done |
| TEST-48.1.2 | docs/decisions/README.md ADR-050 Accepted + README Latest v1.0 + example.toml v1.0.1 | grep | Done |
| TEST-48.1.3 | smoke v38[57/57] + TestTask481 PASS + release docs | smoke + grep | Done |

## 9. Verification
```bash
# cli.go Version 默认值
grep -q 'var Version = "1.0.1-dev"' internal/cli/cli.go
# Dockerfile ldflags
grep -q 'ARG VERSION' Dockerfile && grep -q 'ldflags.*cli.Version' Dockerfile
# release.yml build-args
grep -q 'build-args' .github/workflows/release.yml && grep -q 'VERSION' .github/workflows/release.yml
# docs/decisions/README.md ADR-050 Accepted
grep -q 'Accepted' docs/decisions/README.md  # ADR-050 行
# README Latest 段 v1.0
grep -q 'v1.0 收口终点' README.md
# example.toml header v1.0.1
head -1 contextforge.example.toml | grep -q 'v1.0.1'
# smoke + test
bash scripts/console_smoke.sh   # v38[57/57]
go test ./internal/cli/ -run TestTask481
# go vet + gofmt
go vet ./internal/cli/ && gofmt -l internal/cli/
```

## 10. Completion Notes
**Status**: Done

1. **完成日期**：2026-07-03
2. **改动文件**：cli.go（Version 默认值）+ Dockerfile（ARG VERSION + ldflags）+ release.yml（build-args）+ docs/decisions/README.md（ADR-050 Accepted）+ README.md（Latest 段）+ contextforge.example.toml（header）+ smoke_syntax_test.go（TestTask481 + TestTask471 精确化）+ console_smoke.sh（v38）+ release docs + roadmap/adapter
3. **commit 列表**：- `ee6c058` feat(v1.0.1): task-48.1 v1.0.1-patch
4. **§9 Verification 结果**：lint ✅（gofmt clean）/ typecheck N/A / unit-test ✅（go test ./... 全过含 TestTask481 + TestTask471）/ docs grep ✅（cli.go 1.0.1-dev + Dockerfile ldflags + release.yml build-args + ADR-050 Accepted + README 无"第二步"）
5. **剩余风险**：无（Dockerfile ldflags 注入首次实践成功——build log 确认 `-ldflags "-X .../cli.Version=v1.0.1"`，run 28691648309 success）
6. **下游影响**：无（patch 修复，v2.0 路线独立）
