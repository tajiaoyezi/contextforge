#!/usr/bin/env bash
# scripts/lib/verify.sh — Verification Plan 执行 + §9 解析
#
# 提供：
#   s2v_run <key> [required]            按 adapter §Commands 字段判读 + 执行
#   s2v_extract_verify_keys <task-spec> 从 task spec §9 抽出 key 列表（固定执行序）
#   s2v_verify_full <"key1 key2 ...">   按列表跑全套，unit-test 强制
#
# 单一来源；implement.md / agents-team.md §0 / agents-solo.md §0 都通过
# `source` 本文件使用，禁止再就地内联同名实现（避免漂移）。
#
# 字段语义（与 docs/s2v/standard.md §11.2 / templates/adapter.md §Commands 一致）：
#   - 真实命令      → 执行
#   - "<占位>"      → adapter 未替换，hard-fail（视为模板事故）
#   - 空 / N/A:<原因> → 合法跳过（required 模式下仍 hard-fail）
#   - unit-test 永远是 required（§9 强制门槛，不接受 N/A）
#
# 依赖：bash 3.2+、awk、scripts/lib/adapter.sh

set -o pipefail

# S2V helper 仅支持 bash（用到 BASH_SOURCE / local 等 bash 语义；macOS 自
# Catalina(2019) 默认 zsh，zsh 下 `local status` 触只读变量、BASH_SOURCE 未定义
# 会静默取错目录）。给非 bash 用户一个可操作信息，而不是 cryptic 报错。
if [ -z "${BASH_VERSION:-}" ]; then
  echo "❌ S2V 脚本需在 bash 下运行（检测到非 bash shell，如 zsh）。" >&2
  echo "   请改用： bash <脚本>，或在 bash 子 shell 内 source。" >&2
  return 1 2>/dev/null || exit 1
fi

# Source 同目录的 adapter helper（不重复定义）
_S2V_VERIFY_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=adapter.sh
source "${_S2V_VERIFY_LIB_DIR}/adapter.sh"

# key → adapter §Commands 字段名（**单一映射**；新增 verification 字段时只改这里）
_s2v_key_to_field() {
  case "$1" in
    install)        echo "Install" ;;
    lint)           echo "Lint" ;;
    typecheck)      echo "Typecheck" ;;
    unit-test)      echo "Unit Test" ;;
    integration)    echo "Integration tests" ;;
    e2e)            echo "E2E tests" ;;
    coverage)       echo "Coverage" ;;
    build)          echo "Build" ;;
    runtime-smoke)  echo "Runtime smoke" ;;
    *) return 1 ;;
  esac
}

# 固定安全执行序（依赖图：装依赖 → 静态检查 → 单测 → 集成/E2E → 构建 → 覆盖率 → 运行时 smoke → 手动）
# 这个顺序由"前置依赖"决定（如 unit-test 必须在 e2e 前；build 必须在 runtime-smoke 前），
# 与具体测试框架无关。新增 key 时**按依赖位置插入**，不要追加到末尾。
_S2V_VERIFY_KEYS_ORDER=(install lint typecheck unit-test integration e2e build coverage runtime-smoke manual)

# 标准字段名（用于 §9 解析的 awk 正则；与上面 order 同步）
_S2V_VERIFY_FIELD_PATTERNS='Install|Lint|Typecheck|Unit|Integration|E2E|Coverage|Build|Runtime|Manual'

