#!/usr/bin/env bash
# scripts/lib/preflight.sh — Task Spec Ready Gate 检查（implement.md 步 2）
#
# 提供：
#   s2v_preflight_input <task-spec>     输入路径形态校验（步 0 子集）
#   s2v_preflight_ready  <task-spec>    Ready Gate 全套（Status / TBD / AC / SCEN 计数）
#
# 单一来源；implement.md 步 0/2 / agents-team.md §3 步 4.5 / agents-solo.md §SOP 步 2.5
# 都通过 `source` 本文件使用。
#
# 退出码：0=通过；1=可补救（如 Status=Draft 触发 §2A 交互审）；2=硬性 STOP（用户漏填 / 文件不存在）

set -o pipefail

# S2V helper 仅支持 bash（用到 BASH_SOURCE / local 等 bash 语义；macOS 自
# Catalina(2019) 默认 zsh，zsh 下 `local status` 触只读变量、BASH_SOURCE 未定义
# 会静默取错目录）。给非 bash 用户一个可操作信息，而不是 cryptic 报错。
if [ -z "${BASH_VERSION:-}" ]; then
  echo "❌ S2V 脚本需在 bash 下运行（检测到非 bash shell，如 zsh）。" >&2
  echo "   请改用： bash <脚本>，或在 bash 子 shell 内 source。" >&2
  return 1 2>/dev/null || exit 1
fi

# 解析 task spec 顶部 Status 字段（保留多词值，如 "In Progress"）
# 不能用 awk '{print $1}'：会把 "In Progress" 拆成 "In"。
s2v_read_status() {
  local task_spec="$1"
  grep -E "^\*\*Status\*\*:" "$task_spec" | head -1 \
    | sed -E 's/^\*\*Status\*\*:[[:space:]]*//' \
    | sed -E 's/[[:space:]]+$//'
}

