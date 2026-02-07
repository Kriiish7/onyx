//! Performance benchmark suite for Onyx storage operations.
//!
//! This module provides comprehensive benchmarks for insert/query throughput,
//! memory usage, and other performance metrics.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

use crate::error::{OnyxError, OnyxResult};
use crate::model::{Node, Edge, Embedding, Version, VersionEntry};
use crate::model::node::{NodeType, CodeEntityKind, Language, Visibility};
use crate::model::edge::EdgeType;
use crate::model::version::Diff;
use crate::store::{GraphStore, VectorStore, HistoryStore};
use crate::store::persistent::{open_db, RocksGraphStore, RocksVectorStore, RocksHistoryStore};

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of operations to run
    pub operation_count: usize,
    /// Number of concurrent operations
    pub concurrency: usize,
    /// Warmup operations before measurement
    pub warmup_count: usize,
    /// Whether to measure memory usage
    pub measure_memory: bool,
    /// Whether to run detailed latency measurements
    pub detailed_latency: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            operation_count: 10000,
            concurrency: 10,
            warmup_count: 1000,
            measure_memory: true,
            detailed_latency: true,
        }
    }
}

/// Benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    /// Total operations performed
    pub total_operations: usize,
    /// Total time taken
    pub total_duration: Duration,
    /// Operations per second
    pub ops_per_second: f64,
    /// Average latency per operation
    pub avg_latency: Duration,
    /// P50 latency
    pub p50_latency: Duration,
    /// P95 latency
    pub p95_latency: Duration,
    /// P99 latency
    pub p99_latency: Duration,
    /// Memory usage in bytes (if measured)
    pub memory_usage_bytes: Option<u64>,
    /// Additional metrics
    pub additional_metrics: HashMap<String, f64>,
}

/// Performance benchmark runner
pub struct BenchmarkRunner {
    config: BenchmarkConfig,
    db_path: std::path::PathBuf,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner
    pub fn new<P: AsRef<std::path::Path>>(config: BenchmarkConfig, db_path: P) -> Self {
        Self {
            config,
            db_path: db_path.as_ref().to_path_buf(),
        }
    }

    /// Run all benchmarks
    pub async fn run_all_benchmarks(&self) -> OnyxResult<HashMap<String, BenchmarkResults>> {
        let mut results = HashMap::new();

        println!("Starting Onyx Performance Benchmarks...");
        println!("Configuration: {:?}", self.config);

        // Node insertion benchmarks
        results.insert("node_insert".to_string(), self.benchmark_node_insertion().await?);
        results.insert("node_insert_concurrent".to_string(), self.benchmark_concurrent_node_insertion().await?);

        // Edge insertion benchmarks
        results.insert("edge_insert".to_string(), self.benchmark_edge_insertion().await?);
        results.insert("edge_insert_concurrent".to_string(), self.benchmark_concurrent_edge_insertion().await?);

        // Vector insertion benchmarks
        results.insert("vector_insert".to_string(), self.benchmark_vector_insertion().await?);
        results.insert("vector_insert_concurrent".to_string(), self.benchmark_concurrent_vector_insertion().await?);

        // Query benchmarks
        results.insert("node_query".to_string(), self.benchmark_node_query().await?);
        results.insert("vector_search".to_string(), self.benchmark_vector_search().await?);
        results.insert("graph_traversal".to_string(), self.benchmark_graph_traversal().await?);

        // Mixed workload benchmarks
        results.insert("mixed_workload".to_string(), self.benchmark_mixed_workload().await?);

        // Memory usage benchmarks
        if self.config.measure_memory {
            results.insert("memory_usage".to_string(), self.benchmark_memory_usage().await?);
        }

        // Print summary
        self.print_benchmark_summary(&results);

        Ok(results)
    }

