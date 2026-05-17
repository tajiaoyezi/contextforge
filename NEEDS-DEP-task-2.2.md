# NEEDS-DEP-task-2.2 — parser crates (R7)

**Task**: task-2.2-parser (docs/specs/tasks/task-2.2-parser.md)

## Required dependencies (for core/src/parser/)

- `tree_sitter = "0.22"` (core + dynamic loading or bundled)
- Language grammars (at minimum for P0):
  - `tree-sitter-go = "0.20"`
  - `tree-sitter-rust = "0.20"`
  - `tree-sitter-python = "0.20"`
  - `tree-sitter-typescript = "0.20"`
  - `tree-sitter-javascript = "0.20"`
- `pulldown-cmark = "0.11"` (for Markdown heading/paragraph/code-block extraction with source positions)
- `thiserror = "1"` (for ParseError, if not already present in workspace)

## Purpose / AC coverage
- AC1: tree-sitter based code parsing for .go/.rs/.py/.ts/.tsx/.js/.jsx → ParsedUnit with accurate line ranges + kind (function, struct, etc.)
- AC2: pulldown-cmark for .md → heading levels, code fences, paragraphs with line_start/end
- AC3: log/JSONL line-based or simple record parsing
- AC4/AC5: language tag + fallback text path (no dep needed)
- Keeps alignment with ADR-008 and PRD D8 / R8 (language + position preservation for later tokenizer/boost)

## Why independent chore-dep PR (not fold-in)
- Avoids Cargo.lock churn / conflict with parallel Phase 2 task-2.1 (scanner may also declare crates).
- Follows R7 + dispatch instruction: task agent writes this NEEDS file; main agent creates `chore/dep-parser-crates` (or similar), adds exact versions, self-PRs to master, notifies rebase.
- After rebase, this task branch gets the deps + can upgrade the stub impl to real tree-sitter/pulldown in GREEN/REFACTOR without lockfile write in this PR.

## Suggested versions (to be locked by main agent)
See above. Exact versions should be chosen to be compatible with current Rust edition and other workspace crates (tantivy etc.).

## Alternative considered
- Feature-gate the parser behind `#[cfg(feature = "tree-sitter")]` — rejected for v0.1 (parser is P0 core, must be on by default for `contextforge index`).

**Owner note**: tajiaoyezi — please queue the dep PR before or immediately after this task's PR to unblock 2.2 GREEN and 2.3 (chunker depends on parser output shape).

---

This file should be removed or marked resolved after the dep PR merges and this task rebases.
