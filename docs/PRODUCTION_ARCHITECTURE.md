# Onyx Production Architecture

## Overview

This document defines the production architecture for Onyx, upgrading from the prototype to a scalable, multi-tenant, production-grade system supporting concurrent users, persistent storage, real-time collaboration, and distributed deployment.

---

## System Layers

```
┌─────────────────────────────────────────────────────────────┐
│                     Web Frontend (React)                     │
│  Graph Explorer | Search Console | Code Browser | Analytics │
└────────────────┬────────────────────────────────────────────┘
                 │ WebSocket + REST
┌────────────────▼────────────────────────────────────────────┐
│                   Axum API Server (Rust)                     │
│  ├─ Auth Middleware (JWT + API Keys)                        │
│  ├─ Rate Limiting & Quotas                                  │
│  ├─ WebSocket Manager (real-time updates)                   │
│  └─ REST Handlers (query, ingest, traverse, admin)          │
└────────────────┬────────────────────────────────────────────┘
                 │
┌────────────────▼────────────────────────────────────────────┐
│                     Query Engine (Rust)                      │
│  ├─ Query Planner (cost-based optimization)                 │
│  ├─ Multi-hop Resolver (BFS/DFS with pruning)               │
│  ├─ Result Fusion (score blending, ranking)                 │
│  └─ Caching Layer (LRU cache for hot queries)               │
└────────────────┬────────────────────────────────────────────┘
                 │
┌────────────────▼────────────────────────────────────────────┐
│              Transaction Coordinator (Rust)                  │
│  ├─ MVCC Snapshot Isolation                                 │
│  ├─ Write-Ahead Log (persistent, crash recovery)            │
│  ├─ Distributed Lock Manager (for multi-instance)           │
│  └─ Background Compaction Worker                            │
└────┬───────────┬──────────────┬──────────────┬──────────────┘
     │           │              │              │
┌────▼─────┐ ┌──▼───────┐ ┌───▼──────┐ ┌────▼─────────┐
│ Vector   │ │  Graph   │ │ History  │ │ Metadata     │
│ Store    │ │  Store   │ │ Store    │ │ Store        │
│ (HNSW)   │ │ (RocksDB)│ │(RocksDB) │ │ (RocksDB)    │
└──────────┘ └──────────┘ └──────────┘ └──────────────┘
     │            │            │              │
┌────▼────────────▼────────────▼──────────────▼───────────────┐
│                    RocksDB (Persistent KV Store)             │
│  ├─ Column Families: nodes, edges, versions, vectors, meta  │
│  ├─ WAL for durability                                       │
│  └─ Compaction + Background threads                          │
└──────────────────────────────────────────────────────────────┘
```

---

## Storage Architecture

### RocksDB Layout

All data is persisted in RocksDB with separate column families for isolation:

| Column Family | Purpose | Key Format | Value Format |
|---|---|---|---|
| `nodes` | Node metadata | `node:{uuid}` | Bincode-serialized `Node` |
| `edges` | Edge metadata | `edge:{uuid}` | Bincode-serialized `Edge` |
| `versions` | Version history | `version:{version_id}` | Bincode-serialized `VersionEntry` |
| `entity_versions` | Entity → version index | `entity_versions:{uuid}` | Vec<VersionId> |
| `adjacency_out` | Outbound edges index | `adj_out:{uuid}:{edge_type}` | Vec<Uuid> (edge IDs) |
| `adjacency_in` | Inbound edges index | `adj_in:{uuid}:{edge_type}` | Vec<Uuid> (edge IDs) |
| `vectors` | Vector embeddings | `vec:{uuid}` | Vec<f32> |
| `hnsw_layers` | HNSW index layers | `hnsw:layer:{N}:{uuid}` | Vec<Uuid> (neighbors) |
| `metadata` | System metadata | `meta:{key}` | String (JSON) |

**Compaction Strategy**:
- Level-based compaction for all column families
- Background threads: 4 compaction, 2 flush
- Block cache: 512 MB (shared across CFs)
- Write buffer: 64 MB per CF

### Vector Index (HNSW)

**Production HNSW Implementation**:
- **Algorithm**: Hierarchical Navigable Small World
- **Parameters**:
  - `M = 16` (connections per layer)
  - `ef_construction = 200` (build quality)
  - `ef_search = 100` (query quality)
- **Storage**: Layers persisted in `hnsw_layers` CF
- **Distance**: Cosine similarity (dot product on normalized vectors)
- **Concurrency**: Read-mostly workload, write lock on insert

**Index Building**:
- Incremental: insert one node at a time during ingestion
- Batch rebuild: background job for large imports
- Compaction: prune weak edges periodically

---

## Embedding Strategy

### Model: `all-MiniLM-L6-v2`