# s2v_run <key> [required]
#
# 按 adapter §Commands 字段值判读语义并执行。
# unit-test 自动设 required（§9 强制门槛）；manual 走交互式确认（不读 adapter）。
#
# 退出码：0=成功（执行通过 / 合法跳过 / manual 已确认）；1=命令执行失败；2=配置错误（占位/required 缺失）。
s2v_run() {
  local key="$1" required="${2:-}"
  [ "$key" = "unit-test" ] && required="required"

  # manual 不读 adapter — 由 task spec §9 直接列步骤，agent / 用户交互式确认
  if [ "$key" = "manual" ]; then
    echo "⏸  manual 步骤不可自动执行 — 按 task §9 Manual 列出的核验步骤手工完成，OK 后输入 y 继续"
    local ack
    read -r ack < /dev/tty 2>/dev/null || ack="n"
    if [ "$ack" = "y" ]; then
      echo "✅ manual 已确认"
      return 0
    fi
    echo "❌ manual 未确认（输入≠y）"
    return 2
  fi

  local field
  if ! field="$(_s2v_key_to_field "$key")"; then
    echo "❌ s2v_run 未知 key: ${key}"
    return 2
  fi

  local cmd
  cmd="$(s2v_load_cmd "$field")"

  case "$cmd" in
    "<"*">"*)
      echo "❌ adapter §Commands - ${field} 仍是未替换占位: ${cmd}"
      echo "   编辑 ${S2V_ADAPTER_PATH} 把 <...> 替换为真实命令或显式 \"N/A: <原因>\" 再来。"
      return 2
      ;;
    ""|"N/A"|"N/A:"*)
      if [ "$required" = "required" ]; then
        echo "❌ ${key} 是强制门槛, adapter 不允许 N/A / 留空 (当前值=\"${cmd}\")"
        return 2
      fi
      echo "⏭  跳过 ${key}: adapter 显式 N/A 或留空 (值=\"${cmd}\")"
      return 0
      ;;
    *)
      echo "▶ ${key}: ${cmd}"
      eval "$cmd"
      ;;
  esac
}

# 冷启动检测的"非源/测试脚手架" denylist（**单一源** —— C9' A1）：只放**绝不可能
# 是源码/测试**的类别（docs/manifest/lockfile/VCS/license/.gitkeep）。**源码扩展名
# 永不入此名单**，否则会把"有真实代码 + 真红"误判 greenfield 静默放过（前一版正向
# 白名单的危险不对称即此）。匹配 basename / 扩展名。
_S2V_BASELINE_SCAFFOLD_RE='(^|/)(\.gitignore|\.gitattributes|\.gitkeep|\.keep|\.editorconfig|\.dockerignore|Dockerfile|Makefile|LICENSE[^/]*|COPYING[^/]*|NOTICE[^/]*|AGENTS\.md|README[^/]*|CHANGELOG[^/]*|go\.mod|go\.sum|package\.json|package-lock\.json|npm-shrinkwrap\.json|yarn\.lock|pnpm-lock\.yaml|Cargo\.lock|Pipfile|Pipfile\.lock|requirements[^/]*\.txt|setup\.py|setup\.cfg|tox\.ini|pyproject\.toml|build\.gradle(\.kts)?|settings\.gradle(\.kts)?|gradle\.properties|pom\.xml|__init__\.py)$|\.(md|markdown|txt|rst|adoc|toml|cfg|ini|ya?ml|json|xml|lock|properties|env)$'

# 冷启动扫描 -prune 的依赖/构建/VCS/spec 目录（**单一源** —— C9' A2）：镜像 init
# baseline .gitignore + S2V `docs/` spec 与拷贝工具家目录。install 产物 / vendored
# 代码 / 拷进项目的 helper 脚本不得污染 greenfield 判定。
_S2V_BASELINE_PRUNE_DIRS='.git node_modules vendor target dist build out bin obj .venv venv __pycache__ .pytest_cache .mypy_cache .gradle .idea .vscode coverage docs'

