//! Storage migration utilities for Onyx.
//!
//! This module provides tools for migrating data between different storage backends,
//! particularly from SurrealDB (in-memory) to RocksDB (persistent).

use crate::error::{OnyxError, OnyxResult};
use crate::model::{Node, Edge, Embedding, Version, VersionChain};
use crate::store::{GraphStore, HistoryStore, VectorStore};
use crate::store::{SurrealGraphStore, SurrealHistoryStore, SurrealVectorStore};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(feature = "rocksdb-storage")]
use crate::store::persistent::{RocksGraphStore, RocksHistoryStore, RocksVectorStore, open_db};

/// Migration configuration
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    /// Batch size for processing records
    pub batch_size: usize,
    /// Whether to verify data after migration
    pub verify_after: bool,
    /// Maximum number of retries for failed operations
    pub max_retries: usize,
    /// Progress reporting interval
    pub progress_interval: usize,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            verify_after: true,
            max_retries: 3,
            progress_interval: 100,
        }
    }
}

/// Migration statistics
#[derive(Debug, Default)]
pub struct MigrationStats {
    pub nodes_migrated: usize,
    pub edges_migrated: usize,
    pub embeddings_migrated: usize,
    pub versions_migrated: usize,
    pub errors: usize,
    pub start_time: std::time::Instant,
    pub end_time: Option<std::time::Instant>,
}

impl MigrationStats {
    pub fn duration(&self) -> Option<std::time::Duration> {
        self.end_time.map(|end| end.duration_since(self.start_time))
    }

    pub fn total_records(&self) -> usize {
        self.nodes_migrated + self.edges_migrated + self.embeddings_migrated + self.versions_migrated
    }
}

/// Storage migrator
pub struct StorageMigrator {
    config: MigrationConfig,
    stats: MigrationStats,
}

impl StorageMigrator {
    pub fn new(config: MigrationConfig) -> Self {
        Self {
            config,
            stats: MigrationStats::default(),
        }
    }

    /// Migrate from SurrealDB to RocksDB
    #[cfg(feature = "rocksdb-storage")]
    pub async fn migrate_surreal_to_rocks<P: AsRef<Path>>(
        &mut self,
        rocks_path: P,
    ) -> OnyxResult<MigrationStats> {
        self.stats.start_time = std::time::Instant::now();

        // Initialize source stores (SurrealDB)
        let source_graph = Arc::new(SurrealGraphStore::new().await?);
        let source_vector = Arc::new(SurrealVectorStore::new().await?);
        let source_history = Arc::new(SurrealHistoryStore::new().await?);

        // Initialize target stores (RocksDB)
        let db = open_db(rocks_path)?;
        let target_graph = Arc::new(RocksGraphStore::new(db.clone())?);
        let target_vector = Arc::new(RocksVectorStore::new(db.clone())?);
        let target_history = Arc::new(RocksHistoryStore::new(db)?);

        println!("Starting migration from SurrealDB to RocksDB...");

        // Migrate nodes
        self.migrate_nodes(&source_graph, &target_graph).await?;

        // Migrate edges
        self.migrate_edges(&source_graph, &target_graph).await?;

        // Migrate embeddings
        self.migrate_embeddings(&source_vector, &target_vector).await?;

        // Migrate versions
        self.migrate_versions(&source_history, &target_history).await?;

        self.stats.end_time = Some(std::time::Instant::now());

        // Verify migration if requested
        if self.config.verify_after {
            self.verify_migration(&source_graph, &target_graph, &source_vector, &target_vector).await?;
        }

        Ok(self.stats.clone())
    }

    /// Migrate all nodes from source to target
    async fn migrate_nodes(
        &mut self,
        source: &SurrealGraphStore,
        target: &RocksGraphStore,
    ) -> OnyxResult<()> {
        println!("Migrating nodes...");
        
        // Get all node IDs from source
        let node_ids = source.get_all_node_ids().await?;
        let total_nodes = node_ids.len();
        
        println!("Found {} nodes to migrate", total_nodes);

        // Process in batches
        for (batch_idx, batch) in node_ids.chunks(self.config.batch_size).enumerate() {
            let mut batch_success = 0;
            let mut batch_errors = 0;

            for &node_id in batch {
                match self.migrate_single_node(source, target, node_id).await {
                    Ok(_) => batch_success += 1,
                    Err(e) => {
                        eprintln!("Error migrating node {}: {}", node_id, e);
                        batch_errors += 1;
                        self.stats.errors += 1;
                    }
                }

                // Progress reporting
                if (batch_idx * self.config.batch_size + batch_success + batch_errors) % self.config.progress_interval == 0 {
                    let progress = (batch_idx * self.config.batch_size + batch_success + batch_errors) as f32 / total_nodes as f32 * 100.0;
                    println!("Nodes migration progress: {:.1}% ({}/{})", 
                            progress, batch_idx * self.config.batch_size + batch_success + batch_errors, total_nodes);
                }
            }

            self.stats.nodes_migrated += batch_success;
            println!("Batch {} complete: {} nodes migrated, {} errors", 
                    batch_idx + 1, batch_success, batch_errors);
        }

        println!("Node migration complete: {} nodes migrated", self.stats.nodes_migrated);
        Ok(())
    }

