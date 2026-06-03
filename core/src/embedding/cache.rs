//! task-22.2: content-hash embedding cache decorator (Phase 22 embedding-provider-completion).
//!
//! `CachingEmbeddingProvider` wraps any `Arc<dyn EmbeddingProvider>` and is itself an
//! `EmbeddingProvider`, so it drops transparently into `Retriever::with_embedder` / the task-22.1
//! factory. The cache key is `Sha256(text)` hex (same hash as `deterministic.rs` — 0 new dep): same
//! text ⇒ same key ⇒ hit (skips the inner provider); changed text ⇒ different key ⇒ miss (= implicit
//! invalidation). Memory (`HashMap`) is the L1 default; an optional SQLite file (ADR-002 layered,
//! `rusqlite` bundled) is the L2 persistence opt-in. Cache entries are scoped by `(provider, dim)`
//! so a cache file is never mis-read across providers.

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::embedding::traits::{EmbeddingError, EmbeddingProvider};

/// task-31.2 (ADR-036 D2): default L1 cap. The L1 cache was an unbounded `HashMap`, so a long-running
/// daemon embedding many distinct texts grew memory without bound. ~50k entries × 384 f32 × 4 B ≈ 75 MB
/// upper bound — generous for normal workflows (same text still hits) but bounded.
pub const DEFAULT_EMBEDDING_CACHE_CAP: usize = 50_000;

/// task-33.1 (ADR-038 D1): default L2 (SQLite) row-count cap, scoped per (provider, dim). The L2
/// `embedding_cache` table was unbounded (`INSERT OR REPLACE` only grows); this bounds it with
/// rowid-FIFO eviction (0 schema change — uses the implicit rowid as insert order). Same default as L1.
pub const DEFAULT_L2_EMBEDDING_CACHE_CAP: usize = 50_000;

/// task-31.2: capacity-bounded L1 cache with FIFO-on-insert eviction (0 new dep — hand-rolled over
/// `std`). Re-inserting an existing key updates the value in place without changing eviction order.
struct BoundedCache {
    map: HashMap<String, Vec<f32>>,
    order: VecDeque<String>,
    cap: usize,
}

impl BoundedCache {
    fn new(cap: usize) -> Self {
        Self { map: HashMap::new(), order: VecDeque::new(), cap }
    }

    fn get(&self, k: &str) -> Option<&Vec<f32>> {
        self.map.get(k)
    }

    fn insert(&mut self, k: String, v: Vec<f32>) {
        if self.map.insert(k.clone(), v).is_some() {
            // Key already present → value updated in place; eviction order unchanged.
            return;
        }
        // New key: record insertion order, then evict oldest (FIFO) while over cap.
        // cap == 0 ⇒ unbounded.
        self.order.push_back(k);
        if self.cap > 0 {
            while self.map.len() > self.cap {
                match self.order.pop_front() {
                    Some(old) => {
                        self.map.remove(&old);
                    }
                    None => break,
                }
            }
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.map.len()
    }
}

/// Embedding provider decorator that caches `Sha256(text) → embedding`.
pub struct CachingEmbeddingProvider {
    inner: Arc<dyn EmbeddingProvider>,
    mem: Mutex<BoundedCache>,
    store: Option<Mutex<Connection>>,
    /// task-33.1: L2 row-count cap per (provider, dim); 0 ⇒ unbounded. Only meaningful when `store` is `Some`.
    l2_cap: usize,
}

impl std::fmt::Debug for CachingEmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CachingEmbeddingProvider {{ inner: {:?}, persistent: {} }}",
            self.inner,
            self.store.is_some()
        )
    }
}

impl CachingEmbeddingProvider {
    /// Wrap `inner` with an in-memory cache (no persistence), L1 bounded at
    /// `DEFAULT_EMBEDDING_CACHE_CAP` (task-31.2).
    pub fn new(inner: Arc<dyn EmbeddingProvider>) -> Self {
        Self::with_capacity(inner, DEFAULT_EMBEDDING_CACHE_CAP)
    }

