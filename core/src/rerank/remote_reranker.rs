//! task-38.1: remote reranker provider (OpenAI/Cohere/SiliconFlow-style HTTP rerank).
//!
//! `RemoteRerankerProvider` posts a cross-encoder rerank request to a remote HTTP endpoint and maps
//! the `{index, relevance_score}` results back onto the original candidates. Gated behind the
//! `reranker-remote` feature — the default build does not compile this module or pull the HTTP client
//! (ADR-004 local-first: 0 network dep by default; it reuses the already-present optional `ureq` dep
//! from `embedding-remote`, so 0 NEW crate / no Cargo.lock change — ADR-008).
//!
//! It mirrors two existing patterns: the by-index map-back + score-overwrite + provenance annotation
//! of `CrossEncoderReranker` (`cross_encoder.rs:76-90`, only the local fastembed model is swapped for
//! a `ureq` POST), and the pure request/response functions + `Debug`-never-logs-api_key of
//! `RemoteEmbeddingProvider` (`remote_provider.rs:47-123`). `build_rerank_request_body` /
//! `parse_rerank_response` are pure (no network), so the contract tests assert them with fixtures and
//! never touch the network (ADR-013). Real network reachability / API keys / real rerank quality are
//! env-gated + honest-defer in `core/tests/remote_rerank_recall.rs` — CI has no credentials.

use serde_json::Value;

use crate::rerank::traits::{RerankError, Reranker};
use crate::retriever::SearchResult;

/// Provenance marker written to a reranked result's `reason` (mirrors the identity / cross-encoder
/// markers — ADR-026 D2: annotate the rerank source).
pub const REMOTE_RERANK_REASON: &str = "reranked:remote";

/// Remote HTTP cross-encoder reranker (OpenAI/Cohere/SiliconFlow style). The API key is read from
/// the environment / config by the factory and is never logged or persisted (PRD security baseline).
pub struct RemoteRerankerProvider {
    endpoint: String,
    model: String,
    api_key: Option<String>,
    name: &'static str,
}

impl RemoteRerankerProvider {
    pub fn new(endpoint: String, model: String, provider: &str, api_key: Option<String>) -> Self {
        let name = match provider {
            "cohere" => "remote-cohere",
            _ => "remote-rerank",
        };
        Self {
            endpoint,
            model,
            api_key,
            name,
        }
    }
}

impl std::fmt::Debug for RemoteRerankerProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // endpoint/model only — never the api_key (security baseline; mirrors RemoteEmbeddingProvider).
        write!(
            f,
            "RemoteRerankerProvider {{ name: {}, endpoint: {:?}, model: {:?} }}",
            self.name, self.endpoint, self.model
        )
    }
}

/// Build a SiliconFlow/Cohere-style rerank request body. `return_documents:false` because we already
/// hold the candidates and map back by `index`. Pure — no network (mirrors
/// `remote_provider.rs::build_request_body`).
pub fn build_rerank_request_body(model: &str, query: &str, documents: &[String], top_n: usize) -> Value {
    serde_json::json!({
        "model": model,
        "query": query,
        "documents": documents,
        "top_n": top_n,
        "return_documents": false,
    })
}

/// Parse a rerank response (`{"results":[{"index":i,"relevance_score":s}, ...]}`) into `(index,
/// score)` pairs, sorted by score descending. Pure — no network. Malformed JSON / missing `results`
/// / a missing `index` or `relevance_score` map to an explicit `RerankError` (never a panic) —
/// mirrors `remote_provider.rs::parse_response`.
pub fn parse_rerank_response(body: &str) -> Result<Vec<(usize, f32)>, RerankError> {
    let json: Value = serde_json::from_str(body)
        .map_err(|e| RerankError::Other(format!("remote rerank response parse: {e}")))?;
    let results = json
        .get("results")
        .and_then(|r| r.as_array())
        .ok_or_else(|| RerankError::Other("remote rerank response missing 'results' array".into()))?;
    if results.is_empty() {
        return Err(RerankError::Other(
            "remote rerank response 'results' is empty".into(),
        ));
    }
    let mut out = Vec::with_capacity(results.len());
    for (i, item) in results.iter().enumerate() {
        let index = item
            .get("index")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                RerankError::Other(format!("remote rerank response results[{i}] missing 'index'"))
            })? as usize;
        let score = item
            .get("relevance_score")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                RerankError::Other(format!(
                    "remote rerank response results[{i}] missing 'relevance_score'"
                ))
            })? as f32;
        out.push((index, score));
    }
    // Sort by score descending (the endpoint usually returns this order already; we guarantee it).
    out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}

