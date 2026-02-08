/**
 * Data models for the Onyx SDK.
 *
 * These interfaces mirror the server-side Rust structures and are used for
 * request / response serialization over JSON. All date fields are ISO-8601
 * strings.
 *
 * @module
 */

// ============================================================================
// Enums
// ============================================================================

/** Kind of code entity. */
export type CodeEntityKind =
  | "Function"
  | "Struct"
  | "Enum"
  | "Trait"
  | "Impl"
  | "Module"
  | "Constant"
  | "TypeAlias"
  | "Macro";

/** Programming language. */
export type Language =
  | "Rust"
  | "Python"
  | "TypeScript"
  | "JavaScript"
  | "Go"
  | { Other: string };

/** Visibility level. */
export type Visibility = "Public" | "PubCrate" | "Private";

/** Relationship categories. */
export type EdgeType =
  | "Defines"
  | "Calls"
  | "Imports"
  | "Documents"
  | "TestsOf"
  | "VersionedBy"
  | "Contains"
  | "Implements"
  | "DependsOn"
  | "Configures";

/** Documentation type. */
export type DocType = "Inline" | "Readme" | "ApiDoc" | "Tutorial";

/** Documentation format. */
export type DocFormat = "Markdown" | "RustDoc" | "PlainText";

/** Test kind. */
export type TestKind = "Unit" | "Integration" | "Property" | "Benchmark";

/** Configuration type. */
export type ConfigType = "Cargo" | "CI" | "Docker" | "Env" | "Build";

/** Configuration format. */
export type ConfigFormat = "Toml" | "Yaml" | "Json" | "Ini";

/** How a search result was discovered. */
export type ResultSource = "VectorSearch" | "GraphTraversal" | "Combined";

// ============================================================================
// Node models
// ============================================================================

/** Categorises what kind of artifact a node represents. */
export type NodeType =
  | { type: "CodeEntity"; kind: CodeEntityKind }
  | { type: "Doc" }
  | { type: "Test" }
  | { type: "Config" };

/** Tracks the origin of a node. */
export interface Provenance {
  filePath?: string;
  lineRange?: [number, number];
  commitId?: string;
  repoUrl?: string;
  branch?: string;
}

/** Extension data for code entities. */
export interface CodeEntityExt {
  language: Language;
  signature?: string;
  visibility: Visibility;
  modulePath: string[];
  lineRange?: [number, number];
}

/** Extension data for documentation nodes. */
export interface DocExt {
  docType: DocType;
  format: DocFormat;
  targetId?: string;
}

/** Test execution result. */
export interface TestResult {
  passed: boolean;
  timestamp: string;
  message?: string;
}

/** Extension data for test nodes. */
export interface TestExt {
  testKind: TestKind;
  targetIds: string[];
  lastResult?: TestResult;
}

/** Extension data for configuration nodes. */
export interface ConfigExt {
  configType: ConfigType;
  format: ConfigFormat;
}

/** Type-specific extension data for a node. */
export type NodeExtension =
  | { type: "CodeEntity"; data: CodeEntityExt }
  | { type: "Doc"; data: DocExt }
  | { type: "Test"; data: TestExt }
  | { type: "Config"; data: ConfigExt }
  | { type: "None" };

/** A node in the Onyx knowledge graph. */
export interface Node {
  id: string;
  nodeType: NodeType;
  name: string;
  content: string;
  contentHash: string;
  metadata: Record<string, string>;
  provenance: Provenance;
  embedding?: number[];
  currentVersion?: string;
  createdAt: string;
  updatedAt: string;
  extension?: NodeExtension;
}

/** Request body for creating a node. */
export interface CreateNodeRequest {
  name: string;
  content: string;
  nodeType?: NodeType;
  metadata?: Record<string, string>;
  provenance?: Provenance;
  embedding?: number[];
}

/** Request body for updating a node. */
export interface UpdateNodeRequest {
  name?: string;
  content?: string;
  nodeType?: NodeType;
  metadata?: Record<string, string>;
  provenance?: Provenance;
  embedding?: number[];
}

/** Paginated node list response. */
export interface ListNodesResponse {
  nodes: Node[];
  total: number;
  page: number;
  perPage: number;
}

// ============================================================================
// Edge models
// ============================================================================

/** Temporal metadata for an edge. */
export interface TemporalContext {
  since?: string;
  until?: string;
  viaCommit?: string;
  sinceTimestamp: string;
  untilTimestamp?: string;
}

/** A directed edge between two nodes. */
export interface Edge {
  id: string;
  edgeType: EdgeType;
  sourceId: string;
  targetId: string;
  confidence: number;
  metadata: Record<string, string>;
  temporal: TemporalContext;
}