- **Dimensions**: 384
- **Architecture**: Sentence-BERT (SBERT)
- **Inference**: ONNX Runtime (Rust binding via `ort` crate)
- **Model file**: `models/all-MiniLM-L6-v2.onnx` (bundled in binary or downloaded on first run)

### Preprocessing Pipeline

```rust
fn generate_embedding(text: &str) -> Vec<f32> {
    // 1. Tokenize with BERT tokenizer
    let tokens = tokenizer.encode(text, max_length=512);
    
    // 2. Run ONNX inference
    let output = model.run(tokens);
    
    // 3. Mean pooling over token embeddings
    let embedding = mean_pool(&output, &attention_mask);
    
    // 4. L2 normalize
    normalize(&embedding)
}
```

### Batch Inference

For large codebases (>1000 files):
- Batch size: 32
- GPU support via CUDA (optional, falls back to CPU)
- Progress reporting via WebSocket

---

## Parsing Strategy: Tree-sitter

### Supported Languages

| Language | Grammar | Extracts |
|---|---|---|
| Rust | `tree-sitter-rust` | fns, structs, enums, traits, impls, mods |
| Python | `tree-sitter-python` | fns, classes, methods, imports |
| TypeScript | `tree-sitter-typescript` | fns, classes, interfaces, exports |
| JavaScript | `tree-sitter-javascript` | fns, classes, exports |
| Go | `tree-sitter-go` | fns, structs, interfaces, imports |
| Java | `tree-sitter-java` | classes, methods, imports |

### AST Traversal

```rust
fn parse_file(path: &Path, lang: Language) -> Vec<CodeUnit> {
    let source = read_file(path)?;
    let mut parser = Parser::new();
    parser.set_language(lang.grammar())?;
    
    let tree = parser.parse(&source, None)?;
    let root = tree.root_node();
    
    let mut units = Vec::new();
    traverse_ast(root, &source, &mut units);
    units
}
```

**Relationship Extraction**:
- **Calls**: Query `call_expression` nodes, resolve target
- **Imports**: Query `import_statement` nodes
- **Contains**: Derive from AST parent-child structure

---

## Concurrency: MVCC Snapshot Isolation

### Write Path

```rust
pub struct Transaction {
    snapshot_id: u64,
    read_set: HashSet<Uuid>,
    write_set: Vec<Operation>,
    status: TxStatus,
}
```

**Protocol**:
1. Begin: Allocate snapshot ID from monotonic counter
2. Read: All reads see snapshot at `snapshot_id`
3. Write: Buffer writes in `write_set`
4. Commit: Acquire write lock, validate read set, apply writes to WAL, release lock
5. Abort: Discard `write_set`, retry

**Conflict Detection**:
- Read-write conflicts: Check if any read node was modified since snapshot
- Write-write conflicts: Last-writer-wins (version numbers)

### Read Path

Reads are lock-free:
- Query snapshot at current global version
- MVCC chains: store multiple versions per key with timestamps
- Garbage collection: background thread prunes old versions

---

## API Server: Axum

### Endpoints

#### REST

| Method | Path | Description |
|---|---|---|
| POST | `/api/v1/ingest` | Ingest code (file upload or git URL) |
| POST | `/api/v1/query` | Semantic search + graph expansion |
| POST | `/api/v1/traverse` | Graph traversal from a node |
| GET | `/api/v1/nodes/:id` | Get node details |
| GET | `/api/v1/edges/:id` | Get edge details |
| GET | `/api/v1/history/:id` | Get version history |
| GET | `/api/v1/impact/:id` | Impact analysis |
| GET | `/api/v1/tests/:id` | Find covering tests |
| GET | `/api/v1/stats` | Store statistics |
| POST | `/api/v1/auth/login` | Authenticate (returns JWT) |
| POST | `/api/v1/projects` | Create project |
| GET | `/api/v1/projects` | List projects |

#### WebSocket

| Event | Direction | Payload |
|---|---|---|
| `subscribe` | Client → Server | `{project_id, filters}` |
| `node_updated` | Server → Client | `{node, diff}` |
| `edge_added` | Server → Client | `{edge}` |
| `ingest_progress` | Server → Client | `{files_done, files_total}` |
| `query_results` | Server → Client | `{results, streaming=true}` |

### Middleware Stack

```rust
Router::new()
    .route("/api/v1/query", post(query_handler))
    .layer(AuthLayer::new())
    .layer(RateLimitLayer::new(100, Duration::from_secs(60)))
    .layer(CompressionLayer::new())
    .layer(TracingLayer::new())
```

---

## Authentication & Authorization

### JWT Tokens

**Claims**:
```json
{
  "sub": "user_id",
  "exp": 1735689600,
  "roles": ["admin", "developer"],
  "projects": ["proj_123", "proj_456"]
}
```

