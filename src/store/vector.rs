use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::OnyxDatabase;
use crate::error::{OnyxError, OnyxResult};

// ---------------------------------------------------------------------------
// VectorStore trait: interface for semantic similarity search
// ---------------------------------------------------------------------------

/// Trait for vector storage and similarity search backends.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Insert an embedding for a given node ID.
    async fn insert(&self, id: Uuid, embedding: Vec<f32>) -> OnyxResult<()>;

    /// Search for the k nearest neighbors to a query embedding.
    /// Returns (node_id, similarity_score) pairs sorted by descending similarity.
    async fn search(&self, query: &[f32], k: usize) -> OnyxResult<Vec<(Uuid, f32)>>;

    /// Delete an embedding by node ID.
    async fn delete(&self, id: &Uuid) -> OnyxResult<()>;

    /// Update an existing embedding.
    async fn update(&self, id: Uuid, embedding: Vec<f32>) -> OnyxResult<()>;

    /// Get the embedding for a specific node ID.
    async fn get(&self, id: &Uuid) -> OnyxResult<Option<Vec<f32>>>;

    /// Return the number of stored embeddings.
    async fn len(&self) -> usize;

    /// Check if the store is empty.
    async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Get all embedding IDs in the store.
    async fn get_all_embedding_ids(&self) -> OnyxResult<Vec<Uuid>>;

    /// Insert an embedding object.
    async fn insert_embedding(&self, embedding: crate::model::embedding::Embedding) -> OnyxResult<()>;

    /// Get an embedding object by ID.
    async fn get_embedding(&self, id: &Uuid) -> OnyxResult<Option<crate::model::embedding::Embedding>>;

    /// Get all embedding IDs in the store.
    async fn get_all_embedding_ids(&self) -> OnyxResult<Vec<Uuid>>;

    /// Insert an embedding object.
    async fn insert_embedding(&self, embedding: crate::model::embedding::Embedding) -> OnyxResult<()>;

    /// Get an embedding object by ID.
    async fn get_embedding(&self, id: &Uuid) -> OnyxResult<Option<crate::model::embedding::Embedding>>;
}

// ---------------------------------------------------------------------------
// SurrealDB Vector Store
// ---------------------------------------------------------------------------

/// A SurrealDB-backed vector store using native vector indexing and similarity search.
#[derive(Clone)]
pub struct SurrealVectorStore {
    db: Arc<OnyxDatabase>,
    dimensions: Option<usize>,
}

/// Record structure for storing embeddings in SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmbeddingRecord {
    #[serde(rename = "id")]
    record_id: String,
    node_id: String,
    #[serde(with = "vector_f32_serde")]
    vector: Vec<f32>,
    dimensions: usize,
}

impl SurrealVectorStore {
    /// Create a new SurrealDB vector store.
    pub fn new(db: Arc<OnyxDatabase>) -> Self {
        Self {
            db,
            dimensions: None,
        }
    }

    /// Create a new store with a fixed expected dimensionality.
    pub fn with_dimensions(db: Arc<OnyxDatabase>, dimensions: usize) -> Self {
        Self {
            db,
            dimensions: Some(dimensions),
        }
    }

    /// Initialize vector-specific indexes. Call this after creating the store.
    pub async fn init_indexes(&self, dimensions: usize) -> OnyxResult<()> {
        // Create a vector index using MTREE for similarity search
        let query = format!(
            "DEFINE INDEX IF NOT EXISTS embedding_vector ON embedding FIELDS vector MTREE DIMENSION {}",
            dimensions
        );
        
        self.db.query(&query).await.map_err(|e| {
            OnyxError::Internal(format!("Failed to create vector index: {}", e))
        })?;

        Ok(())
    }

    /// Compute cosine similarity between two vectors.
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
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
impl VectorStore for SurrealVectorStore {
    async fn insert(&self, id: Uuid, embedding: Vec<f32>) -> OnyxResult<()> {
        // Validate dimensions
        match self.dimensions {
            Some(d) if d != embedding.len() => {
                return Err(OnyxError::DimensionMismatch {
                    expected: d,
                    got: embedding.len(),
                });
            }
            None => {}
            _ => {}
        }

        let record = EmbeddingRecord {
            record_id: id.to_string(),
            node_id: id.to_string(),
            dimensions: embedding.len(),
            vector: embedding,
        };

        self.db
            .create_with_id("embedding", id.to_string(), record)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to insert embedding: {}", e)))?;

        Ok(())
    }

