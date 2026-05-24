//! Rust data-plane proto codegen (AC4: tonic/prost, no FFI).
//!
//! Uses the vendored `protoc` binary so the build is hermetic and needs no
//! system protoc. The frozen proto SSOT lives in ../proto/contextforge/v1.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", &protoc);

    let protos_v1 = [
        "../proto/contextforge/v1/context.proto",
        "../proto/contextforge/v1/search.proto",
        "../proto/contextforge/v1/service.proto",
        "../proto/contextforge/v1/import.proto",
        "../proto/contextforge/v1/eval.proto",
    ];

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_protos(&protos_v1, &["../proto"])?;

    // task-11.1 (ADR-016 §D2): Console data plane gRPC services
    // 4 service × 14 RPC, snake_case 与 Go contractv1 JSON tag 1:1.
    // task-11.2: proto file moved to ../proto/contextforge/console_data_plane/v1/
    // to align with buf-driven Go binding generation (option go_package).
    let protos_console = ["../proto/contextforge/console_data_plane/v1/console_data_plane.proto"];

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_protos(&protos_console, &["../proto"])?;

    for p in protos_v1.iter().chain(protos_console.iter()) {
        println!("cargo:rerun-if-changed={p}");
    }
    Ok(())
}
