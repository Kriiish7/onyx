use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::model::version::VersionId;

// ---------------------------------------------------------------------------
// Edge: relationships between nodes in the knowledge graph
// ---------------------------------------------------------------------------

/// A directed edge connecting two nodes in the Onyx knowledge graph.
/// Edges carry type, confidence, metadata, and temporal validity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Globally unique edge identifier.
    pub id: Uuid,
    /// The relationship type.
    pub edge_type: EdgeType,
    /// Source node ID (edge goes FROM this node).
    pub source_id: Uuid,
    /// Target node ID (edge goes TO this node).
    pub target_id: Uuid,
    /// Confidence score in [0.0, 1.0]. 1.0 = statically determined.
    pub confidence: f64,
    /// Extensible key-value metadata.
    pub metadata: HashMap<String, String>,
    /// Temporal context: when this relationship was valid.
    pub temporal: TemporalContext,
}

impl Edge {
    /// Create a new edge with full confidence and no temporal bounds.
    pub fn new(edge_type: EdgeType, source_id: Uuid, target_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            edge_type,
            source_id,
            target_id,
            confidence: 1.0,
            metadata: HashMap::new(),
            temporal: TemporalContext::new_active(),
        }
    }

    /// Set the confidence score for this edge.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Add metadata to this edge.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set temporal context via a commit.
    pub fn with_commit(mut self, commit: impl Into<String>) -> Self {
        self.temporal.via_commit = Some(commit.into());
        self
    }

    /// Check if this edge is currently active (not yet terminated).
    pub fn is_active(&self) -> bool {
        self.temporal.until.is_none()
    }

    /// Terminate this edge at a given version and timestamp.
    pub fn terminate(&mut self, version: VersionId) {
        self.temporal.until = Some(version);
        self.temporal.until_timestamp = Some(Utc::now());
    }
}

// ---------------------------------------------------------------------------
// EdgeType: categories of relationships
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    /// A code entity defines another (e.g., impl block defines methods).
    Defines,
    /// A function calls another function.
    Calls,
    /// A module imports another module or entity.
    Imports,
    /// Documentation describes a code entity.
    Documents,
    /// A test covers a code entity.
    TestsOf,
    /// An entity is versioned by a history entry.
    VersionedBy,
    /// A module contains sub-entities.
    Contains,
    /// A struct/enum implements a trait.
    Implements,
    /// An entity depends on another entity (generic dependency).
    DependsOn,
    /// A config file configures a code entity or module.
    Configures,
}

impl EdgeType {
    /// Returns the inverse relationship type, if one exists.
    pub fn inverse(&self) -> Option<EdgeType> {
        match self {
            EdgeType::Calls => Some(EdgeType::Calls),      // called_by
            EdgeType::Imports => Some(EdgeType::Imports),  // imported_by
            EdgeType::Defines => Some(EdgeType::Contains), // defined_in
            EdgeType::Contains => Some(EdgeType::Defines), // contained_by
            EdgeType::Documents => Some(EdgeType::Documents), // documented_by
            EdgeType::TestsOf => Some(EdgeType::TestsOf),  // tested_by
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// TemporalContext: tracks when a relationship was valid
// ---------------------------------------------------------------------------

/// Temporal metadata for an edge, tracking when the relationship existed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalContext {
    /// Version when the edge was created.
    pub since: Option<VersionId>,
    /// Version when the edge was removed (None = still active).
    pub until: Option<VersionId>,
    /// Commit that introduced this relationship.
    pub via_commit: Option<String>,
    /// Timestamp of creation.
    pub since_timestamp: DateTime<Utc>,
    /// Timestamp of removal (None = still active).
    pub until_timestamp: Option<DateTime<Utc>>,
}

impl TemporalContext {
    /// Create a new active temporal context (created now, not yet terminated).
    pub fn new_active() -> Self {
        Self {
            since: None,
            until: None,
            via_commit: None,
            since_timestamp: Utc::now(),
            until_timestamp: None,
        }
    }

    /// Check if this context is valid at a given timestamp.
    pub fn is_valid_at(&self, timestamp: &DateTime<Utc>) -> bool {
        if *timestamp < self.since_timestamp {
            return false;
        }
        match &self.until_timestamp {
            Some(until) => *timestamp <= *until,
            None => true,
        }
    }
}
