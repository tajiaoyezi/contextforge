//! Frozen ContextForge proto / canonical-record contract (task-1.1),
//! Rust data-plane side. Ties the frozen proto SSOT
//! (`proto/contextforge/v1/*.proto`) to the tonic/prost generated bindings.

use std::path::PathBuf;

use crate::pb;

/// FROZEN canonical-record schema version. Per the contract freeze rule
/// (PRD §Technical Risks R1 / proto CONTRACT FREEZE RULE) v0.1 may only add
/// fields with new tags — never delete or renumber an existing tag.
pub const SCHEMA_VERSION: &str = "0.1";

/// Frozen canonical-record schema version ("0.1").
pub fn schema_version() -> &'static str {
    SCHEMA_VERSION
}

/// Locate the frozen proto SSOT directory `proto/contextforge/v1` by walking
/// up from the crate manifest dir (cargo sets CARGO_MANIFEST_DIR = core/).
fn proto_dir() -> Option<PathBuf> {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let p = d.join("proto").join("contextforge").join("v1");
        if p.is_dir() {
            return Some(p);
        }
        if !d.pop() {
            return None;
        }
    }
}

/// task-27.1: locate the console data-plane proto SSOT
/// `proto/contextforge/console_data_plane/v1` (separate module from the frozen
/// core `contextforge/v1`). Used by the MemoryService / MemoryItem freeze guard.
fn console_proto_dir() -> Option<PathBuf> {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let p = d
            .join("proto")
            .join("contextforge")
            .join("console_data_plane")
            .join("v1");
        if p.is_dir() {
            return Some(p);
        }
        if !d.pop() {
            return None;
        }
    }
}

fn console_proto_text() -> String {
    let Some(dir) = console_proto_dir() else {
        return String::new();
    };
    let mut out = String::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return String::new();
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "proto") {
            if let Ok(s) = std::fs::read_to_string(&p) {
                out.push_str(&s);
                out.push('\n');
            }
        }
    }
    out
}

/// task-27.1 (ADR-032 D1): proto field names on a console data-plane message
/// (e.g. `MemoryItem`), read from the console_data_plane SSOT. Mirrors
/// `message_fields` but over the console module proto.
pub fn console_message_fields(msg: &str) -> Vec<String> {
    let txt = console_proto_text();
    let Some(body) = message_block(&txt, msg) else {
        return Vec::new();
    };
    // Strip `//` line comments first so trailing comments (and any `;` inside a
    // comment) cannot corrupt the `;`-split field parse.
    let stripped: String = body
        .lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut fields = Vec::new();
    for raw in stripped.split(';') {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let Some((decl, _tag)) = line.split_once('=') else {
            continue;
        };
        if let Some(name) = decl.trim().split_whitespace().last() {
            if name.chars().all(|c| c.is_alphanumeric() || c == '_') && !name.is_empty() {
                fields.push(name.to_string());
            }
        }
    }
    fields.sort();
    fields
}

/// Extract the `{ ... }` body of `service <svc> { ... }` (brace-counted).
fn service_block(txt: &str, svc: &str) -> Option<String> {
    let needle = format!("service {svc}");
    let start = txt.find(&needle)?;
    let open = start + txt[start..].find('{')?;
    let mut depth = 0i32;
    for (i, ch) in txt[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(txt[open + 1..open + i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// task-27.1: RPC method names declared on a console data-plane `service`
/// (e.g. `MemoryService`). Used by the proto-freeze superset guard.
pub fn console_service_methods(svc: &str) -> Vec<String> {
    let txt = console_proto_text();
    let Some(body) = service_block(&txt, svc) else {
        return Vec::new();
    };
    let mut methods = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("rpc ") {
            if let Some(name) = rest.split('(').next() {
                let name = name.trim();
                if !name.is_empty() {
                    methods.push(name.to_string());
                }
            }
        }
    }
    methods.sort();
    methods
}

fn proto_text() -> String {
    let Some(dir) = proto_dir() else {
        return String::new();
    };
    let mut out = String::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return String::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "proto"))
        .collect();
    files.sort();
    for f in files {
        if let Ok(s) = std::fs::read_to_string(&f) {
            out.push_str(&s);
            out.push('\n');
        }
    }
    out
}

/// Extract the `{ ... }` body of `message <msg> { ... }` from flat
/// (non-nested) proto messages via a brace counter.
fn message_block(txt: &str, msg: &str) -> Option<String> {
    let needle = format!("message {msg}");
    let mut search_from = 0;
    while let Some(rel) = txt[search_from..].find(&needle) {
        let start = search_from + rel;
        // Ensure a word boundary after the message name (avoid `ContextRecordX`).
        let after = txt[start + needle.len()..].chars().next();
        if matches!(after, Some(c) if c.is_alphanumeric() || c == '_') {
            search_from = start + needle.len();
            continue;
        }
        if let Some(brace_rel) = txt[start..].find('{') {
            let open = start + brace_rel;
            let mut depth = 0i32;
            for (i, ch) in txt[open..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(txt[open + 1..open + i].to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
        return None;
    }
    None
}

/// Proto field names declared on the given message, read from the frozen
/// proto SSOT.
pub fn message_fields(msg: &str) -> Vec<String> {
    let txt = proto_text();
    let Some(body) = message_block(&txt, msg) else {
        return Vec::new();
    };
    let mut fields = Vec::new();
    for raw in body.split(';') {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        // Take the last token before '=' as the field name:
        //   [repeated] <type> <name> = <tag>
        let Some((decl, _tag)) = line.split_once('=') else {
            continue;
        };
        let decl = decl.trim();
        // Skip leading line comments embedded before a field.
        let decl = decl.rsplit("//").next().unwrap_or(decl).trim();
        if let Some(name) = decl.split_whitespace().last() {
            if name.chars().all(|c| c.is_alphanumeric() || c == '_') && !name.is_empty() {
                fields.push(name.to_string());
            }
        }
    }
    fields.sort();
    fields
}

/// Whether the proto SSOT documents both the schema_version and the
/// versioning freeze rule (only add fields, never delete or renumber tags).
pub fn freeze_rule_documented() -> bool {
    let low = proto_text().to_lowercase();
    low.contains("schema_version")
        && low.contains("frozen")
        && low.contains("only add")
        && low.contains("never delete")
        && low.contains("renumber")
}

/// Proves the Rust bindings were code-generated (tonic/prost, no FFI) by
/// constructing a generated message and binding a generated service type.
pub fn generated_rust_smoke() -> Result<(), String> {
    let rec = pb::ContextRecord {
        id: "ctx_smoke".to_string(),
        schema_version: SCHEMA_VERSION.to_string(),
        ..Default::default()
    };
    if rec.schema_version != SCHEMA_VERSION {
        return Err(format!("generated field mismatch: {:?}", rec.schema_version));
    }
    // tonic generated the gRPC client (no FFI / cgo): bind its type to prove
    // `protoc-gen` tonic codegen ran for `service ContextService`.
    type _Client = pb::context_service_client::ContextServiceClient<tonic::transport::Channel>;
    Ok(())
}
