//! HTTP client and sub-clients for the Onyx API.
//!
//! The main entry point is [`OnyxClient`], which is built via
//! [`OnyxClientBuilder`]. Sub-clients for each domain area are accessible via
//! methods on the main client.

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::sync::Arc;
use url::Url;
use uuid::Uuid;

use crate::error::{OnyxError, OnyxResult};
use crate::models::*;

// ---------------------------------------------------------------------------
// Internal shared state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ClientInner {
    http: reqwest::Client,
    base_url: Url,
    api_key: Option<String>,
}

impl ClientInner {
    /// Build the full URL for an API path.
    fn url(&self, path: &str) -> OnyxResult<Url> {
        self.base_url.join(path).map_err(OnyxError::UrlParseError)
    }

    /// Execute a GET request and deserialize the JSON response.
    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> OnyxResult<T> {
        let url = self.url(path)?;
        let resp = self.http.get(url).send().await?;
        Self::handle_response(resp).await
    }

    /// Execute a POST request with a JSON body.
    async fn post<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> OnyxResult<T> {
        let url = self.url(path)?;
        let resp = self.http.post(url).json(body).send().await?;
        Self::handle_response(resp).await
    }

    /// Execute a PUT request with a JSON body.
    async fn put<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> OnyxResult<T> {
        let url = self.url(path)?;
        let resp = self.http.put(url).json(body).send().await?;
        Self::handle_response(resp).await
    }