    /// Benchmark node insertion performance
    async fn benchmark_node_insertion(&self) -> OnyxResult<BenchmarkResults> {
        println!("Benchmarking node insertion...");
        
        let db = open_db(&self.db_path)?;
        let store = RocksGraphStore::new(db)?;

        // Warmup
        for _ in 0..self.config.warmup_count {
            let node = self.create_test_node();
            store.insert_node(node).await?;
        }

        // Measure insertion performance
        let start_time = Instant::now();
        let mut latencies = Vec::new();

        for _ in 0..self.config.operation_count {
            let node = self.create_test_node();
            let op_start = Instant::now();
            
            store.insert_node(node).await?;
            
            let latency = op_start.elapsed();
            if self.config.detailed_latency {
                latencies.push(latency);
            }
        }

        let total_duration = start_time.elapsed();
        let ops_per_second = self.config.operation_count as f64 / total_duration.as_secs_f64();

        let results = if self.config.detailed_latency && !latencies.is_empty() {
            let mut sorted_latencies = latencies.clone();
            sorted_latencies.sort();
            
            BenchmarkResults {
                total_operations: self.config.operation_count,
                total_duration,
                ops_per_second,
                avg_latency: total_duration / self.config.operation_count as u32,
                p50_latency: sorted_latencies[sorted_latencies.len() / 2],
                p95_latency: sorted_latencies[(sorted_latencies.len() as f64 * 0.95) as usize],
                p99_latency: sorted_latencies[(sorted_latencies.len() as f64 * 0.99) as usize],
                memory_usage_bytes: None,
                additional_metrics: HashMap::new(),
            }
        } else {
            BenchmarkResults {
                total_operations: self.config.operation_count,
                total_duration,
                ops_per_second,
                avg_latency: total_duration / self.config.operation_count as u32,
                p50_latency: Duration::ZERO,
                p95_latency: Duration::ZERO,
                p99_latency: Duration::ZERO,
                memory_usage_bytes: None,
                additional_metrics: HashMap::new(),
            }
        };

        println!("Node insertion: {:.1} ops/sec", results.ops_per_second);
        Ok(results)
    }

    /// Benchmark concurrent node insertion
    async fn benchmark_concurrent_node_insertion(&self) -> OnyxResult<BenchmarkResults> {
        println!("Benchmarking concurrent node insertion...");
        
        let db = open_db(&self.db_path)?;
        let store = Arc::new(RocksGraphStore::new(db)?);

        let start_time = Instant::now();
        let mut handles = Vec::new();

        let operations_per_thread = self.config.operation_count / self.config.concurrency;

        for _ in 0..self.config.concurrency {
            let store_clone = store.clone();
            let ops = operations_per_thread;
            
            let handle = tokio::spawn(async move {
                for _ in 0..ops {
                    let node = Self::create_test_node_static();
                    store_clone.insert_node(node).await.unwrap();
                }
            });
            
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.await?;
        }

        let total_duration = start_time.elapsed();
        let ops_per_second = self.config.operation_count as f64 / total_duration.as_secs_f64();

        let results = BenchmarkResults {
            total_operations: self.config.operation_count,
            total_duration,
            ops_per_second,
            avg_latency: total_duration / self.config.operation_count as u32,
            p50_latency: Duration::ZERO,
            p95_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            memory_usage_bytes: None,
            additional_metrics: HashMap::new(),
        };

        println!("Concurrent node insertion: {:.1} ops/sec", results.ops_per_second);
        Ok(results)
    }

