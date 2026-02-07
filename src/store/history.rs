use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::OnyxDatabase;
use crate::error::{OnyxError, OnyxResult};
use crate::model::version::{new_version_id, Branch, Diff, VersionEntry, VersionId};

// ---------------------------------------------------------------------------
// HistoryStore trait: interface for temporal versioning
// ---------------------------------------------------------------------------

/// Trait for history storage backends that manage version chains, branching,
/// and time-travel queries.
#[async_trait]
pub trait HistoryStore: Send + Sync {
    /// Record a new version for an entity.
    async fn record_version(&self, entry: VersionEntry) -> OnyxResult<VersionId>;

    /// Get a specific version entry by version ID.
    async fn get_version(&self, version_id: &VersionId) -> OnyxResult<Option<VersionEntry>>;

    /// Get the content of an entity at a specific version by reconstructing
    /// from the diff chain.
    async fn get_content_at_version(
        &self,
        entity_id: &Uuid,
        version_id: &VersionId,
    ) -> OnyxResult<String>;

    /// Get the content of an entity at a specific timestamp.
    async fn get_content_at_timestamp(
        &self,
        entity_id: &Uuid,
        timestamp: &DateTime<Utc>,
    ) -> OnyxResult<String>;

    /// List all versions for an entity, ordered by timestamp.
    async fn list_versions(&self, entity_id: &Uuid) -> OnyxResult<Vec<VersionEntry>>;

    /// List versions in a range for an entity.
    async fn list_versions_in_range(
        &self,
        entity_id: &Uuid,
        from: &DateTime<Utc>,
        to: &DateTime<Utc>,
    ) -> OnyxResult<Vec<VersionEntry>>;

    /// Get all version IDs in the store.
    async fn get_all_version_ids(&self) -> OnyxResult<Vec<VersionId>>;

    /// Create a version (alias for record_version).
    async fn create_version(&self, entry: VersionEntry) -> OnyxResult<VersionId> {
        self.record_version(entry).await
    }

    /// Get the latest version ID for an entity on a branch.
    async fn get_head(&self, entity_id: &Uuid, branch: &str) -> OnyxResult<Option<VersionId>>;

    /// Create a new branch from a base version.
    async fn create_branch(&self, name: &str, base_version: VersionId) -> OnyxResult<()>;

    /// Get branch metadata.
    async fn get_branch(&self, name: &str) -> OnyxResult<Option<Branch>>;

    /// List all branches.
    async fn list_branches(&self) -> Vec<Branch>;

    /// Merge a source branch into a target branch.
    /// Returns the merge version ID.
    async fn merge_branch(&self, source: &str, target: &str) -> OnyxResult<VersionId>;

    /// Total number of version entries.
    async fn version_count(&self) -> usize;
}

// ---------------------------------------------------------------------------
// SurrealDB History Store
// ---------------------------------------------------------------------------

/// SurrealDB-backed history store for temporal versioning.
#[derive(Clone)]
pub struct SurrealHistoryStore {
    db: Arc<OnyxDatabase>,
}

/// Record structure for storing versions in SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionRecord {
    #[serde(rename = "id")]
    record_id: String,
    entity_id: String,
    version_id: String,
    parent_version: Option<String>,
    branch: String,
    diff: Diff,
    commit_id: Option<String>,
    author: Option<String>,
    message: Option<String>,
    timestamp: DateTime<Utc>,
}

/// Record structure for storing branches in SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BranchRecord {
    #[serde(rename = "id")]
    record_id: String,
    name: String,
    head: String,
    base: String,
    created_at: DateTime<Utc>,
    merged_into: Option<String>,
}

