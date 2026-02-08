/**
 * HTTP client and sub-clients for the Onyx API.
 *
 * The main entry point is {@link OnyxClient}. Sub-clients for each domain area
 * are exposed as properties on the main client instance.
 *
 * @module
 */

import { OnyxApiError, OnyxNetworkError, OnyxNotFoundError } from "./errors";
import type {
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
} from "./models";

// ============================================================================
// Options
// ============================================================================

/** Configuration options for {@link OnyxClient}. */
export interface OnyxClientOptions {
  /** API key for authentication (sent as `Authorization: Bearer <key>`). */
  apiKey?: string;
  /** Request timeout in milliseconds (default: 30 000). */
  timeout?: number;
  /** Custom `fetch` implementation (default: global `fetch`). */
  fetch?: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;
}

// ============================================================================
// Internal HTTP transport
// ============================================================================

class HttpTransport {
  private readonly baseUrl: string;
  private readonly headers: Record<string, string>;
  private readonly timeout: number;
  private readonly fetchFn: (
    input: RequestInfo | URL,
    init?: RequestInit,
  ) => Promise<Response>;

  constructor(baseUrl: string, options: OnyxClientOptions) {
    // Strip trailing slash
    this.baseUrl = baseUrl.replace(/\/+$/, "");
    this.headers = { "Content-Type": "application/json" };
    if (options.apiKey) {
      this.headers["Authorization"] = `Bearer ${options.apiKey}`;
    }
    this.timeout = options.timeout ?? 30_000;
    this.fetchFn = options.fetch ?? globalThis.fetch.bind(globalThis);
  }

  async get<T>(path: string): Promise<T> {
    return this.request<T>("GET", path);
  }

  async getText(path: string): Promise<string> {
    const resp = await this.doFetch("GET", path);
    return resp.text();
  }

  async post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>("POST", path, body);
  }

  async put<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>("PUT", path, body);
  }

  async del(path: string): Promise<void> {
    const resp = await this.doFetch("DELETE", path);
    if (!resp.ok) {
      const text = await resp.text();
      throw new OnyxApiError(resp.status, text);
    }
  }

  // ---- internal -----------------------------------------------------------

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
    const resp = await this.doFetch(method, path, body);
    return this.handleResponse<T>(resp);
  }

  private async doFetch(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<Response> {
    const url = `${this.baseUrl}${path}`;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeout);

    try {
      return await this.fetchFn(url, {
        method,
        headers: this.headers,
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });
    } catch (err) {
      throw new OnyxNetworkError(
        err instanceof Error ? err.message : String(err),
      );
    } finally {
      clearTimeout(timer);
    }
  }

  private async handleResponse<T>(resp: Response): Promise<T> {
    if (resp.status === 404) {
      const text = await resp.text();
      throw new OnyxNotFoundError(text);
    }
    if (!resp.ok) {
      const text = await resp.text();
      throw new OnyxApiError(resp.status, text);
    }
    return (await resp.json()) as T;
  }
}

// ============================================================================
// Sub-clients
// ============================================================================

/**
 * Node CRUD operations.
 *
 * Access via `client.nodes`.
 */
export class NodesClient {
  /** @internal */
  constructor(private readonly t: HttpTransport) {}

  /**
   * Create a new node.
   * @param request - Node creation parameters.
   * @returns The newly created node.
   */
  async create(request: CreateNodeRequest): Promise<Node> {
    return this.t.post<Node>("/api/nodes", request);
  }

  /**
   * Retrieve a node by ID.
   * @param id - UUID of the node.
   * @throws {@link OnyxNotFoundError} if the node does not exist.
   */
  async get(id: string): Promise<Node> {
    return this.t.get<Node>(`/api/nodes/${id}`);
  }

  /**
   * Update an existing node.
   * @param id - UUID of the node.
   * @param request - Fields to change (omitted fields are left unchanged).
   */
  async update(id: string, request: UpdateNodeRequest): Promise<Node> {
    return this.t.put<Node>(`/api/nodes/${id}`, request);
  }