# s2v_baseline_green <unit-test-area-paths>
#
# implement.md 步 3 / agents-solo SOP 步 1 / agents-team §3 步 3 的「基线绿」**单一实现**
# （三处禁止再各自内联 install/typecheck/unit-test —— 防冷启动判定漂移）。
#
# 冷启动豁免（框架无关、确定性、不靠 runner 退出码；**C9' 修正设计**）：
#   greenfield 首个 task 源码与测试基建均未建立 → install（npm ci / pip -r 无
#   manifest）、typecheck（go vet ./... 零包）、unit-test（go test ./... 零包）
#   **都必然非零但非真红**。确证 greenfield → **三者一并跳过**。
#
#   判定**先于 install** 执行（C9' A3：install 不能作无条件前置，否则无 manifest
#   的 greenfield 在判定前就 `s2v_run install || return 1` 死路；提前判定亦使
#   install 产物 / vendored 代码不在判定时存在 —— 釜底抽薪 C9' A2）。
#
#   判据 = **排除式 + 安全偏置**（C9' A1，**纠正前一版正向白名单的危险不对称**）：
#     areas 全部路径存在，且 `find` prune `_S2V_BASELINE_PRUNE_DIRS` 后，剩余文件
#     **全部命中 `_S2V_BASELINE_SCAFFOLD_RE`（脚手架，永不含源码扩展名）**。
#     → 任意"非脚手架"文件（任意语言源码，含 .vue/.hs/.dart 等未枚举语言、Rust
#       src/ 内嵌测试源文件）= 真实内容 = **非冷启动** = 跑门禁 = 真红绝不被掩盖。
#     失败方向安全：最坏 = 异类脚手架的真 greenfield 多一次可恢复死路；**绝不会**
#     反过来把"有真实代码 + 真红"误判 greenfield 而静默放过。
#   未替换占位（字面 <...> 硬错 rc1）/ 未传 areas / 任一路径不存在 → 非冷启动证据
#   → 安全侧跑全部门禁。
#
# 退出码：0=基线绿（或合法冷启动跳过）；非 0=某步真失败（调用方应 exit）
s2v_baseline_green() {
  local areas="$1"

  # 占位未替换（字面 <UNIT_TEST_AREAS>）绝不可当冷启动 → 硬错（**先于冷启动判定**）
  case "$areas" in
    *"<"*">"*)
      echo "❌ s2v_baseline_green: <...> 占位未替换：'$areas'" >&2
      echo "   读 adapter §Source And Test Areas > Unit test areas，替换为真实路径再跑。" >&2
      return 1 ;;
  esac

  # 冷启动判定（**先于 install/typecheck/unit-test** —— C9' A3）：areas 全部存在，
  # prune 依赖/构建/docs 目录（C9' A2）后**无任何"非脚手架"文件**（C9' A1：排除式
  # +安全偏置，未知文件=真实内容=非冷启动，denylist 永不含源码扩展名）。
  local all_exist=1 has_content="" _a
  if [ -z "$areas" ]; then
    all_exist=0
  else
    for _a in $areas; do [ -e "$_a" ] || { all_exist=0; break; }; done
    if [ "$all_exist" = "1" ]; then
      local _pd _prune=""
      for _pd in $_S2V_BASELINE_PRUNE_DIRS; do _prune="$_prune -name $_pd -o"; done
      _prune="${_prune% -o}"
      # shellcheck disable=SC2086  # areas 多 pathspec + _prune find 谓词均需分词展开
      has_content="$(find $areas \( $_prune \) -prune -o -type f -print 2>/dev/null \
        | grep -Ev "$_S2V_BASELINE_SCAFFOLD_RE" | head -n1 || true)"
    fi
  fi

  if [ "$all_exist" = "1" ] && [ -z "$has_content" ]; then
    echo "⏭  冷启动：areas 全部路径存在，prune 依赖/构建/docs 后无任何真实源码/测试文件 = greenfield"
    echo "   首个 task，源码/单测基建未建立 → **跳过基线 install + typecheck + unit-test**"
    echo "   （install 如 npm ci / pip install -r 在无 manifest 时本就失败；typecheck 如"
    echo "    go vet ./... / unit 如 go test ./... 零包必非零而非真红；步 6/7 建好后正常强制）。"
    echo "   ⚠️ 完工时在 task §10 备注：冷启动—首 task 前无源码/测试基建。"
  else
    [ -n "$areas" ] && [ "$all_exist" = "0" ] && \
      echo "ℹ️  §Unit test areas 有路径不存在或未传（'$areas'）—— 不按冷启动跳过（安全侧，防掩盖真红）；若确为 greenfield，确认 /s2v-init 步 5.5 已建**全部**测试目录后重跑。"
    s2v_run install   || return 1
    s2v_run typecheck || return 1
    s2v_run unit-test || return 1
  fi
  echo "✅ 基线绿"
}