    /// Wrap `inner` with an in-memory cache whose L1 is bounded at `cap` (FIFO-on-insert eviction;
    /// `cap == 0` ⇒ unbounded). task-31.2.
    pub fn with_capacity(inner: Arc<dyn EmbeddingProvider>, cap: usize) -> Self {
        Self {
            inner,
            mem: Mutex::new(BoundedCache::new(cap)),
            store: None,
            l2_cap: DEFAULT_L2_EMBEDDING_CACHE_CAP,
        }
    }

    /// Wrap `inner` with an in-memory L1 cache backed by a SQLite file at `path` (L2 persistence,
    /// ADR-002 layered; add-only `embedding_cache` table, does not touch existing schema).
    pub fn with_sqlite(
        inner: Arc<dyn EmbeddingProvider>,
        path: impl AsRef<Path>,
    ) -> Result<Self, EmbeddingError> {
        Self::with_sqlite_capacity(inner, path, DEFAULT_L2_EMBEDDING_CACHE_CAP)
    }

    /// task-33.1: like [`with_sqlite`] but with an explicit L2 row-count cap (`l2_cap == 0` ⇒
    /// unbounded; rowid-FIFO eviction per (provider, dim)). L1 stays bounded at
    /// `DEFAULT_EMBEDDING_CACHE_CAP`. Public ctors stay source-compatible (add-only).
    pub fn with_sqlite_capacity(
        inner: Arc<dyn EmbeddingProvider>,
        path: impl AsRef<Path>,
        l2_cap: usize,
    ) -> Result<Self, EmbeddingError> {
        let conn = Connection::open(path).map_err(to_backend_err)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS embedding_cache (\
                content_hash TEXT NOT NULL, \
                provider TEXT NOT NULL, \
                dim INTEGER NOT NULL, \
                vector BLOB NOT NULL, \
                PRIMARY KEY (content_hash, provider)\
            )",
            [],
        )
        .map_err(to_backend_err)?;
        Ok(Self {
            inner,
            mem: Mutex::new(BoundedCache::new(DEFAULT_EMBEDDING_CACHE_CAP)),
            store: Some(Mutex::new(conn)),
            l2_cap,
        })
    }

    fn key(text: &str) -> String {
        let digest = Sha256::digest(text.as_bytes());
        let mut s = String::with_capacity(64);
        for b in digest {
            use std::fmt::Write;
            let _ = write!(s, "{:02x}", b);
        }
        s
    }

    /// L2 read: a persisted vector for `key`, scoped to the inner provider's (name, dim) so a cache
    /// file is never mis-read across providers. `None` = miss.
    fn sqlite_get(&self, conn: &Connection, key: &str) -> Result<Option<Vec<f32>>, EmbeddingError> {
        let blob: Option<Vec<u8>> = conn
            .query_row(
                "SELECT vector FROM embedding_cache WHERE content_hash=?1 AND provider=?2 AND dim=?3",
                rusqlite::params![key, self.inner.name(), self.inner.dim() as i64],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(to_backend_err)?;
        Ok(blob.map(|b| bytes_to_vec(&b)))
    }

    /// L2 write-through for `key` → `v`, scoped to the inner provider's (name, dim).
    fn sqlite_put(&self, conn: &Connection, key: &str, v: &[f32]) -> Result<(), EmbeddingError> {
        conn.execute(
            "INSERT OR REPLACE INTO embedding_cache (content_hash, provider, dim, vector) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![key, self.inner.name(), self.inner.dim() as i64, vec_to_bytes(v)],
        )
        .map_err(to_backend_err)?;
        // task-33.1: bound the L2 row count with rowid-FIFO eviction, scoped per (provider, dim) so
        // one provider cannot starve another. rowid ASC = insert order; keep the newest `l2_cap` rows
        // and delete the rest. 0 schema change (uses the implicit rowid); mirrors the L1 FIFO contract.
        if self.l2_cap > 0 {
            conn.execute(
                "DELETE FROM embedding_cache WHERE provider=?1 AND dim=?2 AND rowid NOT IN (\
                    SELECT rowid FROM embedding_cache WHERE provider=?1 AND dim=?2 \
                    ORDER BY rowid DESC LIMIT ?3)",
                rusqlite::params![self.inner.name(), self.inner.dim() as i64, self.l2_cap as i64],
            )
            .map_err(to_backend_err)?;
        }
        Ok(())
    }
}