    /// Execute a DELETE request.
    async fn delete(&self, path: &str) -> OnyxResult<()> {
        let url = self.url(path)?;
        let resp = self.http.delete(url).send().await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            Err(OnyxError::ApiError { status, message })
        }
    }

    /// Process an HTTP response, returning the deserialized body or an error.
    async fn handle_response<T: serde::de::DeserializeOwned>(
        resp: reqwest::Response,
    ) -> OnyxResult<T> {
        let status = resp.status();
        if status.is_success() {
            let body = resp.json::<T>().await?;
            Ok(body)
        } else {
            let code = status.as_u16();
            let message = resp.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::NOT_FOUND {
                Err(OnyxError::NotFound(message))
            } else {
                Err(OnyxError::ApiError {
                    status: code,
                    message,
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OnyxClient
// ---------------------------------------------------------------------------

/// The main Onyx API client.
///
/// Use [`OnyxClient::builder`] to create an instance:
///
/// ```rust,no_run
/// use onyx_sdk::OnyxClient;
///
/// # async fn example() -> Result<(), onyx_sdk::OnyxError> {
/// let client = OnyxClient::builder("http://localhost:3000")
///     .api_key("sk-...")
///     .build()?;
///
/// let ok = client.health().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct OnyxClient {
    inner: Arc<ClientInner>,
}

impl OnyxClient {
    /// Start building a new client.
    pub fn builder(base_url: &str) -> OnyxClientBuilder {
        OnyxClientBuilder {
            base_url: base_url.to_string(),
            api_key: None,
            timeout_secs: 30,
        }
    }

    // -- Health ---------------------------------------------------------------

    /// Check whether the Onyx server is healthy.
    pub async fn health(&self) -> OnyxResult<bool> {
        let resp = self.inner.http.get(self.inner.url("/health")?).send().await?;
        Ok(resp.status().is_success())
    }

    // -- Sub-clients ----------------------------------------------------------

    /// Access node CRUD operations.
    pub fn nodes(&self) -> NodesClient {
        NodesClient {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Access edge CRUD operations.
    pub fn edges(&self) -> EdgesClient {
        EdgesClient {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Access semantic search.
    pub fn search(&self) -> SearchClient {
        SearchClient {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Access version history and branching.
    pub fn history(&self) -> HistoryClient {
        HistoryClient {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Access code ingestion.
    pub fn ingest(&self) -> IngestClient {
        IngestClient {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Access billing / Stripe integration.
    pub fn billing(&self) -> BillingClient {
        BillingClient {
            inner: Arc::clone(&self.inner),
        }
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for [`OnyxClient`].
pub struct OnyxClientBuilder {
    base_url: String,
    api_key: Option<String>,
    timeout_secs: u64,
}

impl OnyxClientBuilder {
    /// Set the API key for authentication.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the request timeout in seconds (default: 30).
    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Build the client.
    pub fn build(self) -> OnyxResult<OnyxClient> {
        let base_url: Url = self
            .base_url
            .parse()
            .map_err(|e: url::ParseError| OnyxError::ConfigError(e.to_string()))?;

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if let Some(ref key) = self.api_key {
            let value = HeaderValue::from_str(&format!("Bearer {key}"))
                .map_err(|e| OnyxError::ConfigError(e.to_string()))?;
            headers.insert(AUTHORIZATION, value);
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()?;

        Ok(OnyxClient {
            inner: Arc::new(ClientInner {
                http,
                base_url,
                api_key: self.api_key,
            }),
        })
    }
}

// ---------------------------------------------------------------------------
// NodesClient
// ---------------------------------------------------------------------------

/// Sub-client for node CRUD operations.
#[derive(Debug, Clone)]
pub struct NodesClient {
    inner: Arc<ClientInner>,
}

impl NodesClient {
    /// Create a new node.
    pub async fn create(&self, req: CreateNodeRequest) -> OnyxResult<Node> {
        self.inner.post("/api/nodes", &req).await
    }

    /// Get a node by ID.
    pub async fn get(&self, id: Uuid) -> OnyxResult<Node> {
        self.inner.get(&format!("/api/nodes/{id}")).await
    }

    /// Update an existing node.
    pub async fn update(&self, id: Uuid, req: UpdateNodeRequest) -> OnyxResult<Node> {
        self.inner.put(&format!("/api/nodes/{id}"), &req).await
    }

    /// Delete a node and all its edges.
    pub async fn delete(&self, id: Uuid) -> OnyxResult<()> {
        self.inner.delete(&format!("/api/nodes/{id}")).await
    }

    /// List all nodes with pagination.
    pub async fn list(&self, page: usize, per_page: usize) -> OnyxResult<ListNodesResponse> {
        self.inner
            .get(&format!("/api/nodes?page={page}&per_page={per_page}"))
            .await
    }

    /// Get the neighbors of a node.
    pub async fn neighbors(&self, id: Uuid) -> OnyxResult<Vec<NeighborResult>> {
        self.inner
            .get(&format!("/api/nodes/{id}/neighbors"))
            .await
    }

    /// Get a subgraph rooted at a node.
    pub async fn subgraph(&self, id: Uuid, depth: usize) -> OnyxResult<SubgraphResult> {
        self.inner
            .get(&format!("/api/nodes/{id}/subgraph?depth={depth}"))
            .await
    }
}

// ---------------------------------------------------------------------------
// EdgesClient
// ---------------------------------------------------------------------------

/// Sub-client for edge CRUD operations.
#[derive(Debug, Clone)]
pub struct EdgesClient {
    inner: Arc<ClientInner>,
}

impl EdgesClient {
    /// Create a new edge.
    pub async fn create(&self, req: CreateEdgeRequest) -> OnyxResult<Edge> {
        self.inner.post("/api/edges", &req).await
    }

    /// Get an edge by ID.
    pub async fn get(&self, id: Uuid) -> OnyxResult<Edge> {
        self.inner.get(&format!("/api/edges/{id}")).await
    }

    /// Delete an edge.
    pub async fn delete(&self, id: Uuid) -> OnyxResult<()> {
        self.inner.delete(&format!("/api/edges/{id}")).await
    }

    /// List all edges.
    pub async fn list(&self) -> OnyxResult<ListEdgesResponse> {
        self.inner.get("/api/edges").await
    }
}

// ---------------------------------------------------------------------------
// SearchClient
// ---------------------------------------------------------------------------

/// Sub-client for semantic search operations.
#[derive(Debug, Clone)]
pub struct SearchClient {
    inner: Arc<ClientInner>,
}

impl SearchClient {
    /// Execute a semantic query.
    pub async fn query(&self, req: SearchRequest) -> OnyxResult<SearchResponse> {
        self.inner.post("/api/search", &req).await
    }
}

// ---------------------------------------------------------------------------
// HistoryClient
// ---------------------------------------------------------------------------

/// Sub-client for version history and branching.
#[derive(Debug, Clone)]
pub struct HistoryClient {
    inner: Arc<ClientInner>,
}

impl HistoryClient {
    /// Record a new version.
    pub async fn create_version(&self, req: CreateVersionRequest) -> OnyxResult<VersionEntry> {
        self.inner.post("/api/versions", &req).await
    }

    /// Get a version by ID.
    pub async fn get_version(&self, version_id: &str) -> OnyxResult<VersionEntry> {
        self.inner
            .get(&format!("/api/versions/{version_id}"))
            .await
    }

    /// List all versions for an entity.
    pub async fn list_versions(&self, entity_id: Uuid) -> OnyxResult<ListVersionsResponse> {
        self.inner
            .get(&format!("/api/entities/{entity_id}/versions"))
            .await
    }

    /// Get the content of an entity at a specific version.
    pub async fn get_content_at_version(
        &self,
        entity_id: Uuid,
        version_id: &str,
    ) -> OnyxResult<String> {
        self.inner
            .get(&format!(
                "/api/entities/{entity_id}/versions/{version_id}/content"
            ))
            .await
    }

    /// Create a new branch.
    pub async fn create_branch(&self, req: CreateBranchRequest) -> OnyxResult<Branch> {
        self.inner.post("/api/branches", &req).await
    }

    /// Get branch metadata.
    pub async fn get_branch(&self, name: &str) -> OnyxResult<Branch> {
        self.inner.get(&format!("/api/branches/{name}")).await
    }

    /// List all branches.
    pub async fn list_branches(&self) -> OnyxResult<ListBranchesResponse> {
        self.inner.get("/api/branches").await
    }

    /// Merge a source branch into a target branch.
    pub async fn merge_branch(&self, req: MergeBranchRequest) -> OnyxResult<VersionEntry> {
        self.inner.post("/api/branches/merge", &req).await
    }
}

// ---------------------------------------------------------------------------
// IngestClient
// ---------------------------------------------------------------------------

/// Sub-client for code ingestion.
#[derive(Debug, Clone)]
pub struct IngestClient {
    inner: Arc<ClientInner>,
}

impl IngestClient {
    /// Ingest a single code unit.
    pub async fn ingest_unit(&self, req: IngestCodeUnitRequest) -> OnyxResult<IngestResult> {
        self.inner.post("/api/ingest/unit", &req).await
    }

    /// Ingest an entire codebase (batch).
    pub async fn ingest_codebase(
        &self,
        req: IngestCodebaseRequest,
    ) -> OnyxResult<IngestCodebaseResponse> {
        self.inner.post("/api/ingest/codebase", &req).await
    }
}

// ---------------------------------------------------------------------------
// BillingClient
// ---------------------------------------------------------------------------

/// Sub-client for Stripe billing integration.
#[derive(Debug, Clone)]
pub struct BillingClient {
    inner: Arc<ClientInner>,
}

impl BillingClient {
    /// Create a Stripe checkout session.
    pub async fn create_checkout_session(
        &self,
        req: CheckoutSessionRequest,
    ) -> OnyxResult<CheckoutSessionResponse> {
        self.inner.post("/billing/checkout", &req).await
    }

    /// Create a Stripe billing portal session.
    pub async fn create_billing_portal(
        &self,
        req: BillingPortalRequest,
    ) -> OnyxResult<BillingPortalResponse> {
        self.inner.post("/billing/portal", &req).await
    }
}