    /// Migrate a single node
    async fn migrate_single_node(
        &self,
        source: &SurrealGraphStore,
        target: &RocksGraphStore,
        node_id: Uuid,
    ) -> OnyxResult<()> {
        let node = source.get_node(node_id).await?;
        if let Some(node) = node {
            target.insert_node(node).await?;
        }
        Ok(())
    }

    /// Migrate all edges from source to target
    async fn migrate_edges(
        &mut self,
        source: &SurrealGraphStore,
        target: &RocksGraphStore,
    ) -> OnyxResult<()> {
        println!("Migrating edges...");
        
        // Get all edge IDs from source
        let edge_ids = source.get_all_edge_ids().await?;
        let total_edges = edge_ids.len();
        
        println!("Found {} edges to migrate", total_edges);

        // Process in batches
        for (batch_idx, batch) in edge_ids.chunks(self.config.batch_size).enumerate() {
            let mut batch_success = 0;
            let mut batch_errors = 0;

            for &edge_id in batch {
                match self.migrate_single_edge(source, target, edge_id).await {
                    Ok(_) => batch_success += 1,
                    Err(e) => {
                        eprintln!("Error migrating edge {}: {}", edge_id, e);
                        batch_errors += 1;
                        self.stats.errors += 1;
                    }
                }

                // Progress reporting
                if (batch_idx * self.config.batch_size + batch_success + batch_errors) % self.config.progress_interval == 0 {
                    let progress = (batch_idx * self.config.batch_size + batch_success + batch_errors) as f32 / total_edges as f32 * 100.0;
                    println!("Edges migration progress: {:.1}% ({}/{})", 
                            progress, batch_idx * self.config.batch_size + batch_success + batch_errors, total_edges);
                }
            }

            self.stats.edges_migrated += batch_success;
            println!("Batch {} complete: {} edges migrated, {} errors", 
                    batch_idx + 1, batch_success, batch_errors);
        }

        println!("Edge migration complete: {} edges migrated", self.stats.edges_migrated);
        Ok(())
    }

    /// Migrate a single edge
    async fn migrate_single_edge(
        &self,
        source: &SurrealGraphStore,
        target: &RocksGraphStore,
        edge_id: Uuid,
    ) -> OnyxResult<()> {
        let edge = source.get_edge(edge_id).await?;
        if let Some(edge) = edge {
            target.insert_edge(edge).await?;
        }
        Ok(())
    }

    /// Migrate all embeddings from source to target
    async fn migrate_embeddings(
        &mut self,
        source: &SurrealVectorStore,
        target: &RocksVectorStore,
    ) -> OnyxResult<()> {
        println!("Migrating embeddings...");
        
        // Get all embedding IDs from source
        let embedding_ids = source.get_all_embedding_ids().await?;
        let total_embeddings = embedding_ids.len();
        
        println!("Found {} embeddings to migrate", total_embeddings);

        // Process in batches
        for (batch_idx, batch) in embedding_ids.chunks(self.config.batch_size).enumerate() {
            let mut batch_success = 0;
            let mut batch_errors = 0;

            for &embedding_id in batch {
                match self.migrate_single_embedding(source, target, embedding_id).await {
                    Ok(_) => batch_success += 1,
                    Err(e) => {
                        eprintln!("Error migrating embedding {}: {}", embedding_id, e);
                        batch_errors += 1;
                        self.stats.errors += 1;
                    }
                }

                // Progress reporting
                if (batch_idx * self.config.batch_size + batch_success + batch_errors) % self.config.progress_interval == 0 {
                    let progress = (batch_idx * self.config.batch_size + batch_success + batch_errors) as f32 / total_embeddings as f32 * 100.0;
                    println!("Embeddings migration progress: {:.1}% ({}/{})", 
                            progress, batch_idx * self.config.batch_size + batch_success + batch_errors, total_embeddings);
                }
            }

            self.stats.embeddings_migrated += batch_success;
            println!("Batch {} complete: {} embeddings migrated, {} errors", 
                    batch_idx + 1, batch_success, batch_errors);
        }

        println!("Embedding migration complete: {} embeddings migrated", self.stats.embeddings_migrated);
        Ok(())
    }

    /// Migrate a single embedding
    async fn migrate_single_embedding(
        &self,
        source: &SurrealVectorStore,
        target: &RocksVectorStore,
        embedding_id: Uuid,
    ) -> OnyxResult<()> {
        let embedding = source.get_embedding(embedding_id).await?;
        if let Some(embedding) = embedding {
            target.insert_embedding(embedding).await?;
        }
        Ok(())
    }

