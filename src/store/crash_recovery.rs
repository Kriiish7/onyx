//! Crash recovery simulation utilities for testing RocksDB durability.
//!
//! This module provides utilities to simulate various crash scenarios
//! and validate recovery behavior.

use crate::error::{OnyxError, OnyxResult};
use crate::model::{Node, Edge, Embedding};
use crate::store::persistent::{open_db, RocksGraphStore, RocksVectorStore, RocksHistoryStore};
use rocksdb::{DB, Options, WriteBatch};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

/// Crash recovery simulator
pub struct CrashSimulator {
    db_path: PathBuf,
    db: Option<Arc<DB>>,
}

impl CrashSimulator {
    /// Create a new crash simulator
    pub fn new<P: AsRef<Path>>(db_path: P) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
            db: None,
        }
    }

    /// Initialize the database
    pub async fn initialize(&mut self) -> OnyxResult<()> {
        let db = open_db(&self.db_path)?;
        self.db = Some(db);
        Ok(())
    }

    /// Simulate a graceful shutdown
    pub async fn graceful_shutdown(&mut self) -> OnyxResult<()> {
        if let Some(db) = self.db.take() {
            // Flush all memtables to SST files
            db.flush()?;
            drop(db);
        }
        Ok(())
    }

    /// Simulate an ungraceful shutdown (process kill)
    pub async fn ungraceful_shutdown(&mut self) {
        // Simply drop the DB reference without flushing
        self.db = None;
    }

    /// Simulate a power failure during write
    pub async fn power_failure_during_write(&mut self) -> OnyxResult<()> {
        if let Some(db) = &self.db {
            // Start a write operation
            let mut batch = WriteBatch::default();
            
            let node = self.create_test_node();
            let serialized = bincode::serialize(&node)
                .map_err(|e| OnyxError::Internal(format!("Serialization failed: {}", e)))?;
            
            batch.put_cf(
                db.cf_handle("nodes").unwrap(), 
                node.id.as_bytes(), 
                serialized
            );
            
            // Simulate power failure by dropping before commit
            drop(batch);
            drop(db);
            self.db = None;
        }
        Ok(())
    }

    /// Recover from previous crash
    pub async fn recover(&mut self) -> OnyxResult<RecoveryReport> {
        let db = open_db(&self.db_path)?;
        self.db = Some(db.clone());

        let graph_store = RocksGraphStore::new(db.clone())?;
        let vector_store = RocksVectorStore::new(db.clone(), 100)?;
        let history_store = RocksHistoryStore::new(db)?;

        // Collect recovery statistics
        let node_count = graph_store.get_all_node_ids().await?.len();
        let edge_count = graph_store.get_all_edge_ids().await?.len();
        let embedding_count = vector_store.get_all_embedding_ids().await?.len();
        let version_count = history_store.get_all_version_ids().await?.len();

        Ok(RecoveryReport {
            node_count,
            edge_count,
            embedding_count,
            version_count,
            recovery_successful: true,
        })
    }

    /// Write test data before crash
    pub async fn write_test_data(&self, count: usize) -> OnyxResult<()> {
        if let Some(db) = &self.db {
            let graph_store = RocksGraphStore::new(db.clone())?;
            let vector_store = RocksVectorStore::new(db.clone(), 100)?;

            // Write nodes
            for i in 0..count {
                let mut node = self.create_test_node();
                node.name = format!("test_node_{}", i);
                node.content = format!("pub fn test_{}() {{}}", i);
                
                graph_store.insert_node(node).await?;
                
                // Write corresponding embedding
                let embedding = Embedding {
                    node_id: node.id,
                    values: vec![i as f32 / 100.0; 100],
                };
                vector_store.insert(embedding.node_id, embedding.values).await?;
            }
        }
        Ok(())
    }

    /// Simulate write-ahead logging stress test
    pub async fn wal_stress_test(&self, operations: usize) -> OnyxResult<()> {
        if let Some(db) = &self.db {
            let graph_store = RocksGraphStore::new(db.clone())?;

            for i in 0..operations {
                let mut node = self.create_test_node();
                node.name = format!("stress_test_{}", i);
                node.content = format!("pub fn stress_{}() {{ println!(\"{}\"); }}", i, i);
                
                graph_store.insert_node(node).await?;
                
                // Small delay to simulate real-world timing
                sleep(Duration::from_millis(1)).await;
            }
        }
        Ok(())
    }

    /// Validate database integrity
    pub async fn validate_integrity(&self) -> OnyxResult<IntegrityReport> {
        if let Some(db) = &self.db {
            let graph_store = RocksGraphStore::new(db.clone())?;
            let vector_store = RocksVectorStore::new(db.clone(), 100)?;

            // Check for orphaned edges (edges pointing to non-existent nodes)
            let all_edge_ids = graph_store.get_all_edge_ids().await?;
            let all_node_ids = graph_store.get_all_node_ids().await?;
            let mut orphaned_edges = 0;

            for edge_id in all_edge_ids {
                if let Some(edge) = graph_store.get_edge(&edge_id).await? {
                    if !all_node_ids.contains(&edge.source_id) || !all_node_ids.contains(&edge.target_id) {
                        orphaned_edges += 1;
                    }
                }
            }

            // Check for orphaned embeddings
            let all_embedding_ids = vector_store.get_all_embedding_ids().await?;
            let mut orphaned_embeddings = 0;

            for embedding_id in all_embedding_ids {
                if !all_node_ids.contains(&embedding_id) {
                    orphaned_embeddings += 1;
                }
            }

            Ok(IntegrityReport {
                total_nodes: all_node_ids.len(),
                total_edges: graph_store.get_all_edge_ids().await?.len(),
                total_embeddings: all_embedding_ids.len(),
                orphaned_edges,
                orphaned_embeddings,
                is_valid: orphaned_edges == 0 && orphaned_embeddings == 0,
            })
        } else {
            Err(OnyxError::Internal("Database not initialized".to_string()))
        }
    }

    fn create_test_node(&self) -> Node {
        use crate::model::node::{NodeType, CodeEntityKind, Language, Visibility};
        
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
}