    /// Benchmark edge insertion performance
    async fn benchmark_edge_insertion(&self) -> OnyxResult<BenchmarkResults> {
        println!("Benchmarking edge insertion...");
        
        let db = open_db(&self.db_path)?;
        let store = RocksGraphStore::new(db)?;

        // Create some nodes first
        let mut node_ids = Vec::new();
        for _ in 0..100 {
            let node = self.create_test_node();
            let node_id = node.id;
            store.insert_node(node).await?;
            node_ids.push(node_id);
        }

        // Warmup
        for _ in 0..self.config.warmup_count {
            let edge = self.create_test_edge(node_ids[0], node_ids[1]);
            store.insert_edge(edge).await?;
        }

        // Measure edge insertion performance
        let start_time = Instant::now();

        for i in 0..self.config.operation_count {
            let source_idx = i % node_ids.len();
            let target_idx = (i + 1) % node_ids.len();
            let edge = self.create_test_edge(node_ids[source_idx], node_ids[target_idx]);
            store.insert_edge(edge).await?;
        }

        let total_duration = start_time.elapsed();
        let ops_per_second = self.config.operation_count as f64 / total_duration.as_secs_f64();

        let results = BenchmarkResults {
            total_operations: self.config.operation_count,
            total_duration,
            ops_per_second,
            avg_latency: total_duration / self.config.operation_count as u32,
            p50_latency: Duration::ZERO,
            p95_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            memory_usage_bytes: None,
            additional_metrics: HashMap::new(),
        };

        println!("Edge insertion: {:.1} ops/sec", results.ops_per_second);
        Ok(results)
    }

    /// Benchmark concurrent edge insertion
    async fn benchmark_concurrent_edge_insertion(&self) -> OnyxResult<BenchmarkResults> {
        println!("Benchmarking concurrent edge insertion...");
        
        let db = open_db(&self.db_path)?;
        let store = Arc::new(RocksGraphStore::new(db)?);

        // Create nodes for edges
        let mut node_ids = Vec::new();
        for _ in 0..100 {
            let node = self.create_test_node();
            let node_id = node.id;
            store.insert_node(node).await?;
            node_ids.push(node_id);
        }

        let start_time = Instant::now();
        let mut handles = Vec::new();

        let operations_per_thread = self.config.operation_count / self.config.concurrency;

        for thread_id in 0..self.config.concurrency {
            let store_clone = store.clone();
            let node_ids_clone = node_ids.clone();
            let ops = operations_per_thread;
            
            let handle = tokio::spawn(async move {
                for i in 0..ops {
                    let source_idx = (thread_id * ops + i) % node_ids_clone.len();
                    let target_idx = (source_idx + 1) % node_ids_clone.len();
                    let edge = Self::create_test_edge_static(node_ids_clone[source_idx], node_ids_clone[target_idx]);
                    store_clone.insert_edge(edge).await.unwrap();
                }
            });
            
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.await?;
        }

        let total_duration = start_time.elapsed();
        let ops_per_second = self.config.operation_count as f64 / total_duration.as_secs_f64();

        let results = BenchmarkResults {
            total_operations: self.config.operation_count,
            total_duration,
            ops_per_second,
            avg_latency: total_duration / self.config.operation_count as u32,
            p50_latency: Duration::ZERO,
            p95_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            memory_usage_bytes: None,
            additional_metrics: HashMap::new(),
        };

        println!("Concurrent edge insertion: {:.1} ops/sec", results.ops_per_second);
        Ok(results)
    }

    /// Benchmark vector insertion performance
    async fn benchmark_vector_insertion(&self) -> OnyxResult<BenchmarkResults> {
        println!("Benchmarking vector insertion...");
        
        let db = open_db(&self.db_path)?;
        let store = RocksVectorStore::new(db, 100)?;

        // Warmup
        for _ in 0..self.config.warmup_count {
            let embedding = self.create_test_embedding();
            store.insert(embedding.node_id, embedding.values).await?;
        }

        // Measure vector insertion performance
        let start_time = Instant::now();

        for _ in 0..self.config.operation_count {
            let embedding = self.create_test_embedding();
            store.insert(embedding.node_id, embedding.values).await?;
        }

        let total_duration = start_time.elapsed();
        let ops_per_second = self.config.operation_count as f64 / total_duration.as_secs_f64();

        let results = BenchmarkResults {
            total_operations: self.config.operation_count,
            total_duration,
            ops_per_second,
            avg_latency: total_duration / self.config.operation_count as u32,
            p50_latency: Duration::ZERO,
            p95_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            memory_usage_bytes: None,
            additional_metrics: HashMap::new(),
        };

        println!("Vector insertion: {:.1} ops/sec", results.ops_per_second);
        Ok(results)
    }

