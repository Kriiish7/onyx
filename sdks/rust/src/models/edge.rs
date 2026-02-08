//! Edge models — knowledge graph relationships.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A directed edge connecting two nodes in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: Uuid,
    pub edge_type: EdgeType,
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub confidence: f64,
    pub metadata: HashMap<String, String>,
    pub temporal: TemporalContext,
}

/// Relationship categories.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    Defines,
    Calls,
    Imports,
    Documents,
    TestsOf,
    VersionedBy,
    Contains,
    Implements,
    DependsOn,
    Configures,
}

/// Temporal metadata tracking when a relationship was valid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalContext {
    pub since: Option<String>,
    pub until: Option<String>,
    pub via_commit: Option<String>,
    pub since_timestamp: DateTime<Utc>,
    pub until_timestamp: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

/// Request body for creating an edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEdgeRequest {
    pub edge_type: EdgeType,
    pub source_id: Uuid,
    pub target_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl CreateEdgeRequest {
    /// Create a new edge request.
    pub fn new(edge_type: EdgeType, source_id: Uuid, target_id: Uuid) -> Self {
        Self {
            edge_type,
            source_id,
            target_id,
            confidence: None,
            metadata: None,
        }
    }

    /// Set the confidence score.
    pub fn confidence(mut self, confidence: f64) -> Self {
        self.confidence = Some(confidence);
        self
    }

    /// Set metadata.
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Response for listing edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListEdgesResponse {
    pub edges: Vec<Edge>,
    pub total: usize,
}

/// A neighbor result from graph traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborResult {
    pub edge: Edge,
    pub node: super::node::Node,
}

/// Result of a multi-hop graph traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    pub nodes: Vec<(Uuid, usize)>,
    pub edges_followed: usize,
}

/// Result of a subgraph extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphResult {
    pub nodes: Vec<super::node::Node>,
    pub edges: Vec<Edge>,
}
