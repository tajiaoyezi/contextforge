# Task `46.3`: `release-flow-and-closeout — release.yml 加 GitHub Release 对象自动创建 + smoke v36[55/55] + v0.39.0 closeout + ADR-050 D3/D4 ratify`

**Status**: Ready
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 46 (v1.0-docs-and-release-flow)
**Dependencies**: task-46.1（README v0.2 limitations 已删，Release 声明同步）/ task-46.2（CHANGELOG.md 首版就绪）/ ADR-050（D3/D4 ratify）/ ADR-007（分发定义 Amendment）/ ADR-014（第三十七次激活）

## 1. Background
release.yml（101 行）当前只推 image 到 GHCR + cosign keyless sign + SBOM attestation。README `v0.2 limitations` 段自承 "does not publish a GitHub Release object or source tarball"——这正是 D4 缺口。task-46.1 删了 v0.2 limitations，本 task 把 Release 对象自动化落地 + closeout。ADR-050 D3（task-46.1/46.2 交付）+ D4（本 task 交付）完成后 ratify。

## 2. Goal
(1) release.yml 加 `softprops/action-gh-release@v2` step（tag push 触发，在 cosign sign + SBOM attest 之后；body 从 RELEASE_NOTES.md 对应版本段拼接 + 标注 GHCR image + cosign verify + SBOM provenance 链接）。
(2) README 同步：确保 task-46.1 删的 "does not publish a GitHub Release object" 声明不复活，并加指向 GitHub Releases 的链接（Releases 段）。
(3) smoke v35→v36[55/55] + TestTask463 + release docs + ADR-050 D3/D4 ratify + ADR-007 Amendment + roadmap/adapter。

## 3. Scope
- 改 `.github/workflows/release.yml`：
  - 加 `permissions: contents: write`（GitHub Release 对象创建需要；当前 release.yml 只有 `contents: read`，需改为 write）
  - 加 step `Create GitHub Release`（`softprops/action-gh-release@v2`，在 sign + attest 之后）：
    - `tag_name: ${{ steps.ref.outputs.tag }}`
    - `name: ${{ steps.ref.outputs.tag }}`
    - `body`: 从 RELEASE_NOTES.md 对应版本段提取（用 awk/grep 抽 `## vX.Y.Z` 标题下的内容）+ 追加 GHCR image + cosign verify 命令 + SBOM provenance 链接模板
    - `draft: false` / `prerelease: false`（正式 release）
    - `generate_release_notes: false`（用自定义 body，不用 GitHub auto-generated）
- 改 `README.md`（Releases 段同步——task-46.1 建的 Releases 段指向 GitHub Releases，本 task 确保链接在 Release 对象落地后成立）
- 改 `scripts/console_smoke.sh`：v35→v36，step [54/54]→[55/55]（加 step 55：release.yml Release step 在场 grep）
- 加 `internal/cli/smoke_syntax_test.go`：`TestTask463`（release.yml Release step + README 无过时声明 + no-regression denominator [37/37]..[54/54]）
- 改 `docs/decisions/adr-050-v1.0-definition.md`：D3/D4 ratify（Proposed → Accepted for D3/D4）
- 改 `docs/decisions/adr-007-minimal-tarball-distribution.md`：add-only Amendment（分发定义补 GitHub Release 对象）
- 改 `docs/roadmap.md`：§3.28 + §v1.0 锚点段（Phase 46 D3/D4 落地记录）
- 改 `docs/s2v-adapter.md`：Phase 46 行 + task 行 + ADR-050 ratify
- 新增 `docs/releases/v0.39.0-evidence.md` + `v0.39.0-artifacts.md`

## 6. AC
- [ ] **AC1**（D4 release.yml Release step）: release.yml 加 `softprops/action-gh-release@v2` step + `contents: write` permission — verified by **TEST-46.3.1**
- [ ] **AC2**（README Release 声明一致）: README 无 "does not publish a GitHub Release object" 过时声明 + Releases 段指向 GitHub Releases — verified by **TEST-46.3.2**
- [ ] **AC3**（v0.39.0 closeout）: smoke v36[55/55] + TestTask463 + release docs + ADR-050 D3/D4 ratify + ADR-007 Amendment + roadmap/adapter — verified by **TEST-46.3.3**
- [ ] **AC4**（ADR-014 cross-validation gate）: D1-D5（第三十七次激活） — verified by PR body + LAST TEST

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-46.3.1 | release.yml softprops/action-gh-release step + contents: write 在场 | yaml grep | Not Started |
| TEST-46.3.2 | README 无 "does not publish a GitHub Release object" + Releases 段在场 | docs grep | Not Started |
| TEST-46.3.3 | smoke v36[55/55] + TestTask463 PASS + release docs + ADR-050 D3/D4 ratify | smoke + docs grep | Not Started |

## 9. Verification
```bash
# release.yml Release step 在场 + permissions contents: write
grep -q "softprops/action-gh-release" .github/workflows/release.yml
grep -q "contents: write" .github/workflows/release.yml
# README 无过时声明 + Releases 段在场
! grep -q "does not publish a GitHub Release object" README.md
# smoke
bash scripts/console_smoke.sh   # v36[55/55]
# TestTask463
go test ./internal/cli/ -run TestTask463
# ADR-050 D3/D4 ratify
grep -q "D3.*Accepted\|D3.*ratif" docs/decisions/adr-050-v1.0-definition.md
grep -q "D4.*Accepted\|D4.*ratif" docs/decisions/adr-050-v1.0-definition.md
```

## 10. Completion Notes
**Status**: Ready