# s2v_extract_verify_keys <task-spec-path>
#
# 从 task spec §9 Verification Plan 段抽出 key，按 _S2V_VERIFY_KEYS_ORDER 排序输出。
# 不依赖 §9 内书写顺序；不会被 sort -u 的字母序打乱。
#
# 副作用：§9 段中如有非标字段名（拼写错），向 stderr 打印 loud-warn（不 fail —
# 允许 §9 列说明性自由文本，如 "- 注：跳过 e2e 因为 ..."）。
s2v_extract_verify_keys() {
  local task_spec="$1"
  if [ ! -f "$task_spec" ]; then
    echo "❌ task spec 不存在: ${task_spec}" >&2
    return 2
  fi

  local section9
  section9="$(awk '/^## 9\. /,/^## 10\./' "$task_spec")"

  local raw
  raw="$(echo "$section9" | awk '
    /^- (\*\*)?Install/        { print "install" }
    /^- (\*\*)?Lint/           { print "lint" }
    /^- (\*\*)?Typecheck/      { print "typecheck" }
    /^- (\*\*)?Unit/           { print "unit-test" }
    /^- (\*\*)?Integration/    { print "integration" }
    /^- (\*\*)?E2E/            { print "e2e" }
    /^- (\*\*)?Coverage/       { print "coverage" }
    /^- (\*\*)?Build/          { print "build" }
    /^- (\*\*)?Runtime/        { print "runtime-smoke" }
    /^- (\*\*)?Manual/         { print "manual" }
  ')"

  # Sanity check：§9 一级 list 行总数 vs 识别行数 → 字段名拼错的 silent skip 风险
  local total recognized
  total=$(echo "$section9" | grep -cE "^- " || true)
  recognized=$(echo "$raw" | grep -c . || true)
  if [ "$total" -gt "$recognized" ]; then
    {
      echo "⚠️  §9 中有 $((total - recognized)) 行未识别为标准验证字段，将被跳过："
      echo "$section9" | grep -E "^- " \
        | grep -vE "^- (\*\*)?(${_S2V_VERIFY_FIELD_PATTERNS})" \
        | sed 's/^/      /'
      echo "    标准字段名：Install / Lint / Typecheck / Unit Test / Integration tests / E2E tests / Coverage / Build / Runtime smoke / Manual"
      echo "    若上述行确为说明性自由文本，可忽略；否则修正字段名（常见错误：'Type Check' 多空格 / 'type-check' 连字符 / 复数缺失）。"
    } >&2
  fi

  # 按固定执行序输出
  local out=""
  local key
  for key in "${_S2V_VERIFY_KEYS_ORDER[@]}"; do
    if echo "$raw" | grep -qx "$key"; then
      out="$out $key"
    fi
  done
  echo "${out# }"
}

