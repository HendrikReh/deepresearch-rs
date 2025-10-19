use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use fastembed::TextEmbedding;
use qdrant_client::qdrant::{
    value::Kind as QValueKind, Condition, CreateCollectionBuilder, Distance, Filter, ListValue,
    PointStruct, SearchPointsBuilder, UpsertPointsBuilder, Value as QValue, VectorParamsBuilder,
};
use qdrant_client::{Payload, Qdrant};
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

use super::{IngestDocument, RetrievedDocument, Retriever};

const KEY_SESSION: &str = "session_id";
const KEY_TEXT: &str = "text";
const KEY_SOURCE: &str = "source";
const KEY_KEYWORDS: &str = "keywords";
const MIN_KEYWORD_LEN: usize = 3;
const MAX_KEYWORDS: usize = 32;

#[derive(Clone, Debug)]
pub struct QdrantConfig {
    pub url: String,
    pub collection: String,
    pub concurrency_limit: usize,
}

pub struct HybridRetriever {
    client: Qdrant,
    collection: String,
    semaphore: Arc<Semaphore>,
    dense_model: Arc<Mutex<TextEmbedding>>,
}

impl HybridRetriever {
    pub async fn new(config: QdrantConfig) -> anyhow::Result<Self> {
        let (dense_model, dimension) = tokio::task::spawn_blocking(|| -> anyhow::Result<_> {
            let mut model = TextEmbedding::try_new(Default::default())
                .map_err(|err| anyhow!("failed to initialise FastEmbed model: {err}"))?;

            let warmup = model
                .embed(vec!["deepresearch warmup"], Some(1))
                .map_err(|err| anyhow!("failed to warm up FastEmbed model: {err}"))?;
            let dimension = warmup
                .first()
                .map(|vector| vector.len())
                .filter(|len| *len > 0)
                .ok_or_else(|| anyhow!("FastEmbed warmup returned no embedding rows"))?;

            Ok((model, dimension))
        })
        .await??;

        let client = Qdrant::from_url(&config.url)
            .build()
            .map_err(|err| anyhow!("failed to create Qdrant client: {err}"))?;

        ensure_collection(&client, &config.collection, dimension).await?;

        Ok(Self {
            client,
            collection: config.collection,
            semaphore: Arc::new(Semaphore::new(config.concurrency_limit.max(1))),
            dense_model: Arc::new(Mutex::new(dense_model)),
        })
    }
}

async fn ensure_collection(
    client: &Qdrant,
    collection: &str,
    dimension: usize,
) -> anyhow::Result<()> {
    if client.collection_exists(collection).await? {
        return Ok(());
    }

    client
        .create_collection(
            CreateCollectionBuilder::new(collection)
                .vectors_config(VectorParamsBuilder::new(dimension as u64, Distance::Cosine)),
        )
        .await
        .map_err(|err| anyhow!("failed to create qdrant collection '{collection}': {err}"))?;
    info!(collection, dimension, "created qdrant collection");
    Ok(())
}

fn tokenize(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut keywords = Vec::new();

    for token in text.split(|c: char| !c.is_alphanumeric()) {
        let token = token.trim().to_lowercase();
        if token.len() < MIN_KEYWORD_LEN {
            continue;
        }
        if seen.insert(token.clone()) {
            keywords.push(token);
        }
        if keywords.len() >= MAX_KEYWORDS {
            break;
        }
    }

    keywords
}

fn lexical_boost(query_tokens: &HashSet<String>, doc_keywords: &[String]) -> f32 {
    if query_tokens.is_empty() || doc_keywords.is_empty() {
        return 0.0;
    }

    let overlap = doc_keywords
        .iter()
        .filter(|kw| query_tokens.contains(kw.as_str()))
        .count();

    if overlap == 0 {
        0.0
    } else {
        overlap as f32 / query_tokens.len() as f32
    }
}

fn payload_from_scored(payload: Payload) -> (String, Option<String>, Vec<String>) {
    let mut map: HashMap<String, QValue> = payload.into();
    let text = map
        .remove(KEY_TEXT)
        .and_then(value_as_string)
        .unwrap_or_default();
    let source = map.remove(KEY_SOURCE).and_then(value_as_string);
    let keywords = map
        .remove(KEY_KEYWORDS)
        .map(value_as_string_list)
        .unwrap_or_default();

    (text, source, keywords)
}

fn build_payload(
    session_id: &str,
    text: &str,
    source: Option<&String>,
    keywords: Vec<String>,
) -> anyhow::Result<Payload> {
    let mut payload = Payload::default();

    payload.insert(
        KEY_SESSION.to_string(),
        QValue {
            kind: Some(QValueKind::StringValue(session_id.to_string())),
        },
    );

    payload.insert(
        KEY_TEXT.to_string(),
        QValue {
            kind: Some(QValueKind::StringValue(text.to_string())),
        },
    );

    if let Some(source) = source {
        payload.insert(
            KEY_SOURCE.to_string(),
            QValue {
                kind: Some(QValueKind::StringValue(source.clone())),
            },
        );
    }

    if !keywords.is_empty() {
        let values = keywords
            .into_iter()
            .map(|keyword| QValue {
                kind: Some(QValueKind::StringValue(keyword)),
            })
            .collect();

        payload.insert(
            KEY_KEYWORDS.to_string(),
            QValue {
                kind: Some(QValueKind::ListValue(ListValue { values })),
            },
        );
    }

    Ok(payload)
}

