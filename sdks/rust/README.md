# Onyx SDK for Rust

Official Rust client library for the **Onyx** AI infrastructure engine — combining semantic search, knowledge graphs, and temporal versioning in a graph-native vector memory system.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
onyx-sdk = { path = "../sdks/rust" }  # or from crates.io when published
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use onyx_sdk::{OnyxClient, CreateNodeRequest, NodeType, CodeEntityKind};

#[tokio::main]
async fn main() -> Result<(), onyx_sdk::OnyxError> {
    // 1. Create a client
    let client = OnyxClient::builder("http://localhost:3000")
        .api_key("your-api-key")
        .timeout(60)
        .build()?;

    // 2. Health check
    let healthy = client.health().await?;
    println!("Server healthy: {healthy}");

    // 3. Create a node
    let node = client.nodes().create(
        CreateNodeRequest::new("hello_world", "fn hello() { println!(\"hello\"); }")
            .node_type(NodeType::code_entity(CodeEntityKind::Function))
    ).await?;
    println!("Created node: {}", node.id);

    // 4. Semantic search
    let results = client.search().query(
        onyx_sdk::SearchRequest::new(vec![0.1, 0.2, 0.3]).top_k(5)
    ).await?;
    for item in &results.items {
        println!("  {} (score: {:.3})", item.name, item.score);
    }

    Ok(())
}
```

## Sub-Clients

| Sub-client      | Access             | Description                        |
| --------------- | ------------------ | ---------------------------------- |
| `NodesClient`   | `client.nodes()`   | Create, read, update, delete nodes |
| `EdgesClient`   | `client.edges()`   | Manage typed relationships         |
| `SearchClient`  | `client.search()`  | Vector similarity search           |
| `HistoryClient` | `client.history()` | Version history & branching        |
| `IngestClient`  | `client.ingest()`  | Code ingestion pipeline            |
| `BillingClient` | `client.billing()` | Stripe checkout & portal           |

## Node Operations

```rust
use onyx_sdk::*;

// Create
let node = client.nodes().create(
    CreateNodeRequest::new("MyStruct", "pub struct MyStruct { ... }")
        .node_type(NodeType::code_entity(CodeEntityKind::Struct))
        .provenance(Provenance::new("src/lib.rs").with_lines(10, 20))
).await?;

// Read
let fetched = client.nodes().get(node.id).await?;

// Update
let updated = client.nodes().update(node.id, UpdateNodeRequest {
    name: Some("RenamedStruct".into()),
    ..Default::default()
}).await?;

// Delete
client.nodes().delete(node.id).await?;

// List (paginated)
let page = client.nodes().list(1, 50).await?;

// Subgraph
let sub = client.nodes().subgraph(node.id, 2).await?;
```

## Edge Operations

```rust
use onyx_sdk::*;

let edge = client.edges().create(
    CreateEdgeRequest::new(EdgeType::Calls, source_id, target_id)
        .confidence(0.95)
).await?;

client.edges().delete(edge.id).await?;
```

## Semantic Search

```rust
use onyx_sdk::*;

let results = client.search().query(
    SearchRequest::new(embedding_vec)
        .top_k(10)
        .max_depth(3)
        .edge_types(vec![EdgeType::Calls, EdgeType::Imports])
        .include_history(true)
        .min_confidence(0.5)
).await?;

println!("Found {} items in {}ms", results.items.len(), results.query_time_ms);
```

## Version History

```rust
use onyx_sdk::*;

// List versions
let versions = client.history().list_versions(entity_id).await?;

// Create a branch
let branch = client.history().create_branch(CreateBranchRequest {
    name: "feature/new-api".into(),
    base_version: "v-abc123".into(),
}).await?;

// Merge
client.history().merge_branch(MergeBranchRequest {
    source: "feature/new-api".into(),
    target: "main".into(),
}).await?;
```

## Code Ingestion

```rust
use onyx_sdk::*;

// Single unit
let result = client.ingest().ingest_unit(
    IngestCodeUnitRequest::new(
        "process_data",
        "pub fn process_data(input: &str) -> Result<()> { ... }",
        CodeEntityKind::Function,
        Language::Rust,
        "src/processing.rs",
    )
    .line_range(42, 58)
    .signature("pub fn process_data(input: &str) -> Result<()>")
    .visibility(Visibility::Public)
).await?;

// Batch ingestion
let batch = client.ingest().ingest_codebase(IngestCodebaseRequest {
    units: vec![/* ... */],
}).await?;
```

## Billing

```rust
use onyx_sdk::*;

// Create checkout
let session = client.billing().create_checkout_session(
    CheckoutSessionRequest::new()
        .customer_email("user@example.com")
        .price_id("price_xxx")
        .success_url("https://app.example.com/success")
        .cancel_url("https://app.example.com/cancel")
).await?;
println!("Checkout URL: {}", session.url);

// Billing portal
let portal = client.billing().create_billing_portal(
    BillingPortalRequest::new("cus_xxx")
        .return_url("https://app.example.com/settings")
).await?;
```

## Error Handling

All methods return `Result<T, onyx_sdk::OnyxError>`. Error variants:

| Variant              | Description                                       |
| -------------------- | ------------------------------------------------- |
| `ApiError`           | HTTP error from the server (includes status code) |
| `NetworkError`       | Connection / transport failure                    |
| `SerializationError` | JSON serialization issue                          |
| `ConfigError`        | Invalid client configuration                      |
| `NotFound`           | Resource not found (404)                          |
| `InvalidArgument`    | Bad input parameter                               |

```rust
match client.nodes().get(some_id).await {
    Ok(node) => println!("Got: {}", node.name),
    Err(onyx_sdk::OnyxError::NotFound(msg)) => eprintln!("Not found: {msg}"),
    Err(e) => eprintln!("Error: {e}"),
}
```

## License

MIT — see [LICENSE](../../LICENSE) for details.