/** Request body for creating an edge. */
export interface CreateEdgeRequest {
  edgeType: EdgeType;
  sourceId: string;
  targetId: string;
  confidence?: number;
  metadata?: Record<string, string>;
}

/** Response for listing edges. */
export interface ListEdgesResponse {
  edges: Edge[];
  total: number;
}

/** A neighbor result from graph traversal. */
export interface NeighborResult {
  edge: Edge;
  node: Node;
}

/** Result of a multi-hop graph traversal. */
export interface TraversalResult {
  nodes: Array<[string, number]>;
  edgesFollowed: number;
}

/** Result of a subgraph extraction. */
export interface SubgraphResult {
  nodes: Node[];
  edges: Edge[];
}

// ============================================================================
// Search models
// ============================================================================

/** Summary of a version for display in query results. */
export interface VersionInfo {
  versionId: string;
  timestamp: string;
  message?: string;
  author?: string;
  linesChanged: number;
}

/** Request body for a semantic search. */
export interface SearchRequest {
  embedding: number[];
  topK?: number;
  maxDepth?: number;
  edgeTypes?: EdgeType[];
  includeHistory?: boolean;
  minConfidence?: number;
}

/** A single search result item. */
export interface SearchResultItem {
  nodeId: string;
  name: string;
  content: string;
  source: ResultSource;
  score: number;
  depth: number;
  edgePath: EdgeType[];
  versions: VersionInfo[];
}

/** Complete search response. */
export interface SearchResponse {
  items: SearchResultItem[];
  nodesExamined: number;
  queryTimeMs: number;
}

// ============================================================================
// Version / history models
// ============================================================================

/** A diff between two versions (tagged union). */
export type Diff =
  | { type: "Initial"; content: string }
  | {
      type: "ContentChanged";
      patch: string;
      additions: number;
      deletions: number;
    }
  | { type: "MetadataChanged"; changedFields: Record<string, [string, string]> }
  | { type: "Composite"; diffs: Diff[] };

/** A single version entry in an entity's history chain. */
export interface VersionEntry {
  versionId: string;
  entityId: string;
  parentVersion?: string;
  branch: string;
  diff: Diff;
  commitId?: string;
  author?: string;
  message?: string;
  timestamp: string;
}

/** A named branch in the history store. */
export interface Branch {
  name: string;
  head: string;
  base: string;
  createdAt: string;
  mergedInto?: string;
}

/** Request to record a new version. */
export interface CreateVersionRequest {
  entityId: string;
  diff: Diff;
  parentVersion?: string;
  branch?: string;
  commitId?: string;
  author?: string;
  message?: string;
}

/** Request to create a branch. */
export interface CreateBranchRequest {
  name: string;
  baseVersion: string;
}

/** Request to merge branches. */
export interface MergeBranchRequest {
  source: string;
  target: string;
}

/** Response for listing versions. */
export interface ListVersionsResponse {
  versions: VersionEntry[];
  total: number;
}

/** Response for listing branches. */
export interface ListBranchesResponse {
  branches: Branch[];
}

// ============================================================================
// Ingestion models
// ============================================================================

/** A code unit to ingest into Onyx. */
export interface IngestCodeUnitRequest {
  name: string;
  content: string;
  kind: CodeEntityKind;
  language: Language;
  filePath: string;
  lineRange?: [number, number];
  signature?: string;
  visibility?: Visibility;
  modulePath?: string[];
  commitId?: string;
  branch?: string;
}

/** Batch ingestion request. */
export interface IngestCodebaseRequest {
  units: IngestCodeUnitRequest[];
}

/** Result of ingesting a single code unit. */
export interface IngestResult {
  nodeId: string;
  versionId: string;
  edgesCreated: number;
}

/** Result of batch ingestion. */
export interface IngestCodebaseResponse {
  results: IngestResult[];
  totalEdges: number;
}

// ============================================================================
// Billing models
// ============================================================================

/** Request body for creating a Stripe checkout session. */
export interface CheckoutSessionRequest {
  customerEmail?: string;
  customerId?: string;
  priceId?: string;
  successUrl?: string;
  cancelUrl?: string;
  referenceId?: string;
  metadata?: Record<string, string>;
}

/** Response from creating a checkout session. */
export interface CheckoutSessionResponse {
  id: string;
  url: string;
}

/** Request body for creating a billing portal session. */
export interface BillingPortalRequest {
  customerId: string;
  returnUrl?: string;
}

/** Response from creating a billing portal session. */
export interface BillingPortalResponse {
  url: string;
}
