//! task-1.3 (AC1) / task-6.1: `contextforge-core` data-plane binary entrypoint.
//!
//! Resolves a safe local listen address (Unix socket or 127.0.0.1; never a
//! default `0.0.0.0`) and a data root (cmd-arg / `$CONTEXTFORGE_DATA_DIR` /
//! `$HOME/.contextforge`), then serves the tonic gRPC `ContextService` with
//! a `CoreService::new(data_dir)` so `Search` can open a `Retriever`.
//!
//! Command-line form (backward-compatible with task-1.3):
//!   contextforge-core [listen_addr] [data_dir]
//! When `data_dir` is omitted (task-1.4 daemon spawns with only the 1st arg),
//! `server::resolve_data_dir(None)` falls back through env → `~/.contextforge`.

use contextforge_core::server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let addr = server::resolve_listen_addr(args.get(1).map(String::as_str))?;
    let data_dir = server::resolve_data_dir(args.get(2).map(String::as_str));
    let svc = server::CoreService::new(data_dir.clone());
    // task-11.1 §6 AC5 (ADR-016 §D2): register Phase 9 ContextService +
    // Phase 11 4 Console data plane services on one tonic Server.
    server::serve_full(addr, svc, &data_dir).await
}
