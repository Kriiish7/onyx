"""
Pydantic data models for the Onyx SDK.

All models mirror the server-side Rust structures and are used for
request / response serialization over JSON.
"""

from __future__ import annotations

from datetime import datetime
from enum import Enum
from typing import Any, Dict, List, Optional, Tuple
from uuid import UUID

from pydantic import BaseModel, Field

# ============================================================================
# Enums
# ============================================================================


class CodeEntityKind(str, Enum):
    """Kind of code entity."""

    FUNCTION = "Function"
    STRUCT = "Struct"
    ENUM = "Enum"
    TRAIT = "Trait"
    IMPL = "Impl"
    MODULE = "Module"
    CONSTANT = "Constant"
    TYPE_ALIAS = "TypeAlias"
    MACRO = "Macro"


class Language(str, Enum):
    """Programming language."""

    RUST = "Rust"
    PYTHON = "Python"
    TYPESCRIPT = "TypeScript"
    JAVASCRIPT = "JavaScript"
    GO = "Go"


class Visibility(str, Enum):
    """Visibility level."""

    PUBLIC = "Public"
    PUB_CRATE = "PubCrate"
    PRIVATE = "Private"


class DocType(str, Enum):
    INLINE = "Inline"
    README = "Readme"
    API_DOC = "ApiDoc"
    TUTORIAL = "Tutorial"


class DocFormat(str, Enum):
    MARKDOWN = "Markdown"
    RUST_DOC = "RustDoc"
    PLAIN_TEXT = "PlainText"


class TestKind(str, Enum):
    UNIT = "Unit"
    INTEGRATION = "Integration"
    PROPERTY = "Property"
    BENCHMARK = "Benchmark"


class ConfigType(str, Enum):
    CARGO = "Cargo"
    CI = "CI"
    DOCKER = "Docker"
    ENV = "Env"
    BUILD = "Build"


class ConfigFormat(str, Enum):
    TOML = "Toml"
    YAML = "Yaml"
    JSON = "Json"
    INI = "Ini"


class EdgeType(str, Enum):
    """Relationship categories."""

    DEFINES = "Defines"
    CALLS = "Calls"
    IMPORTS = "Imports"
    DOCUMENTS = "Documents"
    TESTS_OF = "TestsOf"
    VERSIONED_BY = "VersionedBy"
    CONTAINS = "Contains"
    IMPLEMENTS = "Implements"
    DEPENDS_ON = "DependsOn"
    CONFIGURES = "Configures"


class ResultSource(str, Enum):
    """How a search result was discovered."""

    VECTOR_SEARCH = "VectorSearch"
    GRAPH_TRAVERSAL = "GraphTraversal"
    COMBINED = "Combined"


# ============================================================================
# Node models
# ============================================================================


class NodeType(BaseModel):
    """Categorises what kind of artifact a node represents.

    For code entities, set ``type="CodeEntity"`` and ``kind`` to the specific
    :class:`CodeEntityKind`.  For other types use ``"Doc"``, ``"Test"``, or
    ``"Config"`` and leave ``kind`` as ``None``.
    """

    type: str
    kind: Optional[str] = None


class Provenance(BaseModel):
    """Tracks the origin of a node."""

    file_path: Optional[str] = None
    line_range: Optional[Tuple[int, int]] = None
    commit_id: Optional[str] = None
    repo_url: Optional[str] = None
    branch: Optional[str] = None


class TestResult(BaseModel):
    passed: bool
    timestamp: datetime
    message: Optional[str] = None


class CodeEntityExt(BaseModel):
    language: Language
    signature: Optional[str] = None
    visibility: Visibility = Visibility.PRIVATE
    module_path: List[str] = Field(default_factory=list)
    line_range: Optional[Tuple[int, int]] = None


class DocExt(BaseModel):
    doc_type: DocType = DocType.README
    format: DocFormat = DocFormat.MARKDOWN
    target_id: Optional[UUID] = None


class TestExt(BaseModel):
    test_kind: TestKind = TestKind.UNIT
    target_ids: List[UUID] = Field(default_factory=list)
    last_result: Optional[TestResult] = None


class ConfigExt(BaseModel):
    config_type: ConfigType = ConfigType.CARGO
    format: ConfigFormat = ConfigFormat.TOML


class NodeExtension(BaseModel):
    """Type-specific extension data for a node."""

    type: str
    code_entity: Optional[CodeEntityExt] = None
    doc: Optional[DocExt] = None
    test: Optional[TestExt] = None
    config: Optional[ConfigExt] = None


class Node(BaseModel):
    """A node in the Onyx knowledge graph."""

    id: UUID
    node_type: NodeType
    name: str
    content: str
    content_hash: str
    metadata: Dict[str, str] = Field(default_factory=dict)
    provenance: Provenance = Field(default_factory=Provenance)
    embedding: Optional[List[float]] = None
    current_version: Optional[str] = None
    created_at: datetime
    updated_at: datetime
    extension: Optional[NodeExtension] = None


class CreateNodeRequest(BaseModel):
    """Request body for creating a node."""

    name: str
    content: str
    node_type: Optional[NodeType] = None
    metadata: Optional[Dict[str, str]] = None
    provenance: Optional[Provenance] = None
    embedding: Optional[List[float]] = None


