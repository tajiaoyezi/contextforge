//! task-2.1 scanner tests — TEST-2.1.1 ~ TEST-2.1.5 (SCEN-2.1.*).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use contextforge_core::scanner::{
    redact_content, scan_file, scan_path, RedactionStatus, ScanError, ScanOptions, SecretKind,
    SkipReason,
};

fn temp_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "contextforge-scanner-{name}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn rel(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}

// SCEN-2.1.1 / AC1: default denylist skips sensitive/build paths.
#[test]
fn test_2_1_1_default_denylist_skips_sensitive_paths() {
    // TEST-2.1.1
    let root = temp_root("denylist");
    write_file(&root.join("src/main.rs"), "fn main() {}\n");
    write_file(&root.join(".env"), "SECRET=should-not-be-read\n");
    write_file(&root.join(".ssh/id_rsa"), "private key\n");
    write_file(&root.join(".git/objects/aa/bb"), "git object\n");
    write_file(&root.join("node_modules/pkg/index.js"), "module\n");
    write_file(&root.join("target/debug/app"), "binary\n");

    let report = scan_path(&root, &ScanOptions::default()).expect("scan succeeds");
    let scanned: BTreeSet<String> = report.files.iter().map(|f| rel(&f.path, &root)).collect();
    assert_eq!(
        scanned,
        BTreeSet::from(["src/main.rs".to_string()]),
        "only non-denylisted source should be scanned"
    );
    for denied in [".env", ".ssh", ".git/objects", "node_modules", "target"] {
        assert!(
            report
                .skipped
                .iter()
                .any(|s| rel(&s.path, &root).starts_with(denied)
                    && matches!(s.reason, SkipReason::Denylisted(_))),
            "expected denylisted skip for {denied}"
        );
    }
}

// SCEN-2.1.2 / AC2: allowlist narrows scanning; denylist override requires confirmation.
#[test]
fn test_2_1_2_allowlist_and_denylist_override_confirmation() {
    // TEST-2.1.2
    let root = temp_root("allowlist");
    let allowed = root.join("allowed");
    write_file(&allowed.join("keep.md"), "keep\n");
    write_file(&root.join("outside/drop.md"), "drop\n");
    write_file(&root.join(".env"), "TOKEN=secret\n");

    let mut options = ScanOptions {
        allowlist: vec![allowed.clone()],
        ..ScanOptions::default()
    };
    let report = scan_path(&root, &options).expect("allowlist scan succeeds");
    assert_eq!(report.files.len(), 1);
    assert_eq!(rel(&report.files[0].path, &root), "allowed/keep.md");
    assert!(
        report
            .skipped
            .iter()
            .any(|s| rel(&s.path, &root) == "outside"
                && matches!(s.reason, SkipReason::NotAllowlisted)),
        "outside allowlist path should be skipped"
    );

    options.allowlist = vec![root.join(".env")];
    let err = scan_path(&root, &options).expect_err("override must require confirmation");
    assert!(
        matches!(err, ScanError::DenylistOverrideRequired),
        "denylist override must be explicit"
    );

    options.allow_denylist_override = true;
    let report = scan_path(&root, &options).expect("confirmed denylist override succeeds");
    assert_eq!(report.files.len(), 1);
    assert_eq!(rel(&report.files[0].path, &root), ".env");
}

// SCEN-2.1.3 / AC3: secret patterns redact with labels and do not modify source files.
#[test]
fn test_2_1_3_secret_redaction_keeps_source_file_unchanged() {
    // TEST-2.1.3
    let root = temp_root("secrets");
    let file = root.join("config.txt");
    let original = concat!(
        "api_key = \"cf_live_1234567890abcdef\"\n",
        "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.secret\n",
        "-----BEGIN PRIVATE KEY-----\nabc123\n-----END PRIVATE KEY-----\n",
        "aws_access_key_id = AKIAIOSFODNN7EXAMPLE\n",
        "aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY\n",
        "github = ghp_1234567890abcdefghijklmnopqrstuv\n",
        "password = \"correct-horse-battery-staple\"\n",
        "Cookie: sessionid=abcdef1234567890\n",
    );
    write_file(&file, original);

    let scanned = scan_file(&file, &ScanOptions::default()).expect("scan file succeeds");
    let redacted = scanned
        .redacted_content
        .as_ref()
        .expect("redacted content should be produced");
    for label in [
        "[REDACTED:API_KEY]",
        "[REDACTED:BEARER_TOKEN]",
        "[REDACTED:PRIVATE_KEY]",
        "[REDACTED:AWS_ACCESS_KEY]",
        "[REDACTED:AWS_SECRET_KEY]",
        "[REDACTED:GITHUB_TOKEN]",
        "[REDACTED:PASSWORD]",
        "[REDACTED:COOKIE]",
    ] {
        assert!(redacted.contains(label), "missing redaction label {label}");
    }
    let kinds: BTreeSet<SecretKind> = scanned.redactions.iter().map(|m| m.kind).collect();
    assert_eq!(
        kinds,
        BTreeSet::from([
            SecretKind::ApiKey,
            SecretKind::BearerToken,
            SecretKind::PrivateKey,
            SecretKind::AwsAccessKey,
            SecretKind::AwsSecretKey,
            SecretKind::GithubToken,
            SecretKind::Password,
            SecretKind::Cookie,
        ])
    );
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        original,
        "scanner must not modify the source file"
    );
    assert_eq!(scanned.redaction_status.as_str(), "partial");
}

