#!/usr/bin/env bash
# scripts/lib/adapter.sh — 从 docs/s2v-adapter.md §Commands 读取字段值
#
# 单一来源；implement.md / agents-team.md §0 / agents-solo.md §0 都通过
# `source` 本文件使用，禁止再就地内联同名实现（避免漂移）。
#
# 依赖：bash 3.2+、awk（GNU 或 BSD 均可）。
# 调用方：在项目根（或 worktree 根）执行，依赖 docs/s2v-adapter.md 存在。

set -o pipefail

# 判断 adapter 路径（允许调用方覆盖；默认 repo 根 docs/s2v-adapter.md）
: "${S2V_ADAPTER_PATH:=docs/s2v-adapter.md}"

# s2v_load_cmd <字段名>
#
# 字段名取值（与 templates/adapter.md §Commands 行首一致，**大小写敏感**）：
#   Install / Lint / Typecheck / Unit Test / Integration tests / E2E tests
#   / Coverage / Build / Runtime smoke
#
# 实现说明：
#   - 用 awk `sub()` 删前缀，**不**用 -F: 切分；防止 `pnpm run test:unit` 这种命令值
#     里含冒号被截断。
#   - 找到首个匹配即 exit；adapter 字段不允许重复定义。
#
# 输出：字段值字面量（可能含尾随空格 / `<占位>` / `N/A: <原因>` / 真实命令），
#       由调用方（s2v_run）判读语义。
s2v_load_cmd() {
  awk -v F="$1" '
    BEGIN { pat="^- \\*\\*"F"\\*\\*:[[:space:]]*" }
    $0 ~ pat { sub(pat, ""); print; exit }
  ' "$S2V_ADAPTER_PATH"
}
