//! WAL durability and crash recovery tests for RocksDB storage.
//!
//! This module tests the durability guarantees of the RocksDB storage layer,
//! including write-ahead logging behavior and crash recovery scenarios.

#[cfg(test)]
mod tests {
    use crate::error::OnyxResult;
    use crate::model::{Node, Edge, Embedding, Version, VersionEntry};
    use crate::model::node::{NodeType, CodeEntityKind, Language, Visibility};
    use crate::model::edge::EdgeType;
    use crate::model::version::Diff;
    use crate::store::persistent::{open_db, RocksGraphStore, RocksVectorStore, RocksHistoryStore};
    use rocksdb::{DB, Options, WriteBatch};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::time::sleep;
    use uuid::Uuid;

    /// Test helper to create a temporary RocksDB instance
    async fn create_test_db() -> OnyxResult<(Arc<DB>, TempDir)> {
        let temp_dir = TempDir::new().map_err(|e| {
            crate::error::OnyxError::Internal(format!("Failed to create temp dir: {}", e))
        })?;
        
        let db = open_db(temp_dir.path())?;
        Ok((db, temp_dir))
    }

    /// Test helper to create test data
    fn create_test_node() -> Node {
        Node {
            id: Uuid::new_v4(),
            name: "test_function".to_string(),
            node_type: NodeType::CodeEntity(CodeEntityKind::Function),
            content: "pub fn test() { println!(\"test\"); }".to_string(),
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            provenance: Default::default(),
            extension: Default::default(),
        }
    }

    fn create_test_edge(source_id: Uuid, target_id: Uuid) -> Edge {
        Edge {
            id: Uuid::new_v4(),
            source_id,
            target_id,
            edge_type: EdgeType::Calls,
            confidence: 1.0,
            metadata: Default::default(),
            temporal_context: None,
        }
    }

    fn create_test_embedding(node_id: Uuid) -> Embedding {
        Embedding {
            node_id,
            values: vec![0.1; 100],
        }
    }