    async fn search(&self, query: &[f32], k: usize) -> OnyxResult<Vec<(Uuid, f32)>> {
        if let Some(d) = self.dimensions {
            if query.len() != d {
                return Err(OnyxError::DimensionMismatch {
                    expected: d,
                    got: query.len(),
                });
            }
        }

        // Use SurrealDB's vector similarity search
        // The vector<->vector operator computes Euclidean distance
        // We'll convert to cosine similarity
        let query_str = format!(
            "SELECT node_id, vector FROM embedding ORDER BY vector <|-> {:?} LIMIT {}",
            query, k
        );

        let mut response = self.db.query(query_str).await.map_err(|e| {
            OnyxError::Internal(format!("Vector search query failed: {}", e))
        })?;

        let records: Vec<EmbeddingRecord> = response.take(0).map_err(|e| {
            OnyxError::Internal(format!("Failed to parse search results: {}", e))
        })?;

        // Compute cosine similarity for the results
        let results: Vec<(Uuid, f32)> = records
            .into_iter()
            .map(|record| {
                let similarity = Self::cosine_similarity(query, &record.vector);
                let node_id = Uuid::parse_str(&record.node_id).unwrap_or_default();
                (node_id, similarity)
            })
            .collect();

        Ok(results)
    }

    async fn delete(&self, id: &Uuid) -> OnyxResult<()> {
        self.db
            .delete("embedding", &id.to_string())
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to delete embedding: {}", e)))?;
        Ok(())
    }

    async fn update(&self, id: Uuid, embedding: Vec<f32>) -> OnyxResult<()> {
        // Check if exists first
        let exists: Option<EmbeddingRecord> = self
            .db
            .select("embedding", id.to_string())
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to check embedding: {}", e)))?;

        if exists.is_none() {
            return Err(OnyxError::NodeNotFound(id));
        }

        let record = EmbeddingRecord {
            record_id: id.to_string(),
            node_id: id.to_string(),
            dimensions: embedding.len(),
            vector: embedding,
        };

        self.db
            .update("embedding", id.to_string(), record)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to update embedding: {}", e)))?;

        Ok(())
    }

    async fn get(&self, id: &Uuid) -> OnyxResult<Option<Vec<f32>>> {
        let record: Option<EmbeddingRecord> = self
            .db
            .select("embedding", id.to_string())
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to get embedding: {}", e)))?;

        Ok(record.map(|r| r.vector))
    }

    async fn len(&self) -> usize {
        match self.db.query("SELECT count() FROM embedding GROUP BY count").await {
            Ok(mut response) => {
                let count: Option<i64> = response.take(0).ok().flatten();
                count.unwrap_or(0) as usize
            }
            Err(_) => 0,
        }
    }

    async fn get_all_embedding_ids(&self) -> OnyxResult<Vec<Uuid>> {
        let query = "SELECT record_id FROM embedding";
        let mut response = self.db.query(query).await
            .map_err(|e| OnyxError::Internal(format!("Failed to query embedding IDs: {}", e)))?;
        
        let records: Vec<serde_json::Value> = response.take(0).unwrap_or_default();
        let mut ids = Vec::new();
        
        for record in records {
            if let Some(id_str) = record.get("record_id").and_then(|v| v.as_str()) {
                if let Ok(id) = Uuid::parse_str(id_str) {
                    ids.push(id);
                }
            }
        }
        
        Ok(ids)
    }

    async fn insert_embedding(&self, embedding: crate::model::embedding::Embedding) -> OnyxResult<()> {
        self.insert(embedding.node_id, embedding.values).await
    }

    async fn get_embedding(&self, id: &Uuid) -> OnyxResult<Option<crate::model::embedding::Embedding>> {
        let vector = self.get(id).await?;
        Ok(vector.map(|v| crate::model::embedding::Embedding {
            node_id: *id,
            values: v,
        }))
    }
}

// ---------------------------------------------------------------------------
// InMemoryVectorStore: for testing and fallback
// ---------------------------------------------------------------------------

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use tokio::sync::RwLock;

/// In-memory vector store using brute-force cosine similarity search.
pub struct InMemoryVectorStore {
    embeddings: RwLock<HashMap<Uuid, Vec<f32>>>,
    dimensions: Option<usize>,
}

impl InMemoryVectorStore {
    pub fn new() -> Self {
        Self {
            embeddings: RwLock::new(HashMap::new()),
            dimensions: None,
        }
    }