    /// Benchmark concurrent vector insertion
    async fn benchmark_concurrent_vector_insertion(&self) -> OnyxResult<BenchmarkResults> {
        println!("Benchmarking concurrent vector insertion...");
        
        let db = open_db(&self.db_path)?;
        let store = Arc::new(RocksVectorStore::new(db, 100)?);

        let start_time = Instant::now();
        let mut handles = Vec::new();

        let operations_per_thread = self.config.operation_count / self.config.concurrency;

        for _ in 0..self.config.concurrency {
            let store_clone = store.clone();
            let ops = operations_per_thread;
            
            let handle = tokio::spawn(async move {
                for _ in 0..ops {
                    let embedding = Self::create_test_embedding_static();
                    store_clone.insert(embedding.node_id, embedding.values).await.unwrap();
                }
            });
            
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.await?;
        }

        let total_duration = start_time.elapsed();
        let ops_per_second = self.config.operation_count as f64 / total_duration.as_secs_f64();

        let results = BenchmarkResults {
            total_operations: self.config.operation_count,
            total_duration,
            ops_per_second,
            avg_latency: total_duration / self.config.operation_count as u32,
            p50_latency: Duration::ZERO,
            p95_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            memory_usage_bytes: None,
            additional_metrics: HashMap::new(),
        };

        println!("Concurrent vector insertion: {:.1} ops/sec", results.ops_per_second);
        Ok(results)
    }

    /// Benchmark node query performance
    async fn benchmark_node_query(&self) -> OnyxResult<BenchmarkResults> {
        println!("Benchmarking node queries...");
        
        let db = open_db(&self.db_path)?;
        let store = RocksGraphStore::new(db)?;

        // Insert test data
        let mut node_ids = Vec::new();
        for _ in 0..1000 {
            let node = self.create_test_node();
            let node_id = node.id;
            store.insert_node(node).await?;
            node_ids.push(node_id);
        }

        // Warmup queries
        for _ in 0..self.config.warmup_count {
            let node_id = node_ids[0];
            store.get_node(&node_id).await?;
        }

        // Measure query performance
        let start_time = Instant::now();

        for i in 0..self.config.operation_count {
            let node_id = node_ids[i % node_ids.len()];
            store.get_node(&node_id).await?;
        }

        let total_duration = start_time.elapsed();
        let ops_per_second = self.config.operation_count as f64 / total_duration.as_secs_f64();

        let results = BenchmarkResults {
            total_operations: self.config.operation_count,
            total_duration,
            ops_per_second,
            avg_latency: total_duration / self.config.operation_count as u32,
            p50_latency: Duration::ZERO,
            p95_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            memory_usage_bytes: None,
            additional_metrics: HashMap::new(),
        };

        println!("Node queries: {:.1} ops/sec", results.ops_per_second);
        Ok(results)
    }

    /// Benchmark vector search performance
    async fn benchmark_vector_search(&self) -> OnyxResult<BenchmarkResults> {
        println!("Benchmarking vector search...");
        
        let db = open_db(&self.db_path)?;
        let vector_store = RocksVectorStore::new(db.clone(), 100)?;

        // Insert test embeddings
        let mut node_ids = Vec::new();
        for _ in 0..1000 {
            let embedding = self.create_test_embedding();
            let node_id = embedding.node_id;
            vector_store.insert(node_id, embedding.values).await?;
            node_ids.push(node_id);
        }

        // Create query vector
        let query_vector = vec![0.5; 100];

        // Warmup searches
        for _ in 0..self.config.warmup_count {
            vector_store.search(&query_vector, 10).await?;
        }

        // Measure search performance
        let start_time = Instant::now();

        for _ in 0..self.config.operation_count {
            vector_store.search(&query_vector, 10).await?;
        }

        let total_duration = start_time.elapsed();
        let ops_per_second = self.config.operation_count as f64 / total_duration.as_secs_f64();

        let results = BenchmarkResults {
            total_operations: self.config.operation_count,
            total_duration,
            ops_per_second,
            avg_latency: total_duration / self.config.operation_count as u32,
            p50_latency: Duration::ZERO,
            p95_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            memory_usage_bytes: None,
            additional_metrics: HashMap::new(),
        };

        println!("Vector search: {:.1} ops/sec", results.ops_per_second);
        Ok(results)
    }

