//! Persistent storage implementations using RocksDB.
//!
//! This module provides production-grade persistent storage for nodes, edges,
//! embeddings, and version history using RocksDB with column families.
//!
//! This module is only available when the `rocksdb-storage` feature is enabled.

#[cfg(feature = "rocksdb-storage")]
pub mod rocks_graph;
#[cfg(feature = "rocksdb-storage")]
pub mod rocks_history;
#[cfg(feature = "rocksdb-storage")]
pub mod rocks_vector;

#[cfg(feature = "rocksdb-storage")]
pub use rocks_graph::RocksGraphStore;
#[cfg(feature = "rocksdb-storage")]
pub use rocks_history::RocksHistoryStore;
#[cfg(feature = "rocksdb-storage")]
pub use rocks_vector::RocksVectorStore;

#[cfg(feature = "rocksdb-storage")]
use rocksdb::{Options, DB};
#[cfg(feature = "rocksdb-storage")]
use std::path::Path;
#[cfg(feature = "rocksdb-storage")]
use std::sync::Arc;

#[cfg(feature = "rocksdb-storage")]
use crate::error::{OnyxError, OnyxResult};

/// Column family names
#[cfg(feature = "rocksdb-storage")]
pub const CF_NODES: &str = "nodes";
#[cfg(feature = "rocksdb-storage")]
pub const CF_EDGES: &str = "edges";
#[cfg(feature = "rocksdb-storage")]
pub const CF_NODE_OUTBOUND: &str = "node_outbound";
#[cfg(feature = "rocksdb-storage")]
pub const CF_NODE_INBOUND: &str = "node_inbound";
#[cfg(feature = "rocksdb-storage")]
pub const CF_EMBEDDINGS: &str = "embeddings";
#[cfg(feature = "rocksdb-storage")]
pub const CF_HNSW_LAYERS: &str = "hnsw_layers";
#[cfg(feature = "rocksdb-storage")]
pub const CF_VERSIONS: &str = "versions";
#[cfg(feature = "rocksdb-storage")]
pub const CF_VERSION_CHAINS: &str = "version_chains";
#[cfg(feature = "rocksdb-storage")]
pub const CF_BRANCHES: &str = "branches";

/// Opens a RocksDB instance with all required column families.
#[cfg(feature = "rocksdb-storage")]
pub fn open_db<P: AsRef<Path>>(path: P) -> OnyxResult<Arc<DB>> {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);

    let column_families = vec![
        CF_NODES,
        CF_EDGES,
        CF_NODE_OUTBOUND,
        CF_NODE_INBOUND,
        CF_EMBEDDINGS,
        CF_HNSW_LAYERS,
        CF_VERSIONS,
        CF_VERSION_CHAINS,
        CF_BRANCHES,
    ];

    let db = DB::open_cf(&opts, path, &column_families)
        .map_err(|e| OnyxError::Internal(format!("Failed to open RocksDB: {}", e)))?;

    Ok(Arc::new(db))
}