// SCEN-2.1.4 / AC4: dry-run reports redaction hits without producing index content.
#[test]
fn test_2_1_4_dry_run_lists_hits_without_index_content() {
    // TEST-2.1.4
    let root = temp_root("dry-run");
    write_file(&root.join("src/app.env"), "password = \"secret-value\"\n");

    let options = ScanOptions {
        dry_run: true,
        ..ScanOptions::default()
    };
    let report = scan_path(&root, &options).expect("dry-run scan succeeds");
    assert!(report.dry_run);
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.redaction_hits.len(), 1);
    assert_eq!(report.redaction_hits[0].kind, SecretKind::Password);
    assert!(
        report.files[0].redacted_content.is_none(),
        "dry-run must not produce indexable redacted content"
    );
    assert!(
        report.files[0].content.is_none(),
        "dry-run must not carry original indexable content"
    );
}

// SCEN-2.1.5 / AC5: oversized files are skipped before content is read.
#[test]
fn test_2_1_5_oversized_file_is_skipped() {
    // TEST-2.1.5
    let root = temp_root("too-large");
    let file = root.join("large.log");
    write_file(&file, "this content is larger than the test limit\n");

    let options = ScanOptions {
        max_file_bytes: 1,
        ..ScanOptions::default()
    };
    let report = scan_path(&root, &options).expect("scan succeeds with oversized skip");
    assert!(
        report.files.is_empty(),
        "oversized file must not be scanned"
    );
    assert!(
        report.skipped.iter().any(|s| {
            rel(&s.path, &root) == "large.log"
                && matches!(s.reason, SkipReason::TooLarge { size, max } if size > max && max == 1)
        }),
        "oversized file should be recorded with TooLarge"
    );
}

// Review FIX-1 / AC1+AC3: public scan_file must not bypass denylist.
#[test]
fn test_2_1_review_scan_file_rejects_denylisted_path_without_override() {
    let root = temp_root("scan-file-denylist");
    let file = root.join(".env");
    write_file(&file, "DB_HOST=internal.corp\nDBPASS=not-patterned\n");

    let err = scan_file(&file, &ScanOptions::default())
        .expect_err("scan_file must fail closed on denylisted paths");
    assert!(
        matches!(err, ScanError::DenylistOverrideRequired),
        "scan_file should require explicit denylist override"
    );

    let options = ScanOptions {
        allowlist: vec![file.clone()],
        allow_denylist_override: true,
        ..ScanOptions::default()
    };
    let scanned = scan_file(&file, &options).expect("explicit override should allow scan_file");
    assert_eq!(
        scanned.content.as_deref(),
        Some("DB_HOST=internal.corp\nDBPASS=not-patterned\n")
    );
}

// Review FIX-1 / AC2: public scan_file must honor explicit allowlist scopes.
#[test]
fn test_2_1_review_scan_file_rejects_not_allowlisted_path() {
    let root = temp_root("scan-file-allowlist");
    let file = root.join("outside.md");
    let allowed = root.join("allowed");
    write_file(&file, "outside\n");
    fs::create_dir_all(&allowed).unwrap();

    let options = ScanOptions {
        allowlist: vec![allowed],
        ..ScanOptions::default()
    };
    let err = scan_file(&file, &options).expect_err("scan_file must honor allowlist");
    assert!(matches!(err, ScanError::NotAllowlisted));
}