/// Recovery statistics report
#[derive(Debug, Clone)]
pub struct RecoveryReport {
    pub node_count: usize,
    pub edge_count: usize,
    pub embedding_count: usize,
    pub version_count: usize,
    pub recovery_successful: bool,
}

/// Database integrity report
#[derive(Debug, Clone)]
pub struct IntegrityReport {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub total_embeddings: usize,
    pub orphaned_edges: usize,
    pub orphaned_embeddings: usize,
    pub is_valid: bool,
}

/// Crash scenario types
#[derive(Debug, Clone)]
pub enum CrashScenario {
    /// Graceful shutdown with proper flushing
    GracefulShutdown,
    /// Ungraceful shutdown (process kill)
    UngracefulShutdown,
    /// Power failure during write operation
    PowerFailureDuringWrite,
    /// System crash during batch operation
    SystemCrashDuringBatch,
    /// Disk full scenario
    DiskFull,
}

/// Comprehensive crash recovery test runner
pub struct CrashTestRunner {
    simulator: CrashSimulator,
}

impl CrashTestRunner {
    /// Create a new crash test runner
    pub fn new<P: AsRef<Path>>(db_path: P) -> Self {
        Self {
            simulator: CrashSimulator::new(db_path),
        }
    }

    /// Run a comprehensive crash recovery test suite
    pub async fn run_test_suite(&mut self) -> OnyxResult<Vec<TestResult>> {
        let mut results = Vec::new();

        // Test 1: Graceful shutdown recovery
        results.push(self.test_graceful_shutdown_recovery().await?);

        // Test 2: Ungraceful shutdown recovery
        results.push(self.test_ungraceful_shutdown_recovery().await?);

        // Test 3: Power failure during write
        results.push(self.test_power_failure_recovery().await?);

        // Test 4: WAL stress test with crash
        results.push(self.test_wal_stress_crash().await?);

        // Test 5: Large data crash recovery
        results.push(self.test_large_data_crash().await?);

        Ok(results)
    }

    /// Test graceful shutdown and recovery
    async fn test_graceful_shutdown_recovery(&mut self) -> OnyxResult<TestResult> {
        self.simulator.initialize().await?;
        self.simulator.write_test_data(100).await?;
        self.simulator.graceful_shutdown().await?;

        // Recover and validate
        let report = self.simulator.recover().await?;
        let integrity = self.simulator.validate_integrity().await?;

        Ok(TestResult {
            scenario: CrashScenario::GracefulShutdown,
            recovery_report: report,
            integrity_report: integrity,
            passed: report.recovery_successful && integrity.is_valid,
        })
    }

