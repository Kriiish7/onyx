//! RocksDB-backed history store for temporal versioning.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rocksdb::DB;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{OnyxError, OnyxResult};
use crate::model::version::{Branch, Diff, VersionEntry, VersionId};
use crate::store::history::HistoryStore;

use super::{CF_BRANCHES, CF_VERSIONS, CF_VERSION_CHAINS};

/// RocksDB-backed history store for version chains and branching.
#[derive(Clone)]
pub struct RocksHistoryStore {
    db: Arc<DB>,
}

impl RocksHistoryStore {
    /// Create a new RocksDB history store.
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    /// Serialize a version entry to bytes.
    fn serialize_version(&self, entry: &VersionEntry) -> OnyxResult<Vec<u8>> {
        bincode::serialize(entry)
            .map_err(|e| OnyxError::Internal(format!("Failed to serialize version: {}", e)))
    }

    /// Deserialize a version entry from bytes.
    fn deserialize_version(&self, bytes: &[u8]) -> OnyxResult<VersionEntry> {
        bincode::deserialize(bytes)
            .map_err(|e| OnyxError::Internal(format!("Failed to deserialize version: {}", e)))
    }

    /// Serialize a branch to bytes.
    fn serialize_branch(&self, branch: &Branch) -> OnyxResult<Vec<u8>> {
        bincode::serialize(branch)
            .map_err(|e| OnyxError::Internal(format!("Failed to serialize branch: {}", e)))
    }

    /// Deserialize a branch from bytes.
    fn deserialize_branch(&self, bytes: &[u8]) -> OnyxResult<Branch> {
        bincode::deserialize(bytes)
            .map_err(|e| OnyxError::Internal(format!("Failed to deserialize branch: {}", e)))
    }