  /**
   * Delete a node and all its edges.
   * @param id - UUID of the node to remove.
   */
  async delete(id: string): Promise<void> {
    return this.t.del(`/api/nodes/${id}`);
  }

  /**
   * Paginated list of all nodes.
   * @param page - Page number (1-indexed).
   * @param perPage - Items per page.
   */
  async list(page = 1, perPage = 50): Promise<ListNodesResponse> {
    return this.t.get<ListNodesResponse>(
      `/api/nodes?page=${page}&per_page=${perPage}`,
    );
  }

  /**
   * Get the neighbors of a node.
   * @param id - UUID of the node.
   */
  async neighbors(id: string): Promise<NeighborResult[]> {
    return this.t.get<NeighborResult[]>(`/api/nodes/${id}/neighbors`);
  }

  /**
   * Extract a subgraph rooted at a node.
   * @param id - Root node UUID.
   * @param depth - Maximum traversal depth (default: 2).
   */
  async subgraph(id: string, depth = 2): Promise<SubgraphResult> {
    return this.t.get<SubgraphResult>(
      `/api/nodes/${id}/subgraph?depth=${depth}`,
    );
  }
}

/**
 * Edge CRUD operations.
 *
 * Access via `client.edges`.
 */
export class EdgesClient {
  /** @internal */
  constructor(private readonly t: HttpTransport) {}

  /** Create a new directed edge. */
  async create(request: CreateEdgeRequest): Promise<Edge> {
    return this.t.post<Edge>("/api/edges", request);
  }

  /** Retrieve an edge by ID. */
  async get(id: string): Promise<Edge> {
    return this.t.get<Edge>(`/api/edges/${id}`);
  }

  /** Delete an edge by ID. */
  async delete(id: string): Promise<void> {
    return this.t.del(`/api/edges/${id}`);
  }

  /** List all edges. */
  async list(): Promise<ListEdgesResponse> {
    return this.t.get<ListEdgesResponse>("/api/edges");
  }
}

/**
 * Semantic search operations.
 *
 * Access via `client.search`.
 */
export class SearchClient {
  /** @internal */
  constructor(private readonly t: HttpTransport) {}

  /**
   * Execute a semantic query.
   * @param request - Search parameters including the query embedding.
   * @returns A search response with ranked result items.
   */
  async query(request: SearchRequest): Promise<SearchResponse> {
    return this.t.post<SearchResponse>("/api/search", request);
  }
}

/**
 * Version history and branching.
 *
 * Access via `client.history`.
 */
export class HistoryClient {
  /** @internal */
  constructor(private readonly t: HttpTransport) {}

  /** Record a new version. */
  async createVersion(request: CreateVersionRequest): Promise<VersionEntry> {
    return this.t.post<VersionEntry>("/api/versions", request);
  }

  /** Get a version by ID. */
  async getVersion(versionId: string): Promise<VersionEntry> {
    return this.t.get<VersionEntry>(`/api/versions/${versionId}`);
  }

  /** List all versions for an entity. */
  async listVersions(entityId: string): Promise<ListVersionsResponse> {
    return this.t.get<ListVersionsResponse>(
      `/api/entities/${entityId}/versions`,
    );
  }

  /** Get entity content at a specific version. */
  async getContentAtVersion(
    entityId: string,
    versionId: string,
  ): Promise<string> {
    return this.t.getText(
      `/api/entities/${entityId}/versions/${versionId}/content`,
    );
  }

  /** Create a new branch. */
  async createBranch(request: CreateBranchRequest): Promise<Branch> {
    return this.t.post<Branch>("/api/branches", request);
  }

  /** Get branch metadata. */
  async getBranch(name: string): Promise<Branch> {
    return this.t.get<Branch>(`/api/branches/${name}`);
  }