# s2v_preflight_input <task-spec>
#
# 校验路径形态：
#   - 拒绝绝对路径（team 模式 cd worktree 后会写错文件）
#   - 拒绝 ./ ../  前缀（同上）
#   - 必须形如 docs/specs/tasks/*.md
#   - 文件存在
#
# 不校验 Status / TBD / AC（那是 ready gate 的职责，留给 s2v_preflight_ready）
s2v_preflight_input() {
  local task_spec="$1"
  if [ -z "$task_spec" ]; then
    echo "❌ 缺少参数。用法：/s2v-implement docs/specs/tasks/task-X.Y-<name>.md"
    return 2
  fi

  case "$task_spec" in
    /*)
      echo "❌ 不接受绝对路径：$task_spec"
      echo "   原因：team 模式 cd 进 worktree 后绝对路径仍指向主 repo，会污染主 repo 或导致 git add 失败。"
      echo "   请用 repo-relative 路径，如 docs/specs/tasks/task-1.1-parser.md"
      return 2
      ;;
    ./*|../*)
      echo "❌ 不接受 . / .. 前缀的相对路径：$task_spec"
      echo "   请直接用 docs/specs/tasks/task-X.Y-<name>.md"
      return 2
      ;;
    docs/specs/tasks/*.md)
      : # OK
      ;;
    *)
      echo "❌ 路径不是 S2V task spec（应在 docs/specs/tasks/ 下，且为 repo-relative 路径）"
      echo "   PRP plan 请用 /prp-implement"
      return 2
      ;;
  esac

  if [ ! -f "$task_spec" ]; then
    echo "❌ 文件不存在：$task_spec"
    echo "   当前目录：$PWD"
    echo "   提示：必须从 git repo 根目录调用 /s2v-implement"
    return 2
  fi

  return 0
}

# ── 单一源：标准保留字面剥除 ──────────────────────────────────────────
# full-standard.md §8.3 / §10 模板把两类字面**复制进生成的 task spec 且明示
# "review / 完工后无需删除"**：
#   (a) §6 渲染规则 / §9 字段的 `<!-- ... -->` 注释（可跨行）
#   (b) 顶部 Draft 提示 + §10 schema 指引的 `^>` blockquote
#   (c) markdown code spans（inline `...` + fenced ```...``` / ~~~...~~~）——
#       §10 Completion Notes 常含代码示例里的 Rust/Go 类型字面 `Option<String>` /
#       `Vec<u8>` / `HashMap<K,V>` 等，**必须**从 placeholder 检测中剥除，否则宽
#       口径 `<...>` grep 会 false-positive（chore-agents 根治的 Gate 4 缺陷）。
# 二者（a/b）合法含尖括号字面（`<TBD-by-user>` / `<TBD-after-impl>` / 示例 token）。
# 任何对 spec 做 `<...>` 占位 grep 的门（preflight TBD 门 + §10 token 门 + phase §6）
# 若不先剥这两类，就会把保留字面误判为"用户漏填 / 未替换" → 合规 spec 被误杀：
#   DEFECT-1   ：合规 Ready spec rc=2，init→填空→Ready→implement 主链路阻断
#   DEFECT-P3-C：正确完工的 §10 被 team Gate 4 / solo 升档预检误 BLOCK
#   DEFECT-chore-agents：Gate 4 第2道把 `Option<String>` 等反引号内类型误判为
#       未替换 <token>（PR#29 实证），所有含代码例的 PR 都会反复踩。
#
# 剥除逻辑只此一份（下方两个 helper 复用同一 preamble）；任何新增的、对 spec
# 做 `<...>` / `<TBD...>` 占位 grep 的门**必须走这两个 helper 之一，禁止再就地
# 裸 grep** —— DEFECT-1 已立此规，DEFECT-P3-C 即因 §10 token 门未遵守而回归。
# 本次 chore 扩展 (c) code span 剥除后，Gate 4 宽 regex 仍保留（§10 模板需捕获
# 任意 <source-file-1> / <hash1> / <RISK_OR_NONE> 等），但 code 里的 <T> 安全放过。
_S2V_STRIP_PREAMBLE='
  BEGIN { in_c=0; in_fence=0 }
  { line = $0; gsub(/<!--.*-->/, "", line) }                       # 单行注释剥除
  in_c == 0 && line ~ /<!--/ { in_c = 1; sub(/<!--.*/, "", line) } # 进多行注释
  in_c == 1 { if (line ~ /-->/) { in_c = 0; sub(/.*-->/, "", line) } else next }
  # 剥除 fenced code block（``` 或 ~~~ 起始的代码围栏，整块跳过，包括定界行本身）
  # 这样 §10 示例代码块 / phase §6 示例里的 <Type> 不会进入下游 <...> 检测
  line ~ /^[[:space:]]*```/ || line ~ /^[[:space:]]*~~~/ { in_fence = !in_fence; next }
  in_fence { next }
  line ~ /^[[:space:]]*>/ { next }                                 # blockquote 跳过
  # 剥除行内 code span `...`（支持一行多个），使 `Option<String>` / `Result<T,E>` 等
  # 合法类型字面不被 Gate 4 / 其他宽 <[A-Za-z_]...> 检测误杀（chore-agents fix）
  { gsub(/`[^`]*`/, "", line) }
'

# _s2v_real_tbd_hits <task-spec>
# 输出"真实未填 <TBD-by-user> 占位"命中行（格式 `NR: 原行` —— 原始行号 + 原始
# 整行，供 §2A 交互 / preflight 错误信息按原文展示）。剥除保留字面后再判。
# 单一实现：preflight.sh 步 2 与 §2A 交互审协议（references/preflight-interactive-review.md）都调本函数。
_s2v_real_tbd_hits() {
  awk "${_S2V_STRIP_PREAMBLE}"'
    line ~ /<TBD-by-user>/ { print NR ": " $0 }
  ' "$1"
}

# _s2v_strip_retained [file]
# 把"剥除保留字面后的真实内容"逐行输出（保留字面行被丢弃 / 注释/blockquote/code span 被抹空），
# 供下游对 §10 做 `grep -oE "<...>" | sort -u` 占位检测（team Gate 4 第 2 道 /
# solo 升档预检 / phase §6 占位）。无 file 参数则读 stdin（awk 无文件名即读 stdin，最可移植）。
# 现已包含 fenced code block + inline `...` 剥除（chore-agents Gate 4 根治）。
_s2v_strip_retained() {
  if [ -n "${1:-}" ]; then
    awk "${_S2V_STRIP_PREAMBLE}"' { print line }' "$1"
  else
    awk "${_S2V_STRIP_PREAMBLE}"' { print line }'
  fi
}

