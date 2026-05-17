# Task `1.2`: `config — TOML 配置 + denylist/allowlist`

> ✅ 已过 `/s2v-implement` §2A 前置审核（2026-05-17）：§3/§4/§5.2/§5.3 `<TBD-by-user>` 已清零、§6 AC 经用户审定接受、Owner=tajiaoyezi、TOML 采 stdlib 手写（不引第三方依赖，规避 R7）。实时状态以下方 `**Status**` 字段为准；状态机见 `docs/s2v/standard.md` §10.5.1。

**Status**: In Progress

**Priority**: P0
**Owner**: tajiaoyezi
**Related Phase**: Phase 1 (foundation)
**Dependencies**: 1.1 (proto/canonical schema)

## 1. Background

ContextForge 默认本地优先、隐私基线（PRD §Constraints / §Decisions Log D4）。需要一份可被 CLI 读取的 TOML 配置 + 默认 denylist/allowlist，作为索引/导入的第一道安全防线（PRD §Technical Risks R4：denylist 路径优先）。

## 2. Goal

`~/.contextforge/config.toml` 默认配置可生成与读取；denylist 默认含 PRD §Constraints 列出的全部敏感路径；allowlist 路径导入模型可配置；config/token 文件权限受限（0600）。

## 3. Scope

### In Scope

- `internal/config/` Go 包：默认配置数据结构 + 生成默认 `~/.contextforge/config.toml` 及本地数据目录骨架（`collections/` `logs/` `runtime/`，PRD §Technical Approach 本地数据目录结构 v0.1）
- TOML 序列化/反序列化：写默认 config 后可被读回，往返字段一致（AC1）
- 默认 denylist：内置 PRD §Constraints 安全列出的全部敏感路径（AC2 枚举的 16 项），可被 CLI / 下游读取
- allowlist 路径导入模型 + `collection` / `agent_scope` 配置结构（AC3）
- config.toml 及 config 模块写入文件的权限 `0600`、数据目录 `0700`（AC4）
- 远程 provider 配置结构：默认 `enabled=false`，须显式 opt-in 字段才启用（AC5）
- 用户覆盖默认 denylist 需显式确认标志（AC3 后半）

### Out Of Scope

- 实际文件扫描 / denylist·allowlist 匹配执行（`scanner`，Phase 2）
- secret pattern 检测 / redaction（Phase 2）
- 随机 REST token 的生成 / 轮转、daemon 监听绑定（task 1.3 / Phase 6 task 6.2；本 task 仅定义 `runtime/contextforge.token` 路径与 0600 权限策略，不生成 token 值）
- `contextforge init` CLI 子命令编排与终端交互（task 1.4；本 task 只提供 config 库能力）
- 远程 provider 实际网络调用 / 凭证管理（P1，v0.1 不强依赖）
- viper / koanf 等完整配置框架多源合并（env / flag override）—— v0.1 仅 TOML 文件、stdlib 实现

## 4. Users / Actors

- `contextforge` CLI（Go 控制面）：`init` 生成默认配置；`import` / `index` / `search` 等子命令读取 denylist/allowlist/collection/provider 配置（下游 task 1.4 / Phase 2+ 消费）
- 索引 / 导入流程（`scanner` / `agent-importer`，下游 phase）：消费默认 denylist + allowlist 路径模型 + collection / agent_scope
- 本地优先 / 隐私敏感开发者（最终受益人）：受默认 denylist + 0600 权限 + 远程 provider 默认关闭保护（ADR-004 隐私基线）

## 5. Behavior Contract

### 5.1 Required Reading

- `docs/prds/context-forge.prd.md`（§Constraints 安全基线 + 本地数据目录结构 v0.1）
- `docs/specs/phases/phase-1-foundation.md`
- `docs/specs/tasks/task-1.1-proto.md`
- `docs/decisions/adr-004-local-first-privacy-baseline.md`
- `test/features/config.feature`

### 5.2 Imports

- Go 标准库：`os` / `path/filepath` / `strings` / `strconv` / `fmt` / `errors`（手写最小 TOML 编解码 + 文件/目录权限）
- 本 task **不引入新第三方依赖**：v0.1 config schema 简单（string / []string / bool），stdlib 实现规避 R7（go.mod 不改 — 经 §2A 用户决策）
- 契约上游：task-1.1 冻结的 `proto/contextforge/v1`（仅在 collection / agent_scope 概念对齐层面参考，不直接 import 生成包；config 是纯 Go 控制面配置，无 proto 运行期依赖）
- 测试侧：`testing` / `os` / `path/filepath`（temp dir 隔离 + 权限断言）

### 5.3 函数签名

> Go 包 `config`，落 `internal/config/`（adapter §Source areas `internal/`）。