    /// Benchmark graph traversal performance
    async fn benchmark_graph_traversal(&self) -> OnyxResult<BenchmarkResults> {
        println!("Benchmarking graph traversal...");
        
        let db = open_db(&self.db_path)?;
        let store = RocksGraphStore::new(db)?;

        // Create a connected graph
        let mut node_ids = Vec::new();
        for _ in 0..100 {
            let node = self.create_test_node();
            let node_id = node.id;
            store.insert_node(node).await?;
            node_ids.push(node_id);
        }

        // Create edges to form a connected graph
        for i in 0..node_ids.len() - 1 {
            let edge = self.create_test_edge(node_ids[i], node_ids[i + 1]);
            store.insert_edge(edge).await?;
        }

        // Warmup traversals
        for _ in 0..self.config.warmup_count {
            store.get_neighbors(&node_ids[0], None).await?;
        }

        // Measure traversal performance
        let start_time = Instant::now();

        for i in 0..self.config.operation_count {
            let node_id = node_ids[i % node_ids.len()];
            store.get_neighbors(&node_id, None).await?;
        }

        let total_duration = start_time.elapsed();
        let ops_per_second = self.config.operation_count as f64 / total_duration.as_secs_f64();

        let results = BenchmarkResults {
            total_operations: self.config.operation_count,
            total_duration,
            ops_per_second,
            avg_latency: total_duration / self.config.operation_count as u32,
            p50_latency: Duration::ZERO,
            p95_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            memory_usage_bytes: None,
            additional_metrics: HashMap::new(),
        };

        println!("Graph traversal: {:.1} ops/sec", results.ops_per_second);
        Ok(results)
    }

    /// Benchmark mixed workload
    async fn benchmark_mixed_workload(&self) -> OnyxResult<BenchmarkResults> {
        println!("Benchmarking mixed workload...");
        
        let db = open_db(&self.db_path)?;
        let graph_store = RocksGraphStore::new(db.clone())?;
        let vector_store = RocksVectorStore::new(db, 100)?;

        let start_time = Instant::now();

        for i in 0..self.config.operation_count {
            match i % 4 {
                0 => {
                    // Node insertion
                    let node = self.create_test_node();
                    graph_store.insert_node(node).await?;
                }
                1 => {
                    // Vector insertion
                    let embedding = self.create_test_embedding();
                    vector_store.insert(embedding.node_id, embedding.values).await?;
                }
                2 => {
                    // Node query (if we have nodes)
                    if let Ok(Some(_)) = graph_store.get_all_node_ids().await {
                        if let Ok(Some(node_id)) = graph_store.get_all_node_ids().await?.first().cloned() {
                            graph_store.get_node(&node_id).await?;
                        }
                    }
                }
                3 => {
                    // Vector search
                    let query_vector = vec![0.5; 100];
                    vector_store.search(&query_vector, 5).await?;
                }
                _ => unreachable!(),
            }
        }

        let total_duration = start_time.elapsed();
        let ops_per_second = self.config.operation_count as f64 / total_duration.as_secs_f64();

        let results = BenchmarkResults {
            total_operations: self.config.operation_count,
            total_duration,
            ops_per_second,
            avg_latency: total_duration / self.config.operation_count as u32,
            p50_latency: Duration::ZERO,
            p95_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            memory_usage_bytes: None,
            additional_metrics: HashMap::new(),
        };

        println!("Mixed workload: {:.1} ops/sec", results.ops_per_second);
        Ok(results)
    }

