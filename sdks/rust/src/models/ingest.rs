//! Ingestion models — code unit ingestion pipeline.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::node::{CodeEntityKind, Language, Visibility};

/// A code unit to ingest into Onyx.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestCodeUnitRequest {
    pub name: String,
    pub content: String,
    pub kind: CodeEntityKind,
    pub language: Language,
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_range: Option<(usize, usize)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

impl IngestCodeUnitRequest {
    /// Create a new ingestion request.
    pub fn new(
        name: impl Into<String>,
        content: impl Into<String>,
        kind: CodeEntityKind,
        language: Language,
        file_path: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            kind,
            language,
            file_path: file_path.into(),
            line_range: None,
            signature: None,
            visibility: None,
            module_path: None,
            commit_id: None,
            branch: None,
        }
    }

    pub fn line_range(mut self, start: usize, end: usize) -> Self {
        self.line_range = Some((start, end));
        self
    }

    pub fn signature(mut self, sig: impl Into<String>) -> Self {
        self.signature = Some(sig.into());
        self
    }

    pub fn visibility(mut self, vis: Visibility) -> Self {
        self.visibility = Some(vis);
        self
    }

    pub fn module_path(mut self, path: Vec<String>) -> Self {
        self.module_path = Some(path);
        self
    }

    pub fn commit_id(mut self, commit: impl Into<String>) -> Self {
        self.commit_id = Some(commit.into());
        self
    }

    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }
}

/// Batch ingestion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestCodebaseRequest {
    pub units: Vec<IngestCodeUnitRequest>,
}

/// Result of ingesting a single code unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub node_id: Uuid,
    pub version_id: String,
    pub edges_created: usize,
}

/// Result of batch ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestCodebaseResponse {
    pub results: Vec<IngestResult>,
    pub total_edges: usize,
}