```go
package config

const (
    SchemaVersion = "0.1"
    FileMode      = 0o600 // AC4：config.toml / token 文件
    DirMode       = 0o700 // AC4：数据目录
)

type Config struct {
    SchemaVersion         string             // "0.1"
    DataDir               string             // ~/.contextforge
    Denylist              []string           // 默认含 AC2 全部敏感路径
    AllowDenylistOverride bool               // AC3：用户显式覆盖默认 denylist 的确认位（默认 false）
    Collections           []CollectionConfig // AC3：allowlist 路径导入模型
    Remote                RemoteProviderConfig
}

type CollectionConfig struct {
    ID         string
    Allowlist  []string // 允许导入的路径前缀
    AgentScope []string
}

type RemoteProviderConfig struct {
    Enabled  bool   // AC5：默认 false，显式 opt-in 才 true
    Provider string // 默认 ""
    Endpoint string // 默认 ""
}

func DefaultRootDir() (string, error)  // ~/.contextforge（基于 os.UserHomeDir）
func DefaultDenylist() []string        // AC2：返回 16 项敏感路径默认 denylist
func DefaultConfig() Config            // AC1/AC2/AC5：含默认 denylist、Remote.Enabled=false
func Init(root string) (Config, error) // AC1/AC4：生成 config.toml + collections/ logs/ runtime/ 骨架（文件 0600 / 目录 0700）；已存在则不覆盖直接 Load
func Load(root string) (Config, error) // AC1：读取 root/config.toml（与 Save 往返一致）
func Save(root string, c Config) error // AC4：写 root/config.toml，权限 0600
func (c Config) RemoteEnabled() bool   // AC5：远程 provider 是否已显式启用
```

- SCEN/TEST-1.2.1 → `Init` 生成 config.toml + `collections/`·`logs/`·`runtime/` 目录骨架（AC1）
- SCEN/TEST-1.2.2 → `DefaultDenylist()` 含 AC2 枚举的全部 16 项（AC2）
- SCEN/TEST-1.2.3 → allowlist/collection 模型 + `AllowDenylistOverride` 默认 false、覆盖需显式置 true（AC3）
- SCEN/TEST-1.2.4 → `Init`/`Save` 落盘后 config.toml 权限 == 0600、数据目录 == 0700（AC4）
- SCEN/TEST-1.2.5 → `DefaultConfig().Remote.Enabled == false`，opt-in 后 `RemoteEnabled()==true`（AC5）

## 6. Acceptance Criteria

<!-- 渲染规则（**模式 A：完整给值 + PRD 引用标注**）：
     - init/add 基于 PRD 推导出 AC 内容，**完整写出**（不挂 <TBD-by-user> 前缀）
     - 每条 AC 加引用：`- [ ] **AC<N>** (PRD §<reference>): <内容>`
       - PRD 已写明 → 引用精确章节；PRD 没写、由 task 推导 → 标 `(本 task 新增)`
     - 用户 review 阶段：发现偏差直接改 AC 内容；review 通过**无需删除本注释**
     - **严禁** `- [ ] <TBD-by-user> AC<N>: 内容` 混合写法
-->

- [ ] **AC1** (PRD §Technical Approach 本地数据目录结构 v0.1): `contextforge` 能生成默认 `~/.contextforge/config.toml` 与目录骨架（collections/ logs/ runtime/）。
- [ ] **AC2** (PRD §Constraints 安全): 默认 denylist 包含 `.env` / `.env.*` / `*.pem` / `*.key` / `*.p12` / `*.pfx` / `id_rsa` / `id_ed25519` / `.ssh/` / `.git/objects/` / `node_modules/` / `target/` / `dist/` / `build/` / `.cache/` / `vendor/`，且可被 CLI 读取。
- [ ] **AC3** (PRD §Constraints 安全): collection 采用 allowlist 路径导入模型；用户覆盖 denylist 需显式确认。
- [ ] **AC4** (PRD §Constraints Local service security baseline): `config.toml` 与 token 文件权限为 `0600`（当前用户可读写）。
- [ ] **AC5** (PRD §Decisions Log D4 / 本 task 新增): 远程 provider 配置默认关闭，须显式 opt-in 字段才启用。

## 7. SDD / BDD / TDD Traceability

| Acceptance Criterion | BDD Scenario | TDD Test | Integration / E2E Test | Verification | Status |
|---|---|---|---|---|---|
| AC1 默认配置/目录生成 | SCEN-1.2.1 | TEST-1.2.1 | - | unit-test | Not Started |
| AC2 默认 denylist 完整 | SCEN-1.2.2 | TEST-1.2.2 | - | unit-test | Not Started |
| AC3 allowlist 导入模型 | SCEN-1.2.3 | TEST-1.2.3 | - | unit-test | Not Started |
| AC4 文件权限 0600 | SCEN-1.2.4 | TEST-1.2.4 | - | unit-test | Not Started |
| AC5 远程 provider 默认关 | SCEN-1.2.5 | TEST-1.2.5 | - | unit-test | Not Started |

## 8. Risks

- 关联 PRD §Technical Risks **R4**（secret 漏检）：denylist 是第一道防线，本 task 必须保证默认 denylist 完整且不可被静默绕过。
- 关联 PRD §Open Questions **O7**（v0.1 威胁模型边界）/ **O10**（本地 API/MCP 安全边界）。

## 9. Verification Plan

- **Install**: go mod download && cargo fetch
- **Typecheck**: go vet ./... && cargo check --workspace
- **Unit**: go test ./... && cargo test --workspace  <!-- 强制 -->

> 仅列 Install/Typecheck/Unit：adapter 其余 §Commands 字段为占位，按 init.md 步 8 §9 规则省略。

## 10. Completion Notes

- **完成日期**：`<TBD-after-impl>`
- **改动文件**：`<TBD-after-impl>`
- **commit 列表**：`<TBD-after-impl>`
- **§9 Verification 结果**：
  - install: `<TBD-after-impl>`
  - typecheck: `<TBD-after-impl>`
  - unit-test: `<TBD-after-impl>`
- **剩余风险 / 未做项**：`<TBD-after-impl>`
- **下游 task 影响**：`<TBD-after-impl>`