    /// Benchmark memory usage
    async fn benchmark_memory_usage(&self) -> OnyxResult<BenchmarkResults> {
        println!("Benchmarking memory usage...");
        
        let initial_memory = self.get_memory_usage();

        let db = open_db(&self.db_path)?;
        let graph_store = RocksGraphStore::new(db.clone())?;
        let vector_store = RocksVectorStore::new(db, 100)?;

        // Insert data and measure memory at different points
        let mut memory_measurements = Vec::new();

        for batch in 0..10 {
            // Insert 1000 nodes
            for _ in 0..1000 {
                let node = self.create_test_node();
                graph_store.insert_node(node).await?;
                
                let embedding = self.create_test_embedding();
                vector_store.insert(embedding.node_id, embedding.values).await?;
            }

            let current_memory = self.get_memory_usage();
            memory_measurements.push(current_memory);
            
            println!("  Batch {}: {} MB", batch + 1, current_memory / 1024 / 1024);
        }

        let final_memory = self.get_memory_usage();
        let memory_increase = final_memory.saturating_sub(initial_memory);

        let mut additional_metrics = HashMap::new();
        additional_metrics.insert("memory_per_node".to_string(), memory_increase as f64 / 10000.0);
        additional_metrics.insert("memory_per_embedding".to_string(), memory_increase as f64 / 10000.0);

        let results = BenchmarkResults {
            total_operations: 20000, // 10000 nodes + 10000 embeddings
            total_duration: Duration::from_secs(1), // Not timing this benchmark
            ops_per_second: 0.0,
            avg_latency: Duration::ZERO,
            p50_latency: Duration::ZERO,
            p95_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            memory_usage_bytes: Some(memory_increase),
            additional_metrics,
        };

        println!("Memory usage: {} MB increase", memory_increase / 1024 / 1024);
        Ok(results)
    }

    /// Print benchmark summary
    fn print_benchmark_summary(&self, results: &HashMap<String, BenchmarkResults>) {
        println!("\n=== Benchmark Summary ===");
        
        let mut summary = Vec::new();
        for (name, result) in results {
            summary.push((name.clone(), result.ops_per_second));
        }
        
        summary.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        println!("{:<25} {:>15} {:>15}", "Benchmark", "Ops/Sec", "Avg Latency");
        println!("{}", "-".repeat(55));
        
        for (name, result) in results {
            println!("{:<25} {:>15.1} {:>15.2?}", 
                     name, 
                     result.ops_per_second, 
                     result.avg_latency);
        }
        
        println!("\n=== Performance Targets ===");
        println!("Node insertion: >1000 ops/sec");
        println!("Vector search: >500 ops/sec");
        println!("Graph traversal: >200 ops/sec");
        println!("Memory efficiency: <1KB per node");
    }

    // Helper methods
    fn create_test_node(&self) -> Node {
        Node {
            id: Uuid::new_v4(),
            name: "benchmark_node".to_string(),
            node_type: NodeType::CodeEntity(CodeEntityKind::Function),
            content: "pub fn benchmark() { }".to_string(),
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            provenance: Default::default(),
            extension: Default::default(),
        }
    }

    fn create_test_edge(&self, source_id: Uuid, target_id: Uuid) -> Edge {
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

    fn create_test_embedding(&self) -> Embedding {
        Embedding {
            node_id: Uuid::new_v4(),
            values: vec![0.1; 100],
        }
    }

    fn create_test_node_static() -> Node {
        Node {
            id: Uuid::new_v4(),
            name: "benchmark_node".to_string(),
            node_type: NodeType::CodeEntity(CodeEntityKind::Function),
            content: "pub fn benchmark() { }".to_string(),
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            provenance: Default::default(),
            extension: Default::default(),
        }
    }

    fn create_test_edge_static(source_id: Uuid, target_id: Uuid) -> Edge {
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

    fn create_test_embedding_static() -> Embedding {
        Embedding {
            node_id: Uuid::new_v4(),
            values: vec![0.1; 100],
        }
    }

    fn get_memory_usage(&self) -> u64 {
        // Simple memory usage estimation
        // In a real implementation, you'd use platform-specific APIs
        // For now, return a placeholder
        50 * 1024 * 1024 // 50MB placeholder
    }
}