impl Reranker for RemoteRerankerProvider {
    fn rerank(
        &self,
        query: &str,
        candidates: &[SearchResult],
    ) -> Result<Vec<SearchResult>, RerankError> {
        if candidates.is_empty() {
            return Ok(vec![]);
        }
        let documents: Vec<String> = candidates.iter().map(|c| c.content.clone()).collect();
        let body = build_rerank_request_body(&self.model, query, &documents, candidates.len());
        let body_str =
            serde_json::to_string(&body).map_err(|e| RerankError::Other(e.to_string()))?;
        let mut req = ureq::post(&self.endpoint).set("Content-Type", "application/json");
        if let Some(key) = &self.api_key {
            req = req.set("Authorization", &format!("Bearer {key}"));
        }
        let resp = req
            .send_string(&body_str)
            .map_err(|e| RerankError::Backend { source: Box::new(e) })?;
        let text = resp
            .into_string()
            .map_err(|e| RerankError::Backend { source: Box::new(e) })?;
        let ranked = parse_rerank_response(&text)?;

        let mut out = Vec::with_capacity(ranked.len());
        for (index, score) in ranked {
            // `index` is the original position in `candidates` (mirrors cross_encoder.rs:79-81).
            let src = candidates.get(index).ok_or_else(|| {
                RerankError::Other(format!("rerank index {index} out of range"))
            })?;
            let mut c = src.clone();
            c.score = score;
            c.reason = if c.reason.is_empty() {
                format!("{REMOTE_RERANK_REASON}:{}", self.name())
            } else {
                format!("{REMOTE_RERANK_REASON}:{}; {}", self.name(), c.reason)
            };
            out.push(c);
        }
        Ok(out)
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> String {
        v.to_string()
    }

    // TEST-38.1 contract (pure, no network): build_rerank_request_body constructs the expected
    // SiliconFlow/Cohere-style body (model / query / documents array / top_n / return_documents=false).
    #[test]
    fn build_rerank_request_body_shape() {
        let body =
            build_rerank_request_body("Qwen/Qwen3-VL-Reranker-8B", "save config", &[s("a"), s("b")], 2);
        assert_eq!(body["model"], "Qwen/Qwen3-VL-Reranker-8B");
        assert_eq!(body["query"], "save config");
        assert_eq!(body["documents"][0], "a");
        assert_eq!(body["documents"][1], "b");
        assert_eq!(body["top_n"], 2);
        assert_eq!(body["return_documents"], false);
    }

    // TEST-38.1 contract (pure, no network): parse_rerank_response parses results[].{index,
    // relevance_score} into (usize, f32) sorted by score desc; malformed / empty / missing → Err.
    #[test]
    fn parse_rerank_response_sorts_and_errors() {
        let fixture = r#"{"results":[
            {"index":2,"relevance_score":0.10},
            {"index":0,"relevance_score":0.90},
            {"index":1,"relevance_score":0.50}
        ],"meta":{}}"#;
        let out = parse_rerank_response(fixture).expect("fixture parses");
        assert_eq!(out, vec![(0usize, 0.90f32), (1, 0.50), (2, 0.10)], "sorted by score desc");

        assert!(parse_rerank_response("{not json").is_err(), "malformed JSON → Err");
        assert!(parse_rerank_response(r#"{"results":[]}"#).is_err(), "empty results → Err");
        assert!(
            parse_rerank_response(r#"{"results":[{"relevance_score":0.5}]}"#).is_err(),
            "missing index → Err"
        );
        assert!(
            parse_rerank_response(r#"{"results":[{"index":0}]}"#).is_err(),
            "missing relevance_score → Err"
        );
    }

    // Provider identity + Debug never leaks the api_key.
    #[test]
    fn provider_name_and_debug_redacts_api_key() {
        let p = RemoteRerankerProvider::new(
            "https://api.siliconflow.cn/v1/rerank".into(),
            "Qwen/Qwen3-VL-Reranker-8B".into(),
            "siliconflow",
            Some("super-secret-key".into()),
        );
        assert_eq!(p.name(), "remote-rerank");
        let dbg = format!("{p:?}");
        assert!(!dbg.contains("super-secret-key"), "Debug must never print the api_key");
        assert!(dbg.contains("rerank"), "Debug surfaces endpoint/model");
    }
}