    /// Migrate all versions from source to target
    async fn migrate_versions(
        &mut self,
        source: &SurrealHistoryStore,
        target: &RocksHistoryStore,
    ) -> OnyxResult<()> {
        println!("Migrating versions...");
        
        // Get all version IDs from source
        let version_ids = source.get_all_version_ids().await?;
        let total_versions = version_ids.len();
        
        println!("Found {} versions to migrate", total_versions);

        // Process in batches
        for (batch_idx, batch) in version_ids.chunks(self.config.batch_size).enumerate() {
            let mut batch_success = 0;
            let mut batch_errors = 0;

            for &version_id in batch {
                match self.migrate_single_version(source, target, version_id).await {
                    Ok(_) => batch_success += 1,
                    Err(e) => {
                        eprintln!("Error migrating version {}: {}", version_id, e);
                        batch_errors += 1;
                        self.stats.errors += 1;
                    }
                }

                // Progress reporting
                if (batch_idx * self.config.batch_size + batch_success + batch_errors) % self.config.progress_interval == 0 {
                    let progress = (batch_idx * self.config.batch_size + batch_success + batch_errors) as f32 / total_versions as f32 * 100.0;
                    println!("Versions migration progress: {:.1}% ({}/{})", 
                            progress, batch_idx * self.config.batch_size + batch_success + batch_errors, total_versions);
                }
            }

            self.stats.versions_migrated += batch_success;
            println!("Batch {} complete: {} versions migrated, {} errors", 
                    batch_idx + 1, batch_success, batch_errors);
        }

        println!("Version migration complete: {} versions migrated", self.stats.versions_migrated);
        Ok(())
    }

    /// Migrate a single version
    async fn migrate_single_version(
        &self,
        source: &SurrealHistoryStore,
        target: &RocksHistoryStore,
        version_id: Uuid,
    ) -> OnyxResult<()> {
        let version = source.get_version(version_id).await?;
        if let Some(version) = version {
            target.create_version(version).await?;
        }
        Ok(())
    }

    /// Verify migration integrity
    #[cfg(feature = "rocksdb-storage")]
    async fn verify_migration(
        &self,
        source_graph: &SurrealGraphStore,
        target_graph: &RocksGraphStore,
        source_vector: &SurrealVectorStore,
        target_vector: &RocksVectorStore,
    ) -> OnyxResult<()> {
        println!("Verifying migration integrity...");

        // Verify node counts
        let source_node_count = source_graph.get_all_node_ids().await?.len();
        let target_node_count = target_graph.get_all_node_ids().await?.len();
        
        if source_node_count != target_node_count {
            return Err(OnyxError::Internal(format!(
                "Node count mismatch: source={}, target={}", 
                source_node_count, target_node_count
            )));
        }

        // Verify edge counts
        let source_edge_count = source_graph.get_all_edge_ids().await?.len();
        let target_edge_count = target_graph.get_all_edge_ids().await?.len();
        
        if source_edge_count != target_edge_count {
            return Err(OnyxError::Internal(format!(
                "Edge count mismatch: source={}, target={}", 
                source_edge_count, target_edge_count
            )));
        }

        // Verify embedding counts
        let source_embedding_count = source_vector.get_all_embedding_ids().await?.len();
        let target_embedding_count = target_vector.get_all_embedding_ids().await?.len();
        
        if source_embedding_count != target_embedding_count {
            return Err(OnyxError::Internal(format!(
                "Embedding count mismatch: source={}, target={}", 
                source_embedding_count, target_embedding_count
            )));
        }

        println!("✓ Migration verification successful!");
        println!("  Nodes: {}", source_node_count);
        println!("  Edges: {}", source_edge_count);
        println!("  Embeddings: {}", source_embedding_count);

        Ok(())
    }
}

/// CLI command for storage migration
pub async fn run_migration(rocks_path: &str) -> OnyxResult<()> {
    let config = MigrationConfig::default();
    let mut migrator = StorageMigrator::new(config);

    println!("Starting storage migration...");
    println!("Source: SurrealDB (in-memory)");
    println!("Target: RocksDB (persistent) at {}", rocks_path);
    println!();

    let stats = migrator.migrate_surreal_to_rocks(rocks_path).await?;

    println!();
    println!("Migration completed!");
    println!("Statistics:");
    println!("  Nodes migrated: {}", stats.nodes_migrated);
    println!("  Edges migrated: {}", stats.edges_migrated);
    println!("  Embeddings migrated: {}", stats.embeddings_migrated);
    println!("  Versions migrated: {}", stats.versions_migrated);
    println!("  Errors: {}", stats.errors);
    
    if let Some(duration) = stats.duration() {
        println!("  Duration: {:?}", duration);
        let rate = stats.total_records() as f64 / duration.as_secs_f64();
        println!("  Rate: {:.1} records/second", rate);
    }

    Ok(())
}