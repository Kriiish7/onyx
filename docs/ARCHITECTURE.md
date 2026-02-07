# Onyx Architecture

## Graph-Native Vector Memory for AI Agents

---

## System Overview

Onyx is a Rust-native infrastructure engine that fuses three storage layers into a
unified retrieval system for AI agents operating on interconnected codebases:

```
                    ┌─────────────────────────────────────────────┐
                    │              Onyx Public API                 │
                    │  ingest() | query() | traverse() | snapshot()│
                    └──────────┬──────────────────────┬───────────┘
                               │                      │
                    ┌──────────▼──────────┐  ┌───────▼──────────┐
                    │   Query Engine       │  │  Ingestion Engine │
                    │  - Plan generation   │  │  - Code parsing   │
                    │  - Multi-hop resolve │  │  - Embedding gen  │
                    │  - Result fusion     │  │  - Relation extract│
                    │  - Reasoning layer   │  │  - Version diffing │
                    └──────────┬──────────┘  └───────┬──────────┘
                               │                      │
              ┌────────────────┼──────────────────────┼────────────┐
              │                │    Unified Store      │            │
              │  ┌─────────────▼────────────┐         │            │
              │  │     Transaction Manager   │         │            │
              │  │  (cross-store consistency) │        │            │
              │  └──┬──────────┬──────────┬─┘         │            │
              │     │          │          │            │            │
              │  ┌──▼───┐  ┌──▼───┐  ┌───▼────┐      │            │
              │  │Vector │  │Graph │  │History │      │            │
              │  │Store  │  │Store │  │Store   │      │            │
              │  │       │  │      │  │        │      │            │
              │  │HNSW   │  │Adj.  │  │Diff    │      │            │
              │  │Index   │  │Lists │  │Chains  │      │            │
              │  │Embeddings│Relations│Snapshots│      │            │
              │  └────────┘  └──────┘  └────────┘      │            │
              └────────────────────────────────────────┘            │
                                                                    │
              ┌─────────────────────────────────────────────────────┘
              │
              ▼
    ┌───────────────────┐
    │   CLI / REPL      │
    │  - Load artifacts  │
    │  - Inspect graphs  │
    │  - Run queries     │
    │  - Agent simulation│
    └───────────────────┘
```

---

## Component Descriptions

### 1. Public API Layer (`onyx::api`)

The external interface for all Onyx operations. Four primary entry points:

- **`ingest(code_unit, metadata)`**: Accepts a code artifact (function, module, doc, test,
  config), parses it, generates embeddings, extracts relationships, and atomically writes
  to all three stores.

- **`query(question, context_options)`**: Accepts a natural-language or structured query
  with options (depth, relation filters, time range). Plans a retrieval strategy across
  vector similarity and graph traversal, fuses results, and returns a coherent context
  bundle.

- **`traverse(start_node, relation_types, depth)`**: Direct graph traversal from a node
  following specified edge types to a given depth. Returns the subgraph as a structured
  result.

- **`snapshot(version_or_timestamp)`**: Reconstructs the complete state of the store at a
  given version ID or timestamp. Returns a read-only view.

### 2. Ingestion Engine (`onyx::ingest`)

Responsible for transforming raw code artifacts into the three-layer representation:

- **Code Parser**: Extracts structural information (function signatures, imports, module
  boundaries). Initially supports Rust source; extensible to other languages.
- **Embedding Generator**: Produces vector embeddings for semantic similarity search.
  Uses a pluggable backend (initially a simple TF-IDF or bag-of-words for the prototype;
  production would use a transformer model via ONNX or external API).
- **Relationship Extractor**: Identifies edges: `calls`, `imports`, `defines`,
  `documents`, `tests_of` relationships between code entities.
- **Version Differ**: Computes structural diffs when re-ingesting updated artifacts.

**Data flow**: Raw artifact -> Parse -> (Embedding, Relations, Diff) -> Transaction -> Stores

### 3. Query Engine (`onyx::query`)

The retrieval and reasoning layer. Handles multi-hop, cross-store queries:

- **Query Planner**: Decomposes a query into a retrieval plan:
  1. Vector similarity search to find semantically relevant nodes
  2. Graph traversal to expand context along structural relationships
  3. Temporal filtering to scope results to a version range
  4. Result fusion to produce a single coherent context bundle

- **Multi-hop Resolver**: Follows edges in the graph store to a configurable depth,
  collecting related entities. Respects edge confidence scores and temporal validity.

- **Reasoning Layer**: Applies graph-based inference rules:
  - Impact analysis: "If node X changes, what downstream nodes are affected?"
  - Coverage analysis: "What tests cover the behavior of function X?"
  - Provenance tracing: "Where did this code pattern originate?"

- **Result Fusioner**: Merges results from different retrieval paths, deduplicates,
  ranks by relevance, and structures output for consumption by an AI agent.

### 4. Unified Store (`onyx::store`)

Three specialized stores unified under a transaction manager:

#### 4a. Vector Store (`onyx::store::vector`)

- **Purpose**: Semantic similarity search over code artifact embeddings.
- **Index**: HNSW (Hierarchical Navigable Small World) graph for approximate nearest
  neighbor search. In-memory for prototype; pluggable for production backends.
