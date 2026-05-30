# Phase 18 spike — `sqlite-vec`

> Outcome: **BUILD-BLOCKED on this platform (x86_64-pc-windows-msvc)** — deferred to a Linux
> runner per phase-18 §7 R1 (P0 = Linux x86_64). No 5-dimension data collected on Windows.

## Attempt

- crate: `sqlite-vec = "0.1.10-alpha.4"` (FFI bindings to the sqlite-vec SQLite extension), wired
  behind `core/Cargo.toml` `[features] vector-sqlite = ["dep:sqlite-vec"]`.
- command: `cargo build -p contextforge-core --features vector-sqlite`
- host: `x86_64-pc-windows-msvc`, rustc 1.95.0, MSVC 14.44.35207 (BuildTools 2022).

## Result

The crate's `build.rs` invokes `cc-rs` → MSVC `cl.exe` to compile the bundled `sqlite-vec.c`
amalgamation, which **fails with exit code 2**:

```
error: failed to run custom build command for `sqlite-vec v0.1.10-alpha.4`
  error occurred in cc-rs: command did not execute successfully (status code exit code: 2):
  "...\MSVC\14.44.35207\bin\HostX64\x64\cl.exe" ... "-DSQLITE_CORE" ... "-c" "sqlite-vec.c"
```

`-vv` shows `sqlite-vec.c` emitting many MSVC conversion diagnostics (C4005 / C4244 / C4267 /
C4305 — `double`→`f32`, `size_t`→`int`, `__int64`→`int`, etc.) followed by a fatal compile error.
The alpha crate's C is primarily exercised on gcc/clang; it is not MSVC-clean.

## Decision

- `[SPEC-DEFER:phase-future.sqlite-vec-cross-platform]` — do not pursue sqlite-vec on Windows MSVC.
- The `vector-sqlite` feature was reverted to an empty placeholder so the default build stays clean
  (0 new dep); no `SqliteVecBackend` was committed.
- **Recommendation**: run the sqlite-vec spike on a Linux x86_64 host (gcc/clang), where the harness
  (`scripts/spike_vector_backends.sh`, extend `BACKENDS`) can collect the 5-dimension evidence.

## 5-dimension measurement

| dimension | value |
|---|---|
| backend | `sqlite-vec` |
| recall@5 / recall@10 | n/a — build blocked |
| P95 latency / RSS / cold-start / reindex | n/a — build blocked |
