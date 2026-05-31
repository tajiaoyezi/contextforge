//! task-22.3: remote embedding provider skeleton (Phase 22 embedding-provider-completion).
//!
//! `RemoteEmbeddingProvider` posts an OpenAI/Cohere-style embedding request and parses the response.
//! Gated behind the `embedding-remote` feature — the default build does not compile this module or
//! pull the HTTP client (ADR-004 local-first: 0 network dep by default; ADR-008 D5: rustls, no
//! system OpenSSL). Request construction (`build_request_body`) and response parsing
//! (`parse_response`) are pure functions, so the contract tests assert them with fixtures and never
//! touch the network (ADR-013). Real network reachability / API keys / real recall quality are
//! deferred — CI has no credentials; see task spec §8 R1 stop-condition.

use serde_json::Value;

use crate::embedding::traits::{EmbeddingError, EmbeddingProvider};

/// Remote HTTP embedding provider (OpenAI/Cohere style). The API key is read from the environment /
/// config by the factory and is never logged or persisted (PRD security baseline).
pub struct RemoteEmbeddingProvider {
    endpoint: String,
    model: String,
    dim: usize,
    api_key: Option<String>,
    name: &'static str,
}

impl RemoteEmbeddingProvider {
    pub fn new(
        endpoint: String,
        model: String,
        dim: usize,
        provider: &str,
        api_key: Option<String>,
    ) -> Self {
        let name = match provider {
            "cohere" => "remote-cohere",
            _ => "remote-openai",
        };
        Self {
            endpoint,
            model,
            dim,
            api_key,
            name,
        }
    }
}

impl std::fmt::Debug for RemoteEmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // endpoint/model only — never the api_key (security baseline).
        write!(
            f,
            "RemoteEmbeddingProvider {{ name: {}, endpoint: {:?}, model: {:?}, dim: {} }}",
            self.name, self.endpoint, self.model, self.dim
        )
    }
}

/// Build an OpenAI/Cohere-style embedding request body. `dim == 0` omits the `dimensions` field
/// (let the model use its native dimension). Pure — no network.
fn build_request_body(model: &str, texts: &[String], dim: usize) -> Value {
    // task-22.3 RED: stub — GREEN constructs the real {model, input, dimensions} body.
    let _ = (model, texts, dim);
    serde_json::json!({})
}

/// Parse an OpenAI/Cohere-style embedding response (`{"data":[{"embedding":[...]}, ...]}`) into one
/// vector per `data` entry, in order. Pure — no network. Malformed JSON / empty `data` / a missing
/// `embedding` field map to an explicit `EmbeddingError` (never a panic).
fn parse_response(body: &str) -> Result<Vec<Vec<f32>>, EmbeddingError> {
    // task-22.3 RED: stub — GREEN parses data[].embedding with explicit error paths.
    let _ = body;
    Err(EmbeddingError::Other("parse_response not implemented (RED)".into()))
}

impl EmbeddingProvider for RemoteEmbeddingProvider {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        let body = build_request_body(&self.model, texts, self.dim);
        let body_str =
            serde_json::to_string(&body).map_err(|e| EmbeddingError::Other(e.to_string()))?;
        let mut req = ureq::post(&self.endpoint).set("Content-Type", "application/json");
        if let Some(key) = &self.api_key {
            req = req.set("Authorization", &format!("Bearer {key}"));
        }
        let resp = req
            .send_string(&body_str)
            .map_err(|e| EmbeddingError::Backend { source: Box::new(e) })?;
        let text = resp
            .into_string()
            .map_err(|e| EmbeddingError::Backend { source: Box::new(e) })?;
        parse_response(&text)
    }

    fn dim(&self) -> usize {
        self.dim
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

    // TEST-22.3.1 — AC1: build_request_body constructs the expected OpenAI-style body
    // (model / input array / dimensions when non-zero).
    #[test]
    fn test_22_3_1_build_request_body() {
        let body = build_request_body("text-embedding-3-small", &[s("alpha"), s("beta")], 384);
        assert_eq!(body["model"], "text-embedding-3-small");
        assert_eq!(body["input"][0], "alpha");
        assert_eq!(body["input"][1], "beta");
        assert_eq!(body["dimensions"], 384, "non-zero dim emits dimensions");

        let no_dim = build_request_body("m", &[s("x")], 0);
        assert!(no_dim.get("dimensions").is_none(), "dim=0 omits dimensions");
    }

    // TEST-22.3.2 — AC2: parse_response parses data[].embedding into ordered vectors.
    #[test]
    fn test_22_3_2_parse_response() {
        let fixture = r#"{"object":"list","data":[
            {"object":"embedding","index":0,"embedding":[0.1,0.2,0.3]},
            {"object":"embedding","index":1,"embedding":[0.4,0.5,0.6]}
        ],"model":"text-embedding-3-small"}"#;
        let out = parse_response(fixture).expect("fixture parses");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], vec![0.1f32, 0.2, 0.3]);
        assert_eq!(out[1], vec![0.4f32, 0.5, 0.6]);
    }

    // TEST-22.3.3 — AC3: error paths — malformed JSON / empty data / missing embedding → explicit
    // EmbeddingError (no panic).
    #[test]
    fn test_22_3_3_parse_response_error_paths() {
        assert!(parse_response("{not json").is_err(), "malformed JSON → Err");
        assert!(
            parse_response(r#"{"data":[]}"#).is_err(),
            "empty data → Err"
        );
        assert!(
            parse_response(r#"{"data":[{"index":0}]}"#).is_err(),
            "missing embedding field → Err"
        );
    }

    // TEST-22.3.4 — AC4: provider name()/dim() + factory "remote" branch under the feature.
    #[test]
    fn test_22_3_4_provider_name_dim_and_factory() {
        let p = RemoteEmbeddingProvider::new(
            "https://api.openai.com/v1/embeddings".into(),
            "text-embedding-3-small".into(),
            384,
            "openai",
            None,
        );
        assert_eq!(p.name(), "remote-openai");
        assert_eq!(p.dim(), 384);

        // factory "remote" branch returns a RemoteEmbeddingProvider under the feature (reads
        // endpoint/model/api_key from env; construction makes no network call).
        let viaf = crate::embedding::select_provider("remote", 384).expect("remote selects under feature");
        assert_eq!(viaf.dim(), 384);
        assert!(viaf.name().starts_with("remote-"), "factory remote provider carries remote-* provenance");
    }
}
