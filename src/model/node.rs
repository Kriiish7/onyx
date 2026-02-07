use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::model::version::VersionId;

// ---------------------------------------------------------------------------
// Node: the fundamental entity in Onyx's knowledge graph
// ---------------------------------------------------------------------------

/// A node in the Onyx knowledge graph. Every code artifact, document, test,
/// and config file is represented as a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Globally unique identifier.
    pub id: Uuid,
    /// The type of this node (code entity, doc, test, config).
    pub node_type: NodeType,
    /// Human-readable name (e.g., function name, doc title).
    pub name: String,
    /// Full source text or document body.
    pub content: String,
    /// SHA-256 hash of content for dedup and change detection.
    pub content_hash: [u8; 32],
    /// Extensible key-value metadata.
    pub metadata: HashMap<String, String>,
    /// Origin information: file path, line range, commit, repo.
    pub provenance: Provenance,
    /// Vector embedding for semantic search (None if not yet computed).
    pub embedding: Option<Vec<f32>>,
    /// Pointer to the latest version in the history store.
    pub current_version: Option<VersionId>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp.
    pub updated_at: DateTime<Utc>,
    /// Type-specific extension data.
    pub extension: NodeExtension,
}

impl Node {
    /// Create a new node with the given type and name.
    /// Content hash is computed automatically.
    pub fn new(node_type: NodeType, name: impl Into<String>, content: impl Into<String>) -> Self {
        use sha2::{Digest, Sha256};

        let name = name.into();
        let content = content.into();
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            node_type: node_type.clone(),
            name,
            content,
            content_hash: hash,
            metadata: HashMap::new(),
            provenance: Provenance::default(),
            embedding: None,
            current_version: None,
            created_at: now,
            updated_at: now,
            extension: NodeExtension::from_node_type(&node_type),
        }
    }

    /// Set provenance information for this node.
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = provenance;
        self
    }

    /// Set metadata on this node.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set the embedding vector.
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }
}

// ---------------------------------------------------------------------------
// NodeType: categorizes what kind of artifact a node represents
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    CodeEntity(CodeEntityKind),
    Doc,
    Test,
    Config,
}

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

// ---------------------------------------------------------------------------
// Type-specific extension data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeExtension {
    CodeEntity(CodeEntityExt),
    Doc(DocExt),
    Test(TestExt),
    Config(ConfigExt),
    None,
}

impl NodeExtension {
    fn from_node_type(nt: &NodeType) -> Self {
        match nt {
            NodeType::CodeEntity(_) => NodeExtension::CodeEntity(CodeEntityExt::default()),
            NodeType::Doc => NodeExtension::Doc(DocExt::default()),
            NodeType::Test => NodeExtension::Test(TestExt::default()),
            NodeType::Config => NodeExtension::Config(ConfigExt::default()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEntityExt {
    pub language: Language,
    pub signature: Option<String>,
    pub visibility: Visibility,
    pub module_path: Vec<String>,
    pub line_range: Option<(usize, usize)>,
}

impl Default for CodeEntityExt {
    fn default() -> Self {
        Self {
            language: Language::Rust,
            signature: None,
            visibility: Visibility::Private,
            module_path: Vec::new(),
            line_range: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Go,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    PubCrate,
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocExt {
    pub doc_type: DocType,
    pub format: DocFormat,
    pub target_id: Option<Uuid>,
}

impl Default for DocExt {
    fn default() -> Self {
        Self {
            doc_type: DocType::Readme,
            format: DocFormat::Markdown,
            target_id: None,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestExt {
    pub test_kind: TestKind,
    pub target_ids: Vec<Uuid>,
    pub last_result: Option<TestResult>,
}

impl Default for TestExt {
    fn default() -> Self {
        Self {
            test_kind: TestKind::Unit,
            target_ids: Vec::new(),
            last_result: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestKind {
    Unit,
    Integration,
    Property,
    Benchmark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub passed: bool,
    pub timestamp: DateTime<Utc>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigExt {
    pub config_type: ConfigType,
    pub format: ConfigFormat,
}

impl Default for ConfigExt {
    fn default() -> Self {
        Self {
            config_type: ConfigType::Cargo,
            format: ConfigFormat::Toml,
        }
    }
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
// Provenance: tracks where a node came from
// ---------------------------------------------------------------------------

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
