#!/usr/bin/env bash
# scripts/spec_drift_lint.sh — ADR-014 D2 击鼓传花条款 lint.
#
# 扫描 docs/specs/ 中 anti-pattern 关键词（"留给 Phase X+" / "out of scope" /
# "本 task 仅" / "stub" / "占位" 等），强制 spec 作者就近用
# [SPEC-DEFER:<name>] 或 [SPEC-OWNER:<task>] 标注。
#
# 模式：
#   (默认 / --report) 全扫报告模式（informational）：列出每条命中 + 区分已/未标注；exit 0
#   --strict                 严格模式：任一未标注命中 → exit 1（适合 spec PR self-check）
#   --touched [BASE]         PR 增量模式：仅 git diff BASE..HEAD 触及行强制 lint；
#                            BASE 默认 origin/master，缺失则 master
#   --self-test              内置 fixture 自检；验证捕获 + 标注豁免；exit 0 on success
#   --help                   显示用法
#
# Scope（ADR-014 §D2 明确）：仅扫 docs/specs/**.md。docs/decisions/ / docs/retrospectives/ /
# docs/prds/ / AGENTS.md / README.md 等不在 lint scope（讨论 anti-pattern 是合法语境）。
#
# 历史 spec 兼容：默认 / --report / --strict 全扫但 --touched 仅强制 PR 触及行（D5 历史不溯改）。
#
# 退出码：0 OK / 1 lint 违规（仅 --strict / --touched）/ 2 用法错误

set -euo pipefail

LINT_SCOPE="docs/specs"

# anti-pattern 词表（ADR-014 §D2）— 用 ERE，按 grep -E 兼容
ANTI_PATTERN_REGEX='留给\s*Phase|留\s*Phase\s*[0-9]+\+?|推给\s*task-|Phase\s*[0-9]+\+1|本\s*task\s*仅|仅\s*scope|out\s+of\s+scope|历史\s*gap|历史\s*drift|历史问题|留\s*future|留\s*v[0-9]+\.[0-9]+\+?|v[0-9]+\.[0-9]+\.x|future\s+task|not\s+implemented|unimplemented|stub|占位|scaffold|mock'
# OOS 单词（避免上下文 false-positive，单独列出 word boundary）
OOS_PATTERN='\bOOS\b'

ANNOTATION_REGEX='\[SPEC-(DEFER|OWNER):'

usage() {
  sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'
  exit 2
}

is_skippable_line() {
  local line="$1"
  # 1. markdown 标题（以 # 开头）不扫
  echo "$line" | grep -qE '^[[:space:]]*#' && return 0
  # 2. 已含标注 [SPEC-DEFER:...] / [SPEC-OWNER:...]
  echo "$line" | grep -qE "$ANNOTATION_REGEX" && return 0
  # 3. 表格分隔行（| --- |）不扫
  echo "$line" | grep -qE '^[[:space:]]*\|[-[:space:]|:]+\|?[[:space:]]*$' && return 0
  return 1
}

