"""
Onyx SDK for Python
===================

Official Python client library for the **Onyx** AI infrastructure engine.
Onyx combines semantic search, knowledge graphs, and temporal versioning in
a graph-native vector memory system.

Quick Start
-----------

.. code-block:: python

    import asyncio
    from onyx_sdk import OnyxClient, CreateNodeRequest, NodeType

    async def main():
        client = OnyxClient("http://localhost:3000", api_key="your-key")

        # Health check
        healthy = await client.health()
        print(f"Server healthy: {healthy}")

        # Create a node
        node = await client.nodes.create(CreateNodeRequest(
            name="hello_world",
            content='fn hello() { println!("hello"); }',
            node_type=NodeType(type="CodeEntity", kind="Function"),
        ))
        print(f"Created node: {node.id}")

        # Semantic search
        results = await client.search.query(embedding=[0.1, 0.2, 0.3], top_k=5)
        for item in results.items:
            print(f"  {item.name} (score: {item.score:.3f})")

        await client.close()

    asyncio.run(main())
"""

from onyx_sdk.client import OnyxClient
from onyx_sdk.models import (
    # Nodes
    Node,
    NodeType,
    CodeEntityKind,
    CreateNodeRequest,
    UpdateNodeRequest,
    ListNodesResponse,
    Provenance,
    NodeExtension,
    CodeEntityExt,
    DocExt,
    TestExt,
    ConfigExt,
    Language,
    Visibility,
    # Edges
    Edge,
    EdgeType,
    TemporalContext,
    CreateEdgeRequest,
    ListEdgesResponse,
    NeighborResult,
    TraversalResult,
    SubgraphResult,
    # Search
    SearchRequest,
    SearchResponse,
    SearchResultItem,
    ResultSource,
    # Version / history
    VersionEntry,
    Diff,
    VersionInfo,
    Branch,
    CreateVersionRequest,
    CreateBranchRequest,
    MergeBranchRequest,
    ListVersionsResponse,
    ListBranchesResponse,
    # Ingestion
    IngestCodeUnitRequest,
    IngestCodebaseRequest,
    IngestResult,
    IngestCodebaseResponse,
    # Billing
    CheckoutSessionRequest,
    CheckoutSessionResponse,
    BillingPortalRequest,
    BillingPortalResponse,
)
from onyx_sdk.errors import (
    OnyxError,
    OnyxApiError,
    OnyxNetworkError,
    OnyxNotFoundError,
    OnyxValidationError,
)

__all__ = [
    # Client
    "OnyxClient",
    # Node models
    "Node",
    "NodeType",
    "CodeEntityKind",
    "CreateNodeRequest",
    "UpdateNodeRequest",
    "ListNodesResponse",
    "Provenance",
    "NodeExtension",
    "CodeEntityExt",
    "DocExt",
    "TestExt",
    "ConfigExt",
    "Language",
    "Visibility",
    # Edge models
    "Edge",
    "EdgeType",
    "TemporalContext",
    "CreateEdgeRequest",
    "ListEdgesResponse",
    "NeighborResult",
    "TraversalResult",
    "SubgraphResult",
    # Search models
    "SearchRequest",
    "SearchResponse",
    "SearchResultItem",
    "ResultSource",
    # Version models
    "VersionEntry",
    "Diff",
    "VersionInfo",
    "Branch",
    "CreateVersionRequest",
    "CreateBranchRequest",
    "MergeBranchRequest",
    "ListVersionsResponse",
    "ListBranchesResponse",
    # Ingestion models
    "IngestCodeUnitRequest",
    "IngestCodebaseRequest",
    "IngestResult",
    "IngestCodebaseResponse",
    # Billing models
    "CheckoutSessionRequest",
    "CheckoutSessionResponse",
    "BillingPortalRequest",
    "BillingPortalResponse",
    # Errors
    "OnyxError",
    "OnyxApiError",
    "OnyxNetworkError",
    "OnyxNotFoundError",
    "OnyxValidationError",
]

__version__ = "0.1.0"