# s2v_verify_full <key-list-string>
#
# 按列表逐个跑 s2v_run。空列表 / 缺 unit-test 一律 hard-fail。
#
# 退出码：0=全套通过；1=某 key 执行失败（task agent 应进卡住协议）；2=配置错误。
s2v_verify_full() {
  local keys="$1"
  if [ -z "$keys" ]; then
    echo "❌ §9 Verification Plan 为空或解析失败"
    echo "   排查：检查 task spec §9 标题是否为 '## 9. Verification Plan'，条目是否以 '- Install/Lint/Typecheck/Unit/Integration/E2E/Coverage/Build/Runtime/Manual' 起头"
    return 2
  fi
  case " $keys " in
    *" unit-test "*) : ;;
    *)
      echo "❌ §9 Verification Plan 缺 Unit Test — unit-test 是强制门槛，不接受省略"
      return 2
      ;;
  esac

  local key
  for key in $keys; do
    if ! s2v_run "$key"; then
      echo "🛑 §9 在 '${key}' 失败 → 走卡住协议"
      return 1
    fi
  done

  local count
  count=$(echo "$keys" | wc -w | tr -d ' ')
  echo "✅ §9 Verification 全套通过（共 $count 项）"
}

# s2v_coverage_threshold_guard <task-spec>
#
# C4（黑盒/dogfood 复审项）：task 写明覆盖率阈值（如"≥ 75%"）但无人
# 强制 —— s2v_run 只看退出码，不焊阈值则实测 64% 也 rc0、绿灯不可信。
#
# 框架无关原则下**无法**可靠解析"实测百分比"（pytest-cov / jest / go tool cover /
# JaCoCo 输出各不同，与本文件框架无关设计冲突）。故采用**契约检查**而非输出解析：
#   task 若声明了数值阈值 → adapter §Commands.Coverage 命令必须自我强制该阈值
#   （含 fail-under 类 token，命令低于阈值时自身 exit 非零，由 s2v_run rc 兜住）；
#   否则视为配置错（声明了却没人强制）。未声明阈值 → 不干预（仅测量是合法选择）。
#
# 诚实边界：本门只保证"声明了阈值就必须把它焊进会失败的命令"，**不**校验实测百分比
# 本身（那需框架特定解析）。这是可机制化的最大正确切片，其余属框架特定、留给命令自身。
#
# 退出码：0=无需强制 / 已焊阈值；2=声明了阈值但命令不自我强制（配置错）
s2v_coverage_threshold_guard() {
  local task_spec="$1"
  [ -n "$task_spec" ] && [ -f "$task_spec" ] || return 0

  # 声明阈值：仅看提到 cover/覆盖 的行，取其中 N% 或 阈值/≥/>= 后的数字
  local thr
  thr="$(grep -hiE 'cover|覆盖' "$task_spec" 2>/dev/null \
        | grep -oE '([0-9]{1,3})[[:space:]]*%|(阈值|≥|>=|至少|fail[_-]?under)[[:space:]]*[0-9]{1,3}' \
        | grep -oE '[0-9]{1,3}' | sort -rn | head -1 || true)"
  [ -z "$thr" ] && return 0   # 未声明阈值 → 无需强制

  local cov
  cov="$(s2v_load_cmd Coverage)"
  case "$cov" in
    "<"*">"*) return 0 ;;   # 占位 → 交 s2v_run 占位门（非 C4 职责）
    ""|"N/A"|"N/A:"*)
      # C4 复审修正：声明了阈值 + Coverage 关掉 = spec 要门禁 / adapter 不门禁 = 矛盾
      echo "❌ C4: task 声明覆盖率阈值≈${thr}% 但 adapter §Commands.Coverage 为空/N/A —" >&2
      echo "   spec 要门禁、adapter 关了 = 矛盾。要么删除 task 的阈值声明，" >&2
      echo "   要么给一个会因低于阈值而失败的 Coverage 命令。" >&2
      return 2 ;;
  esac

  # 已确证强制：精确 fail-under 类 token（按"机制 token"**精确**匹配，非松散子串 ——
  # 二轮复审：旧 jacoco[^|]*check 把 `mvn jacoco:report && echo check` 误判强制；旧
  # WARN→return 0 把 `pytest --cov && echo ok` 静默放行），或作者**显式声明**
  # s2v-cov-enforced:<how>（自写脚本 / POM 绑定时由作者担保 + 留审计痕，确定性放行）。
  if echo "$cov" | grep -qiE '(--?cov-fail-under|--?fail[_-]under|fail_under=|coveragethreshold|--coverage-threshold|--min-coverage|--threshold=|jacoco:check|jacoco-check|jacocoTestCoverageVerification|go-test-coverage|s2v-cov-enforced)'; then
    echo "✅ Coverage 阈值已确证强制（声明≈${thr}%；fail-under 类 token 或显式 s2v-cov-enforced 标记）"
    return 0
  fi
  # 其余一律 STOP —— 无法从字面**确证**强制就不放行（二轮复审：WARN→return 0 等于
  # 没门）。自写脚本 / POM 绑定强制：在命令末尾加注释 `# s2v-cov-enforced: <如何>`
  # 由作者担保（杜绝"静默放行 vs 误杀自定义"两头漏）。
  echo "❌ C4: task 声明覆盖率阈值≈${thr}% 但 adapter §Commands.Coverage 未确证强制：" >&2
  echo "   命令：${cov}" >&2
  echo "   不焊阈值则实测 < ${thr}% 也 rc0、绿灯不可信。三选一：" >&2
  echo "   · 焊 fail-under 类（pytest --cov-fail-under=${thr} / jest coverageThreshold /" >&2
  echo "     jacoco:check / jacocoTestCoverageVerification / 自写脚本低于阈值时 exit 1）；" >&2
  echo "   · 自写脚本或 POM 绑定强制：命令末尾加注释 '# s2v-cov-enforced: <如何>' 显式担保；" >&2
  echo "   · 仅测量不设门槛：task 删除阈值声明（adapter Coverage 写 'N/A: 仅测量' 亦须 task 不声明阈值）。" >&2
  return 2
}

