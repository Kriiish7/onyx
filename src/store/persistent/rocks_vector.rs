//! RocksDB-backed vector store with HNSW index.

use async_trait::async_trait;
use rocksdb::DB;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{OnyxError, OnyxResult};
use crate::model::embedding::Embedding;
use crate::store::vector::VectorStore;

use super::{CF_EMBEDDINGS, CF_HNSW_LAYERS};

/// RocksDB-backed vector store with HNSW indexing for fast similarity search.
#[derive(Clone)]
pub struct RocksVectorStore {
    db: Arc<DB>,
    dimension: usize,
}

impl RocksVectorStore {
    /// Create a new RocksDB vector store.
    pub fn new(db: Arc<DB>, dimension: usize) -> Self {
        Self { db, dimension }
    }

    /// Serialize an embedding to bytes.
    fn serialize_embedding(&self, embedding: &Embedding) -> OnyxResult<Vec<u8>> {
        bincode::serialize(embedding)
            .map_err(|e| OnyxError::Internal(format!("Failed to serialize embedding: {}", e)))
    }

    /// Deserialize an embedding from bytes.
    fn deserialize_embedding(&self, bytes: &[u8]) -> OnyxResult<Embedding> {
        bincode::deserialize(bytes)
            .map_err(|e| OnyxError::Internal(format!("Failed to deserialize embedding: {}", e)))
    }

    /// Get the embeddings column family handle.
    fn cf_embeddings(&self) -> OnyxResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_EMBEDDINGS)
            .ok_or_else(|| OnyxError::Internal("Missing embeddings column family".to_string()))
    }

    /// Calculate cosine similarity between two vectors.
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }
}

#[async_trait]
impl VectorStore for RocksVectorStore {
    async fn add_embedding(&self, embedding: Embedding) -> OnyxResult<()> {
        let cf = self.cf_embeddings()?;
        let key = embedding.node_id.as_bytes();
        let value = self.serialize_embedding(&embedding)?;

        self.db
            .put_cf(cf, key, value)
            .map_err(|e| OnyxError::Internal(format!("Failed to add embedding: {}", e)))?;

        Ok(())
    }

    async fn get_embedding(&self, node_id: &Uuid) -> OnyxResult<Option<Embedding>> {
        let cf = self.cf_embeddings()?;
        let key = node_id.as_bytes();

        match self.db.get_cf(cf, key) {
            Ok(Some(bytes)) => Ok(Some(self.deserialize_embedding(&bytes)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(OnyxError::Internal(format!("Failed to get embedding: {}", e))),
        }
    }

    async fn remove_embedding(&self, node_id: &Uuid) -> OnyxResult<()> {
        let cf = self.cf_embeddings()?;
        let key = node_id.as_bytes();

        self.db
            .delete_cf(cf, key)
            .map_err(|e| OnyxError::Internal(format!("Failed to remove embedding: {}", e)))?;

        Ok(())
    }

    async fn search(&self, query: &[f32], top_k: usize) -> OnyxResult<Vec<(Uuid, f32)>> {
        // TODO: Implement HNSW index for production performance
        // For now, use brute-force linear search as a working baseline

        let cf = self.cf_embeddings()?;
        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);

        let mut results = Vec::new();

        for item in iter {
            let (key, value) = item
                .map_err(|e| OnyxError::Internal(format!("Failed to iterate embeddings: {}", e)))?;

            let node_id = Uuid::from_slice(&key)
                .map_err(|e| OnyxError::Internal(format!("Invalid node UUID: {}", e)))?;

            let embedding = self.deserialize_embedding(&value)?;
            let similarity = self.cosine_similarity(query, &embedding.vector);

            results.push((node_id, similarity));
        }

        // Sort by similarity (descending) and take top_k
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);

        Ok(results)
    }

    async fn batch_add(&self, embeddings: Vec<Embedding>) -> OnyxResult<()> {
        for embedding in embeddings {
            self.add_embedding(embedding).await?;
        }
        Ok(())
    }

    async fn embedding_count(&self) -> usize {
        let cf = match self.cf_embeddings() {
            Ok(cf) => cf,
            Err(_) => return 0,
        };

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        iter.count()
    }

    async fn all_embeddings(&self) -> Vec<Embedding> {
        let cf = match self.cf_embeddings() {
            Ok(cf) => cf,
            Err(_) => return vec![],
        };

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        let mut embeddings = Vec::new();

        for item in iter {
            if let Ok((_, value)) = item {
                if let Ok(embedding) = self.deserialize_embedding(&value) {
                    embeddings.push(embedding);
                }
            }
        }

        embeddings
    }

    async fn get_all_embedding_ids(&self) -> OnyxResult<Vec<Uuid>> {
        let cf = self.cf_embeddings()?;
        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        let mut ids = Vec::new();

        for item in iter {
            if let Ok((key, _)) = item {
                if let Ok(id_str) = std::str::from_utf8(&key) {
                    if let Ok(id) = Uuid::parse_str(id_str) {
                        ids.push(id);
                    }
                }
            }
        }

        Ok(ids)
    }

    async fn insert_embedding(&self, embedding: Embedding) -> OnyxResult<()> {
        self.insert(embedding.node_id, embedding.values).await
    }

    async fn get_embedding(&self, id: &Uuid) -> OnyxResult<Option<Embedding>> {
        let vector = self.get(id).await?;
        Ok(vector.map(|v| Embedding {
            node_id: *id,
            values: v,
        }))
    }
}

// TODO: Production HNSW implementation
//
// The production implementation should:
// 1. Build HNSW layers on add_embedding (using hnsw crate or custom implementation)
// 2. Persist HNSW graph structure in CF_HNSW_LAYERS
// 3. Use HNSW for search() instead of brute-force linear scan
// 4. Support incremental index updates
// 5. Optimize with SIMD for vector operations (e.g., using simdeez or packed_simd)
//
// For initial testing and prototyping, the brute-force approach above is sufficient.
