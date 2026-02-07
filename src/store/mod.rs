pub mod benchmark;
pub mod crash_recovery;
pub mod graph;
pub mod history;
pub mod migration;
pub mod persistent;
pub mod transaction;
pub mod vector;

pub use graph::{GraphStore, SurrealGraphStore, SubgraphResult, TraversalResult};
pub use history::{HistoryStore, SurrealHistoryStore};
pub use migration::{run_migration, MigrationConfig, MigrationStats, StorageMigrator};
pub use transaction::TransactionManager;
pub use vector::{SurrealVectorStore, VectorStore};

#[cfg(feature = "rocksdb-storage")]
pub use persistent::{open_db, RocksGraphStore, RocksHistoryStore, RocksVectorStore};