# s2v_require_green <task-spec>
#
# C8（黑盒/dogfood 复审项）：solo 把 Status 翻 Done 无任何独立复核 ——
# team 合并有 §4 Gate 2 复跑 §9，solo 这层是空的（"没有真实证据不许标完成"在
# solo 实际靠 agent 自觉）。canonical 路径（implement.md 步9 / agents-solo SOP
# 步4 的 `s2v_verify_full || exit`）已隐式绑定；真缺口是**外部/手动 agent 不照
# 脚本跑、直接手改顶部 Status=Done**。本守卫给 solo 的 Status→Done 翻转加等同
# team Gate 2 的硬复核：翻转前当场复跑一次 §9，绿才放行。
#
# 诚实边界：保证 Status=Done ⇒ 此刻 §9 真绿、非自证；**不**校验 §10 里手抄的
# §9 数字字面真伪（那需另一次解析，属固有 scope，见 full-standard §14）。
#
# 退出码：0=§9 全绿（放行 Status→Done）；1=§9 未全绿（不得标 Done）；2=spec 不存在
s2v_require_green() {
  local task_spec="$1"
  if [ -z "$task_spec" ] || [ ! -f "$task_spec" ]; then
    echo "❌ s2v_require_green: task spec 不存在: ${task_spec:-<空>}" >&2
    return 2
  fi
  local keys raw
  raw="$(s2v_extract_verify_keys "$task_spec")"
  # C8 复审修正：复跑排除 manual —— manual 已在 §9 阶段人工确认；s2v_run manual
  # 读 /dev/tty，复跑会重复提示，非交互环境直接 rc2（被误判 §9 红 → 误拦 Done）。
  keys="$(printf '%s\n' $raw | grep -vx 'manual' | tr '\n' ' ')"
  keys="${keys%% }"; keys="${keys# }"
  if [ -z "$keys" ]; then
    if [ -n "$raw" ]; then
      # §9 仅 manual、无 unit-test —— 违反 unit-test 强制契约（二轮复审：旧 return 0
      # 绕过该契约，外部 agent 跳过 step4 时可无任何机械复核标 Done）。manual 不能
      # 机械复核，故此处不放行、也不复跑 manual（会卡 /dev/tty）→ 显式配置错。
      echo "🛑 STOP: §9 仅 manual、无 unit-test —— 违反 unit-test 强制契约，不得据此标 Done" >&2
      echo "   （§9 必须含 unit-test；manual 不能机械复核 Done）。修 task §9 后再来。" >&2
      return 2
    fi
    # 原本就没 §9 key（坏 §9）→ 交 s2v_verify_full 报配置错（保留原安全行为）
    s2v_verify_full "$raw"; return $?
  fi
  echo "⟳ C8：Status→Done 前复跑 §9（排除 manual；Done 不可自证，等同 team Gate 2）..."
  if ! s2v_verify_full "$keys"; then
    echo "🛑 STOP: §9 未全绿 —— 不得把 Status 改 Done（先解决再重跑；不要手改 Status 绕过）" >&2
    return 1
  fi
  return 0
}