# s2v_preflight_ready <task-spec>
#
# Ready Gate 全套：Status 合法性 / TBD 占位 / §6 AC 非空 / §7 SCEN 非空。
#
# 通信约定（重要）：
#   - 退出码 0 = Ready 或 In Progress（合法动手）
#   - 退出码 1 = Status=Draft（合法但需 §2A 交互审；调用方进 §2A）
#   - 退出码 2 = 硬性 STOP（Done/Blocked/Waived/未知值/§6 空/§7 空/Ready 但 TBD 残留）
#   - 失败原因写 stderr；非失败信息写 stdout
#   - **不用 export 通信** — 调用方常会用 $(...) 捕获 stdout（子 shell），
#     export 不会传到父 shell。后续状态字段调用方自己用 s2v_read_status 取。
s2v_preflight_ready() {
  local task_spec="$1"
  if ! s2v_preflight_input "$task_spec"; then
    return 2
  fi

  local status
  status="$(s2v_read_status "$task_spec")"

  local is_draft=0
  case "$status" in
    Ready)         : ;;
    "In Progress") : ;;
    Draft)
      is_draft=1
      echo "⚠️  Status=Draft — 进入 §2A 前置审核交互"
      ;;
    Done)
      echo "🛑 STOP: Status=Done — 此 task 已完成。" >&2
      echo "   要重新实施请先把 Status 改回 Ready 并说明原因。" >&2
      return 2
      ;;
    Blocked|Waived)
      echo "🛑 STOP: Status=${status} — 此 task 已被卡住或豁免。" >&2
      echo "   请先解决 BLOCKED / 撤销 Waiver, 再把 Status 改回 Ready。" >&2
      return 2
      ;;
    *)
      echo "🛑 STOP: Status='${status}' (无效值)" >&2
      echo "   合法值见 standard §10.5.1: Draft | Ready | In Progress | Blocked | Waived | Done" >&2
      return 2
      ;;
  esac

  # TBD 占位检查：Draft 路径交给 §2A；Ready/In Progress 路径用户漏填 → STOP
  # 用 _s2v_real_tbd_hits（跳过模板保留的 <!-- 渲染规则 --> 注释块 / Draft
  # 提示 blockquote 中合法字面 <TBD-by-user>，标准 §8.3 明示"无需删除"），
  # 避免合规 Ready spec 被裸 grep 误杀。
  local tbd_hits
  tbd_hits="$(_s2v_real_tbd_hits "$task_spec")"
  if [ -n "$tbd_hits" ]; then
    if [ "$is_draft" != "1" ]; then
      echo "🛑 STOP: Status=${status} 但仍含 <TBD-by-user> 占位 (用户漏填)" >&2
      echo "$tbd_hits" >&2
      echo "" >&2
      echo "   清零所有占位再重跑（模板保留的渲染规则注释 / Draft 提示中的字面不计）。" >&2
      return 2
    fi
  fi

  # §6 AC 不能为空
  local ac_count
  ac_count=$(awk '/^## 6\. /,/^## 7\./' "$task_spec" | grep -cE "^- \[" || true)
  if [ "$ac_count" -eq 0 ]; then
    echo "🛑 STOP: §6 AC 列表为空" >&2
    return 2
  fi

  # §7 追踪表不能为空（应有 SCEN 或 TEST 编号占位）
  local scen_count
  scen_count=$(awk '/^## 7\. /,/^## 8\./' "$task_spec" | grep -cE "SCEN-|TEST-" || true)
  if [ "$scen_count" -eq 0 ]; then
    echo "🛑 STOP: §7 追踪表为空 (应有 SCEN/TEST 编号占位)" >&2
    return 2
  fi

  if [ "$is_draft" = "1" ]; then
    echo "(Draft 状态预扫: ${ac_count} AC, ${scen_count} SCEN/TEST 占位 — 调用方继续 §2A 审核)"
    return 1
  fi

  echo "✅ Preflight 通过: ${ac_count} AC, ${scen_count} SCEN/TEST 占位"
  return 0
}

