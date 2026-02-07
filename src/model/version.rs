use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Version: temporal versioning with diffs and branching
// ---------------------------------------------------------------------------

/// A version identifier, unique within the history store.
pub type VersionId = String;

/// Generate a new version ID.
pub fn new_version_id() -> VersionId {
    format!("v-{}", Uuid::new_v4().as_simple())
}

/// A single version entry in an entity's history chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    /// Unique version identifier.
    pub version_id: VersionId,
    /// The node this version belongs to.
    pub entity_id: Uuid,
    /// Previous version in the chain (None for the initial version).
    pub parent_version: Option<VersionId>,
    /// Branch name (default: "main").
    pub branch: String,
    /// The diff from the parent version.
    pub diff: Diff,
    /// Associated git commit hash.
    pub commit_id: Option<String>,
    /// Who made the change.
    pub author: Option<String>,
    /// Change description.
    pub message: Option<String>,
    /// When the version was recorded.
    pub timestamp: DateTime<Utc>,
}

impl VersionEntry {
    /// Create a new initial version (full content, no parent).
    pub fn initial(entity_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            version_id: new_version_id(),
            entity_id,
            parent_version: None,
            branch: "main".to_string(),
            diff: Diff::Initial {
                content: content.into(),
            },
            commit_id: None,
            author: None,
            message: Some("Initial version".to_string()),
            timestamp: Utc::now(),
        }
    }

    /// Create a new version with a content diff from the parent.
    pub fn content_change(
        entity_id: Uuid,
        parent_version: VersionId,
        patch: impl Into<String>,
        additions: usize,
        deletions: usize,
    ) -> Self {
        Self {
            version_id: new_version_id(),
            entity_id,
            parent_version: Some(parent_version),
            branch: "main".to_string(),
            diff: Diff::ContentChanged {
                patch: patch.into(),
                additions,
                deletions,
            },
            commit_id: None,
            author: None,
            message: None,
            timestamp: Utc::now(),
        }
    }

    /// Set the commit ID for this version.
    pub fn with_commit(mut self, commit: impl Into<String>) -> Self {
        self.commit_id = Some(commit.into());
        self
    }

    /// Set the author for this version.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set the message for this version.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set the branch for this version.
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = branch.into();
        self
    }
}

// ---------------------------------------------------------------------------
// Diff: represents changes between versions
// ---------------------------------------------------------------------------

/// A diff between two versions of a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Diff {
    /// Initial version: stores the full content.
    Initial { content: String },

    /// Content changed: stores a unified diff patch.
    ContentChanged {
        /// Unified diff format string.
        patch: String,
        /// Number of lines added.
        additions: usize,
        /// Number of lines removed.
        deletions: usize,
    },

    /// Metadata fields changed.
    MetadataChanged {
        /// Map of field name -> (old_value, new_value).
        changed_fields: HashMap<String, (String, String)>,
    },

    /// Multiple changes in one version.
    Composite(Vec<Diff>),
}

impl Diff {
    /// Returns true if this is the initial version diff.
    pub fn is_initial(&self) -> bool {
        matches!(self, Diff::Initial { .. })
    }

    /// Returns the number of lines changed (additions + deletions).
    pub fn lines_changed(&self) -> usize {
        match self {
            Diff::Initial { content } => content.lines().count(),
            Diff::ContentChanged {
                additions,
                deletions,
                ..
            } => additions + deletions,
            Diff::MetadataChanged { changed_fields } => changed_fields.len(),
            Diff::Composite(diffs) => diffs.iter().map(|d| d.lines_changed()).sum(),
        }
    }
}

// ---------------------------------------------------------------------------
// Branch: named version streams
// ---------------------------------------------------------------------------

/// Metadata for a named branch in the history store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    /// Branch name (e.g., "main", "feature/new-api").
    pub name: String,
    /// Latest version on this branch.
    pub head: VersionId,
    /// Version where this branch was created (fork point).
    pub base: VersionId,
    /// When the branch was created.
    pub created_at: DateTime<Utc>,
    /// If this branch was merged, which branch it merged into.
    pub merged_into: Option<String>,
}

impl Branch {
    /// Create a new branch forking from a given version.
    pub fn new(name: impl Into<String>, base_version: VersionId) -> Self {
        Self {
            name: name.into(),
            head: base_version.clone(),
            base: base_version,
            created_at: Utc::now(),
            merged_into: None,
        }
    }
}
