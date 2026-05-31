# Spike: sqlite-vec Windows MSVC cross-platform build (task-23.2 / Phase 23)

> **结论（ADR-013 真实非合成，三态标注）**：🟢 **路径 (a) bundled C amalgamation 在 Windows MSVC 真实构建 + 运行通过** —
> `cargo build --features vector-sqlite` exit 0 + 契约测试 2/2 PASS（`x86_64-pc-windows-msvc`, rustc 1.95.0, 2026-05-31）。
> 这**解除**了 Phase 18 task-18.3 记录的「Windows MSVC build blocked」stop-condition（`docs/spikes/phase-18-sqlite-vec.md`），
> 缩小了 ADR-023 D1 记录的 dev/prod backend parity 缺口。**未做任何源码 / Cargo.toml 改动**——既有 `sqlite-vec = "=0.1.9"`
> 配置在当前工具链下即可经 MSVC 构建；本 task 补契约测试守护 + 如实记录真实凭据。

## 1. 调查背景

Phase 18 task-18.3 用 `sqlite-vec` 0.1.9 `vec0` 虚表实现 `SqliteVecBackend`（经 `rusqlite::ffi::sqlite3_auto_extension` 注册 `sqlite3_vec_init`）。当时实测：**Linux x86_64 gcc 可构建并跑真实数据**（recall@5/10=1.0，`docs/spikes/phase-18-sqlite-vec.md`），但 **Windows MSVC 构建受阻**——`sqlite-vec` 的 C amalgamation 在 MSVC `cl.exe` 下受阻，凭据保留为 `[SPEC-DEFER:phase-future.sqlite-vec-cross-platform]`（ADR-023 D1 Consequences：「the recommended default (sqlite-vec) does not build on the Windows dev box」）。

task-23.2 真实调查三路径（bundled amalgamation / 预编译扩展 / 替代绑定）在 Windows MSVC 的可构建性。

## 2. 调查方法 + 真实凭据

### 路径 (a) — bundled C amalgamation MSVC 编译（🟢 通过）

直接在 Windows MSVC 工具链上对既有 `vector-sqlite` feature 跑真实构建 + 测试：

```
$ rustc -vV
host: x86_64-pc-windows-msvc
release: 1.95.0

$ cargo build --features vector-sqlite -p contextforge-core
   Compiling sqlite-vec v0.1.9
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.29s
   (exit 0)

$ cargo test --features vector-sqlite -p contextforge-core retriever::vector::sqlite_vec
test retriever::vector::sqlite_vec::tests::test_23_2_3_open_index_search_contract ... ok
test retriever::vector::sqlite_vec::tests::test_23_2_3_dim_mismatch_error ... ok
test result: ok. 2 passed; 0 failed
   (exit 0)
```

`sqlite-vec` 0.1.9 的 C amalgamation 经 `cc` crate 在 MSVC `cl.exe` 下**成功编译**；`vec0` 虚表注册、`CREATE VIRTUAL TABLE ... USING vec0`、`embedding MATCH ? ORDER BY distance` KNN、dim mismatch 错误路径在 Windows MSVC **运行时正确**（契约测试 2/2 PASS）。**真实构建 + 运行均通过**，非仅编译。

成因推断（相对 Phase 18 的改善）：Phase 18 的 MSVC 受阻凭据为当时工具链快照；本次在 rustc 1.95.0 + 当前 MSVC / Windows SDK + 当前 `cc` crate 版本下，`sqlite-vec` 0.1.9 的 amalgamation 已可经 MSVC 编译（C 标准 / 内建函数兼容性在工具链演进后不再受阻）。**未改 pin（仍 `=0.1.9`）**——0.1.10-alpha.4 仍缺 `sqlite-vec-diskann.c`（task-18.3 既有记录），故维持 0.1.9 pin。

### 路径 (b) — 预编译扩展运行时加载（未采用，路径 a 已通过）

路径 (a) 通过 → 无需评估预编译 `vec0` 扩展 + 运行时 `load_extension`（该路径触及扩展加载安全基线 ADR-004 / §8 R2，仅在静态编译受阻时才作 fallback）。记录于此备查。

### 路径 (c) — 替代 Rust 绑定（未采用，路径 a 已通过）

路径 (a) 通过 → 无需换底层 crate（保持既有 `sqlite-vec` 绑定 + `VectorSearcher` 契约，0 新供应链表面，符合 ADR-008 最小变更）。

## 3. 结论（ADR-013 三态如实）

- 🟢 **真实通过**：路径 (a) bundled amalgamation 在 `x86_64-pc-windows-msvc`（rustc 1.95.0）`cargo build --features vector-sqlite` exit 0 + 契约测试 2/2 PASS（open→index→KNN + dim mismatch）。**真实非合成凭据**，非伪造跨平台通过。
- **落地动作 = 0 源码 / Cargo.toml 改动 + 补契约测试**：既有配置即可 MSVC 构建；新增 `core/src/retriever/vector/sqlite_vec.rs` 的 `#[cfg(test)] mod tests`（TEST-23.2.3）守护 backend 契约（Linux + Windows MSVC 均可跑）。
- 既有 Linux gcc `vector-sqlite` 路径**不退化**（无源码改动）。
- **诚实 caveat**：(1) 证据来自单台 Windows MSVC dev box（rustc 1.95.0，2026-05-31）；(2) CI 默认不构建 `vector-sqlite` feature（默认 0 新 dep，ADR-023 D5），故跨 CI 的 MSVC 构建非持续守护——本 spike 的真实 build+run 凭据为当前定论，未来工具链 / crate 版本变化需复核；(3) Phase 18 的受阻凭据为历史快照，不溯改（ADR-014 D5），本 spike add-only 记录「现已通过」。

## 4. 对 ADR / 下游影响

- **ADR-023 D1 dev/prod parity 缺口缩小**：sqlite-vec（生产嵌入式推荐默认）现可在 Windows MSVC dev box 构建 + 运行 → dev/prod backend parity 在本机不再 imperfect（历史 Consequences 段不溯改，本 spike + task-23.3 ADR-028 add-only 记录现状）。
- **task-23.3（closeout）**：引用本 spike 作 v0.16.0 evidence + ADR-028 vector-persistence-strategy 的 sqlite-vec 跨平台决策依据（sqlite-vec 现跨 Linux/Windows-MSVC 可用 + hnsw 持久化 task-23.1 作纯 Rust fallback）。
- **`[SPEC-DEFER:phase-future.sqlite-vec-cross-platform]`**：在本机 Windows MSVC 上**已解除**（真实构建+运行通过）；跨更多 MSVC 工具链版本 / CI 持续守护属后续（本 spike 未声称所有 MSVC 环境通过——仅本机真实凭据）。
