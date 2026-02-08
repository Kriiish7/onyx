# Onyx SDK for Python

Official Python client library for the **Onyx** AI infrastructure engine — combining semantic search, knowledge graphs, and temporal versioning in a graph-native vector memory system.

## Installation

```bash
pip install onyx-sdk
# or from source
pip install -e sdks/python
```

**Requirements:** Python ≥ 3.9, `httpx`, `pydantic` v2.

## Quick Start

```python
import asyncio
from onyx_sdk import OnyxClient, CreateNodeRequest, NodeType

async def main():
    # 1. Create a client
    client = OnyxClient("http://localhost:3000", api_key="your-key")

    # 2. Health check
    healthy = await client.health()
    print(f"Server healthy: {healthy}")

    # 3. Create a node
    node = await client.nodes.create(CreateNodeRequest(
        name="hello_world",
        content='fn hello() { println!("hello"); }',
        node_type=NodeType(type="CodeEntity", kind="Function"),
    ))
    print(f"Created node: {node.id}")

    # 4. Clean up
    await client.close()

asyncio.run(main())
```

### Context Manager

```python
async with OnyxClient("http://localhost:3000") as client:
    healthy = await client.health()
```

## Sub-Clients

| Sub-client      | Property         | Description                 |
| --------------- | ---------------- | --------------------------- |
| `NodesClient`   | `client.nodes`   | Node CRUD operations        |
| `EdgesClient`   | `client.edges`   | Edge CRUD operations        |
| `SearchClient`  | `client.search`  | Vector similarity search    |
| `HistoryClient` | `client.history` | Version history & branching |
| `IngestClient`  | `client.ingest`  | Code ingestion pipeline     |
| `BillingClient` | `client.billing` | Stripe billing integration  |

## Node Operations

```python
from onyx_sdk import CreateNodeRequest, UpdateNodeRequest, NodeType, Provenance

# Create
node = await client.nodes.create(CreateNodeRequest(
    name="MyStruct",
    content="pub struct MyStruct { ... }",
    node_type=NodeType(type="CodeEntity", kind="Struct"),
    provenance=Provenance(file_path="src/lib.rs", line_range=(10, 20)),
))

# Read
fetched = await client.nodes.get(node.id)

# Update
updated = await client.nodes.update(node.id, UpdateNodeRequest(name="RenamedStruct"))

# Delete
await client.nodes.delete(node.id)

# List (paginated)
page = await client.nodes.list(page=1, per_page=50)
print(f"Total nodes: {page.total}")

# Subgraph extraction
sub = await client.nodes.subgraph(node.id, depth=2)
```

## Edge Operations

```python
from uuid import UUID
from onyx_sdk import CreateEdgeRequest, EdgeType

edge = await client.edges.create(CreateEdgeRequest(
    edge_type=EdgeType.CALLS,
    source_id=source_id,
    target_id=target_id,
    confidence=0.95,
))

await client.edges.delete(edge.id)
```

## Semantic Search

```python
results = await client.search.query(
    embedding=[0.1, 0.2, 0.3, ...],
    top_k=10,
    max_depth=3,
    edge_types=[EdgeType.CALLS, EdgeType.IMPORTS],
    include_history=True,
    min_confidence=0.5,
)

print(f"Found {len(results.items)} items in {results.query_time_ms}ms")
for item in results.items:
    print(f"  {item.name} (score={item.score:.3f}, source={item.source})")
```

## Version History

```python
from onyx_sdk import CreateBranchRequest, MergeBranchRequest

# List versions for an entity
versions = await client.history.list_versions(entity_id)

# Get content at a specific version
content = await client.history.get_content_at_version(entity_id, "v-abc123")

# Create a branch
branch = await client.history.create_branch(CreateBranchRequest(
    name="feature/new-api",
    base_version="v-abc123",
))

# Merge branches
merged = await client.history.merge_branch(MergeBranchRequest(
    source="feature/new-api",
    target="main",
))
```

## Code Ingestion

```python
from onyx_sdk import (
    IngestCodeUnitRequest, IngestCodebaseRequest,
    CodeEntityKind, Language, Visibility,
)

# Single unit
result = await client.ingest.ingest_unit(IngestCodeUnitRequest(
    name="process_data",
    content="pub fn process_data(input: &str) -> Result<()> { ... }",
    kind=CodeEntityKind.FUNCTION,
    language=Language.RUST,
    file_path="src/processing.rs",
    line_range=(42, 58),
    signature="pub fn process_data(input: &str) -> Result<()>",
    visibility=Visibility.PUBLIC,
))
print(f"Ingested: node_id={result.node_id}")

# Batch ingestion
batch = await client.ingest.ingest_codebase(IngestCodebaseRequest(
    units=[unit1, unit2, unit3],
))
```

## Billing

```python
from onyx_sdk import CheckoutSessionRequest, BillingPortalRequest

# Checkout session
session = await client.billing.create_checkout_session(CheckoutSessionRequest(
    customer_email="user@example.com",
    price_id="price_xxx",
    success_url="https://app.example.com/success",
    cancel_url="https://app.example.com/cancel",
))
print(f"Checkout URL: {session.url}")

# Billing portal
portal = await client.billing.create_billing_portal(BillingPortalRequest(
    customer_id="cus_xxx",
    return_url="https://app.example.com/settings",
))
```

## Error Handling

```python
from onyx_sdk import OnyxError, OnyxApiError, OnyxNotFoundError, OnyxNetworkError

try:
    node = await client.nodes.get(some_id)
except OnyxNotFoundError:
    print("Node not found")
except OnyxApiError as e:
    print(f"API error ({e.status_code}): {e.message}")
except OnyxNetworkError as e:
    print(f"Network error: {e}")
except OnyxError as e:
    print(f"SDK error: {e}")
```

| Exception             | Description                                                  |
| --------------------- | ------------------------------------------------------------ |
| `OnyxError`           | Base class for all SDK errors                                |
| `OnyxApiError`        | HTTP error from the server (has `status_code` and `message`) |
| `OnyxNotFoundError`   | Resource not found (subclass of `OnyxApiError`, status 404)  |
| `OnyxNetworkError`    | Connection / transport failure                               |
| `OnyxValidationError` | Invalid argument provided to the SDK                         |

## Type Safety

The SDK ships with a `py.typed` marker and all models use Pydantic v2 for runtime validation. Full IDE autocompletion and type checking is supported with mypy / pyright.

## License

MIT — see [LICENSE](../../LICENSE) for details.
