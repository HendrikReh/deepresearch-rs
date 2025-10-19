#[cfg(feature = "qdrant-retriever")]
pub mod qdrant;
#[cfg(feature = "qdrant-retriever")]
pub use qdrant::{HybridRetriever, QdrantConfig};

use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;

#[derive(Debug, Clone)]
pub struct RetrievedDocument {
    pub text: String,
    pub score: f32,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IngestDocument {
    pub id: String,
    pub text: String,
    pub source: Option<String>,
}

#[async_trait]
pub trait Retriever: Send + Sync {
    async fn retrieve(
        &self,
        session_id: &str,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<RetrievedDocument>>;

    async fn ingest(&self, session_id: &str, docs: Vec<IngestDocument>) -> anyhow::Result<()>;
}

pub type DynRetriever = Arc<dyn Retriever>;

/// Simple in-memory retriever for tests and offline runs.
pub struct StubRetriever {
    store: DashMap<String, Vec<IngestDocument>>,
}

impl StubRetriever {
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
        }
    }
}

#[async_trait]
impl Retriever for StubRetriever {
    async fn retrieve(
        &self,
        session_id: &str,
        _query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<RetrievedDocument>> {
        let docs = self
            .store
            .get(session_id)
            .map(|entry| entry.clone())
            .unwrap_or_default();

        if docs.is_empty() {
            return Ok(vec![RetrievedDocument {
                text: "No indexed documents yet; returning placeholder finding.".to_string(),
                score: 0.0,
                source: None,
            }]);
        }

        Ok(docs
            .into_iter()
            .take(limit)
            .map(|doc| RetrievedDocument {
                text: doc.text,
                score: 1.0,
                source: doc.source.or_else(|| Some("stub://memory".to_string())),
            })
            .collect())
    }

    async fn ingest(&self, session_id: &str, docs: Vec<IngestDocument>) -> anyhow::Result<()> {
        self.store
            .entry(session_id.to_string())
            .or_default()
            .extend(docs);
        Ok(())
    }
}
