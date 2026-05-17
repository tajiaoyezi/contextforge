> 📌 **快照来源**：本文件由 `/s2v-init` 在 2026-05-17 从全局 skill `/home/tajiaoyezi/.claude/skills/s2v` 复制。
>
> **请勿直接编辑此文件** — 升级 S2V 规范请改全局 skill 后重跑 `/s2v-init`（或手动 `cp` 覆盖）。

---

# scripts/ — S2V 共享 helper

这里是 S2V 在 task 实施 / verification / preflight 时**单一来源**的 bash 实现。`implement.md` / `templates/agents-team.md` / `templates/agents-solo.md` 都通过 `source` 引用，不再各自内联同名实现。

## 运行前置

- **bash 3.2+**：脚本只用 POSIX-ish 语法 + indexed array（无 `mapfile` / `readarray` / `declare -A` / `${var^^}` 等 bash-4-only 特性）→ macOS 默认 bash 3.2 无需升级即可跑
- **awk**：GNU awk 或 BSD awk 均可（不依赖 gawk 扩展）
- **不依赖** `jq` / `gh` / Node / Python —— self-test 用 `mktemp` fixture 自包含
- Windows 用 git-bash / WSL（`< /dev/tty` 在纯 cmd / PowerShell 不可用 — 仅影响 `s2v_run manual` 交互确认）

## 加载方式

S2V 走"项目自包含"原则：`/s2v-init` 时把 `scripts/` 复制到项目内 `docs/s2v/scripts/`（与 `standard.md` / `templates-used/` 同步走 `step 5.5` —— 规范快照前移以兜底 STEPWISE 中途取消；`/s2v-tier` 步 2.5 同步刷新），项目里的 `AGENTS.md` 和 `/s2v-implement` 都用项目相对路径加载：

```bash
source docs/s2v/scripts/lib/adapter.sh
source docs/s2v/scripts/lib/verify.sh
source docs/s2v/scripts/lib/preflight.sh
```

`/s2v-tier` 升降档时也会一并刷新（与 `standard.md` 一致），项目能拿到最新 helper。

## 文件清单

| 文件 | 提供的函数 | 何时用 |
|---|---|---|
| `lib/adapter.sh` | `s2v_load_cmd <字段名>` | 从 `docs/s2v-adapter.md` §Commands 取字段值 |
| `lib/verify.sh` | `s2v_run` / `s2v_extract_verify_keys` / `s2v_verify_full` / `s2v_baseline_green`（基线绿+冷启动豁免，排除式安全偏置·C3/C9'）/ `s2v_coverage_threshold_guard`（覆盖率阈值契约·C4）/ `s2v_require_green`（Status→Done 前复跑 §9·C8）/ `s2v_backfill_notes`（§10 骨架渲染·C7）| task §9 / 基线 / 完工把关 |
| `lib/preflight.sh` | `s2v_read_status` / `s2v_preflight_input` / `s2v_preflight_ready <task-spec>` / `s2v_preflight_phase <phase-spec>`（Phase 层兜底门禁·C1）/ `s2v_guard_fixture_tracked <fixture>`（fixture 防 .gitignore 静默脱 track·C10）/ `s2v_guard_areas_tracked <area...>`（SOURCE/UNIT area 防 .gitignore 静默遮蔽源码·C11）| task / phase spec 校验 + Gate |
| `lib/_self-test.sh` | — | 改 helper 后跑：`bash scripts/lib/_self-test.sh` |
| `install.sh` | — | 两模式：`<dir>` 装 skill 本体（拷贝 + 校验 `full-standard.md` + self-test + 提示 `S2V_SKILL_DIR`，§22.4 方案 A/B 自动化）；`--commands <agent>` 装该 agent 原生 `/s2v-*` 命令桩（源 `commands/<agent>/`，第 2 层）。**skill 仓库 / 分发用，非项目运行时 helper**；两模式均不内置 §22.2 路径表故非 §22.6 同步点 |

> ⚠️ **项目内副本说明**：`/s2v-init` 步 5.5 / `/s2v-tier` 步 2.5 把本 README 连同 `adapter.sh` / `verify.sh` / `preflight.sh` 复制进项目 `docs/s2v/scripts/`，但**不复制 `_self-test.sh`**（dev-only 工具）。下面"修改 helper 的检查清单""跑 self-test"两节**仅对 S2V skill 仓库本身有效**；项目协作者无需也无法在项目内副本跑 self-test。

## 退出码约定

| 函数 | 0 | 1 | 2 |
|---|---|---|---|
| `s2v_run` | 命令通过 / 合法跳过 | 命令本身失败 | 配置错误（占位 / required 缺失） |
| `s2v_verify_full` | 全套通过 | 某 key 失败（task 进卡住协议） | 配置错误（空列表 / 缺 unit-test） |
| `s2v_preflight_input` | 路径合法且文件存在 | — | 路径形态错 / 文件不存在 |
| `s2v_preflight_ready` | Ready / In Progress（合法动手） | Status=Draft 且 §6 AC/§7 追踪表预扫非空（进 §2A 交互审） | 硬性 STOP（Done/Blocked/Waived/未知值；§6 AC 空 / §7 无 SCEN/TEST；Ready/In Progress 但残留 `<TBD-by-user>`；**Draft 但 §6 或 §7 空也走此列**） |

## 通信约定

helper **只用退出码 + stdout + stderr 通信**；不用 `export` 把状态传给调用方。原因：调用方常会用 `$(...)` 捕获 stdout 跑在子 shell，`export` 不传到父 shell。

需要状态值（如 task 的 `Status` 字段），调用方自己用 `s2v_read_status <task-spec>` 取。

## 修改 helper 的检查清单

1. 改 `lib/*.sh` 后必须跑 self-test（命令见下方 "## 跑 self-test" 段 —— 脚本路径自解析、与调用方 cwd 无关；裸相对路径 `bash scripts/lib/_self-test.sh` 需在 skill 根目录执行），全绿才能提交
2. 新增 verification 字段（如加个 `Mutation Test`）时同步改 4 处：
   - `lib/verify.sh` `_S2V_VERIFY_KEYS_ORDER` + `_S2V_VERIFY_FIELD_PATTERNS`（漏改会让新字段被 §9 sanity-check loud-warn 误报为"拼错字段"）+ `_s2v_key_to_field` + `s2v_extract_verify_keys` 内的 awk 模式
   - `templates/adapter.md` §Commands
   - `full-standard.md` §8.3 §9 Task Spec 模板
   - `full-standard.md` §11.2 验证类型表
3. 改了字段名（如 `Unit Test` → `Unit Tests`）需要在 self-test 加一条向后兼容断言（旧字段名也能识别 / 或显式 fail 让用户改）

## 跑 self-test

```bash
# 切到 skill 根目录（路径取决于 agent 工具 — 详见 full-standard.md §22）
cd "${S2V_SKILL_DIR:-$HOME/.claude/skills/s2v}"
bash scripts/lib/_self-test.sh
```

期望：`132 pass / 0 fail`（或更多，self-test 应随 helper 一起增长）。