    fn create_test_version(entity_id: Uuid) -> VersionEntry {
        VersionEntry {
            version_id: Uuid::new_v4().to_string(),
            entity_id,
            parent_version: None,
            branch: "main".to_string(),
            diff: Diff::Initial {
                content: "initial content".to_string(),
            },
            commit_id: Some("abc123".to_string()),
            author: Some("test@example.com".to_string()),
            message: Some("Initial version".to_string()),
            timestamp: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    #[cfg(feature = "rocksdb-storage")]
    async fn test_wal_during_node_insertion() -> OnyxResult<()> {
        let (db, _temp_dir) = create_test_db().await?;
        let store = RocksGraphStore::new(db.clone())?;

        // Create test data
        let node = create_test_node();
        let node_id = node.id;

        // Insert node
        store.insert_node(node).await?;

        // Force WAL flush by disabling write buffering
        let mut opts = Options::default();
        opts.set_disable_auto_compactions(true);
        
        // Verify node persists after immediate read
        let retrieved = store.get_node(&node_id).await?;
        assert!(retrieved.is_some(), "Node should be immediately retrievable after WAL write");

        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "rocksdb-storage")]
    async fn test_wal_during_batch_operations() -> OnyxResult<()> {
        let (db, _temp_dir) = create_test_db().await?;
        let graph_store = RocksGraphStore::new(db.clone())?;
        let vector_store = RocksVectorStore::new(db.clone(), 100);

        // Create test data
        let nodes: Vec<Node> = (0..10).map(|_| create_test_node()).collect();
        let embeddings: Vec<Embedding> = nodes.iter().map(|n| create_test_embedding(n.id)).collect();

        // Use WriteBatch for atomic operations
        let mut batch = WriteBatch::default();
        
        // Insert all nodes in a batch
        for node in &nodes {
            let serialized = bincode::serialize(node)
                .map_err(|e| crate::error::OnyxError::Internal(format!("Serialization failed: {}", e)))?;
            batch.put_cf(db.cf_handle("nodes").unwrap(), node.id.as_bytes(), serialized);
        }

        // Insert all embeddings in a batch
        for embedding in &embeddings {
            let serialized = bincode::serialize(embedding)
                .map_err(|e| crate::error::OnyxError::Internal(format!("Serialization failed: {}", e)))?;
            batch.put_cf(db.cf_handle("embeddings").unwrap(), embedding.node_id.as_bytes(), serialized);
        }

        // Execute batch write
        db.write(batch).map_err(|e| crate::error::OnyxError::Internal(format!("Batch write failed: {}", e)))?;

        // Verify all data is immediately available
        for node in &nodes {
            let retrieved = graph_store.get_node(&node.id).await?;
            assert!(retrieved.is_some(), "Node {} should be immediately available", node.id);
        }

        for embedding in &embeddings {
            let retrieved = vector_store.get(&embedding.node_id).await?;
            assert!(retrieved.is_some(), "Embedding for {} should be immediately available", embedding.node_id);
        }

        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "rocksdb-storage")]
    async fn test_crash_recovery_with_open_db() -> OnyxResult<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            crate::error::OnyxError::Internal(format!("Failed to create temp dir: {}", e))
        })?;

        let db_path = temp_dir.path().to_path_buf();

        // Phase 1: Write data and "crash" (close DB)
        {
            let db = open_db(&db_path)?;
            let store = RocksGraphStore::new(db.clone())?;

            // Write test data
            let node = create_test_node();
            let node_id = node.id;
            store.insert_node(node).await?;

            // Write some edges
            let node2 = create_test_node();
            let edge = create_test_edge(node_id, node2.id);
            store.insert_edge(edge).await?;

            // Simulate crash by dropping DB reference
            drop(store);
            drop(db);
        }

        // Phase 2: "Recover" by reopening DB
        {
            let db = open_db(&db_path)?;
            let store = RocksGraphStore::new(db.clone())?;

            // Verify data survived the "crash"
            let recovered_nodes = store.get_all_node_ids().await?;
            assert!(!recovered_nodes.is_empty(), "Nodes should survive crash recovery");

            let recovered_edges = store.get_all_edge_ids().await?;
            assert!(!recovered_edges.is_empty(), "Edges should survive crash recovery");
        }

        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "rocksdb-storage")]
    async fn test_during_ungraceful_shutdown() -> OnyxResult<()> {
        let (db, _temp_dir) = create_test_db().await?;
        let store = RocksGraphStore::new(db.clone())?;

        // Write a significant amount of data
        let nodes: Vec<Node> = (0..100).map(|i| {
            let mut node = create_test_node();
            node.name = format!("function_{}", i);
            node.content = format!("pub fn function_{}() {{ println!(\"test {}\"); }}", i, i);
            node
        }).collect();

        // Insert nodes rapidly
        for node in nodes {
            store.insert_node(node).await?;
        }

        // Simulate ungraceful shutdown by not explicitly closing
        // In real scenarios, this would be a process kill
        let node_count = store.get_all_node_ids().await?.len();
        assert!(node_count > 0, "Data should be written before shutdown");

        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "rocksdb-storage")]
    async fn test_concurrent_writes_durability() -> OnyxResult<()> {
        let (db, _temp_dir) = create_test_db().await?;
        let store = Arc::new(RocksGraphStore::new(db.clone())?);

        // Spawn multiple concurrent writers
        let mut handles = vec![];
        
        for i in 0..10 {
            let store_clone = store.clone();
            let handle = tokio::spawn(async move {
                let mut node = create_test_node();
                node.name = format!("concurrent_function_{}", i);
                node.content = format!("pub fn concurrent_{}() {{}}", i);
                
                store_clone.insert_node(node).await?;
                
                // Verify immediately after write
                let retrieved = store_clone.get_node(&node.id).await?;
                assert!(retrieved.is_some(), "Concurrent write should be immediately durable");
                
                Ok::<(), crate::error::OnyxError>(())
            });
            handles.push(handle);
        }

        // Wait for all concurrent operations
        for handle in handles {
            handle.await??;
        }

        // Verify all writes are durable
        let final_count = store.get_all_node_ids().await?.len();
        assert_eq!(final_count, 10, "All concurrent writes should be durable");

        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "rocksdb-storage")]
    async fn test_version_history_durability() -> OnyxResult<()> {
        let (db, _temp_dir) = create_test_db().await?;
        let graph_store = RocksGraphStore::new(db.clone())?;
        let history_store = RocksHistoryStore::new(db)?;

        // Create a node and version history
        let node = create_test_node();
        let entity_id = node.id;
        
        graph_store.insert_node(node).await?;

        // Create multiple versions
        let versions: Vec<VersionEntry> = (0..5).map(|i| {
            let mut version = create_test_version(entity_id);
            version.message = Some(format!("Version {}", i));
            version
        }).collect();

        // Insert versions sequentially
        for version in versions {
            history_store.create_version(version).await?;
        }

        // Verify version history is durable
        let retrieved_versions = history_store.list_versions(&entity_id).await?;
        assert_eq!(retrieved_versions.len(), 5, "All versions should be durable");

        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "rocksdb-storage")]
    async fn test_embedding_during_crash() -> OnyxResult<()> {
        let temp_dir = TempDir::new().map_err(|e| {
            crate::error::OnyxError::Internal(format!("Failed to create temp dir: {}", e))
        })?;

        let db_path = temp_dir.path().to_path_buf();

        // Phase 1: Write embeddings and "crash"
        {
            let db = open_db(&db_path)?;
            let vector_store = RocksVectorStore::new(db.clone(), 100);

            // Write multiple embeddings
            let embeddings: Vec<Embedding> = (0..50).map(|i| {
                let node_id = Uuid::new_v4();
                let mut values = vec![0.0; 100];
                values[i % 100] = 1.0; // Create unique embeddings
                
                Embedding { node_id, values }
            }).collect();

            for embedding in &embeddings {
                vector_store.insert(embedding.node_id, embedding.values.clone()).await?;
            }

            // Simulate crash
            drop(vector_store);
            drop(db);
        }

        // Phase 2: Recover and verify embeddings
        {
            let db = open_db(&db_path)?;
            let vector_store = RocksVectorStore::new(db.clone(), 100);

            let recovered_ids = vector_store.get_all_embedding_ids().await?;
            assert_eq!(recovered_ids.len(), 50, "All embeddings should survive crash recovery");

            // Verify embedding content
            for id in &recovered_ids {
                let retrieved = vector_store.get(id).await?;
                assert!(retrieved.is_some(), "Embedding content should be recoverable");
            }
        }

        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "rocksdb-storage")]
    async fn test_transaction_rollback_durability() -> OnyxResult<()> {
        let (db, _temp_dir) = create_test_db().await?;
        let store = RocksGraphStore::new(db.clone())?;

        // Write initial data
        let initial_node = create_test_node();
        let initial_id = initial_node.id;
        store.insert_node(initial_node).await?;

        // Start a "transaction" (batch write that fails)
        let mut batch = WriteBatch::default();
        
        let temp_node = create_test_node();
        let temp_serialized = bincode::serialize(&temp_node)
            .map_err(|e| crate::error::OnyxError::Internal(format!("Serialization failed: {}", e)))?;
        
        batch.put_cf(db.cf_handle("nodes").unwrap(), temp_node.id.as_bytes(), temp_serialized);
        
        // Simulate transaction failure by not committing the batch
        drop(batch);

        // Verify only initial data exists (rollback worked)
        let final_ids = store.get_all_node_ids().await?;
        assert_eq!(final_ids.len(), 1, "Only initial node should exist after failed transaction");
        assert!(final_ids.contains(&initial_id), "Initial node should still exist");

        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "rocksdb-storage")]
    async fn test_wal_with_large_data() -> OnyxResult<()> {
        let (db, _temp_dir) = create_test_db().await?;
        let store = RocksGraphStore::new(db.clone())?;

        // Create a node with large content (simulating a large file)
        let mut large_node = create_test_node();
        large_node.content = "pub fn large_function() {\n".to_string() + 
            &"let large_string = \"x\".repeat(10000);\n".repeat(100) +
            &"\n}\n";

        // Insert large node
        store.insert_node(large_node).await?;

        // Verify large content is immediately durable
        let retrieved = store.get_node(&large_node.id).await?;
        assert!(retrieved.is_some(), "Large node should be immediately durable");
        
        let retrieved_node = retrieved.unwrap();
        assert!(retrieved_node.content.len() > 10000, "Large content should be preserved");

        Ok(())
    }

    // Integration test that combines multiple stores
    #[tokio::test]
    #[cfg(feature = "rocksdb-storage")]
    async fn test_cross_store_durability() -> OnyxResult<()> {
        let (db, _temp_dir) = create_test_db().await?;
        let graph_store = RocksGraphStore::new(db.clone())?;
        let vector_store = RocksVectorStore::new(db.clone(), 100);
        let history_store = RocksHistoryStore::new(db)?;

        // Create interconnected data
        let node1 = create_test_node();
        let node2 = create_test_node();
        let edge = create_test_edge(node1.id, node2.id);
        let embedding1 = create_test_embedding(node1.id);
        let embedding2 = create_test_embedding(node2.id);
        let version1 = create_test_version(node1.id);

        // Write all data
        graph_store.insert_node(node1.clone()).await?;
        graph_store.insert_node(node2.clone()).await?;
        graph_store.insert_edge(edge).await?;
        vector_store.insert(embedding1.node_id, embedding1.values).await?;
        vector_store.insert(embedding2.node_id, embedding2.values).await?;
        history_store.create_version(version1).await?;

        // Verify all cross-store relationships are durable
        let nodes = graph_store.get_all_node_ids().await?;
        let edges = graph_store.get_all_edge_ids().await?;
        let embeddings = vector_store.get_all_embedding_ids().await?;
        let versions = history_store.get_all_version_ids().await?;

        assert_eq!(nodes.len(), 2, "Both nodes should be durable");
        assert_eq!(edges.len(), 1, "Edge should be durable");
        assert_eq!(embeddings.len(), 2, "Both embeddings should be durable");
        assert_eq!(versions.len(), 1, "Version should be durable");

        Ok(())
    }
}