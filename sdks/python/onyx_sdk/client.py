"""
HTTP client and sub-clients for the Onyx API.

The main entry point is :class:`OnyxClient`.  Sub-clients for each domain
area are exposed as properties.

Example
-------
.. code-block:: python

    from onyx_sdk import OnyxClient

    client = OnyxClient("http://localhost:3000", api_key="sk-...")
    node = await client.nodes.create(CreateNodeRequest(name="f", content="..."))
    await client.close()

Context-manager usage (auto-close):

.. code-block:: python

    async with OnyxClient("http://localhost:3000") as client:
        healthy = await client.health()
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional, Type, TypeVar
from uuid import UUID

import httpx

from onyx_sdk.errors import (
    OnyxApiError,
    OnyxNetworkError,
    OnyxNotFoundError,
)
from onyx_sdk.models import (
    Branch,
    BillingPortalRequest,
    BillingPortalResponse,
    CheckoutSessionRequest,
    CheckoutSessionResponse,
    CreateBranchRequest,
    CreateEdgeRequest,
    CreateNodeRequest,
    CreateVersionRequest,
    Edge,
    IngestCodeUnitRequest,
    IngestCodebaseRequest,
    IngestCodebaseResponse,
    IngestResult,
    ListBranchesResponse,
    ListEdgesResponse,
    ListNodesResponse,
    ListVersionsResponse,
    MergeBranchRequest,
    NeighborResult,
    Node,
    SearchRequest,
    SearchResponse,
    SubgraphResult,
    UpdateNodeRequest,
    VersionEntry,
)

T = TypeVar("T")


# ============================================================================
# Internal helpers
# ============================================================================


class _HttpTransport:
    """Thin wrapper around :class:`httpx.AsyncClient`."""

    def __init__(self, base_url: str, api_key: Optional[str], timeout: float) -> None:
        headers: Dict[str, str] = {"Content-Type": "application/json"}
        if api_key:
            headers["Authorization"] = f"Bearer {api_key}"
        self._client = httpx.AsyncClient(
            base_url=base_url,
            headers=headers,
            timeout=timeout,
        )

    async def close(self) -> None:
        await self._client.aclose()

    # -- request helpers -------------------------------------------------------

    async def get(self, path: str, **params: Any) -> httpx.Response:
        return await self._request("GET", path, params=params)

    async def post(self, path: str, json: Any = None) -> httpx.Response:
        return await self._request("POST", path, json=json)

    async def put(self, path: str, json: Any = None) -> httpx.Response:
        return await self._request("PUT", path, json=json)

    async def delete(self, path: str) -> httpx.Response:
        return await self._request("DELETE", path)

    async def _request(self, method: str, path: str, **kwargs: Any) -> httpx.Response:
        try:
            resp = await self._client.request(method, path, **kwargs)
        except httpx.HTTPError as exc:
            raise OnyxNetworkError(str(exc)) from exc

        if resp.status_code == 404:
            raise OnyxNotFoundError(resp.text)
        if not resp.is_success:
            raise OnyxApiError(resp.status_code, resp.text)

        return resp


# ============================================================================
# Sub-clients
# ============================================================================


class NodesClient:
    """Node CRUD operations.

    Accessed via ``client.nodes``.
    """

    def __init__(self, transport: _HttpTransport) -> None:
        self._t = transport

    async def create(self, request: CreateNodeRequest) -> Node:
        """Create a new node.

        Args:
            request: Node creation parameters.

        Returns:
            The newly created :class:`Node`.
        """
        resp = await self._t.post("/api/nodes", json=request.model_dump(exclude_none=True))
        return Node.model_validate(resp.json())

    async def get(self, node_id: UUID) -> Node:
        """Retrieve a node by ID.

        Args:
            node_id: UUID of the node.

        Returns:
            The matching :class:`Node`.

        Raises:
            OnyxNotFoundError: If the node does not exist.
        """
        resp = await self._t.get(f"/api/nodes/{node_id}")
        return Node.model_validate(resp.json())

    async def update(self, node_id: UUID, request: UpdateNodeRequest) -> Node:
        """Update an existing node.

        Args:
            node_id: UUID of the node to update.
            request: Fields to change (``None`` fields are left unchanged).

        Returns:
            The updated :class:`Node`.
        """
        resp = await self._t.put(
            f"/api/nodes/{node_id}",
            json=request.model_dump(exclude_none=True),
        )
        return Node.model_validate(resp.json())

    async def delete(self, node_id: UUID) -> None:
        """Delete a node and all its edges.

        Args:
            node_id: UUID of the node to remove.
        """
        await self._t.delete(f"/api/nodes/{node_id}")

    async def list(self, page: int = 1, per_page: int = 50) -> ListNodesResponse:
        """Paginated list of all nodes.

        Args:
            page: Page number (1-indexed).
            per_page: Items per page.
        """
        resp = await self._t.get(f"/api/nodes?page={page}&per_page={per_page}")
        return ListNodesResponse.model_validate(resp.json())

    async def neighbors(self, node_id: UUID) -> List[NeighborResult]:
        """Get the neighbors of a node.

        Args:
            node_id: UUID of the node.

        Returns:
            List of (edge, node) neighbor pairs.
        """
        resp = await self._t.get(f"/api/nodes/{node_id}/neighbors")
        return [NeighborResult.model_validate(item) for item in resp.json()]

    async def subgraph(self, node_id: UUID, depth: int = 2) -> SubgraphResult:
        """Extract a subgraph rooted at a node.

        Args:
            node_id: Root node UUID.
            depth: Maximum traversal depth.

        Returns:
            The extracted :class:`SubgraphResult`.
        """
        resp = await self._t.get(f"/api/nodes/{node_id}/subgraph?depth={depth}")
        return SubgraphResult.model_validate(resp.json())


class EdgesClient:
    """Edge CRUD operations.

    Accessed via ``client.edges``.
    """

    def __init__(self, transport: _HttpTransport) -> None:
        self._t = transport

    async def create(self, request: CreateEdgeRequest) -> Edge:
        """Create a new directed edge.

        Args:
            request: Edge creation parameters.

        Returns:
            The newly created :class:`Edge`.
        """
        resp = await self._t.post("/api/edges", json=request.model_dump(exclude_none=True))
        return Edge.model_validate(resp.json())

    async def get(self, edge_id: UUID) -> Edge:
        """Retrieve an edge by ID."""
        resp = await self._t.get(f"/api/edges/{edge_id}")
        return Edge.model_validate(resp.json())

    async def delete(self, edge_id: UUID) -> None:
        """Delete an edge by ID."""
        await self._t.delete(f"/api/edges/{edge_id}")

    async def list(self) -> ListEdgesResponse:
        """List all edges."""
        resp = await self._t.get("/api/edges")
        return ListEdgesResponse.model_validate(resp.json())


class SearchClient:
    """Semantic search operations.

    Accessed via ``client.search``.
    """

    def __init__(self, transport: _HttpTransport) -> None:
        self._t = transport

    async def query(
        self,
        embedding: List[float],
        *,
        top_k: Optional[int] = None,
        max_depth: Optional[int] = None,
        edge_types: Optional[List[str]] = None,
        include_history: Optional[bool] = None,
        min_confidence: Optional[float] = None,
    ) -> SearchResponse:
        """Execute a semantic query.

        Args:
            embedding: The query vector.
            top_k: Number of results (default: server decides).
            max_depth: Max graph traversal depth.
            edge_types: Edge types to follow during traversal.
            include_history: Include version history in results.
            min_confidence: Minimum edge confidence score.

        Returns:
            A :class:`SearchResponse` with ranked result items.
        """
        req = SearchRequest(
            embedding=embedding,
            top_k=top_k,
            max_depth=max_depth,
            edge_types=edge_types,  # type: ignore[arg-type]
            include_history=include_history,
            min_confidence=min_confidence,
        )
        resp = await self._t.post("/api/search", json=req.model_dump(exclude_none=True))
        return SearchResponse.model_validate(resp.json())


class HistoryClient:
    """Version history and branching.

    Accessed via ``client.history``.
    """

    def __init__(self, transport: _HttpTransport) -> None:
        self._t = transport

    async def create_version(self, request: CreateVersionRequest) -> VersionEntry:
        """Record a new version."""
        resp = await self._t.post("/api/versions", json=request.model_dump(exclude_none=True))
        return VersionEntry.model_validate(resp.json())

    async def get_version(self, version_id: str) -> VersionEntry:
        """Get a version by ID."""
        resp = await self._t.get(f"/api/versions/{version_id}")
        return VersionEntry.model_validate(resp.json())

    async def list_versions(self, entity_id: UUID) -> ListVersionsResponse:
        """List all versions for an entity."""
        resp = await self._t.get(f"/api/entities/{entity_id}/versions")
        return ListVersionsResponse.model_validate(resp.json())

    async def get_content_at_version(self, entity_id: UUID, version_id: str) -> str:
        """Get entity content at a specific version."""
        resp = await self._t.get(
            f"/api/entities/{entity_id}/versions/{version_id}/content"
        )
        return resp.text

    async def create_branch(self, request: CreateBranchRequest) -> Branch:
        """Create a new branch."""
        resp = await self._t.post("/api/branches", json=request.model_dump())
        return Branch.model_validate(resp.json())

    async def get_branch(self, name: str) -> Branch:
        """Get branch metadata."""
        resp = await self._t.get(f"/api/branches/{name}")
        return Branch.model_validate(resp.json())

    async def list_branches(self) -> ListBranchesResponse:
        """List all branches."""
        resp = await self._t.get("/api/branches")
        return ListBranchesResponse.model_validate(resp.json())

    async def merge_branch(self, request: MergeBranchRequest) -> VersionEntry:
        """Merge a source branch into a target branch."""
        resp = await self._t.post("/api/branches/merge", json=request.model_dump())
        return VersionEntry.model_validate(resp.json())


class IngestClient:
    """Code ingestion pipeline.

    Accessed via ``client.ingest``.
    """

    def __init__(self, transport: _HttpTransport) -> None:
        self._t = transport

    async def ingest_unit(self, request: IngestCodeUnitRequest) -> IngestResult:
        """Ingest a single code unit.

        Args:
            request: Code unit parameters.

        Returns:
            An :class:`IngestResult` with the new node ID and version.
        """
        resp = await self._t.post(
            "/api/ingest/unit", json=request.model_dump(exclude_none=True)
        )
        return IngestResult.model_validate(resp.json())

    async def ingest_codebase(self, request: IngestCodebaseRequest) -> IngestCodebaseResponse:
        """Ingest an entire codebase (batch).

        Args:
            request: Batch of code units.

        Returns:
            An :class:`IngestCodebaseResponse` with per-unit results.
        """
        resp = await self._t.post(
            "/api/ingest/codebase", json=request.model_dump(exclude_none=True)
        )
        return IngestCodebaseResponse.model_validate(resp.json())


class BillingClient:
    """Stripe billing integration.

    Accessed via ``client.billing``.
    """

    def __init__(self, transport: _HttpTransport) -> None:
        self._t = transport

    async def create_checkout_session(
        self, request: CheckoutSessionRequest
    ) -> CheckoutSessionResponse:
        """Create a Stripe checkout session.

        Args:
            request: Checkout parameters.

        Returns:
            A :class:`CheckoutSessionResponse` with the session ID and URL.
        """
        resp = await self._t.post(
            "/billing/checkout", json=request.model_dump(exclude_none=True)
        )
        return CheckoutSessionResponse.model_validate(resp.json())

    async def create_billing_portal(
        self, request: BillingPortalRequest
    ) -> BillingPortalResponse:
        """Create a Stripe billing portal session.

        Args:
            request: Portal parameters.

        Returns:
            A :class:`BillingPortalResponse` with the portal URL.
        """
        resp = await self._t.post(
            "/billing/portal", json=request.model_dump(exclude_none=True)
        )
        return BillingPortalResponse.model_validate(resp.json())


# ============================================================================
# Main client
# ============================================================================


class OnyxClient:
    """The main Onyx API client.

    Provides access to all Onyx services through typed sub-clients.

    Args:
        base_url: The base URL of the Onyx server (e.g. ``http://localhost:3000``).
        api_key: Optional API key for authentication.
        timeout: Request timeout in seconds (default: 30).

    Usage::

        client = OnyxClient("http://localhost:3000", api_key="sk-...")
        node = await client.nodes.create(CreateNodeRequest(name="f", content="..."))
        await client.close()

    Or as an async context manager::

        async with OnyxClient("http://localhost:3000") as client:
            healthy = await client.health()
    """

    def __init__(
        self,
        base_url: str,
        *,
        api_key: Optional[str] = None,
        timeout: float = 30.0,
    ) -> None:
        self._transport = _HttpTransport(base_url, api_key, timeout)

        self.nodes = NodesClient(self._transport)
        """Node CRUD operations."""

        self.edges = EdgesClient(self._transport)
        """Edge CRUD operations."""

        self.search = SearchClient(self._transport)
        """Semantic search operations."""

        self.history = HistoryClient(self._transport)
        """Version history and branching."""

        self.ingest = IngestClient(self._transport)
        """Code ingestion pipeline."""

        self.billing = BillingClient(self._transport)
        """Stripe billing integration."""

    async def health(self) -> bool:
        """Check whether the Onyx server is healthy.

        Returns:
            ``True`` if the server responded successfully.
        """
        try:
            await self._transport.get("/health")
            return True
        except Exception:
            return False

    async def close(self) -> None:
        """Close the underlying HTTP connection pool."""
        await self._transport.close()

    async def __aenter__(self) -> "OnyxClient":
        return self

    async def __aexit__(self, *exc: Any) -> None:
        await self.close()