# s2v_preflight_phase <phase-spec>
#
# Phase 层兜底门禁（C1，三轮黑盒/dogfood 复审互证的最强缺口）。
# S2V 把每个 task 隔离严格 TDD，但"task 拼起来能否集成/协作"的兜底是 phase
# spec §6（阶段级 AC + 端到端 smoke）。该字段 init 时渲染为 <TBD-by-user>、
# 全流程无步骤强制填、此前 preflight **只有 task 层**（solo 完全裸奔，team
# 仅靠 §4 Gate 3 却依赖那个没人填的 <TBD>）。本门禁补这层：phase 的最后一个
# task 完工/合并前，phase §6 必须已填实，且 phase Status 是 §10.5.1 合法值。
#
# §6 按**章节号**范围抽取（与 §6 标题文字无关，防标题漂移）；空/占位判定
# 复用单一源 _s2v_strip_retained（**禁止裸 grep**，与 DEFECT-1 / P3-C / chore-agents 同规 —
# 见上方 _S2V_STRIP_PREAMBLE 注释，已含 code span 剥除）。容忍 **Status**: 与 Status: 两种渲染。
#
# 退出码：0=§6 已填实且 Status 合法（放行最后 task 完工/合并）
#         2=硬性 STOP（路径非法/文件不存在/Status 缺失或非法/§6 空/§6 含未替换占位）
s2v_preflight_phase() {
  local phase_spec="$1"
  if [ -z "$phase_spec" ]; then
    echo "❌ 缺少参数。用法：s2v_preflight_phase docs/specs/phases/phase-N-<name>.md" >&2
    return 2
  fi
  case "$phase_spec" in
    docs/specs/phases/*.md) : ;;
    *)
      echo "❌ 不是 phase spec 路径（应在 docs/specs/phases/ 下，repo-relative）：$phase_spec" >&2
      return 2 ;;
  esac
  if [ ! -f "$phase_spec" ]; then
    echo "❌ phase spec 不存在：$phase_spec" >&2
    return 2
  fi

  # Status 合法性（phase 与 task 共用 §10.5.1 枚举；容忍 **Status**: / Status: 两种渲染）
  local pstatus
  pstatus="$(grep -E "^(\*\*Status\*\*|Status):" "$phase_spec" | head -1 \
    | sed -E 's/^(\*\*Status\*\*|Status):[[:space:]]*//; s/[[:space:]]+$//')"
  case "$pstatus" in
    Draft|Ready|"In Progress"|Blocked|Waived|Done) : ;;
    "")
      echo "🛑 STOP: phase spec 缺 Status 字段（§10.5.1 合法值 Draft/Ready/In Progress/Blocked/Waived/Done）" >&2
      return 2 ;;
    *)
      echo "🛑 STOP: phase Status='${pstatus}' 非 §10.5.1 合法值" >&2
      return 2 ;;
  esac

  # §6（阶段级 AC + 端到端 smoke）必须已填实：按章节号抽正文，剥模板保留字面后判
  local body
  body="$(awk '/^## 6\. /,/^## 7\. /' "$phase_spec" \
    | awk 'NR>1 && $0 !~ /^## 7\. /')"
  local real
  real="$(printf '%s\n' "$body" | _s2v_strip_retained | sed '/^[[:space:]]*$/d')"
  if [ -z "$real" ]; then
    echo "🛑 STOP: phase spec §6（阶段级 AC + 端到端 smoke）为空 —" >&2
    echo "   该 phase 的最后 task 完工/合并前必须填实（集成兜底，C1）。" >&2
    return 2
  fi
  # §6 占位判定：**只认 S2V 保留占位 token**（<TBD-by-user> / <TBD-after-impl>）。
  # 不再用宽口径 <任意词> —— 后者把 §6 里合法的示例尖括号语法（如 `status <run>`、
  # `<pipeline-id>`）误判为"未替换占位" → 主链路假阳性 STOP（与 DEFECT-1 同类：
  # 合法 spec 字面被误杀，且规范无转义/豁免出口，逼用户改写绕过）。init 在 §6
  # 只渲染 <TBD-by-user>（init.md 步 8 §6 提示），保留 token 集封闭且 S2V 自控，
  # 故收窄是**安全方向**：不漏任何 S2V 真渲染的未填标记，仅放过用户写的示例性
  # 尖括号（合法 spec 内容）。仍走 _s2v_strip_retained（保留注释/blockquote 字面剥除）。
  local place
  place="$(printf '%s\n' "$body" | _s2v_strip_retained \
    | grep -oE "<TBD-by-user>|<TBD-after-impl>" | sort -u || true)"
  if [ -n "$place" ]; then
    echo "🛑 STOP: phase spec §6 仍含未替换占位（必须填实，含 init 渲染的 <TBD-by-user>）：" >&2
    echo "$place" | sed 's/^/  - /' >&2
    return 2
  fi

  echo "✅ Phase preflight 通过: §6 阶段级 AC + 端到端 smoke 已填实 (Status=${pstatus})"
  return 0
}

