//! Version / history models — temporal versioning and branching.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A single version entry in an entity's history chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    pub version_id: String,
    pub entity_id: Uuid,
    pub parent_version: Option<String>,
    pub branch: String,
    pub diff: Diff,
    pub commit_id: Option<String>,
    pub author: Option<String>,
    pub message: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// A diff between two versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Diff {
    Initial {
        content: String,
    },
    ContentChanged {
        patch: String,
        additions: usize,
        deletions: usize,
    },
    MetadataChanged {
        changed_fields: HashMap<String, (String, String)>,
    },
    Composite(Vec<Diff>),
}

/// Summary of a version for display in query results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version_id: String,
    pub timestamp: DateTime<Utc>,
    pub message: Option<String>,
    pub author: Option<String>,
    pub lines_changed: usize,
}

/// A named branch in the history store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub head: String,
    pub base: String,
    pub created_at: DateTime<Utc>,
    pub merged_into: Option<String>,
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

/// Request to record a new version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVersionRequest {
    pub entity_id: Uuid,
    pub diff: Diff,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Request to create a branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBranchRequest {
    pub name: String,
    pub base_version: String,
}

/// Request to merge branches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeBranchRequest {
    pub source: String,
    pub target: String,
}

/// Response for listing versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListVersionsResponse {
    pub versions: Vec<VersionEntry>,
    pub total: usize,
}

/// Response for listing branches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBranchesResponse {
    pub branches: Vec<Branch>,
}