scan_file() {
  local f="$1"
  local in_code_fence=0
  local line_no=0
  while IFS='' read -r line; do
    line_no=$((line_no + 1))
    # 代码围栏 toggle
    if [[ "$line" =~ ^[[:space:]]*\`\`\` ]]; then
      in_code_fence=$((1 - in_code_fence))
      continue
    fi
    [ "$in_code_fence" -eq 1 ] && continue
    is_skippable_line "$line" && continue
    # 主 grep
    if echo "$line" | grep -qE "$ANTI_PATTERN_REGEX" \
       || echo "$line" | grep -qE "$OOS_PATTERN"; then
      printf '%s:%d:%s\n' "$f" "$line_no" "$line"
    fi
  done < "$f"
}

scan_all() {
  local files=()
  while IFS='' read -r f; do
    files+=("$f")
  done < <(find "$LINT_SCOPE" -type f -name '*.md' 2>/dev/null | sort)
  [ "${#files[@]}" -eq 0 ] && {
    echo "WARN: no files under $LINT_SCOPE" >&2
    return 0
  }
  for f in "${files[@]}"; do
    scan_file "$f"
  done
}

mode_report() {
  local hits
  hits="$(scan_all)"
  if [ -z "$hits" ]; then
    echo "✅ spec_drift_lint: 0 anti-pattern hits in $LINT_SCOPE"
    return 0
  fi
  local count
  count="$(echo "$hits" | grep -c .)"
  echo "📋 spec_drift_lint: $count anti-pattern hits (informational; baseline 模式不阻断)"
  echo ""
  echo "$hits" | sed 's/^/  /'
  echo ""
  echo "→ 加 [SPEC-DEFER:<name>] 或 [SPEC-OWNER:<task>] 标注即可豁免。"
  return 0
}

mode_strict() {
  local hits
  hits="$(scan_all)"
  if [ -z "$hits" ]; then
    echo "✅ spec_drift_lint --strict: 0 unannotated anti-pattern hits"
    return 0
  fi
  local count
  count="$(echo "$hits" | grep -c .)"
  echo "❌ spec_drift_lint --strict: $count unannotated anti-pattern hits"
  echo ""
  echo "$hits" | sed 's/^/  /'
  echo ""
  echo "→ 加 [SPEC-DEFER:<name>] 或 [SPEC-OWNER:<task>] 标注；或拒绝在 spec 中描述延后行为。"
  return 1
}

mode_touched() {
  local base="${1:-}"
  if [ -z "$base" ]; then
    if git rev-parse --verify origin/master >/dev/null 2>&1; then
      base="origin/master"
    elif git rev-parse --verify master >/dev/null 2>&1; then
      base="master"
    else
      echo "ERROR: --touched needs BASE; pass explicit arg or set origin/master" >&2
      return 2
    fi
  fi
  # 取 PR 增量中 docs/specs/ 下 .md 触及行号
  # git diff --unified=0 输出 hunk header 形如 @@ -L1,N1 +L2,N2 @@
  # 仅收集 + 行（新增/修改）
  local diff_out
  diff_out="$(git diff --unified=0 "$base"...HEAD -- "$LINT_SCOPE/**/*.md" 2>/dev/null || true)"
  if [ -z "$diff_out" ]; then
    echo "✅ spec_drift_lint --touched $base: no docs/specs/ changes in PR"
    return 0
  fi
  # 解析 diff，得到 (file, line_no) → 行内容（新版）映射
  # 复用 scan_all 输出全量命中，然后过滤
  local all_hits
  all_hits="$(scan_all)"
  [ -z "$all_hits" ] && {
    echo "✅ spec_drift_lint --touched $base: 0 hits in changed lines"
    return 0
  }
  # 抽 diff 中 (file, +line_no) 集合
  local touched_lines
  touched_lines="$(echo "$diff_out" | awk '
    /^\+\+\+ / { sub(/^\+\+\+ b\//, "", $0); file=$0; next }
    /^@@ / {
      # @@ -X,Y +A,B @@
      if (match($0, /\+[0-9]+(,[0-9]+)?/)) {
        m=substr($0, RSTART+1, RLENGTH-1)
        n=index(m, ",")
        if (n) {
          start=substr(m,1,n-1)+0
          count=substr(m,n+1)+0
        } else { start=m+0; count=1 }
        for (i=0;i<count;i++) print file ":" (start+i)
      }
    }
  ')"
  local touched_hits=""
  while IFS='' read -r hit; do
    [ -z "$hit" ] && continue
    local hit_loc
    hit_loc="$(echo "$hit" | awk -F: '{print $1":"$2}')"
    if echo "$touched_lines" | grep -qx "$hit_loc"; then
      touched_hits+="$hit"$'\n'
    fi
  done <<< "$all_hits"
  if [ -z "$touched_hits" ]; then
    echo "✅ spec_drift_lint --touched $base: 0 unannotated hits in changed lines"
    return 0
  fi
  local count
  count="$(echo "$touched_hits" | grep -c .)"
  echo "❌ spec_drift_lint --touched $base: $count unannotated hits in PR-changed lines"
  echo ""
  echo "$touched_hits" | sed 's/^/  /'
  echo ""
  echo "→ PR 触及行须 [SPEC-DEFER:<name>] 或 [SPEC-OWNER:<task>] 标注（D5 历史不溯改）。"
  return 1
}

mode_self_test() {
  local tmp
  tmp="$(mktemp -d)"
  trap "rm -rf '$tmp'" RETURN
  mkdir -p "$tmp/docs/specs/tasks"
  cat > "$tmp/docs/specs/tasks/task-fake.md" <<'EOF'
# Task fake — self-test fixture

## 3. Scope & OOS

- 完整功能 A 留给 Phase X+1：本行**未**标注，应被捕获
- 完整功能 B 留 future v0.3+ [SPEC-DEFER:task-future.b]：本行已标注，应豁免
- 完整功能 C 推给 task-99.99：本行**未**标注，应被捕获
- 完整功能 D out of scope [SPEC-OWNER:task-99.1]：已标注，应豁免

## 5. Code (内容在代码围栏，应整体跳过)

```rust
fn stub() -> ! { unimplemented!() } // 'stub' / 'unimplemented' 在 code 内
```

# 这是 heading 含 OOS 字面，应跳过

普通段落含 mock 字面但不在任一上下文 — 应被捕获（散文中的 anti-pattern）
EOF
  # 临时切换 LINT_SCOPE 到 fixture
  local saved_scope="$LINT_SCOPE"
  LINT_SCOPE="$tmp/docs/specs"
  local hits
  hits="$(scan_all)"
  LINT_SCOPE="$saved_scope"
  # 预期 3 行命中：未标注 Phase X+1 + 未标注 推给 task- + 散文 mock
  local expected=3
  local actual
  if [ -z "$hits" ]; then
    actual=0
  else
    actual="$(echo "$hits" | grep -c .)"
  fi
  echo "self-test fixture hits:"
  echo "$hits" | sed 's/^/  /'
  echo ""
  if [ "$actual" -eq "$expected" ]; then
    echo "✅ spec_drift_lint --self-test: PASS ($actual hits == expected $expected)"
    return 0
  else
    echo "❌ spec_drift_lint --self-test: FAIL ($actual hits != expected $expected)"
    return 1
  fi
}

main() {
  local mode="report"
  local base=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --report) mode="report"; shift ;;
      --strict) mode="strict"; shift ;;
      --touched) mode="touched"; shift; if [ $# -gt 0 ] && [[ "$1" != --* ]]; then base="$1"; shift; fi ;;
      --self-test) mode="self-test"; shift ;;
      --help|-h) usage ;;
      *) echo "ERROR: unknown arg $1" >&2; usage ;;
    esac
  done
  case "$mode" in
    report)    mode_report ;;
    strict)    mode_strict ;;
    touched)   mode_touched "$base" ;;
    self-test) mode_self_test ;;
  esac
}

main "$@"
