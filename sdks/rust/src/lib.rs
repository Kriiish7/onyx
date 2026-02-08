//! # Onyx SDK for Rust
//!
//! The official Rust client library for the **Onyx** AI infrastructure engine.
//! Onyx combines semantic search, knowledge graphs, and temporal versioning in
//! a graph-native vector memory system.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use onyx_sdk::OnyxClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), onyx_sdk::OnyxError> {
//!     // Connect to a local Onyx server
//!     let client = OnyxClient::builder("http://localhost:3000")
//!         .api_key("your-api-key")
//!         .build()?;
//!
//!     // Health check
//!     let healthy = client.health().await?;
//!     println!("Server healthy: {healthy}");
//!
//!     // Create a node
//!     let node = client.nodes().create(
//!         CreateNodeRequest::new("MyFunction", "fn hello() { }")
//!             .node_type(NodeType::code_entity(CodeEntityKind::Function))
//!     ).await?;
//!
//!     // Semantic search
//!     let results = client.search().query(
//!         SearchRequest::new(vec![0.1, 0.2, 0.3]).top_k(5)
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **Node management** — Create, read, update, and delete knowledge graph nodes
//! - **Edge management** — Define typed relationships between nodes
//! - **Semantic search** — Vector similarity search across embeddings
//! - **Version history** — Temporal versioning with branching and merging
//! - **Ingestion** — Ingest code units with automatic relationship detection
//! - **Billing** — Stripe checkout and billing portal integration
//!
//! ## Architecture
//!
//! The SDK is organized into sub-clients accessible from the main [`OnyxClient`]:
//!
//! | Sub-client | Access | Purpose |
//! |------------|--------|---------|
//! | [`NodesClient`] | `client.nodes()` | Node CRUD operations |
//! | [`EdgesClient`] | `client.edges()` | Edge CRUD operations |
//! | [`SearchClient`] | `client.search()` | Vector similarity search |
//! | [`HistoryClient`] | `client.history()` | Version history & branching |
//! | [`IngestClient`] | `client.ingest()` | Code ingestion pipeline |
//! | [`BillingClient`] | `client.billing()` | Stripe billing integration |

pub mod client;
pub mod error;
pub mod models;

pub use client::{OnyxClient, OnyxClientBuilder};
pub use error::OnyxError;
pub use models::*;