# s2v_backfill_notes <task-spec>
#
# C7（黑盒/dogfood 复审项）：§10 Completion Notes 回填全手工、跨 N task
# 高重复、零工具 —— 高错误密度（team Gate 4 按字段名 grep，错一字段/漏一 §9 key
# 即 BLOCK）。本 helper 渲染 §10 骨架，自动填两处最易错的机械部分：
#   ① 完成日期（正确 YYYY-MM-DD）
#   ② §9 Verification 结果 —— 逐 key 来自 s2v_extract_verify_keys，与本 task §9
#      **1:1 一致**（杜绝 Gate 4 第 1.5 道"§10 漏/多 §9 key"BLOCK）
# 其余业务字段保留 canonical 占位 token，沿用既有占位门强制 agent 填实。
#
# 单一源：字段 schema 唯一权威是 full-standard.md §8.3（顶层 6 项）。本渲染器是
# **登记在册的同步点**（scripts/README 函数表 + self-test C7-3 漂移守卫断言 helper
# 字段全部在 §8.3 权威字面 → schema 漂移会被自测红，不会静默漂移）。
#
# stdout 输出（agent 用 Edit 粘进 task spec §10 再填实剩余占位）。退出码 0；2=spec 不存在
s2v_backfill_notes() {
  local task_spec="$1"
  if [ -z "$task_spec" ] || [ ! -f "$task_spec" ]; then
    echo "❌ s2v_backfill_notes: task spec 不存在: ${task_spec:-<空>}" >&2
    return 2
  fi
  local today keys k
  today="$(date +%F)"
  keys="$(s2v_extract_verify_keys "$task_spec")"
  echo "## 10. Completion Notes"
  echo ""
  echo "- **完成日期**：${today}"
  echo "- **改动文件**："
  echo "  - <source-file-1>（新增/修改）"
  echo "- **commit 列表**：（git log --oneline 取本 task RED/GREEN/REFACTOR/§10 hash）"
  echo "  - <hash1> test: 加 RED 测试"
  echo "  - <hash2> feat: 实现"
  echo "- **§9 Verification 结果**：（下列 key 已按本 task §9 1:1 生成，逐条填实际结果）"
  for k in $keys; do
    case "$k" in
      unit-test) echo "  - unit-test: <N> passed / 0 failed" ;;
      coverage)  echo "  - coverage: <NN.N>% / 阈值 <NN>%" ;;
      manual)    echo "  - manual: ✅ <证据/确认者> / N/A: <原因>" ;;
      *)         echo "  - ${k}: ✅ / skipped: <原因> / N/A" ;;
    esac
  done
  echo "- **剩余风险 / 未做项**：<RISK_OR_NONE>"
  echo "- **下游 task 影响**：<DOWNSTREAM_OR_NONE>"
  echo ""
  echo "> ↑ 字段 schema 唯一权威 full-standard.md §8.3；占位 <...> 必须替换为真实值再 commit"
  echo "> （team Gate 4 / CI 按字段名 grep + 拒 <...> 占位 + §10↔§9 key 1:1 校验）"
  return 0
}