  /** List all branches. */
  async listBranches(): Promise<ListBranchesResponse> {
    return this.t.get<ListBranchesResponse>("/api/branches");
  }

  /** Merge a source branch into a target branch. */
  async mergeBranch(request: MergeBranchRequest): Promise<VersionEntry> {
    return this.t.post<VersionEntry>("/api/branches/merge", request);
  }
}

/**
 * Code ingestion pipeline.
 *
 * Access via `client.ingest`.
 */
export class IngestClient {
  /** @internal */
  constructor(private readonly t: HttpTransport) {}

  /**
   * Ingest a single code unit.
   * @param request - Code unit parameters.
   */
  async ingestUnit(request: IngestCodeUnitRequest): Promise<IngestResult> {
    return this.t.post<IngestResult>("/api/ingest/unit", request);
  }

  /**
   * Ingest an entire codebase (batch).
   * @param request - Batch of code units.
   */
  async ingestCodebase(
    request: IngestCodebaseRequest,
  ): Promise<IngestCodebaseResponse> {
    return this.t.post<IngestCodebaseResponse>("/api/ingest/codebase", request);
  }
}

/**
 * Stripe billing integration.
 *
 * Access via `client.billing`.
 */
export class BillingClient {
  /** @internal */
  constructor(private readonly t: HttpTransport) {}

  /** Create a Stripe checkout session. */
  async createCheckoutSession(
    request: CheckoutSessionRequest,
  ): Promise<CheckoutSessionResponse> {
    return this.t.post<CheckoutSessionResponse>("/billing/checkout", request);
  }

  /** Create a Stripe billing portal session. */
  async createBillingPortal(
    request: BillingPortalRequest,
  ): Promise<BillingPortalResponse> {
    return this.t.post<BillingPortalResponse>("/billing/portal", request);
  }
}

// ============================================================================
// Main client
// ============================================================================

/**
 * The main Onyx API client.
 *
 * Provides access to all Onyx services through typed sub-clients.
 *
 * @example
 * ```typescript
 * const client = new OnyxClient("http://localhost:3000", { apiKey: "sk-..." });
 *
 * const node = await client.nodes.create({
 *   name: "main",
 *   content: "fn main() {}",
 *   nodeType: { type: "CodeEntity", kind: "Function" },
 * });
 *
 * const results = await client.search.query({
 *   embedding: [0.1, 0.2, 0.3],
 *   topK: 5,
 * });
 * ```
 */
export class OnyxClient {
  private readonly transport: HttpTransport;

  /** Node CRUD operations. */
  public readonly nodes: NodesClient;
  /** Edge CRUD operations. */
  public readonly edges: EdgesClient;
  /** Semantic search operations. */
  public readonly search: SearchClient;
  /** Version history and branching. */
  public readonly history: HistoryClient;
  /** Code ingestion pipeline. */
  public readonly ingest: IngestClient;
  /** Stripe billing integration. */
  public readonly billing: BillingClient;

  /**
   * Create a new Onyx client.
   *
   * @param baseUrl - The base URL of the Onyx server (e.g. `http://localhost:3000`).
   * @param options - Optional configuration (API key, timeout, custom fetch).
   */
  constructor(baseUrl: string, options: OnyxClientOptions = {}) {
    this.transport = new HttpTransport(baseUrl, options);
    this.nodes = new NodesClient(this.transport);
    this.edges = new EdgesClient(this.transport);
    this.search = new SearchClient(this.transport);
    this.history = new HistoryClient(this.transport);
    this.ingest = new IngestClient(this.transport);
    this.billing = new BillingClient(this.transport);
  }

  /**
   * Check whether the Onyx server is healthy.
   * @returns `true` if the server responded successfully.
   */
  async health(): Promise<boolean> {
    try {
      await this.transport.get<string>("/health");
      return true;
    } catch {
      return false;
    }
  }
}
