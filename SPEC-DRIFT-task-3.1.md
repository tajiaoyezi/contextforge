# SPEC-DRIFT — task-3.1 importer-core

## 裁决：选项 A（主 agent，2026-05-17，用户授权）—— 已落地 task-3.1 §3；task-2.1/2.4 承接 redaction 责任

## 漂移项

importer 产出 ContextRecord 的 redaction 责任边界未在 task-3.1 spec 中澄清。

### 具体表现

1. `internal/importer/record.go` `buildRecord` 硬编码 `RedactionStatus: "none"`（PR 评审 Major 发现）。
2. `internal/importer/` 包内无任何 denylist/secret scan/redaction 逻辑。
3. task-3.1 §3 Scope / §5 Behavior Contract 未声明 importer 输出的 Content 是「原始明文」还是「已脱敏」。
4. ADR-004（local-first-privacy-baseline）要求 "secret 检测 + redaction 后入索引"，但 task-3.1 与 task-2.1/2.4 的衔接点未说明 redaction 发生在哪一阶段。

### 影响

- 若 indexer（task-2.4）假设 importer 已脱敏 → 可能跳过 redaction → secret 入索引（违反 ADR-004）。
- 若 indexer 负责脱敏 → importer 硬编码 `"none"` 应改为 `"pending"` 或 `"unknown"`，避免误导下游。
- task-3.2/3.3/3.4 的具体 importer 需知道是否要在映射阶段做 redaction。

### 建议决策（供主 agent/用户选）

**选项 A — importer 输出原始明文，redaction 由下游 scanner/indexer 负责（推荐）**
- task-3.1 §3 Out-of-Scope 增加：「importer 不做 secret redaction，输出原始内容」
- task-3.1 §5.3 buildRecord 将 RedactionStatus 从 `"none"` 改为 `"pending"`（表明待下游处理）
- task-2.1/2.4 spec 明确承接 redaction 责任
- 与 PRD §Constraints 安全基线一致：denylist + secret scan 在 scanner 阶段生效

**选项 B — importer 内嵌轻量 redaction**
- task-3.1 §3 In-Scope 增加 redaction 子项
- buildRecord 接入 config.DefaultDenylist() 与 secret pattern 扫描
- 增加 AC：importer 输出 RedactionStatus="partial" 时 Content 已脱敏
- 工作量增加，可能拖慢 3.1/3.2/3.3/3.4 节奏

### 关联

- ADR-004 local-first-privacy-baseline
- PRD §Constraints 安全（denylist + secret redaction）
- task-2.1 scanner / task-2.4 indexer / task-5.3 audit

### 请求

请主 agent/用户选定选项 A 或 B，或给出其他边界定义；task agent 按选定方案在 task-3.1 或下游 task 中落地。