fn to_backend_err<E: std::error::Error + Send + Sync + 'static>(e: E) -> EmbeddingError {
    EmbeddingError::Backend { source: Box::new(e) }
}

/// f32 vector → little-endian byte BLOB (exact round-trip, no precision loss).
fn vec_to_bytes(v: &[f32]) -> Vec<u8> {
    let mut b = Vec::with_capacity(v.len() * 4);
    for x in v {
        b.extend_from_slice(&x.to_le_bytes());
    }
    b
}

/// Little-endian byte BLOB → f32 vector (inverse of `vec_to_bytes`).
fn bytes_to_vec(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

impl EmbeddingProvider for CachingEmbeddingProvider {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let mut out: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
        let mut miss_idx: Vec<usize> = Vec::new();

        // L1: memory cache.
        {
            let mem = self.mem.lock().unwrap();
            for (i, t) in texts.iter().enumerate() {
                if let Some(v) = mem.get(&Self::key(t)) {
                    out[i] = Some(v.clone());
                } else {
                    miss_idx.push(i);
                }
            }
        }

        // L2: SQLite (for L1 misses); promote hits into L1.
        if let Some(store) = &self.store {
            let conn = store.lock().unwrap();
            let mut still_miss = Vec::new();
            for &i in &miss_idx {
                let k = Self::key(&texts[i]);
                match self.sqlite_get(&conn, &k)? {
                    Some(v) => {
                        self.mem.lock().unwrap().insert(k, v.clone());
                        out[i] = Some(v);
                    }
                    None => still_miss.push(i),
                }
            }
            miss_idx = still_miss;
        }

        // Inner provider for the remaining misses; write-through to L1 (+ L2 if persistent).
        if !miss_idx.is_empty() {
            let miss_texts: Vec<String> = miss_idx.iter().map(|&i| texts[i].clone()).collect();
            let embedded = self.inner.embed(&miss_texts)?;
            for (j, &i) in miss_idx.iter().enumerate() {
                let v = &embedded[j];
                let k = Self::key(&texts[i]);
                self.mem.lock().unwrap().insert(k.clone(), v.clone());
                if let Some(store) = &self.store {
                    let conn = store.lock().unwrap();
                    self.sqlite_put(&conn, &k, v)?;
                }
                out[i] = Some(v.clone());
            }
        }

        Ok(out.into_iter().map(|o| o.expect("every slot filled")).collect())
    }

    fn dim(&self) -> usize {
        self.inner.dim()
    }

    fn name(&self) -> &'static str {
        "cached"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::deterministic::DeterministicEmbeddingProvider;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Test double: counts inner `embed` invocations and total texts embedded, delegating the actual
    /// vectors to a deterministic provider (so cache hits can be asserted by observing the counters).
    #[derive(Debug)]
    struct CountingProvider {
        inner: DeterministicEmbeddingProvider,
        embedded: AtomicUsize,
    }
    impl CountingProvider {
        fn new(dim: usize) -> Self {
            Self {
                inner: DeterministicEmbeddingProvider::new(dim),
                embedded: AtomicUsize::new(0),
            }
        }
        fn embedded(&self) -> usize {
            self.embedded.load(Ordering::SeqCst)
        }
    }
    impl EmbeddingProvider for CountingProvider {
        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
            self.embedded.fetch_add(texts.len(), Ordering::SeqCst);
            self.inner.embed(texts)
        }
        fn dim(&self) -> usize {
            self.inner.dim()
        }
        fn name(&self) -> &'static str {
            "counting-deterministic"
        }
    }

    fn s(v: &str) -> String {
        v.to_string()
    }

    // TEST-22.2.1 — AC1: same text → 2nd embed hits cache (inner not re-invoked) + bytes identical.
    #[test]
    fn test_22_2_1_hit_skips_inner_and_bytes_identical() {
        let counter = Arc::new(CountingProvider::new(384));
        let cache = CachingEmbeddingProvider::new(counter.clone());
        let t = vec![s("where is the config loader")];
        let first = cache.embed(&t).unwrap();
        let second = cache.embed(&t).unwrap();
        assert_eq!(first, second, "cache hit must return byte-identical vectors");
        assert_eq!(
            counter.embedded(),
            1,
            "2nd embed must hit cache — inner embedded only the 1 unique text"
        );
        let direct = DeterministicEmbeddingProvider::new(384).embed(&t).unwrap();
        assert_eq!(first, direct, "cached vector equals the inner provider's direct embed");
    }

    // TEST-22.2.2 — AC2: invalidation — new text misses; batch with mixed hit/miss only embeds the
    // misses, and results stay in input order.
    #[test]
    fn test_22_2_2_miss_on_new_text_and_batch_order() {
        let counter = Arc::new(CountingProvider::new(384));
        let cache = CachingEmbeddingProvider::new(counter.clone());
        let a = cache.embed(&[s("alpha")]).unwrap();
        assert_eq!(counter.embedded(), 1);
        // batch: "alpha" is cached, "beta" is new — only "beta" should reach the inner provider.
        let batch = cache.embed(&[s("alpha"), s("beta")]).unwrap();
        assert_eq!(counter.embedded(), 2, "only the new text 'beta' is embedded by inner");
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0], a[0], "batch[0] is the cached 'alpha' vector, in order");
        let beta_direct = DeterministicEmbeddingProvider::new(384).embed(&[s("beta")]).unwrap();
        assert_eq!(batch[1], beta_direct[0], "batch[1] is the freshly-embedded 'beta', in order");
    }

    // TEST-22.2.3 — AC3: SQLite persistence round-trip — a fresh provider over the same file reads
    // back the cached vector (inner 0 calls); the default (memory) provider does not persist.
    #[test]
    fn test_22_2_3_sqlite_roundtrip_and_memory_default_no_persist() {
        let dir = std::env::temp_dir().join(format!("cf-cache-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("embcache.sqlite");

        let counter1 = Arc::new(CountingProvider::new(384));
        let cache1 = CachingEmbeddingProvider::with_sqlite(counter1.clone(), &db).unwrap();
        let want = cache1.embed(&[s("persist me")]).unwrap();
        assert_eq!(counter1.embedded(), 1);

        // fresh provider, same file: must read back from SQLite without touching inner.
        let counter2 = Arc::new(CountingProvider::new(384));
        let cache2 = CachingEmbeddingProvider::with_sqlite(counter2.clone(), &db).unwrap();
        let got = cache2.embed(&[s("persist me")]).unwrap();
        assert_eq!(got, want, "SQLite-persisted vector reads back identically");
        assert_eq!(counter2.embedded(), 0, "persisted hit must not call the inner provider");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // TEST-31.2.1 — AC1: L1 capacity bound with FIFO-on-insert eviction. Over-cap inserts evict the
    // oldest; an evicted key re-embeds as a miss (inner re-invoked); a still-cached key stays a hit.
    #[test]
    fn test_31_2_1_l1_cap_evicts_oldest_fifo() {
        let counter = Arc::new(CountingProvider::new(384));
        let cache = CachingEmbeddingProvider::with_capacity(counter.clone(), 2);
        cache.embed(&[s("a")]).unwrap();
        cache.embed(&[s("b")]).unwrap();
        cache.embed(&[s("c")]).unwrap(); // cap=2 → "a" (oldest) evicted; cache = {b, c}
        assert_eq!(counter.embedded(), 3, "3 distinct texts each miss inner once");
        assert!(cache.mem.lock().unwrap().len() <= 2, "L1 entry count bounded at cap");
        // "a" was evicted → re-embed is a miss (inner re-called).
        cache.embed(&[s("a")]).unwrap(); // evicts "b"; cache = {c, a}
        assert_eq!(counter.embedded(), 4, "evicted 'a' re-embed is a miss");
        // "c" still cached → re-embed is a hit (inner not called).
        cache.embed(&[s("c")]).unwrap();
        assert_eq!(counter.embedded(), 4, "still-cached 'c' re-embed is a hit");
        assert!(cache.mem.lock().unwrap().len() <= 2, "L1 stays bounded after re-inserts");
    }

    // TEST-33.1.1 — AC1: L2 SQLite row-count cap with rowid-FIFO eviction (mirrors L1 TEST-31.2.1,
    // against the SQLite store). A fresh-L1 provider over the same file isolates the L2 layer: the
    // oldest-inserted hash is evicted from L2 (re-embed = miss → inner re-called), a still-resident
    // hash is an L2 hit (inner not called), and COUNT(*) stays bounded at the L2 cap.
    #[test]
    fn test_33_1_1_l2_cap_evicts_oldest_fifo() {
        let dir = std::env::temp_dir().join(format!("cf-l2cap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("l2cap.sqlite");

        let counter1 = Arc::new(CountingProvider::new(384));
        let cache1 = CachingEmbeddingProvider::with_sqlite_capacity(counter1.clone(), &db, 2).unwrap();
        cache1.embed(&[s("a")]).unwrap();
        cache1.embed(&[s("b")]).unwrap();
        cache1.embed(&[s("c")]).unwrap(); // L2 cap=2 → "a" (oldest rowid) evicted; L2 = {b, c}

        let conn = rusqlite::Connection::open(&db).unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM embedding_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2, "L2 row count bounded at cap");
        drop(conn);

        // fresh provider (empty L1) over same file: "a" was evicted from L2 → miss (inner re-called);
        // "c" still resident → L2 hit (inner not called).
        let counter2 = Arc::new(CountingProvider::new(384));
        let cache2 = CachingEmbeddingProvider::with_sqlite_capacity(counter2.clone(), &db, 2).unwrap();
        cache2.embed(&[s("a")]).unwrap();
        assert_eq!(counter2.embedded(), 1, "evicted 'a' is an L2 miss → inner re-called");
        cache2.embed(&[s("c")]).unwrap();
        assert_eq!(counter2.embedded(), 1, "still-resident 'c' is an L2 hit → inner not called");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // TEST-33.1.2 — AC2: default L2 cap does not prematurely evict — a modest workload keeps every
    // entry (guards behavior-unchanged for the default with_sqlite path, ADR-004; existing TEST-22.2.*
    // roundtrip stays green).
    #[test]
    fn test_33_1_2_default_l2_cap_keeps_modest_workload() {
        let dir = std::env::temp_dir().join(format!("cf-l2def-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("l2def.sqlite");

        let counter = Arc::new(CountingProvider::new(384));
        let cache = CachingEmbeddingProvider::with_sqlite(counter.clone(), &db).unwrap();
        for t in ["t1", "t2", "t3", "t4", "t5"] {
            cache.embed(&[s(t)]).unwrap();
        }
        let conn = rusqlite::Connection::open(&db).unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM embedding_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 5, "default cap keeps all 5 entries (no premature eviction)");
        drop(conn);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // TEST-22.2.4 — AC4: dim()/name() passthrough + usable as Arc<dyn EmbeddingProvider>.
    #[test]
    fn test_22_2_4_passthrough_and_trait_object() {
        let counter = Arc::new(CountingProvider::new(256));
        let cache: Arc<dyn EmbeddingProvider> = Arc::new(CachingEmbeddingProvider::new(counter));
        assert_eq!(cache.dim(), 256, "dim() passes through the inner provider");
        assert_eq!(cache.name(), "cached", "name() carries the cached provenance");
    }
}
