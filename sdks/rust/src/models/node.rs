//! Node models — knowledge graph vertices.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Core node types
// ---------------------------------------------------------------------------

/// A node in the Onyx knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Globally unique identifier.
    pub id: Uuid,
    /// The type of this node.
    pub node_type: NodeType,
    /// Human-readable name.
    pub name: String,
    /// Full source text or document body.
    pub content: String,
    /// SHA-256 content hash (hex-encoded).
    pub content_hash: String,
    /// Extensible key-value metadata.
    pub metadata: HashMap<String, String>,
    /// Origin information.
    pub provenance: Provenance,
    /// Vector embedding (if computed).
    pub embedding: Option<Vec<f32>>,
    /// Current version identifier.
    pub current_version: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp.
    pub updated_at: DateTime<Utc>,
    /// Type-specific extension data.
    pub extension: Option<NodeExtension>,
}

/// Categorises what kind of artifact a node represents.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "kind")]
pub enum NodeType {
    /// A code entity with a specific kind.
    CodeEntity(CodeEntityKind),
    /// Documentation.
    Doc,
    /// A test.
    Test,
    /// Configuration file.
    Config,
}

impl NodeType {
    /// Shorthand for creating a code-entity node type.
    pub fn code_entity(kind: CodeEntityKind) -> Self {
        NodeType::CodeEntity(kind)
    }
}

/// The kind of code entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodeEntityKind {
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

/// Type-specific extension data carried by a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NodeExtension {
    CodeEntity(CodeEntityExt),
    Doc(DocExt),
    Test(TestExt),
    Config(ConfigExt),
    None,
}

/// Extension data for code entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEntityExt {
    pub language: Language,
    pub signature: Option<String>,
    pub visibility: Visibility,
    pub module_path: Vec<String>,
    pub line_range: Option<(usize, usize)>,
}

/// Programming language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Go,
    Other(String),
}

/// Visibility level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    PubCrate,
    Private,
}

/// Extension data for documentation nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocExt {
    pub doc_type: DocType,
    pub format: DocFormat,
    pub target_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocType {
    Inline,
    Readme,
    ApiDoc,
    Tutorial,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocFormat {
    Markdown,
    RustDoc,
    PlainText,
}

/// Extension data for test nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestExt {
    pub test_kind: TestKind,
    pub target_ids: Vec<Uuid>,
    pub last_result: Option<TestResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestKind {
    Unit,
    Integration,
    Property,
    Benchmark,
}

/// Result of a test execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub passed: bool,
    pub timestamp: DateTime<Utc>,
    pub message: Option<String>,
}

/// Extension data for configuration nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigExt {
    pub config_type: ConfigType,
    pub format: ConfigFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigType {
    Cargo,
    CI,
    Docker,
    Env,
    Build,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigFormat {
    Toml,
    Yaml,
    Json,
    Ini,
}

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

/// Tracks the origin of a node (file, line, commit, repo, branch).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Provenance {
    pub file_path: Option<String>,
    pub line_range: Option<(usize, usize)>,
    pub commit_id: Option<String>,
    pub repo_url: Option<String>,
    pub branch: Option<String>,
}

impl Provenance {
    pub fn new(file_path: impl Into<String>) -> Self {
        Self {
            file_path: Some(file_path.into()),
            ..Default::default()
        }
    }

    pub fn with_lines(mut self, start: usize, end: usize) -> Self {
        self.line_range = Some((start, end));
        self
    }

    pub fn with_commit(mut self, commit: impl Into<String>) -> Self {
        self.commit_id = Some(commit.into());
        self
    }

    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

/// Request body for creating a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNodeRequest {
    pub name: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_type: Option<NodeType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

impl CreateNodeRequest {
    /// Create a new request with the required fields.
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            node_type: None,
            metadata: None,
            provenance: None,
            embedding: None,
        }
    }

    /// Set the node type.
    pub fn node_type(mut self, nt: NodeType) -> Self {
        self.node_type = Some(nt);
        self
    }

    /// Set metadata.
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set provenance.
    pub fn provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// Set the embedding vector.
    pub fn embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }
}

/// Request body for updating a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNodeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_type: Option<NodeType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

impl Default for UpdateNodeRequest {
    fn default() -> Self {
        Self {
            name: None,
            content: None,
            node_type: None,
            metadata: None,
            provenance: None,
            embedding: None,
        }
    }
}

/// Paginated list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListNodesResponse {
    pub nodes: Vec<Node>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}
