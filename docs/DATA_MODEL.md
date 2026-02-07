# Onyx Data Model Specification

## Node and Edge Schemas

---

## Node Types

All nodes share a common base schema, with type-specific extensions.

### Base Node Schema

| Field           | Type                | Description                                          | Indexed |
|-----------------|---------------------|------------------------------------------------------|---------|
| `id`            | `Uuid`              | Globally unique identifier                           | Primary |
| `node_type`     | `NodeType` (enum)   | One of: CodeEntity, Doc, Test, Config                | B-tree  |
| `name`          | `String`            | Human-readable name (e.g., function name, doc title) | B-tree  |
| `content`       | `String`            | Full source text or document body                    | None    |
| `content_hash`  | `[u8; 32]`          | SHA-256 of content for dedup/change detection        | Hash    |
| `metadata`      | `HashMap<String, String>` | Extensible key-value metadata               | None    |
| `provenance`    | `Provenance`        | Origin information (file path, line range, commit)   | None    |
| `embedding`     | `Option<Vec<f32>>`  | Vector embedding for semantic search                 | HNSW    |
| `current_version` | `VersionId`       | Pointer to latest version in history store           | B-tree  |
| `created_at`    | `DateTime<Utc>`     | Creation timestamp                                   | B-tree  |
| `updated_at`    | `DateTime<Utc>`     | Last modification timestamp                          | B-tree  |

### NodeType Enum

```rust
enum NodeType {
    CodeEntity(CodeEntityKind),
    Doc,
    Test,
    Config,
}

enum CodeEntityKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
    Constant,
    TypeAlias,
    Macro,
}
```

### Type-Specific Extensions

#### CodeEntity

| Field          | Type                   | Description                                |
|----------------|------------------------|--------------------------------------------|
| `language`     | `Language` (enum)      | Programming language (Rust, Python, etc.)  |
| `signature`    | `Option<String>`       | Function/method signature                  |
| `visibility`   | `Visibility` (enum)    | pub, pub(crate), private                   |
| `module_path`  | `Vec<String>`          | Full module path (e.g., ["onyx", "store"]) |
| `line_range`   | `(usize, usize)`      | Start and end line in source file          |

#### Doc

| Field          | Type                   | Description                                |
|----------------|------------------------|--------------------------------------------|
| `doc_type`     | `DocType` (enum)       | Inline, Readme, ApiDoc, Tutorial           |
| `format`       | `DocFormat` (enum)     | Markdown, RustDoc, PlainText               |
| `target_id`    | `Option<Uuid>`         | Node this doc describes (if inline)        |

#### Test

| Field          | Type                   | Description                                |
|----------------|------------------------|--------------------------------------------|
| `test_kind`    | `TestKind` (enum)      | Unit, Integration, Property, Benchmark     |
| `target_ids`   | `Vec<Uuid>`            | Nodes this test covers                     |
| `last_result`  | `Option<TestResult>`   | Pass/Fail/Skip with timestamp              |

#### Config

| Field          | Type                   | Description                                |
|----------------|------------------------|--------------------------------------------|
| `config_type`  | `ConfigType` (enum)    | Cargo, CI, Docker, Env, Build              |
| `format`       | `ConfigFormat` (enum)  | Toml, Yaml, Json, Ini                      |

### Provenance Schema

| Field          | Type                   | Description                                |
|----------------|------------------------|--------------------------------------------|
| `file_path`    | `String`               | Relative path from repo root               |
| `line_range`   | `Option<(usize, usize)>` | Line range within file                  |
| `commit_id`    | `Option<String>`       | Git commit hash at ingestion time          |
| `repo_url`     | `Option<String>`       | Repository URL                             |
| `branch`       | `Option<String>`       | Git branch name                            |

---

## Edge Types

### Base Edge Schema

| Field          | Type                   | Description                                | Indexed |
|----------------|------------------------|--------------------------------------------|---------|
| `id`           | `Uuid`                 | Globally unique edge identifier            | Primary |
| `edge_type`    | `EdgeType` (enum)      | Relationship category                      | B-tree  |
| `source_id`    | `Uuid`                 | Source node ID                             | B-tree  |
| `target_id`    | `Uuid`                 | Target node ID                             | B-tree  |
| `confidence`   | `f64`                  | Confidence score [0.0, 1.0]                | None    |
| `metadata`     | `HashMap<String, String>` | Extensible key-value metadata           | None    |
| `temporal`     | `TemporalContext`      | When this relationship was valid           | B-tree  |

### EdgeType Enum

```rust
enum EdgeType {
    /// Function/struct defines another entity (e.g., impl block defines methods)
    Defines,
    /// Function calls another function
    Calls,
    /// Module imports another module or entity
    Imports,
    /// Documentation describes a code entity
    Documents,
    /// Test covers a code entity
    TestsOf,
    /// Entity is versioned by a history entry
    VersionedBy,
    /// Module contains sub-entities
    Contains,
    /// Struct/enum implements a trait
    Implements,
    /// Entity depends on another entity (generic dependency)
    DependsOn,
    /// Config configures a code entity or module
    Configures,
}
```

### Temporal Context Schema

| Field          | Type                   | Description                                |
|----------------|------------------------|--------------------------------------------|
| `since`        | `VersionId`            | Version when edge was created              |
| `until`        | `Option<VersionId>`    | Version when edge was removed (None=active)|
| `via_commit`   | `Option<String>`       | Commit that introduced this relationship   |
| `since_timestamp` | `DateTime<Utc>`     | Timestamp of creation                      |
| `until_timestamp` | `Option<DateTime<Utc>>` | Timestamp of removal                   |