# s2v_guard_fixture_tracked <fixture-path>
#
# C10/D-3（黑盒复审项）：日志/数据类项目 fixture 常用 *.log / *.tmp / *.csv，
# 恰被 /s2v-init baseline .gitignore（或用户自定义忽略段）命中。git add 对忽略
# 文件**静默拒绝**（不报错、git status 不显示）→ fixture 不入版本库 → 其他
# agent / CI 无法复现该测试，而 /s2v-add 仍报 ✅ = 产物违背自身契约 + 误导。
# 本守卫在 git add 前显式探测：命中忽略即 STOP（不静默、不放行）。也兜用户
# 自定义 .gitignore，不止 S2V baseline。
#
# 诚实边界：只保证"作为 fixture 提交的文件确实没被 .gitignore 吞"，不校验内容。
#
# 退出码：0=不被忽略（可入库）；1=被 .gitignore 命中（STOP）；2=参数/环境错
s2v_guard_fixture_tracked() {
  local f="${1:-}"   # A8：set -u 下零参须落到下方 rc2 分支，不可裸 $1（unbound 退出违反契约）
  if [ -z "$f" ]; then
    echo "❌ s2v_guard_fixture_tracked: 缺 fixture 路径参数" >&2
    return 2
  fi
  if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "❌ s2v_guard_fixture_tracked: 不在 git 工作树内（无法判定是否被忽略）" >&2
    return 2
  fi
  if git check-ignore -q -- "$f" 2>/dev/null; then
    local rule
    rule="$(git check-ignore -v -- "$f" 2>/dev/null || true)"
    echo "🛑 STOP: fixture '$f' 命中 .gitignore，不会进版本库 —— 其他 agent / CI 无法复现该测试。" >&2
    echo "   命中规则：${rule:-（git check-ignore -v 无输出）}" >&2
    echo "   三选一后重跑：" >&2
    echo "   · .gitignore 末尾加例外（推荐——fixture 是测试事实源必须入库）：" >&2
    echo "       !test/fixtures/" >&2
    echo "       !test/fixtures/**" >&2
    echo "     ⚠️ A4：若某**父目录整体被忽略**（.gitignore 含 \`/test\` 或 \`test/\` 等），" >&2
    echo "        git 规则下无法只 re-include 子文件 —— 必须**逐层、父在子前**解忽略：" >&2
    echo "          !test/" >&2
    echo "          !test/fixtures/" >&2
    echo "          !test/fixtures/**" >&2
    echo "        （\`git check-ignore -v\` 显示的命中规则即指出是哪一层在忽略）。" >&2
    echo "   · 该 fixture 换不被忽略的扩展名 / 路径；" >&2
    echo "   · 若确属临时数据不该入库 → 它就不该作 fixture 提交（改测试内联生成）。" >&2
    return 1
  fi
  return 0
}