- **Storage**: Embedding vectors (f32 arrays) keyed by node unique_id.
- **Operations**: `insert(id, embedding)`, `search(query_embedding, k, filter)`,
  `delete(id)`.

#### 4b. Graph Store (`onyx::store::graph`)

- **Purpose**: Structural relationship storage and traversal.
- **Implementation**: Adjacency list representation with typed, weighted, temporal edges.
- **Storage**: Nodes stored by unique_id; edges stored as adjacency lists with full edge
  metadata (type, confidence, temporal range, commit provenance).
- **Operations**: `add_node(node)`, `add_edge(edge)`, `get_neighbors(id, edge_types, depth)`,
  `find_paths(from, to, max_depth)`, `subgraph(root, depth)`.

#### 4c. History Store (`onyx::store::history`)

- **Purpose**: Temporal versioning with immutable history, branching, and time travel.
- **Implementation**: Append-only diff chain per entity. Each version stores a diff from
  the previous version plus metadata (timestamp, commit_id, author, message).
- **Storage**: Version chains keyed by node unique_id. Branch metadata stored separately.
- **Operations**: `record_version(id, diff, metadata)`, `get_at_version(id, version)`,
  `get_at_timestamp(id, timestamp)`, `list_versions(id)`, `create_branch(name, base)`,
  `merge_branch(source, target)`.

#### 4d. Transaction Manager (`onyx::store::transaction`)

- **Purpose**: Ensure atomicity across the three stores. An ingest or update operation
  must succeed in ALL stores or roll back entirely.
- **Implementation**: Write-ahead log (WAL) pattern. Operations are logged, then applied
  to each store. On failure, the WAL is replayed in reverse to undo partial writes.
- **Operations**: `begin()`, `commit()`, `rollback()`.

### 5. CLI Tool (`onyx::cli`)

Interactive command-line interface for inspection and experimentation:

- `onyx ingest <path>` -- Load a file or directory of code artifacts
- `onyx query "<question>"` -- Run a natural-language query
- `onyx traverse <node-id> --relations calls,imports --depth 3` -- Graph traversal
- `onyx inspect <node-id>` -- Show node details, edges, and version history
- `onyx snapshot <version>` -- Show state at a version
- `onyx status` -- Show store statistics (node count, edge count, version count)

---

## Data Flows

### Ingestion Flow

```
User/Agent provides code artifact
        │
        ▼
  ┌─────────────┐
  │ Code Parser  │─── Extracts: name, type, body, imports, signature
  └──────┬──────┘
         │
    ┌────┼──────────────────┐
    │    │                  │
    ▼    ▼                  ▼
┌──────┐ ┌──────────┐ ┌──────────┐
│Embed │ │Relations │ │Versioner │
│Gen   │ │Extractor │ │(Differ)  │
└──┬───┘ └────┬─────┘ └────┬─────┘
   │          │             │
   ▼          ▼             ▼
┌──────┐ ┌──────┐    ┌──────────┐
│Vector│ │Graph │    │History   │
│Store │ │Store │    │Store     │
└──────┘ └──────┘    └──────────┘
   └──────────┴────────────┘
              │
        Transaction Manager
        (atomic commit)
```

### Query Flow

```
User/Agent submits query
        │
        ▼
  ┌─────────────┐
  │Query Planner │─── Determines retrieval strategy
  └──────┬──────┘
         │
    ┌────┼──────────────────┐
    │    │                  │
    ▼    ▼                  ▼
┌──────┐ ┌──────────┐ ┌──────────┐
│Vector│ │Graph     │ │History   │
│Search│ │Traversal │ │Lookup    │
└──┬───┘ └────┬─────┘ └────┬─────┘
   │          │             │
   └──────────┴─────────────┘
              │
        ┌─────▼─────┐
        │Result      │
        │Fusioner    │─── Dedup, rank, structure
        └─────┬─────┘
              │
              ▼
        Coherent context bundle
        returned to agent
```

---

## Interfaces Between Components

| Source           | Target            | Interface                                      |
|------------------|-------------------|-------------------------------------------------|
| API              | Ingestion Engine  | `IngestRequest { artifact, metadata }`          |
| API              | Query Engine      | `QueryRequest { question, options }`            |
| Ingestion Engine | Transaction Mgr   | `Transaction { vector_ops, graph_ops, hist_ops }`|
| Query Engine     | Vector Store      | `VectorQuery { embedding, k, filters }`         |
| Query Engine     | Graph Store       | `TraversalQuery { start, relations, depth }`    |
| Query Engine     | History Store     | `TemporalQuery { id, version_range }`           |
| Transaction Mgr  | All Stores        | `StoreOp::Insert/Update/Delete`                 |
| CLI              | API               | Same API interfaces via library calls            |

---

## Scaling Strategy (Future)

- **Vector Store**: Shard by embedding space partitions. Each shard holds a subset of
  the HNSW graph. Route queries to relevant shards based on coarse quantization.
- **Graph Store**: Partition by connected component or module boundary. Cross-partition
  edges handled via a routing layer.
- **History Store**: Partition by entity ID range. Time-travel queries are local to
  the entity's partition.
- **Consistency**: Strong consistency within a partition, eventual consistency across
  partitions with a conflict resolution protocol for concurrent writes.
- **Horizontal scaling**: Stateless query engines + stateful store shards. Add query
  engine instances for throughput; add store shards for capacity.
