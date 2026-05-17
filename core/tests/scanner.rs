//! task-2.1 scanner tests — TEST-2.1.1 ~ TEST-2.1.5 (SCEN-2.1.*).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use contextforge_core::scanner::{
    scan_file, scan_path, ScanError, ScanOptions, SecretKind, SkipReason,
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
            .any(|s| rel(&s.path, &root) == "outside/drop.md"
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
    assert!(report.files.is_empty(), "oversized file must not be scanned");
    assert!(
        report.skipped.iter().any(|s| {
            rel(&s.path, &root) == "large.log"
                && matches!(s.reason, SkipReason::TooLarge { size, max } if size > max && max == 1)
        }),
        "oversized file should be recorded with TooLarge"
    );
}