fn value_as_string(value: QValue) -> Option<String> {
    match value.kind? {
        QValueKind::StringValue(v) => Some(v),
        _ => None,
    }
}

fn value_as_string_list(value: QValue) -> Vec<String> {
    match value.kind {
        Some(QValueKind::ListValue(list)) => list
            .values
            .into_iter()
            .filter_map(value_as_string)
            .collect(),
        _ => Vec::new(),
    }
}

#[async_trait]
impl Retriever for HybridRetriever {
    async fn retrieve(
        &self,
        session_id: &str,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<RetrievedDocument>> {
        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .context("semaphore closed unexpectedly")?;

        let query_owned = query.to_string();
        let dense_model = self.dense_model.clone();
        let query_embedding = tokio::task::spawn_blocking({
            let query_for_embed = query_owned.clone();
            move || -> anyhow::Result<Vec<f32>> {
                let mut model = dense_model
                    .lock()
                    .map_err(|_| anyhow!("embedding model poisoned"))?;
                let embeddings = model
                    .embed(vec![query_for_embed], Some(1))
                    .map_err(|err| anyhow!("failed to embed query: {err}"))?;
                embeddings
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow!("embedding model returned empty result"))
            }
        })
        .await??;

        let filter = Filter::all([Condition::matches(KEY_SESSION, session_id.to_string())]);

        let search = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.collection, query_embedding.clone(), limit as u64)
                    .filter(filter)
                    .with_payload(true),
            )
            .await
            .map_err(|err| anyhow!("qdrant search failed: {err}"))?;

        let query_tokens: HashSet<String> = tokenize(&query_owned).into_iter().collect();

        let mut documents: Vec<RetrievedDocument> = search
            .result
            .into_iter()
            .map(|point| {
                let payload = Payload::from(point.payload.clone());
                let (text, source, keywords) = payload_from_scored(payload);
                let lexical = lexical_boost(&query_tokens, &keywords);
                RetrievedDocument {
                    text,
                    score: point.score as f32 + lexical,
                    source,
                }
            })
            .collect();

        documents.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        documents.truncate(limit);

        if documents.is_empty() {
            warn!(
                %session_id,
                "qdrant returned no hits for query; providing placeholder response"
            );
            return Ok(vec![RetrievedDocument {
                text: "No indexed documents matched the query yet; consider ingesting supporting material."
                    .to_string(),
                score: 0.0,
                source: None,
            }]);
        }

        Ok(documents)
    }

    async fn ingest(&self, session_id: &str, docs: Vec<IngestDocument>) -> anyhow::Result<()> {
        if docs.is_empty() {
            return Ok(());
        }

        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .context("semaphore closed unexpectedly")?;

        let texts: Vec<String> = docs.iter().map(|doc| doc.text.clone()).collect();
        let dense_model = self.dense_model.clone();

        let embeddings = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<Vec<f32>>> {
            let mut model = dense_model
                .lock()
                .map_err(|_| anyhow!("embedding model poisoned"))?;
            model
                .embed(texts, Some(32))
                .map_err(|err| anyhow!("failed to embed documents: {err}"))
        })
        .await??;

        let mut points = Vec::with_capacity(docs.len());

        for (doc, vector) in docs.iter().zip(embeddings.into_iter()) {
            let keywords = tokenize(&doc.text);
            let payload = build_payload(session_id, &doc.text, doc.source.as_ref(), keywords)?;
            points.push(PointStruct::new(doc.id.clone(), vector, payload));
        }

        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection, points).wait(true))
            .await
            .map_err(|err| anyhow!("failed to upsert documents into qdrant: {err}"))?;

        debug!(session_id, count = %docs.len(), "ingested documents into qdrant");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn tokenize_deduplicates_keywords() {
        let tokens = tokenize("Rust enables resilient Rust research agents, rust!");
        assert!(tokens.contains(&"rust".to_string()));
        assert!(tokens.contains(&"enables".to_string()));
        assert_eq!(tokens.len(), tokens.iter().collect::<HashSet<_>>().len());
        assert!(tokens.iter().all(|token| token.len() >= MIN_KEYWORD_LEN));
    }

    #[test]
    fn lexical_boost_returns_overlap_ratio() {
        let query_tokens = HashSet::from([String::from("rust"), String::from("research")]);
        let score = lexical_boost(
            &query_tokens,
            &[String::from("rust"), String::from("agent")],
        );
        assert!(score > 0.0);

        let zero = lexical_boost(&query_tokens, &[String::from("python")]);
        assert_eq!(zero, 0.0);
    }
}
