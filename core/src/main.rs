//! task-1.3 (AC1): `contextforge-core` data-plane binary entrypoint.
//!
//! Resolves a safe local listen address (Unix socket or 127.0.0.1; never a
//! default `0.0.0.0`) then serves the tonic gRPC `ContextService`.

use contextforge_core::server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = server::resolve_listen_addr(std::env::args().nth(1).as_deref())?;
    server::serve(addr).await
}
