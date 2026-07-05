//! task-10.2 §6 AC3 integration smoke — workspace CRUD + collection dir 1:1
//! mapping under a real temp data_dir.
//!
//! Validates the end-to-end SqliteWorkspaceStore::open → create → list → get →
//! update_config → soft_delete flow + verifies the physical
//! `<data_dir>/collections/<workspace_id>/` directory is created (ADR-015 §D2
//! workspace_id ↔ collection_id 1:1 mapping).

use contextforge_core::workspace::{SqliteWorkspaceStore, WorkspaceCreate, WorkspaceStore};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!("cfg-ws-smoke-{}-{}-{}", label, std::process::id(), nanos))
}

#[test]
fn workspace_smoke_create_to_delete() {
    let data_dir = unique_dir("e2e");
    let _ = fs::remove_dir_all(&data_dir);
    let store = SqliteWorkspaceStore::open(&data_dir).expect("open store");

    let root_path = unique_dir("root").to_string_lossy().into_owned();
    let req = WorkspaceCreate { owner_id: None,
        workspace_id: "smoke-ws-1".to_string(),
        name: "smoke ws 1".to_string(),
        root_path: root_path.clone(),
        allowlist: vec!["*.md".to_string(), "*.go".to_string()],
        denylist: vec![".env".to_string(), ".ssh/".to_string()],
    };

    // 1. create
    let created = store.create(&req).expect("create");
    assert_eq!(created.workspace_id, "smoke-ws-1");
    assert_eq!(created.status, "ready");
    assert_eq!(created.allowlist.len(), 2);
    assert!(created.created_at_unix > 0);
    assert!(created.updated_at_unix > 0);

    // 2. AC3 — workspace_id ↔ collection_id 1:1 physical dir
    let collection_dir = data_dir.join("collections").join("smoke-ws-1");
    assert!(collection_dir.exists(), "collection dir must be physically created at {collection_dir:?}");
    assert!(collection_dir.is_dir());

    // 3. list
    let listed = store.list().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].workspace_id, "smoke-ws-1");

    // 4. get
    let got = store.get("smoke-ws-1").expect("get").expect("present");
    assert_eq!(got.root_path, root_path);

    // 5. update_config (changes allowlist / denylist)
    let updated = store
        .update_config(
            "smoke-ws-1",
            vec!["*.txt".to_string()],
            vec![".env".to_string()],
        )
        .expect("update");
    assert_eq!(updated.allowlist, vec!["*.txt"]);
    assert!(updated.updated_at_unix >= created.updated_at_unix);

    // 6. soft_delete preserves row but excludes from default list
    store.soft_delete("smoke-ws-1").expect("soft delete");
    let post_get = store.get("smoke-ws-1").expect("get post-delete").expect("present");
    assert_eq!(post_get.status, "deleted");
    let post_list = store.list().expect("list post-delete");
    assert!(post_list.is_empty(), "soft-deleted excluded from default list");

    // 7. collection dir survives soft_delete (per ADR-015 §D2 — physical
    //    cleanup deferred to v0.4)
    assert!(collection_dir.exists(), "soft-delete must NOT remove physical dir");

    // cleanup
    let _ = fs::remove_dir_all(&data_dir);
}