    pub fn with_dimensions(dimensions: usize) -> Self {
        Self {
            embeddings: RwLock::new(HashMap::new()),
            dimensions: Some(dimensions),
        }
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
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

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn insert(&self, id: Uuid, embedding: Vec<f32>) -> OnyxResult<()> {
        match self.dimensions {
            Some(d) if d != embedding.len() => {
                return Err(OnyxError::DimensionMismatch {
                    expected: d,
                    got: embedding.len(),
                });
            }
            None => {}
            _ => {}
        }

        let mut embeddings = self.embeddings.write().map_err(|_| {
            OnyxError::Internal("Failed to acquire write lock".to_string())
        })?;
        embeddings.insert(id, embedding);
        Ok(())
    }

    async fn search(&self, query: &[f32], k: usize) -> OnyxResult<Vec<(Uuid, f32)>> {
        if let Some(d) = self.dimensions {
            if query.len() != d {
                return Err(OnyxError::DimensionMismatch {
                    expected: d,
                    got: query.len(),
                });
            }
        }

        let embeddings = self.embeddings.read().map_err(|_| {
            OnyxError::Internal("Failed to acquire read lock".to_string())
        })?;

        let mut heap: BinaryHeap<ScoredItem> = BinaryHeap::new();

        for (id, embedding) in embeddings.iter() {
            let score = Self::cosine_similarity(query, embedding);
            let item = ScoredItem { id: *id, score };

            if heap.len() < k {
                heap.push(item);
            } else if let Some(min) = heap.peek() {
                if score > min.score {
                    heap.pop();
                    heap.push(item);
                }
            }
        }

        let mut results: Vec<(Uuid, f32)> =
            heap.into_iter().map(|item| (item.id, item.score)).collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

        Ok(results)
    }

    async fn delete(&self, id: &Uuid) -> OnyxResult<()> {
        let mut embeddings = self.embeddings.write().map_err(|_| {
            OnyxError::Internal("Failed to acquire write lock".to_string())
        })?;
        embeddings.remove(id);
        Ok(())
    }

    async fn update(&self, id: Uuid, embedding: Vec<f32>) -> OnyxResult<()> {
        let mut embeddings = self.embeddings.write().map_err(|_| {
            OnyxError::Internal("Failed to acquire write lock".to_string())
        })?;
        
        if !embeddings.contains_key(&id) {
            return Err(OnyxError::NodeNotFound(id));
        }
        embeddings.insert(id, embedding);
        Ok(())
    }

    async fn get(&self, id: &Uuid) -> OnyxResult<Option<Vec<f32>>> {
        let embeddings = self.embeddings.read().map_err(|_| {
            OnyxError::Internal("Failed to acquire read lock".to_string())
        })?;
        Ok(embeddings.get(id).cloned())
    }

    async fn len(&self) -> usize {
        let embeddings = self.embeddings.read().unwrap();
        embeddings.len()
    }
}

#[derive(Debug, Clone)]
struct ScoredItem {
    id: Uuid,
    score: f32,
}

impl PartialEq for ScoredItem {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for ScoredItem {}

impl PartialOrd for ScoredItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoredItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other.score.partial_cmp(&self.score).unwrap_or(Ordering::Equal)
    }
}

// ---------------------------------------------------------------------------
// Vector serialization helper
// ---------------------------------------------------------------------------

mod vector_f32_serde {
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<f32>, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_none()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<f32>, D::Error> {
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_insert_and_search() {
        let store = InMemoryVectorStore::new();
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let id_c = Uuid::new_v4();
        
        store.insert(id_a, vec![1.0, 0.0, 0.0]).await.unwrap();
        store.insert(id_b, vec![0.0, 1.0, 0.0]).await.unwrap();
        store.insert(id_c, vec![0.9, 0.1, 0.0]).await.unwrap();

        let results = store.search(&[1.0, 0.0, 0.0], 2).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!((results[0].1 - 1.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_in_memory_dimension_mismatch() {
        let store = InMemoryVectorStore::new();
        store.insert(Uuid::new_v4(), vec![1.0, 2.0]).await.unwrap();
        let result = store.insert(Uuid::new_v4(), vec![1.0, 2.0, 3.0]).await;
        assert!(result.is_err());
    }
}
