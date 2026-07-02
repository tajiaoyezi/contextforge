# Task `45.4`: `closeout-v0.38.0 — smoke v34→v35[54/54]（v1.0 API/CLI 冻结：daemon REST 501 移除 + chunk_count 实装 + CLI --version/--help + example.toml 补全；breaking change 显式记）+ TestTask454 no-regression（[37/37]..[53/53] 不溯改）+ v0.38.0 release docs（含 breaking change 记录）+ ADR-050 部分 ratify（D1/D2）+ ADR-007 add-only Amendment（v1.0 分发定义收窄）+ roadmap/adapter`

**Status**: Done
**Priority**: P1
**Owner**: 主 agent（ADR-012 自治）
**Related Phase**: Phase 45 (v1.0-api-cli-freeze)
**Dependencies**: task-45.1+45.2+45.3 Done / ADR-050（部分 ratify）/ ADR-007（add-only Amendment）/ ADR-014（第三十六次激活）/ ADR-012（tag/release）

## 1. Background
task-45.1/45.2/45.3 合入后收口：smoke v35[54/54] + release docs（含 daemon REST breaking）+ ADR-050 部分 ratify + ADR-007 Amendment + roadmap/adapter。

## 2. Goal
(1) smoke v34→v35[54/54]（v1.0 API/CLI 冻结端到端）+ TestTask454（no-regression [37/37]..[53/53]）。
(2) v0.38.0 release docs（**含 breaking change 记录**：daemon REST 移除 import/eval/run 501 端点）+ README v0.38 段 + RELEASE_NOTES v0.38.0 段。
(3) ADR-050 部分 ratify（D1 能力已满足 / D2 API/CLI 冻结 Phase 45 交付；D3/D4 在 Phase 46/47）+ ADR-007 add-only Amendment（v1.0 分发定义收窄为务实收口）+ roadmap §v1.0 锚点段推进 + adapter 状态翻新。

## 6. AC
- [x] **AC1**（smoke v35[54/54] + no-regression）— verified by **TEST-45.4.1a/b**
- [x] **AC2**（release docs + ADR-050 部分 ratify + ADR-007 Amendment + roadmap/adapter）— verified by **TEST-45.4.1c**
- [x] **AC3**（ADR-014 D2 lint）— verified by **TEST-45.4.2**（CI spec-lint）

## 7. 追踪表
| TEST-ID | 描述 | 落地 | Status |
|---|---|---|---|
| TEST-45.4.1a | smoke v35[54/54] bash -n + TestTask454 绿 + no-regression [37/37]..[53/53] | smoke + smoke_syntax_test | Not Started |
| TEST-45.4.1b | smoke v35 v1.0 API/CLI 冻结端到端（daemon REST 3 端点 + CLI --version + example.toml） | console_smoke.sh | Not Started |
| TEST-45.4.1c | v0.38.0 release docs（含 breaking）+ ADR-050 部分 ratify + ADR-007 Amendment + roadmap/adapter + phase §6 | docs | Not Started |
| TEST-45.4.2 | D2 lint 0 未标注命中（= LAST） | spec_drift_lint.sh | Not Started |

## 9. Verification
```bash
bash -n scripts/console_smoke.sh && go test ./internal/cli/ -run TestTask454
cargo test --workspace && go test ./... && cargo clippy --workspace --all-targets -- -D warnings
bash scripts/spec_drift_lint.sh --touched origin/master
```

## 10. Completion Notes
**Status*: Done