impl SurrealHistoryStore {
    /// Create a new SurrealDB history store.
    pub fn new(db: Arc<OnyxDatabase>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl HistoryStore for SurrealHistoryStore {
    async fn record_version(&self, entry: VersionEntry) -> OnyxResult<VersionId> {
        let version_id = entry.version_id.clone();
        let entity_id = entry.entity_id;
        let branch = entry.branch.clone();

        // Verify parent version exists if specified
        if let Some(ref parent) = entry.parent_version {
            let exists: Option<VersionRecord> = self
                .db
                .select("version", parent.clone())
                .await
                .map_err(|e| {
                    OnyxError::Internal(format!("Failed to check parent version: {}", e))
                })?;

            if exists.is_none() {
                return Err(OnyxError::VersionNotFound(parent.clone()));
            }
        }

        // Create record
        let record = VersionRecord {
            record_id: version_id.clone(),
            entity_id: entity_id.to_string(),
            version_id: version_id.clone(),
            parent_version: entry.parent_version.clone(),
            branch: branch.clone(),
            diff: entry.diff,
            commit_id: entry.commit_id,
            author: entry.author,
            message: entry.message,
            timestamp: entry.timestamp,
        };

        // Store the version entry
        self.db
            .create_with_id("version", &version_id, record)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to record version: {}", e)))?;

        // Update or create branch head record
        let branch_head_id = format!("{}:{}", entity_id, branch);
        let branch_head = serde_json::json!({
            "id": branch_head_id,
            "entity_id": entity_id.to_string(),
            "branch": branch,
            "version_id": version_id.clone(),
            "timestamp": entry.timestamp,
        });

        let _ = self
            .db
            .create_with_id("branch_head", &branch_head_id, branch_head)
            .await;

        Ok(version_id)
    }

    async fn get_version(&self, version_id: &VersionId) -> OnyxResult<Option<VersionEntry>> {
        let record: Option<VersionRecord> = self
            .db
            .select("version", version_id.clone())
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to get version: {}", e)))?;

        Ok(record.map(|r| VersionEntry {
            version_id: r.version_id,
            entity_id: Uuid::parse_str(&r.entity_id).unwrap_or_default(),
            parent_version: r.parent_version,
            branch: r.branch,
            diff: r.diff,
            commit_id: r.commit_id,
            author: r.author,
            message: r.message,
            timestamp: r.timestamp,
        }))
    }

    async fn get_content_at_version(
        &self,
        entity_id: &Uuid,
        version_id: &VersionId,
    ) -> OnyxResult<String> {
        // Walk the version chain from the requested version back to initial
        let mut chain: Vec<VersionEntry> = Vec::new();
        let mut current_id = Some(version_id.clone());

        while let Some(vid) = current_id {
            let entry = self
                .get_version(&vid)
                .await?
                .ok_or_else(|| OnyxError::VersionNotFound(vid.clone()))?;

            if entry.entity_id != *entity_id {
                return Err(OnyxError::Internal(format!(
                    "Version {} belongs to entity {}, not {}",
                    vid, entry.entity_id, entity_id
                )));
            }

            chain.push(entry);
            current_id = chain.last().unwrap().parent_version.clone();
        }

        // Reconstruct content from initial version forward
        chain.reverse();
        let mut content = String::new();

        for entry in &chain {
            match &entry.diff {
                Diff::Initial { content: c } => {
                    content = c.clone();
                }
                Diff::ContentChanged { patch, .. } => {
                    content = patch.clone();
                }
                Diff::MetadataChanged { .. } => {
                    // Content unchanged for metadata-only changes
                }
                Diff::Composite(diffs) => {
                    for diff in diffs {
                        if let Diff::ContentChanged { patch, .. } = diff {
                            content = patch.clone();
                        }
                    }
                }
            }
        }

        Ok(content)
    }

    async fn get_content_at_timestamp(
        &self,
        entity_id: &Uuid,
        timestamp: &DateTime<Utc>,
    ) -> OnyxResult<String> {
        // Query for the latest version at or before the given timestamp
        let query = format!(
            "SELECT * FROM version WHERE entity_id = '{}' AND timestamp <= '{}' ORDER BY timestamp DESC LIMIT 1",
            entity_id, timestamp.to_rfc3339()
        );

        let mut response = self
            .db
            .query(&query)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to query versions: {}", e)))?;

        let records: Vec<VersionRecord> = response
            .take(0)
            .map_err(|e| OnyxError::Internal(format!("Failed to parse versions: {}", e)))?;

        let entry = records.into_iter().next().ok_or_else(|| {
            OnyxError::Internal(format!(
                "No version found for entity {} at timestamp {}",
                entity_id, timestamp
            ))
        })?;

        self.get_content_at_version(entity_id, &entry.version_id)
            .await
    }

    async fn list_versions(&self, entity_id: &Uuid) -> OnyxResult<Vec<VersionEntry>> {
        let query = format!(
            "SELECT * FROM version WHERE entity_id = '{}' ORDER BY timestamp ASC",
            entity_id
        );

        let mut response = self
            .db
            .query(&query)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to list versions: {}", e)))?;

        let records: Vec<VersionRecord> = response
            .take(0)
            .map_err(|e| OnyxError::Internal(format!("Failed to parse versions: {}", e)))?;

        let entries: Vec<VersionEntry> = records
            .into_iter()
            .map(|r| VersionEntry {
                version_id: r.version_id,
                entity_id: Uuid::parse_str(&r.entity_id).unwrap_or_default(),
                parent_version: r.parent_version,
                branch: r.branch,
                diff: r.diff,
                commit_id: r.commit_id,
                author: r.author,
                message: r.message,
                timestamp: r.timestamp,
            })
            .collect();

        Ok(entries)
    }

    async fn list_versions_in_range(
        &self,
        entity_id: &Uuid,
        from: &DateTime<Utc>,
        to: &DateTime<Utc>,
    ) -> OnyxResult<Vec<VersionEntry>> {
        let query = format!(
            "SELECT * FROM version WHERE entity_id = '{}' AND timestamp >= '{}' AND timestamp <= '{}' ORDER BY timestamp ASC",
            entity_id, from.to_rfc3339(), to.to_rfc3339()
        );

        let mut response =
            self.db.query(&query).await.map_err(|e| {
                OnyxError::Internal(format!("Failed to list versions in range: {}", e))
            })?;

        let records: Vec<VersionRecord> = response
            .take(0)
            .map_err(|e| OnyxError::Internal(format!("Failed to parse versions: {}", e)))?;

        let entries: Vec<VersionEntry> = records
            .into_iter()
            .map(|r| VersionEntry {
                version_id: r.version_id,
                entity_id: Uuid::parse_str(&r.entity_id).unwrap_or_default(),
                parent_version: r.parent_version,
                branch: r.branch,
                diff: r.diff,
                commit_id: r.commit_id,
                author: r.author,
                message: r.message,
                timestamp: r.timestamp,
            })
            .collect();

        Ok(entries)
    }

    async fn get_head(&self, entity_id: &Uuid, branch: &str) -> OnyxResult<Option<VersionId>> {
        let branch_head_id = format!("{}:{}", entity_id, branch);

        let record: Option<serde_json::Value> = self
            .db
            .select("branch_head", branch_head_id)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to get branch head: {}", e)))?;

        Ok(record.and_then(|r| {
            r.get("version_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        }))
    }

    async fn create_branch(&self, name: &str, base_version: VersionId) -> OnyxResult<()> {
        // Verify base version exists
        let exists = self.get_version(&base_version).await?;
        if exists.is_none() {
            return Err(OnyxError::VersionNotFound(base_version.clone()));
        }

        // Check if branch already exists
        let existing: Option<BranchRecord> = self
            .db
            .select("branch", name.to_string())
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to check branch: {}", e)))?;

        if existing.is_some() {
            return Err(OnyxError::BranchAlreadyExists(name.to_string()));
        }

        let branch = Branch::new(name, base_version.clone());

        let record = BranchRecord {
            record_id: name.to_string(),
            name: branch.name,
            head: branch.head,
            base: branch.base,
            created_at: branch.created_at,
            merged_into: branch.merged_into,
        };

        self.db
            .create_with_id("branch", name, record)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to create branch: {}", e)))?;

        Ok(())
    }

    async fn get_branch(&self, name: &str) -> OnyxResult<Option<Branch>> {
        let record: Option<BranchRecord> = self
            .db
            .select("branch", name.to_string())
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to get branch: {}", e)))?;

        Ok(record.map(|r| Branch {
            name: r.name,
            head: r.head,
            base: r.base,
            created_at: r.created_at,
            merged_into: r.merged_into,
        }))
    }

    async fn list_branches(&self) -> Vec<Branch> {
        match self.db.query("SELECT * FROM branch").await {
            Ok(mut response) => {
                let records: Vec<BranchRecord> = response.take(0).unwrap_or_default();
                records
                    .into_iter()
                    .map(|r| Branch {
                        name: r.name,
                        head: r.head,
                        base: r.base,
                        created_at: r.created_at,
                        merged_into: r.merged_into,
                    })
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    }

    async fn merge_branch(&self, source: &str, target: &str) -> OnyxResult<VersionId> {
        let source_branch = self
            .get_branch(source)
            .await?
            .ok_or_else(|| OnyxError::BranchNotFound(source.to_string()))?;

        if self.get_branch(target).await?.is_none() {
            return Err(OnyxError::BranchNotFound(target.to_string()));
        }

        let merge_version_id = new_version_id();

        // Mark source branch as merged
        let _ = self
            .db
            .query(&format!(
                "UPDATE branch:{} SET merged_into = '{}'",
                source, target
            ))
            .await;

        // Update target branch head
        let _ = self
            .db
            .query(&format!(
                "UPDATE branch:{} SET head = '{}'",
                target, merge_version_id
            ))
            .await;

        // Record a merge version entry
        let merge_entry = VersionEntry {
            version_id: merge_version_id.clone(),
            entity_id: Uuid::nil(),
            parent_version: Some(source_branch.head),
            branch: target.to_string(),
            diff: Diff::Initial {
                content: format!("Merge branch '{}' into '{}'", source, target),
            },
            commit_id: None,
            author: None,
            message: Some(format!("Merge branch '{}' into '{}'", source, target)),
            timestamp: Utc::now(),
        };

        self.record_version(merge_entry).await?;

        Ok(merge_version_id)
    }

    async fn version_count(&self) -> usize {
        match self
            .db
            .query("SELECT count() FROM version GROUP BY count")
            .await
        {
            Ok(mut response) => {
                let count: Option<i64> = response.take(0).ok().flatten();
                count.unwrap_or(0) as usize
            }
            Err(_) => 0,
        }
    }

    async fn get_all_version_ids(&self) -> OnyxResult<Vec<VersionId>> {
        let query = "SELECT version_id FROM version";
        let mut response = self.db.query(query).await
            .map_err(|e| OnyxError::Internal(format!("Failed to query version IDs: {}", e)))?;
        
        let records: Vec<serde_json::Value> = response.take(0).unwrap_or_default();
        let mut ids = Vec::new();
        
        for record in records {
            if let Some(id_str) = record.get("version_id").and_then(|v| v.as_str()) {
                ids.push(id_str.to_string());
            }
        }
        
        Ok(ids)
    }
}

// ---------------------------------------------------------------------------
// InMemoryHistoryStore: for testing and fallback
// ---------------------------------------------------------------------------

use tokio::sync::RwLock;

/// In-memory history store that maintains version chains per entity.
pub struct InMemoryHistoryStore {
    versions: RwLock<HashMap<VersionId, VersionEntry>>,
    entity_versions: RwLock<HashMap<Uuid, Vec<VersionId>>>,
    branches: RwLock<HashMap<String, Branch>>,
    branch_heads: RwLock<HashMap<(Uuid, String), VersionId>>,
}

impl InMemoryHistoryStore {
    pub fn new() -> Self {
        Self {
            versions: RwLock::new(HashMap::new()),
            entity_versions: RwLock::new(HashMap::new()),
            branches: RwLock::new(HashMap::new()),
            branch_heads: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryHistoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HistoryStore for InMemoryHistoryStore {
    async fn record_version(&self, entry: VersionEntry) -> OnyxResult<VersionId> {
        let version_id = entry.version_id.clone();
        let entity_id = entry.entity_id;
        let branch = entry.branch.clone();

        let mut versions = self.versions.write().await;

        // Verify parent version exists if specified
        if let Some(ref parent) = entry.parent_version {
            if !versions.contains_key(parent) {
                return Err(OnyxError::VersionNotFound(parent.clone()));
            }
        }

        // Store the version entry
        versions.insert(version_id.clone(), entry);
        drop(versions);

        // Update entity version index
        let mut entity_versions = self.entity_versions.write().await;
        entity_versions
            .entry(entity_id)
            .or_default()
            .push(version_id.clone());
        drop(entity_versions);

        // Update branch head
        let mut branch_heads = self.branch_heads.write().await;
        branch_heads.insert((entity_id, branch), version_id.clone());

        Ok(version_id)
    }

    async fn get_version(&self, version_id: &VersionId) -> OnyxResult<Option<VersionEntry>> {
        let versions = self.versions.read().await;
        Ok(versions.get(version_id).cloned())
    }

    async fn get_content_at_version(
        &self,
        entity_id: &Uuid,
        version_id: &VersionId,
    ) -> OnyxResult<String> {
        let mut chain: Vec<VersionEntry> = Vec::new();
        let mut current_id = Some(version_id.clone());

        while let Some(vid) = current_id {
            let entry = self
                .get_version(&vid)
                .await?
                .ok_or_else(|| OnyxError::VersionNotFound(vid.clone()))?;

            if entry.entity_id != *entity_id {
                return Err(OnyxError::Internal(format!(
                    "Version {} belongs to entity {}, not {}",
                    vid, entry.entity_id, entity_id
                )));
            }

            chain.push(entry);
            current_id = chain.last().unwrap().parent_version.clone();
        }

        chain.reverse();
        let mut content = String::new();

        for entry in &chain {
            match &entry.diff {
                Diff::Initial { content: c } => {
                    content = c.clone();
                }
                Diff::ContentChanged { patch, .. } => {
                    content = patch.clone();
                }
                Diff::MetadataChanged { .. } => {}
                Diff::Composite(diffs) => {
                    for diff in diffs {
                        if let Diff::ContentChanged { patch, .. } = diff {
                            content = patch.clone();
                        }
                    }
                }
            }
        }

        Ok(content)
    }

    async fn get_content_at_timestamp(
        &self,
        entity_id: &Uuid,
        timestamp: &DateTime<Utc>,
    ) -> OnyxResult<String> {
        let entity_versions = self.entity_versions.read().await;

        let versions = entity_versions
            .get(entity_id)
            .ok_or_else(|| OnyxError::NodeNotFound(*entity_id))?;

        let versions_guard = self.versions.read().await;
        let mut latest_version: Option<&VersionEntry> = None;

        for vid in versions {
            if let Some(entry) = versions_guard.get(vid) {
                if entry.timestamp <= *timestamp {
                    match latest_version {
                        None => latest_version = Some(entry),
                        Some(current) if entry.timestamp > current.timestamp => {
                            latest_version = Some(entry);
                        }
                        _ => {}
                    }
                }
            }
        }

        let entry = latest_version.ok_or_else(|| {
            OnyxError::Internal(format!(
                "No version found for entity {} at timestamp {}",
                entity_id, timestamp
            ))
        })?;

        self.get_content_at_version(entity_id, &entry.version_id)
            .await
    }

    async fn list_versions(&self, entity_id: &Uuid) -> OnyxResult<Vec<VersionEntry>> {
        let entity_versions = self.entity_versions.read().await;

        let version_ids = entity_versions.get(entity_id).cloned().unwrap_or_default();

        let versions = self.versions.read().await;
        let mut entries: Vec<VersionEntry> = version_ids
            .iter()
            .filter_map(|vid| versions.get(vid).cloned())
            .collect();

        entries.sort_by_key(|e| e.timestamp);
        Ok(entries)
    }

    async fn list_versions_in_range(
        &self,
        entity_id: &Uuid,
        from: &DateTime<Utc>,
        to: &DateTime<Utc>,
    ) -> OnyxResult<Vec<VersionEntry>> {
        let all = self.list_versions(entity_id).await?;
        Ok(all
            .into_iter()
            .filter(|e| e.timestamp >= *from && e.timestamp <= *to)
            .collect())
    }

    async fn get_head(&self, entity_id: &Uuid, branch: &str) -> OnyxResult<Option<VersionId>> {
        let branch_heads = self.branch_heads.read().await;
        Ok(branch_heads.get(&(*entity_id, branch.to_string())).cloned())
    }

    async fn create_branch(&self, name: &str, base_version: VersionId) -> OnyxResult<()> {
        // Verify base version exists
        let versions = self.versions.read().await;
        if !versions.contains_key(&base_version) {
            return Err(OnyxError::VersionNotFound(base_version));
        }
        drop(versions);

        let mut branches = self.branches.write().await;

        if branches.contains_key(name) {
            return Err(OnyxError::BranchAlreadyExists(name.to_string()));
        }

        let branch = Branch::new(name, base_version);
        branches.insert(name.to_string(), branch);
        Ok(())
    }

    async fn get_branch(&self, name: &str) -> OnyxResult<Option<Branch>> {
        let branches = self.branches.read().await;
        Ok(branches.get(name).cloned())
    }

    async fn list_branches(&self) -> Vec<Branch> {
        let branches = self.branches.read().await;
        branches.values().cloned().collect()
    }

    async fn merge_branch(&self, source: &str, target: &str) -> OnyxResult<VersionId> {
        let mut branches = self.branches.write().await;

        let source_branch = branches
            .get(source)
            .ok_or_else(|| OnyxError::BranchNotFound(source.to_string()))?
            .clone();

        if !branches.contains_key(target) {
            return Err(OnyxError::BranchNotFound(target.to_string()));
        }

        let merge_version_id = new_version_id();

        // Mark source branch as merged
        if let Some(branch) = branches.get_mut(source) {
            branch.merged_into = Some(target.to_string());
        }

        // Update target branch head
        if let Some(branch) = branches.get_mut(target) {
            branch.head = merge_version_id.clone();
        }

        drop(branches);

        // Record a merge version entry
        let merge_entry = VersionEntry {
            version_id: merge_version_id.clone(),
            entity_id: Uuid::nil(),
            parent_version: Some(source_branch.head),
            branch: target.to_string(),
            diff: Diff::Initial {
                content: format!("Merge branch '{}' into '{}'", source, target),
            },
            commit_id: None,
            author: None,
            message: Some(format!("Merge branch '{}' into '{}'", source, target)),
            timestamp: Utc::now(),
        };

        self.record_version(merge_entry).await?;

        Ok(merge_version_id)
    }

    async fn version_count(&self) -> usize {
        let versions = self.versions.read().await;
        versions.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_record_and_retrieve_version() {
        let store = InMemoryHistoryStore::new();
        let entity_id = Uuid::new_v4();

        let v1 = VersionEntry::initial(entity_id, "fn hello() {}");
        let v1_id = store.record_version(v1).await.unwrap();

        let retrieved = store.get_version(&v1_id).await.unwrap().unwrap();
        assert_eq!(retrieved.entity_id, entity_id);
        assert!(retrieved.diff.is_initial());
    }

    #[tokio::test]
    async fn test_in_memory_reconstruct_content() {
        let store = InMemoryHistoryStore::new();
        let entity_id = Uuid::new_v4();

        let v1 = VersionEntry::initial(entity_id, "fn hello() {}");
        let v1_id = store.record_version(v1).await.unwrap();

        let content = store
            .get_content_at_version(&entity_id, &v1_id)
            .await
            .unwrap();
        assert_eq!(content, "fn hello() {}");
    }

    #[tokio::test]
    async fn test_in_memory_version_chain() {
        let store = InMemoryHistoryStore::new();
        let entity_id = Uuid::new_v4();

        let v1 = VersionEntry::initial(entity_id, "fn hello() {}");
        let v1_id = store.record_version(v1).await.unwrap();

        let v2 = VersionEntry::content_change(
            entity_id,
            v1_id.clone(),
            "fn hello() { println!(\"hi\"); }",
            1,
            1,
        );
        let v2_id = store.record_version(v2).await.unwrap();

        let versions = store.list_versions(&entity_id).await.unwrap();
        assert_eq!(versions.len(), 2);
    }

    #[tokio::test]
    async fn test_in_memory_branching() {
        let store = InMemoryHistoryStore::new();
        let entity_id = Uuid::new_v4();

        let v1 = VersionEntry::initial(entity_id, "initial");
        let v1_id = store.record_version(v1).await.unwrap();

        store.create_branch("feature", v1_id.clone()).await.unwrap();

        let branch = store.get_branch("feature").await.unwrap().unwrap();
        assert_eq!(branch.name, "feature");
        assert_eq!(branch.base, v1_id);

        assert!(store.create_branch("feature", v1_id).await.is_err());
    }

    #[tokio::test]
    async fn test_in_memory_get_head() {
        let store = InMemoryHistoryStore::new();
        let entity_id = Uuid::new_v4();

        let v1 = VersionEntry::initial(entity_id, "initial");
        let v1_id = store.record_version(v1).await.unwrap();

        let head = store.get_head(&entity_id, "main").await.unwrap();
        assert_eq!(head, Some(v1_id));
    }
}