**Generation**:
- RS256 (RSA with SHA-256)
- Private key: 2048-bit RSA, stored in `config/jwt_private.pem`
- Public key: distributed to clients

### API Keys

For programmatic access:
- Format: `hx_live_abcdef123456789` (prefix + random)
- Stored hashed (Argon2id) in metadata store
- Scoped to projects + permissions

### RBAC

| Role | Permissions |
|---|---|
| `viewer` | Read nodes/edges/history |
| `developer` | viewer + ingest code + query |
| `admin` | developer + manage users + delete data |

---

## Monitoring & Observability

### Metrics (Prometheus)

- **Query metrics**: `helix_query_duration_seconds{query_type}`
- **Store metrics**: `helix_store_size_bytes{store_type}`
- **Ingestion metrics**: `helix_ingest_files_total`, `helix_ingest_duration_seconds`
- **HNSW metrics**: `helix_hnsw_search_latency`, `helix_hnsw_index_size`
- **Transaction metrics**: `helix_tx_commits_total`, `helix_tx_aborts_total`

### Logging (tracing)

- **Structured logs**: JSON format via `tracing-subscriber`
- **Levels**: ERROR (alerts), WARN (review daily), INFO (audit trail), DEBUG (dev only)
- **Spans**: Distributed tracing with trace IDs for multi-hop queries

### Health Checks

- `/health/live`: Always returns 200 (k8s liveness probe)
- `/health/ready`: Returns 200 if RocksDB + HNSW loaded (k8s readiness probe)

---

## Deployment

### Docker Compose (Single Node)

```yaml
version: '3.8'
services:
  helix:
    image: helix:latest
    ports:
      - "8080:8080"
    volumes:
      - ./data:/data
      - ./models:/models
    environment:
      - HELIX_DB_PATH=/data/rocksdb
      - HELIX_MODEL_PATH=/models/all-MiniLM-L6-v2.onnx
      - RUST_LOG=info
```

### Kubernetes (Distributed)

- **StatefulSet**: 3 replicas with persistent volumes
- **Service**: ClusterIP for internal, LoadBalancer for external
- **Ingress**: NGINX with TLS termination
- **PVC**: 100Gi SSD per replica for RocksDB

### Configuration

`config/helix.toml`:
```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 8

[storage]
rocksdb_path = "/data/rocksdb"
wal_dir = "/data/wal"
cache_size_mb = 512

[embedding]
model_path = "/models/all-MiniLM-L6-v2.onnx"
batch_size = 32
use_gpu = false

[hnsw]
m = 16
ef_construction = 200
ef_search = 100

[auth]
jwt_secret_path = "/config/jwt_private.pem"
session_duration_hours = 24
```

---

## Migration Path from Prototype

1. **Refactor store traits** → Add `PersistentVectorStore`, `PersistentGraphStore`, `PersistentHistoryStore`
2. **Implement RocksDB backends** → Replace `HashMap<Uuid, T>` with RocksDB reads/writes
3. **Replace BagOfWords** → Integrate ONNX Runtime + `all-MiniLM-L6-v2`
4. **Replace brute-force kNN** → Implement HNSW index
5. **Add AST parsing** → Integrate tree-sitter for Rust/Python/TypeScript
6. **Build Axum server** → Expose REST + WebSocket endpoints
7. **Add authentication** → JWT + API key middleware
8. **Frontend development** → React app consuming WebSocket + REST API
9. **Containerization** → Dockerfile + k8s manifests
10. **Production testing** → Benchmark with 100K+ nodes, multi-user load testing

---

## Performance Targets

| Metric | Target | Measurement |
|---|---|---|
| Query latency (p50) | <50ms | Prometheus histogram |
| Query latency (p99) | <200ms | Prometheus histogram |
| Ingest throughput | 100 files/sec | Counter + duration |
| HNSW search (k=10) | <10ms | Custom metric |
| Concurrent users | 100 | Load testing (k6) |
| Storage efficiency | <1MB per 1K LOC | DB size / codebase size |

---

## Security Considerations

- **Input validation**: Sanitize all user input (file paths, queries)
- **Resource limits**: Max query depth, max result size, max file size
- **Secrets management**: Use vault or k8s secrets for JWT keys
- **Network isolation**: Deploy behind VPC, restrict RocksDB ports
- **Audit logging**: Log all mutations (ingest, delete) with user ID

---

This architecture supports:
- ✅ Multi-tenant isolation (project-scoped access)
- ✅ Horizontal scaling (read replicas, sharded HNSW)
- ✅ Crash recovery (RocksDB WAL + transaction log)
- ✅ Real-time collaboration (WebSocket pub/sub)
- ✅ Production-grade embeddings (transformer models via ONNX)
- ✅ Multi-language support (tree-sitter grammars)