    /// Get the versions column family handle.
    fn cf_versions(&self) -> OnyxResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_VERSIONS)
            .ok_or_else(|| OnyxError::Internal("Missing versions column family".to_string()))
    }

    /// Get the version chains column family handle.
    fn cf_version_chains(&self) -> OnyxResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_VERSION_CHAINS)
            .ok_or_else(|| OnyxError::Internal("Missing version_chains column family".to_string()))
    }

    /// Get the branches column family handle.
    fn cf_branches(&self) -> OnyxResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_BRANCHES)
            .ok_or_else(|| OnyxError::Internal("Missing branches column family".to_string()))
    }

    /// Build chain index key: [entity_id (16 bytes)][timestamp (8 bytes)]
    fn chain_key(&self, entity_id: &Uuid, timestamp: &DateTime<Utc>) -> Vec<u8> {
        let mut key = entity_id.as_bytes().to_vec();
        key.extend_from_slice(&timestamp.timestamp_millis().to_be_bytes());
        key
    }

    /// Apply a diff to reconstruct content.
    fn apply_diff(&self, base_content: &str, diff: &Diff) -> String {
        match diff {
            Diff::Full(content) => content.clone(),
            Diff::Delta(delta) => {
                // Simple line-based delta reconstruction
                // Format: "+line" (add), "-line" (remove), " line" (unchanged)
                let mut result = String::new();
                let base_lines: Vec<&str> = base_content.lines().collect();
                let mut base_idx = 0;

                for line in delta.lines() {
                    if let Some(op) = line.chars().next() {
                        match op {
                            '+' => {
                                result.push_str(&line[1..]);
                                result.push('\n');
                            }
                            '-' => {
                                base_idx += 1;
                            }
                            ' ' => {
                                if base_idx < base_lines.len() {
                                    result.push_str(base_lines[base_idx]);
                                    result.push('\n');
                                    base_idx += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }

                result
            }
        }
    }
}

#[async_trait]
impl HistoryStore for RocksHistoryStore {
    async fn record_version(&self, entry: VersionEntry) -> OnyxResult<VersionId> {
        let cf_versions = self.cf_versions()?;
        let cf_chains = self.cf_version_chains()?;

        let version_id = entry.version_id.clone();

        // Store the version entry
        let key = version_id.as_bytes();
        let value = self.serialize_version(&entry)?;
        self.db
            .put_cf(cf_versions, key, value)
            .map_err(|e| OnyxError::Internal(format!("Failed to record version: {}", e)))?;

        // Update the chain index
        let chain_key = self.chain_key(&entry.entity_id, &entry.timestamp);
        self.db
            .put_cf(cf_chains, chain_key, version_id.as_bytes())
            .map_err(|e| OnyxError::Internal(format!("Failed to update chain index: {}", e)))?;

        Ok(version_id)
    }

    async fn get_version(&self, version_id: &VersionId) -> OnyxResult<Option<VersionEntry>> {
        let cf = self.cf_versions()?;
        let key = version_id.as_bytes();

        match self.db.get_cf(cf, key) {
            Ok(Some(bytes)) => Ok(Some(self.deserialize_version(&bytes)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(OnyxError::Internal(format!("Failed to get version: {}", e))),
        }
    }

    async fn get_content_at_version(
        &self,
        entity_id: &Uuid,
        version_id: &VersionId,
    ) -> OnyxResult<String> {
        // Build the diff chain from the target version back to the root
        let mut chain = Vec::new();
        let mut current_id = Some(version_id.clone());

        while let Some(vid) = current_id {
            let entry = self
                .get_version(&vid)
                .await?
                .ok_or_else(|| OnyxError::NotFound(format!("Version not found: {}", vid)))?;

            chain.push(entry.clone());

            if entry.parent_version.is_none() {
                break;
            }

            current_id = entry.parent_version;
        }

        // Reverse the chain to apply diffs from root to target
        chain.reverse();

        // Reconstruct content by applying diffs
        let mut content = String::new();
        for entry in chain {
            content = self.apply_diff(&content, &entry.diff);
        }

        Ok(content)
    }

    async fn get_content_at_timestamp(
        &self,
        entity_id: &Uuid,
        timestamp: &DateTime<Utc>,
    ) -> OnyxResult<String> {
        // Find the latest version before or at the timestamp
        let versions = self.list_versions(entity_id).await?;
        let version = versions
            .iter()
            .filter(|v| v.timestamp <= *timestamp)
            .max_by_key(|v| v.timestamp)
            .ok_or_else(|| {
                OnyxError::NotFound(format!("No version found at timestamp: {}", timestamp))
            })?;

        self.get_content_at_version(entity_id, &version.version_id)
            .await
    }

    async fn list_versions(&self, entity_id: &Uuid) -> OnyxResult<Vec<VersionEntry>> {
        let cf_chains = self.cf_version_chains()?;
        let prefix = entity_id.as_bytes();

        let iter = self.db.prefix_iterator_cf(cf_chains, prefix);
        let mut versions = Vec::new();

        for item in iter {
            let (_, value) = item
                .map_err(|e| OnyxError::Internal(format!("Failed to iterate versions: {}", e)))?;

            let version_id = VersionId::from_slice(&value)
                .map_err(|e| OnyxError::Internal(format!("Invalid version UUID: {}", e)))?;

            if let Some(entry) = self.get_version(&version_id).await? {
                versions.push(entry);
            }
        }

        // Sort by timestamp
        versions.sort_by_key(|v| v.timestamp);

        Ok(versions)
    }

    async fn list_versions_in_range(
        &self,
        entity_id: &Uuid,
        from: &DateTime<Utc>,
        to: &DateTime<Utc>,
    ) -> OnyxResult<Vec<VersionEntry>> {
        let versions = self.list_versions(entity_id).await?;

        Ok(versions
            .into_iter()
            .filter(|v| v.timestamp >= *from && v.timestamp <= *to)
            .collect())
    }

    async fn get_head(&self, entity_id: &Uuid, branch: &str) -> OnyxResult<Option<VersionId>> {
        let versions = self.list_versions(entity_id).await?;

        Ok(versions
            .iter()
            .filter(|v| v.branch == branch)
            .max_by_key(|v| v.timestamp)
            .map(|v| v.version_id.clone()))
    }

    async fn create_branch(&self, name: &str, base_version: VersionId) -> OnyxResult<()> {
        let cf = self.cf_branches()?;

        let branch = Branch {
            name: name.to_string(),
            base_version,
            created_at: Utc::now(),
        };

        let key = name.as_bytes();
        let value = self.serialize_branch(&branch)?;

        self.db
            .put_cf(cf, key, value)
            .map_err(|e| OnyxError::Internal(format!("Failed to create branch: {}", e)))?;

        Ok(())
    }

    async fn get_branch(&self, name: &str) -> OnyxResult<Option<Branch>> {
        let cf = self.cf_branches()?;
        let key = name.as_bytes();

        match self.db.get_cf(cf, key) {
            Ok(Some(bytes)) => Ok(Some(self.deserialize_branch(&bytes)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(OnyxError::Internal(format!("Failed to get branch: {}", e))),
        }
    }

    async fn list_branches(&self) -> Vec<Branch> {
        let cf = match self.cf_branches() {
            Ok(cf) => cf,
            Err(_) => return vec![],
        };

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        let mut branches = Vec::new();

        for item in iter {
            if let Ok((_, value)) = item {
                if let Ok(branch) = self.deserialize_branch(&value) {
                    branches.push(branch);
                }
            }
        }

        branches
    }

    async fn merge_branch(&self, source: &str, target: &str) -> OnyxResult<VersionId> {
        // TODO: Implement proper three-way merge logic
        // For now, create a merge version that points to both branches
        let source_branch = self
            .get_branch(source)
            .await?
            .ok_or_else(|| OnyxError::NotFound(format!("Source branch not found: {}", source)))?;

        let target_branch = self
            .get_branch(target)
            .await?
            .ok_or_else(|| OnyxError::NotFound(format!("Target branch not found: {}", target)))?;

        // Get the latest version from source
        let source_version = self
            .get_version(&source_branch.base_version)
            .await?
            .ok_or_else(|| OnyxError::NotFound("Source version not found".to_string()))?;

        // Create a merge version
        let merge_version = VersionEntry {
            version_id: VersionId::new_v4(),
            entity_id: source_version.entity_id,
            parent_version: Some(target_branch.base_version.clone()),
            branch: target.to_string(),
            diff: Diff::Full(format!("Merged {} into {}", source, target)),
            commit_id: None,
            author: Some("system".to_string()),
            message: Some(format!("Merge branch '{}' into '{}'", source, target)),
            timestamp: Utc::now(),
        };

        self.record_version(merge_version.clone()).await?;

        Ok(merge_version.version_id)
    }

    async fn version_count(&self) -> usize {
        let cf = match self.cf_versions() {
            Ok(cf) => cf,
            Err(_) => return 0,
        };

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        iter.count()
    }

    async fn get_all_version_ids(&self) -> OnyxResult<Vec<VersionId>> {
        let cf = self.cf_versions()?;
        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        let mut ids = Vec::new();

        for item in iter {
            if let Ok((key, _)) = item {
                if let Ok(id) = VersionId::from_slice(&key) {
                    ids.push(id);
                }
            }
        }

        Ok(ids)
    }
}
