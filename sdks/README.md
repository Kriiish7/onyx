# Onyx SDKs

Official client libraries for the **Onyx** AI infrastructure engine. Each SDK provides typed, documented access to the full Onyx API: knowledge graph nodes & edges, semantic vector search, temporal versioning with branching, code ingestion, and Stripe billing integration.

## Available SDKs

| Language                    | Directory                         | Package     | Status  |
| --------------------------- | --------------------------------- | ----------- | ------- |
| **Rust**                    | [`sdks/rust/`](rust/)             | `onyx-sdk`  | Alpha   |
| **Python**                  | [`sdks/python/`](python/)         | `onyx-sdk`  | Alpha   |
| **TypeScript / JavaScript** | [`sdks/typescript/`](typescript/) | `@onyx/sdk` | Alpha   |
| **C#**                      | [`sdks/csharp/`](csharp/)         | `Onyx.Sdk`  | Planned |

## Architecture

All SDKs share the same design:

```
OnyxClient
├── .nodes       → Node CRUD (create, get, update, delete, list, subgraph)
├── .edges       → Edge CRUD (create, get, delete, list)
├── .search      → Semantic vector search with graph traversal
├── .history     → Version history, branching, merging
├── .ingest      → Code ingestion pipeline
└── .billing     → Stripe checkout & billing portal
```

### Common Patterns

1. **Builder / options pattern** — Configure base URL, API key, and timeout
2. **Typed sub-clients** — Each domain area has its own client with dedicated methods
3. **Structured errors** — Typed error hierarchy (API errors, network errors, not-found, validation)
4. **Full model coverage** — Every request/response type is fully defined

## Quick Comparison

### Rust

```rust
use onyx_sdk::{OnyxClient, CreateNodeRequest, NodeType, CodeEntityKind};

let client = OnyxClient::builder("http://localhost:3000")
    .api_key("sk-...")
    .build()?;

let node = client.nodes().create(
    CreateNodeRequest::new("main", "fn main() {}")
        .node_type(NodeType::code_entity(CodeEntityKind::Function))
).await?;
```

### Python

```python
from onyx_sdk import OnyxClient, CreateNodeRequest, NodeType

async with OnyxClient("http://localhost:3000", api_key="sk-...") as client:
    node = await client.nodes.create(CreateNodeRequest(
        name="main",
        content="fn main() {}",
        node_type=NodeType(type="CodeEntity", kind="Function"),
    ))
```

### TypeScript

```typescript
import { OnyxClient } from "@onyx/sdk";

const client = new OnyxClient("http://localhost:3000", { apiKey: "sk-..." });

const node = await client.nodes.create({
  name: "main",
  content: "fn main() {}",
  nodeType: { type: "CodeEntity", kind: "Function" },
});
```

## API Endpoints

The SDKs target the following REST API surface:

| Method   | Endpoint                                  | Description          |
| -------- | ----------------------------------------- | -------------------- |
| `GET`    | `/health`                                 | Health check         |
| `POST`   | `/api/nodes`                              | Create node          |
| `GET`    | `/api/nodes/:id`                          | Get node             |
| `PUT`    | `/api/nodes/:id`                          | Update node          |
| `DELETE` | `/api/nodes/:id`                          | Delete node          |
| `GET`    | `/api/nodes?page=&per_page=`              | List nodes           |
| `GET`    | `/api/nodes/:id/neighbors`                | Node neighbors       |
| `GET`    | `/api/nodes/:id/subgraph?depth=`          | Subgraph extraction  |
| `POST`   | `/api/edges`                              | Create edge          |
| `GET`    | `/api/edges/:id`                          | Get edge             |
| `DELETE` | `/api/edges/:id`                          | Delete edge          |
| `GET`    | `/api/edges`                              | List edges           |
| `POST`   | `/api/search`                             | Semantic search      |
| `POST`   | `/api/versions`                           | Record version       |
| `GET`    | `/api/versions/:id`                       | Get version          |
| `GET`    | `/api/entities/:id/versions`              | List entity versions |
| `GET`    | `/api/entities/:id/versions/:vid/content` | Content at version   |
| `POST`   | `/api/branches`                           | Create branch        |
| `GET`    | `/api/branches/:name`                     | Get branch           |
| `GET`    | `/api/branches`                           | List branches        |
| `POST`   | `/api/branches/merge`                     | Merge branches       |
| `POST`   | `/api/ingest/unit`                        | Ingest code unit     |
| `POST`   | `/api/ingest/codebase`                    | Batch ingest         |
| `POST`   | `/billing/checkout`                       | Stripe checkout      |
| `POST`   | `/billing/portal`                         | Billing portal       |
| `POST`   | `/billing/webhook`                        | Stripe webhook       |

## Data Model Reference

### Node Types

| Type                    | Description             |
| ----------------------- | ----------------------- |
| `CodeEntity(Function)`  | A function or method    |
| `CodeEntity(Struct)`    | A struct / class        |
| `CodeEntity(Enum)`      | An enum type            |
| `CodeEntity(Trait)`     | A trait / interface     |
| `CodeEntity(Impl)`      | An implementation block |
| `CodeEntity(Module)`    | A module / namespace    |
| `CodeEntity(Constant)`  | A constant value        |
| `CodeEntity(TypeAlias)` | A type alias            |
| `CodeEntity(Macro)`     | A macro definition      |
| `Doc`                   | Documentation           |
| `Test`                  | A test                  |
| `Config`                | Configuration file      |

### Edge Types

| Type          | Description                   |
| ------------- | ----------------------------- |
| `Defines`     | A code entity defines another |
| `Calls`       | A function calls another      |
| `Imports`     | A module imports another      |
| `Documents`   | Documentation describes code  |
| `TestsOf`     | A test covers a code entity   |
| `VersionedBy` | Versioned by a history entry  |
| `Contains`    | Module contains sub-entities  |
| `Implements`  | Implements a trait/interface  |
| `DependsOn`   | Generic dependency            |
| `Configures`  | Config file configures code   |

### Search Result Sources

| Source           | Description                           |
| ---------------- | ------------------------------------- |
| `VectorSearch`   | Found via embedding similarity        |
| `GraphTraversal` | Found by following graph edges        |
| `Combined`       | Found by both methods (boosted score) |

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines. SDK improvements and new language SDKs are welcome.

## License

MIT — see [LICENSE](../LICENSE) for details.