// Review FIX-2 / AC5: scan_file should report TooLarge instead of a zero-content success.
#[test]
fn test_2_1_review_scan_file_too_large_errors_before_read() {
    let root = temp_root("scan-file-too-large");
    let file = root.join("large.log");
    write_file(&file, "larger than configured limit\n");

    let options = ScanOptions {
        max_file_bytes: 1,
        ..ScanOptions::default()
    };
    let err =
        scan_file(&file, &options).expect_err("scan_file should fail closed on too-large file");
    assert!(
        matches!(err, ScanError::FileTooLarge { size, max, .. } if size > max && max == 1),
        "too-large scan_file should surface exact size/max"
    );
}

// Review FIX-3: denylisted directories should be pruned as one skip entry.
#[test]
fn test_2_1_review_denylisted_directory_is_pruned() {
    let root = temp_root("prune-denylisted-dir");
    write_file(&root.join("node_modules/a/index.js"), "a\n");
    write_file(&root.join("node_modules/b/index.js"), "b\n");
    write_file(&root.join("src/main.rs"), "fn main() {}\n");

    let report = scan_path(&root, &ScanOptions::default()).expect("scan succeeds");
    assert_eq!(
        report
            .skipped
            .iter()
            .filter(|s| rel(&s.path, &root).starts_with("node_modules"))
            .count(),
        1,
        "denylisted directory should be recorded once and pruned"
    );
    assert!(report
        .skipped
        .iter()
        .any(|s| rel(&s.path, &root) == "node_modules"
            && matches!(s.reason, SkipReason::Denylisted(_))));
}

// Review FIX-4: symlinks should be visible as skipped and never followed.
#[cfg(unix)]
#[test]
fn test_2_1_review_symlink_is_recorded_and_not_followed() {
    let root = temp_root("symlink");
    let outside = temp_root("symlink-outside");
    let outside_secret = outside.join("secret.txt");
    let link = root.join("linked-secret.txt");
    write_file(&outside_secret, "password = \"should-not-be-read\"\n");
    std::os::unix::fs::symlink(&outside_secret, &link).unwrap();

    let report = scan_path(&root, &ScanOptions::default()).expect("scan succeeds");
    assert!(
        report.files.is_empty(),
        "symlink target must not be scanned"
    );
    assert!(
        report.redaction_hits.is_empty(),
        "symlink target must not be read"
    );
    assert!(report
        .skipped
        .iter()
        .any(|s| rel(&s.path, &root) == "linked-secret.txt"
            && matches!(s.reason, SkipReason::Symlink)));
}

// Review FIX-5 / AC3: cover common token edges and avoid substring false positives.
#[test]
fn test_2_1_review_secret_pattern_edges_and_key_boundaries() {
    let content = concat!(
        "Authorization: Bearer\tabc.def.ghi\n",
        "x-api-key: edge-api-key-value\n",
        "github fine = github_pat_11AA22BB33CC44DD55EE_66FF77GG88HH99II00JJ\n",
        "github oauth = gho_1234567890abcdefghijklmnopqrstuv\n",
        "my_api_keyword = \"not-secret\"\n",
        "password_hint: do-not-redact-this\n",
    );

    let (redacted, status, hits) = redact_content(content);
    assert_eq!(status, RedactionStatus::Partial);
    assert!(redacted.contains("[REDACTED:BEARER_TOKEN]"));
    assert!(redacted.contains("[REDACTED:API_KEY]"));
    assert_eq!(
        hits.iter()
            .filter(|h| h.kind == SecretKind::GithubToken)
            .count(),
        2,
        "both github_pat_ and gho_ tokens should be redacted"
    );
    assert!(
        redacted.contains("my_api_keyword = \"not-secret\""),
        "key matching should not redact substrings inside longer identifiers"
    );
    assert!(
        redacted.contains("password_hint: do-not-redact-this"),
        "password_hint is not a password assignment"
    );
}

// Review FIX-5: a file whose meaningful content is one private key is fully redacted.
#[test]
fn test_2_1_review_private_key_only_is_full_redaction() {
    let private_key = "-----BEGIN PRIVATE KEY-----\nabc123\n-----END PRIVATE KEY-----\n";
    let (redacted, status, hits) = redact_content(private_key);
    assert_eq!(status, RedactionStatus::Full);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].kind, SecretKind::PrivateKey);
    assert_eq!(redacted.trim(), "[REDACTED:PRIVATE_KEY]");
}
