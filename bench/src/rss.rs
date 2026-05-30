//! Cross-platform RSS sampling. Linux reads /proc/self/statm; other platforms return None
//! (non-Linux sampling is deferred — Phase 18 §7 R1 anchors P0 = Linux).

#[cfg(target_os = "linux")]
pub fn sample_rss_mb() -> Option<f64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let rss_pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some((rss_pages * 4096) as f64 / (1024.0 * 1024.0))
}

#[cfg(not(target_os = "linux"))]
pub fn sample_rss_mb() -> Option<f64> {
    None
}