class UpdateNodeRequest(BaseModel):
    """Request body for updating a node (all fields optional)."""

    name: Optional[str] = None
    content: Optional[str] = None
    node_type: Optional[NodeType] = None
    metadata: Optional[Dict[str, str]] = None
    provenance: Optional[Provenance] = None
    embedding: Optional[List[float]] = None


class ListNodesResponse(BaseModel):
    nodes: List[Node]
    total: int
    page: int
    per_page: int


# ============================================================================
# Edge models
# ============================================================================


class TemporalContext(BaseModel):
    """Temporal metadata for an edge."""

    since: Optional[str] = None
    until: Optional[str] = None
    via_commit: Optional[str] = None
    since_timestamp: datetime
    until_timestamp: Optional[datetime] = None


class Edge(BaseModel):
    """A directed edge between two nodes."""

    id: UUID
    edge_type: EdgeType
    source_id: UUID
    target_id: UUID
    confidence: float
    metadata: Dict[str, str] = Field(default_factory=dict)
    temporal: TemporalContext


class CreateEdgeRequest(BaseModel):
    """Request body for creating an edge."""

    edge_type: EdgeType
    source_id: UUID
    target_id: UUID
    confidence: Optional[float] = None
    metadata: Optional[Dict[str, str]] = None


class ListEdgesResponse(BaseModel):
    edges: List[Edge]
    total: int


class NeighborResult(BaseModel):
    edge: Edge
    node: Node


class TraversalResult(BaseModel):
    nodes: List[Tuple[UUID, int]]
    edges_followed: int


class SubgraphResult(BaseModel):
    nodes: List[Node]
    edges: List[Edge]


# ============================================================================
# Search models
# ============================================================================


class VersionInfo(BaseModel):
    """Summary of a version for display in query results."""

    version_id: str
    timestamp: datetime
    message: Optional[str] = None
    author: Optional[str] = None
    lines_changed: int


class SearchRequest(BaseModel):
    """Request body for a semantic search."""

    embedding: List[float]
    top_k: Optional[int] = None
    max_depth: Optional[int] = None
    edge_types: Optional[List[EdgeType]] = None
    include_history: Optional[bool] = None
    min_confidence: Optional[float] = None


class SearchResultItem(BaseModel):
    node_id: UUID
    name: str
    content: str
    source: ResultSource
    score: float
    depth: int
    edge_path: List[EdgeType] = Field(default_factory=list)
    versions: List[VersionInfo] = Field(default_factory=list)


class SearchResponse(BaseModel):
    items: List[SearchResultItem]
    nodes_examined: int
    query_time_ms: int


# ============================================================================
# Version / history models
# ============================================================================


class Diff(BaseModel):
    """A diff between two versions (tagged union)."""

    type: str
    content: Optional[str] = None
    patch: Optional[str] = None
    additions: Optional[int] = None
    deletions: Optional[int] = None
    changed_fields: Optional[Dict[str, Tuple[str, str]]] = None
    diffs: Optional[List["Diff"]] = None


class VersionEntry(BaseModel):
    """A single version entry in an entity's history chain."""

    version_id: str
    entity_id: UUID
    parent_version: Optional[str] = None
    branch: str = "main"
    diff: Diff
    commit_id: Optional[str] = None
    author: Optional[str] = None
    message: Optional[str] = None
    timestamp: datetime


class Branch(BaseModel):
    """A named branch in the history store."""

    name: str
    head: str
    base: str
    created_at: datetime
    merged_into: Optional[str] = None


class CreateVersionRequest(BaseModel):
    entity_id: UUID
    diff: Diff
    parent_version: Optional[str] = None
    branch: Optional[str] = None
    commit_id: Optional[str] = None
    author: Optional[str] = None
    message: Optional[str] = None


class CreateBranchRequest(BaseModel):
    name: str
    base_version: str


class MergeBranchRequest(BaseModel):
    source: str
    target: str


class ListVersionsResponse(BaseModel):
    versions: List[VersionEntry]
    total: int


class ListBranchesResponse(BaseModel):
    branches: List[Branch]


# ============================================================================
# Ingestion models
# ============================================================================


class IngestCodeUnitRequest(BaseModel):
    """A code unit to ingest into Onyx."""

    name: str
    content: str
    kind: CodeEntityKind
    language: Language
    file_path: str
    line_range: Optional[Tuple[int, int]] = None
    signature: Optional[str] = None
    visibility: Optional[Visibility] = None
    module_path: Optional[List[str]] = None
    commit_id: Optional[str] = None
    branch: Optional[str] = None


class IngestCodebaseRequest(BaseModel):
    units: List[IngestCodeUnitRequest]


class IngestResult(BaseModel):
    node_id: UUID
    version_id: str
    edges_created: int


class IngestCodebaseResponse(BaseModel):
    results: List[IngestResult]
    total_edges: int


# ============================================================================
# Billing models
# ============================================================================


class CheckoutSessionRequest(BaseModel):
    """Request body for creating a Stripe checkout session."""

    customer_email: Optional[str] = None
    customer_id: Optional[str] = None
    price_id: Optional[str] = None
    success_url: Optional[str] = None
    cancel_url: Optional[str] = None
    reference_id: Optional[str] = None
    metadata: Optional[Dict[str, str]] = None


class CheckoutSessionResponse(BaseModel):
    id: str
    url: str


class BillingPortalRequest(BaseModel):
    customer_id: str
    return_url: Optional[str] = None


class BillingPortalResponse(BaseModel):
    url: str
