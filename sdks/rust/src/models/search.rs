//! Search models — vector similarity queries.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::edge::EdgeType;

/// Request body for a semantic search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// The query embedding vector.
    pub embedding: Vec<f32>,
    /// Number of results to return (default: 10).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<usize>,
    /// Maximum graph traversal depth (default: 2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<usize>,
    /// Edge types to follow during traversal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_types: Option<Vec<EdgeType>>,
    /// Whether to include version history in results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_history: Option<bool>,
    /// Minimum confidence score for traversed edges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_confidence: Option<f64>,
}

impl SearchRequest {
    /// Create a new search request with a query embedding.
    pub fn new(embedding: Vec<f32>) -> Self {
        Self {
            embedding,
            top_k: None,
            max_depth: None,
            edge_types: None,
            include_history: None,
            min_confidence: None,
        }
    }

    /// Set the number of results to return.
    pub fn top_k(mut self, k: usize) -> Self {
        self.top_k = Some(k);
        self
    }

    /// Set the maximum graph traversal depth.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Set edge types to follow.
    pub fn edge_types(mut self, types: Vec<EdgeType>) -> Self {
        self.edge_types = Some(types);
        self
    }

    /// Include version history.
    pub fn include_history(mut self, include: bool) -> Self {
        self.include_history = Some(include);
        self
    }

    /// Set minimum confidence.
    pub fn min_confidence(mut self, confidence: f64) -> Self {
        self.min_confidence = Some(confidence);
        self
    }
}

/// A single search result item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub node_id: Uuid,
    pub name: String,
    pub content: String,
    pub source: ResultSource,
    pub score: f64,
    pub depth: usize,
    pub edge_path: Vec<EdgeType>,
    pub versions: Vec<super::version::VersionInfo>,
}

/// How a result was discovered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultSource {
    VectorSearch,
    GraphTraversal,
    Combined,
}

/// Complete search response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub items: Vec<SearchResultItem>,
    pub nodes_examined: usize,
    pub query_time_ms: u64,
}
