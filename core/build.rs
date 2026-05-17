//! Rust data-plane proto codegen (AC4: tonic/prost, no FFI).
//!
//! Uses the vendored `protoc` binary so the build is hermetic and needs no
//! system protoc. The frozen proto SSOT lives in ../proto/contextforge/v1.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", &protoc);

    let protos = [
        "../proto/contextforge/v1/context.proto",
        "../proto/contextforge/v1/search.proto",
        "../proto/contextforge/v1/service.proto",
        "../proto/contextforge/v1/import.proto",
        "../proto/contextforge/v1/eval.proto",
    ];

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_protos(&protos, &["../proto"])?;

    for p in protos {
        println!("cargo:rerun-if-changed={p}");
    }
    Ok(())
}