# s2v_guard_areas_tracked <area> [<area> ...]
#
# claim-3a 残留 / C10 的**源码版**：implement 步 6 RED `git add <UNIT_TEST_AREAS>`
# （编译型再加 <SOURCE_AREAS>）。git add 对被 .gitignore 命中的路径**静默跳过**
# （不报错、git status 不显示）。若声明的 SOURCE/UNIT area 被过宽 ignore 规则
# 遮蔽（典型：裸二进制名 `goflow` 同时吞 Go `cmd/goflow/` 源码目录），RED 会
# 丢源码 / 假绿且无任何提示——C10 的 s2v_guard_fixture_tracked 仅 fixture-scoped，
# 源码目录此前无机械兜底。本守卫在 RED commit 前显式探测。
#
# 安全不对称：只对「磁盘有该文件 + **未 track** + 被 .gitignore 命中」这一**精确
# 静默丢失签名**报警 → 可恢复的 spurious STOP；不对「area 本次无新文件」误报
# （那是合法的，cry-wolf 会逼用户禁用守卫 = 更糟）。已 track 的文件即使匹配
# .gitignore 也不会丢 → 跳过（不误报已安全文件）。prune 依赖/构建/VCS 目录，
# 不被 vendored/build 产物干扰。
#
# 退出码：0=无静默遮蔽；1=存在未 track 且被忽略的源码/测试文件（STOP）；
#         2=参数/环境错（无 area 参 / 不在 git 工作树）—— 与 rc1 区分不混淆
s2v_guard_areas_tracked() {
  if [ "$#" -eq 0 ]; then
    echo "❌ s2v_guard_areas_tracked: 缺 area 参数（SOURCE/UNIT pathspec，空格分隔）" >&2
    return 2
  fi
  if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "❌ s2v_guard_areas_tracked: 不在 git 工作树内（无法判定是否被忽略）" >&2
    return 2
  fi
  local hit=0 a f base
  for a in "$@"; do
    [ -e "$a" ] || continue   # area 未在磁盘 → 非本守卫职责（area 未填充是另一回事）
    # 用 heredoc+命令替换喂 while（**非管道**），使 hit 赋值留在当前 shell（管道子 shell 会丢）
    while IFS= read -r f; do
      [ -n "$f" ] || continue
      base="${f##*/}"
      case "$base" in .DS_Store|Thumbs.db|*.swp) continue ;; esac      # OS/编辑器垃圾非源码，不 cry-wolf
      git ls-files --error-unmatch -- "$f" >/dev/null 2>&1 && continue # 已track显式快路（git check-ignore 默认本就排除已track，此为版本无关的冗余防御、非承重）
      git check-ignore -q -- "$f" 2>/dev/null || continue              # 未被忽略 → git add 正常入库
      if [ "$hit" -eq 0 ]; then
        echo "🛑 STOP: 声明的 SOURCE/UNIT area 下存在**未 track 且被 .gitignore 命中**的文件 —— git add 静默跳过，RED 会丢源码 / 假绿：" >&2
      fi
      hit=1
      echo "  - $f" >&2
      git check-ignore -v -- "$f" 2>/dev/null | sed 's/^/    命中规则：/' >&2
    done <<EOF
$(find "$a" \( -name .git -o -name node_modules -o -name vendor -o -name target -o -name dist -o -name build -o -name out -o -name bin -o -name obj -o -name .venv -o -name venv -o -name __pycache__ -o -name .pytest_cache -o -name .mypy_cache -o -name .gradle -o -name .idea -o -name .vscode \) -prune -o -type f -print 2>/dev/null)
EOF
  done
  if [ "$hit" -ne 0 ]; then
    echo "   修复（与 C10 同款，三选一后重跑 RED）：" >&2
    echo "   · 把过宽 .gitignore 规则 **repo 根锚定**（裸 \`goflow\` → \`/goflow\`），避免吞同名源码目录；" >&2
    echo "   · 或末尾加例外解忽略；若**父目录整体被忽略**须**逐层、父在子前**：" >&2
    echo "       !cmd/" >&2
    echo "       !cmd/<sub>/" >&2
    echo "       !cmd/<sub>/**" >&2
    echo "     （\`git check-ignore -v\` 显示的命中规则即指出哪一层在忽略）；" >&2
    echo "   · 确属不该入库 → 它就不该在声明的 SOURCE/UNIT area 下。" >&2
    return 1
  fi
  return 0
}
