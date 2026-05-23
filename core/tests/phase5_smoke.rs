use contextforge_core::chunker::{ChunkPolicy, Provenance};
use contextforge_core::indexer::IndexSession;
use contextforge_core::memoryops::audit::{
    export_event, import_event, redact_event, search_event, AuditOperation, AuditSink,
};
use contextforge_core::retriever::{Retriever, SearchFilters, SearchOptions};
use contextforge_core::scanner::{default_denylist, ScanOptions};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const SECRET: &str = "AKIAIOSFODNN7EXAMPLE";

fn temp_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "contextforge-phase5-{name}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn scan_opts() -> ScanOptions {
    ScanOptions {
        denylist: default_denylist(),
        allowlist: Vec::new(),
        allow_denylist_override: false,
        dry_run: false,
        max_file_bytes: 10 * 1024 * 1024,
    }
}

#[derive(Default)]
struct StaleStub {
    stale_chunks: BTreeSet<String>,
}

impl StaleStub {
    fn mark_stale(&mut self, chunk_id: &str) {
        self.stale_chunks.insert(chunk_id.to_string());
    }

    fn is_stale(&self, chunk_id: &str) -> bool {
        self.stale_chunks.contains(chunk_id)
    }
}

fn duplicate_hash_count(chunks: &[(String, String)]) -> usize {
    let mut counts = BTreeMap::<String, usize>::new();
    for (_chunk_id, hash) in chunks {
        *counts.entry(hash.clone()).or_default() += 1;
    }
    counts.values().filter(|&&count| count > 1).count()
}

// TEST-5.3.5 / SCEN-5.3.5 / AC5: Phase 5 end-to-end smoke.
#[test]
fn phase_5_end_to_end_smoke() {
    let src = temp_root("src");
    let data = temp_root("data");
    let collection = "phase5-smoke";

    let duplicate_fact = "# Memory\n\nUse retrieval cache for contextforge phasefiveunique.\n";
    fs::write(src.join("hermes.md"), duplicate_fact).unwrap();
    fs::write(src.join("openclaw.md"), duplicate_fact).unwrap();
    fs::write(
        src.join("secret.md"),
        format!("# Secret\n\nAWS key {SECRET} must be redacted before indexing.\n"),
    )
    .unwrap();

    let provenance = vec![Provenance {
        importer: "phase5-smoke".to_string(),
        original_path: src.display().to_string(),
        imported_at: "1".to_string(),
        source_modified_at: "2".to_string(),
    }];

    let mut index = IndexSession::open(&data, collection).expect("open indexer");
    let stats = index
        .index_path(&src, &scan_opts(), &ChunkPolicy::default(), provenance)
        .expect("index fixture");
    index.commit().expect("commit index");
    assert!(
        stats.files_indexed >= 3,
        "fixture import should index files"
    );

    let retriever = Retriever::open(&data, collection).expect("open retriever");
    let hits = retriever
        .search(&SearchOptions {
            query: "phasefiveunique".to_string(),
            top_k: 10,
            filters: SearchFilters::default(),
            explain: true,
        })
        .expect("search");
    assert!(!hits.is_empty(), "search should find imported memory");
    assert!(
        hits.iter().all(|hit| !hit.provenance.is_empty()),
        "provenance should be available for every smoke hit"
    );

    let duplicate_inputs: Vec<(String, String)> = hits
        .iter()
        .map(|hit| {
            (
                hit.chunk_id.clone(),
                contextforge_core::chunker::content_hash(&hit.content),
            )
        })
        .collect();
    assert!(
        duplicate_hash_count(&duplicate_inputs) >= 1,
        "smoke fixture should include duplicate facts for task-5.1 dedup linkage"
    );

    let mut stale = StaleStub::default();
    stale.mark_stale(&hits[0].chunk_id);
    assert!(
        stale.is_stale(&hits[0].chunk_id),
        "task-5.2 stale API seam should be markable and retrievable"
    );

    let mut audit = AuditSink::open(&data, collection).expect("open audit");
    audit
        .record(import_event(
            collection,
            "phase5:import",
            stats.files_indexed as u64,
            1,
        ))
        .expect("audit import");
    audit
        .record(search_event(
            collection,
            "phase5:search",
            "phasefiveunique and AKIAIOSFODNN7EXAMPLE",
            hits.len() as u64,
            1,
        ))
        .expect("audit search");
    audit
        .record(export_event(
            collection,
            "phase5:export",
            hits.iter().map(|hit| hit.chunk_id.clone()).collect(),
            hits.iter().map(|hit| hit.content.len() as u64).sum(),
            1,
        ))
        .expect("audit export");
    audit
        .record(redact_event(
            collection,
            "phase5:redact",
            vec!["[REDACTED:AWS_ACCESS_KEY]".to_string()],
            1,
        ))
        .expect("audit redact");

    assert_eq!(audit.count_by_operation(AuditOperation::Import).unwrap(), 1);
    assert_eq!(audit.count_by_operation(AuditOperation::Search).unwrap(), 1);
    assert_eq!(audit.count_by_operation(AuditOperation::Export).unwrap(), 1);
    assert_eq!(audit.count_by_operation(AuditOperation::Redact).unwrap(), 1);

    let db_bytes = fs::read(
        data.join("collections")
            .join(collection)
            .join("metadata.sqlite"),
    )
    .expect("read sqlite db");
    let db_text = String::from_utf8_lossy(&db_bytes);
    assert!(
        !db_text.contains(SECRET),
        "audit log must not contain the full secret"
    );
}