---

## Version Schema

### VersionEntry

| Field          | Type                   | Description                                | Indexed |
|----------------|------------------------|--------------------------------------------|---------|
| `version_id`   | `VersionId`           | Unique version identifier                  | Primary |
| `entity_id`    | `Uuid`                 | Node this version belongs to               | B-tree  |
| `parent_version` | `Option<VersionId>`  | Previous version (None for initial)        | B-tree  |
| `branch`       | `String`               | Branch name (default: "main")              | B-tree  |
| `diff`         | `Diff`                 | Changes from parent version                | None    |
| `commit_id`    | `Option<String>`       | Associated git commit                      | B-tree  |
| `author`       | `Option<String>`       | Who made the change                        | None    |
| `message`      | `Option<String>`       | Change description                         | None    |
| `timestamp`    | `DateTime<Utc>`        | When the version was recorded              | B-tree  |

### Diff Schema

```rust
enum Diff {
    /// Initial version, stores full content
    Initial { content: String },
    /// Content changed
    ContentChanged {
        /// Unified diff format
        patch: String,
        /// Lines added
        additions: usize,
        /// Lines removed
        deletions: usize,
    },
    /// Metadata changed
    MetadataChanged {
        changed_fields: HashMap<String, (String, String)>, // field -> (old, new)
    },
    /// Multiple changes in one version
    Composite(Vec<Diff>),
}
```

### Branch Schema

| Field          | Type                   | Description                                |
|----------------|------------------------|--------------------------------------------|
| `name`         | `String`               | Branch name                                |
| `head`         | `VersionId`            | Latest version on this branch              |
| `base`         | `VersionId`            | Version where branch was created           |
| `created_at`   | `DateTime<Utc>`        | Branch creation time                       |
| `merged_into`  | `Option<String>`       | Branch this was merged into (if merged)    |

---

## Indexing Strategies

### Vector Index (HNSW)

- **Dimensions**: Configurable (default 384 for lightweight models, 1536 for OpenAI-class)
- **Distance metric**: Cosine similarity (normalized dot product)
- **HNSW parameters**:
  - `M` (max connections per node): 16 (default)
  - `ef_construction` (search width during build): 200
  - `ef_search` (search width during query): 50
- **Index on**: `Node.embedding` field

### Graph Indices

- **Adjacency index**: `(source_id, edge_type) -> Vec<Edge>` for fast outbound traversal
- **Reverse adjacency**: `(target_id, edge_type) -> Vec<Edge>` for fast inbound traversal
- **Temporal index**: `(entity_id, timestamp_range)` for time-scoped queries
- **Type index**: `node_type -> Vec<NodeId>` for type-filtered queries

### History Indices

- **Version chain**: `entity_id -> Vec<VersionId>` ordered by timestamp
- **Branch index**: `branch_name -> BranchMetadata`
- **Commit index**: `commit_id -> Vec<VersionId>` for commit-based lookups

---

## Example Records

### Example Node (CodeEntity - Function)

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "node_type": { "CodeEntity": "Function" },
  "name": "calculate_total",
  "content": "pub fn calculate_total(items: &[Item], tax_rate: f64) -> f64 {\n    let subtotal: f64 = items.iter().map(|i| i.price * i.quantity as f64).sum();\n    subtotal * (1.0 + tax_rate)\n}",
  "content_hash": "a1b2c3d4...",
  "metadata": {
    "complexity": "low",
    "domain": "billing"
  },
  "provenance": {
    "file_path": "src/billing/calculator.rs",
    "line_range": [45, 49],
    "commit_id": "abc123def",
    "repo_url": "https://github.com/example/shop",
    "branch": "main"
  },
  "embedding": [0.12, -0.34, 0.56, ...],
  "current_version": "v-550e-0003",
  "created_at": "2025-01-15T10:30:00Z",
  "updated_at": "2025-02-01T14:22:00Z",
  "language": "Rust",
  "signature": "pub fn calculate_total(items: &[Item], tax_rate: f64) -> f64",
  "visibility": "Public",
  "module_path": ["shop", "billing", "calculator"],
  "line_range": [45, 49]
}
```

### Example Edge (Calls)

```json
{
  "id": "660e8400-e29b-41d4-a716-446655440010",
  "edge_type": "Calls",
  "source_id": "550e8400-e29b-41d4-a716-446655440002",
  "target_id": "550e8400-e29b-41d4-a716-446655440001",
  "confidence": 1.0,
  "metadata": {
    "call_site_line": "78"
  },
  "temporal": {
    "since": "v-550e-0001",
    "until": null,
    "via_commit": "abc123def",
    "since_timestamp": "2025-01-15T10:30:00Z",
    "until_timestamp": null
  }
}
```

### Example Version Entry

```json
{
  "version_id": "v-550e-0003",
  "entity_id": "550e8400-e29b-41d4-a716-446655440001",
  "parent_version": "v-550e-0002",
  "branch": "main",
  "diff": {
    "ContentChanged": {
      "patch": "--- a/src/billing/calculator.rs\n+++ b/src/billing/calculator.rs\n@@ -47,1 +47,2 @@\n-    subtotal * (1.0 + tax_rate)\n+    let tax = subtotal * tax_rate;\n+    subtotal + tax",
      "additions": 2,
      "deletions": 1
    }
  },
  "commit_id": "def456ghi",
  "author": "dev@example.com",
  "message": "Refactor tax calculation for clarity",
  "timestamp": "2025-02-01T14:22:00Z"
}
```
