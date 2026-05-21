# language: en
# Maps to:
#   - docs/specs/tasks/task-2.3-chunker.md
#
# 轻量 BDD（s2v §9.2）；任务 agent 在 §2A 后回填 Given/When/Then 真实语义。

Feature: chunker
  In order to 把 parser 产出的解析单元切成可被 indexer 写入 SQLite/Tantivy 的检索切片
  As indexer 与 memoryops 模块
  I want 文档/代码 chunking + metadata 抽取 + provenance 维护 + content_hash 去重锚点

  # ---
  # Maps to: docs/specs/tasks/task-2.3-chunker.md
  Scenario: SCEN-2.3.1 — 对应 AC1（Chunk 字段完整）
    Given parser 输出 ≥1 个 ParsedUnit 的 Vec
    When  调用 chunk_units(units, file_path, policy, provenance)
    Then  产出的每个 Chunk 含 chunk_id / file_path / line_start / line_end / language / content / content_hash 七个字段全部非空（line_end ≥ line_start，content_hash 含 algo 前缀）

  Scenario: SCEN-2.3.2 — 对应 AC2（provenance 多来源）
    Given 调用方传入含两个不同 importer 的 Vec<Provenance>
    When  调用 chunk_units
    Then  每个 Chunk 的 provenance 列表保留全部来源条目（importer / original_path / imported_at / source_modified_at 全字段透传）

  Scenario: SCEN-2.3.3 — 对应 AC3（chunking 可配置）
    Given 同一组 ParsedUnit + 同 language（"go"）
    When  用 max_chunk_lines=50 与 max_chunk_lines=100 两套 ChunkPolicy 分别 chunk
    Then  前者产出的 chunk 数严格多于后者；且 code / markdown / log / text 四组配置互相独立（改 code 不影响 markdown）

  Scenario: SCEN-2.3.4 — 对应 AC4（大文件分块不爆内存）
    Given 一个 10000 行 ParsedUnit（合理大文件，scanner 已拦截超大文件）
    When  用 max_chunk_lines=200 调 chunk_units
    Then  切出 ≥40 个 Chunk，且每个 Chunk 的内容行数 ≤200、line_start/line_end 单调递增不重叠

  Scenario: SCEN-2.3.5 — 对应 AC5（content_hash 一致性）
    Given 同一段 normalized 内容来源于两个不同 file_path + provenance 组合
    When  分别经 chunk_units 产出 Chunk
    Then  两个 Chunk 的 content_hash 完全相同；不同内容 hash 必不同；CRLF/LF 与行末 trailing whitespace 差异不影响 hash