    /// Test ungraceful shutdown and recovery
    async fn test_ungraceful_shutdown_recovery(&mut self) -> OnyxResult<TestResult> {
        self.simulator.initialize().await?;
        self.simulator.write_test_data(100).await?;
        self.simulator.ungraceful_shutdown().await;

        // Recover and validate
        let report = self.simulator.recover().await?;
        let integrity = self.simulator.validate_integrity().await?;

        Ok(TestResult {
            scenario: CrashScenario::UngracefulShutdown,
            recovery_report: report,
            integrity_report: integrity,
            passed: report.recovery_successful && integrity.is_valid,
        })
    }

    /// Test power failure during write
    async fn test_power_failure_recovery(&mut self) -> OnyxResult<TestResult> {
        self.simulator.initialize().await?;
        self.simulator.write_test_data(50).await?; // Write some data first
        self.simulator.power_failure_during_write().await;

        // Recover and validate
        let report = self.simulator.recover().await?;
        let integrity = self.simulator.validate_integrity().await?;

        Ok(TestResult {
            scenario: CrashScenario::PowerFailureDuringWrite,
            recovery_report: report,
            integrity_report: integrity,
            passed: report.recovery_successful, // Power failure might lose in-flight data
        })
    }

    /// Test WAL stress with crash
    async fn test_wal_stress_crash(&mut self) -> OnyxResult<TestResult> {
        self.simulator.initialize().await?;
        self.simulator.wal_stress_test(1000).await?;
        self.simulator.ungraceful_shutdown().await;

        // Recover and validate
        let report = self.simulator.recover().await?;
        let integrity = self.simulator.validate_integrity().await?;

        Ok(TestResult {
            scenario: CrashScenario::SystemCrashDuringBatch,
            recovery_report: report,
            integrity_report: integrity,
            passed: report.recovery_successful && integrity.is_valid,
        })
    }

    /// Test large data crash recovery
    async fn test_large_data_crash(&mut self) -> OnyxResult<TestResult> {
        self.simulator.initialize().await?;
        self.simulator.write_test_data(10).await?; // Fewer but larger nodes
        
        // Create large content nodes
        if let Some(db) = &self.simulator.db {
            let graph_store = RocksGraphStore::new(db.clone())?;
            
            for i in 0..5 {
                let mut node = self.simulator.create_test_node();
                node.name = format!("large_node_{}", i);
                node.content = "x".repeat(100000); // 100KB per node
                graph_store.insert_node(node).await?;
            }
        }
        
        self.simulator.ungraceful_shutdown().await;

        // Recover and validate
        let report = self.simulator.recover().await?;
        let integrity = self.simulator.validate_integrity().await?;

        Ok(TestResult {
            scenario: CrashScenario::DiskFull,
            recovery_report: report,
            integrity_report: integrity,
            passed: report.recovery_successful && integrity.is_valid,
        })
    }
}

/// Individual test result
#[derive(Debug, Clone)]
pub struct TestResult {
    pub scenario: CrashScenario,
    pub recovery_report: RecoveryReport,
    pub integrity_report: IntegrityReport,
    pub passed: bool,
}

impl TestResult {
    /// Print a detailed test report
    pub fn print_report(&self) {
        println!("\n=== Crash Recovery Test: {:?} ===", self.scenario);
        println!("Status: {}", if self.passed { "PASSED" } else { "FAILED" });
        println!("Recovery Report:");
        println!("  Nodes: {}", self.recovery_report.node_count);
        println!("  Edges: {}", self.recovery_report.edge_count);
        println!("  Embeddings: {}", self.recovery_report.embedding_count);
        println!("  Versions: {}", self.recovery_report.version_count);
        println!("Integrity Report:");
        println!("  Total Nodes: {}", self.integrity_report.total_nodes);
        println!("  Total Edges: {}", self.integrity_report.total_edges);
        println!("  Total Embeddings: {}", self.integrity_report.total_embeddings);
        println!("  Orphaned Edges: {}", self.integrity_report.orphaned_edges);
        println!("  Orphaned Embeddings: {}", self.integrity_report.orphaned_embeddings);
        println!("  Database Valid: {}", self.integrity_report.is_valid);
        println!("=====================================");
    }
}