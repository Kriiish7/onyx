use uuid::Uuid;

use crate::error::{OnyxError, OnyxResult};
use crate::model::edge::{Edge, EdgeType};
use crate::model::embedding::BagOfWordsEmbedder;
use crate::model::node::{
    CodeEntityExt, CodeEntityKind, Language, Node, NodeExtension, NodeType, Provenance, Visibility,
};
use crate::model::version::VersionEntry;
use crate::store::graph::GraphStore;
use crate::store::transaction::{TransactionManager, TransactionOp};

// ---------------------------------------------------------------------------
// Ingestion Engine: parse code artifacts and populate all three stores
// ---------------------------------------------------------------------------

/// A code unit to ingest into Onyx.
#[derive(Debug, Clone)]
pub struct CodeUnit {
    /// The name of this code entity.
    pub name: String,
    /// The full source content.
    pub content: String,
    /// What kind of code entity this is.
    pub kind: CodeEntityKind,
    /// The programming language.
    pub language: Language,
    /// File path within the repository.
    pub file_path: String,
    /// Line range within the file.
    pub line_range: Option<(usize, usize)>,
    /// Function/method signature.
    pub signature: Option<String>,
    /// Visibility.
    pub visibility: Visibility,
    /// Module path.
    pub module_path: Vec<String>,
    /// Git commit hash at time of ingestion.
    pub commit_id: Option<String>,
    /// Branch name.
    pub branch: Option<String>,
}

/// Result of ingesting a code unit.
#[derive(Debug, Clone)]
pub struct IngestResult {
    /// The ID assigned to the ingested node.
    pub node_id: Uuid,
    /// The version ID of the initial version.
    pub version_id: String,
    /// Number of relationships detected.
    pub edges_created: usize,
}

/// Ingest a single code unit into the Onyx stores.
///
/// This function:
/// 1. Creates a Node with type-specific extensions
/// 2. Generates an embedding for semantic search
/// 3. Records an initial version in the history store
/// 4. Commits all operations atomically via the TransactionManager
pub async fn ingest_code_unit(
    stores: &mut TransactionManager,
    unit: &CodeUnit,
    embedder: &BagOfWordsEmbedder,
) -> OnyxResult<IngestResult> {
    // 1. Create the node
    let mut node = Node::new(
        NodeType::CodeEntity(unit.kind.clone()),
        &unit.name,
        &unit.content,
    );

    // Set provenance
    let mut provenance = Provenance::new(&unit.file_path);
    if let Some((start, end)) = unit.line_range {
        provenance = provenance.with_lines(start, end);
    }
    if let Some(ref commit) = unit.commit_id {
        provenance = provenance.with_commit(commit);
    }
    if let Some(ref branch) = unit.branch {
        provenance = provenance.with_branch(branch);
    }
    node.provenance = provenance;

    // Set code entity extension
    node.extension = NodeExtension::CodeEntity(CodeEntityExt {
        language: unit.language.clone(),
        signature: unit.signature.clone(),
        visibility: unit.visibility.clone(),
        module_path: unit.module_path.clone(),
        line_range: unit.line_range,
    });

    // 2. Generate embedding
    let embedding = embedder.embed(&unit.content);
    node.embedding = Some(embedding.values.clone());

    // 3. Create initial version
    let version =
        VersionEntry::initial(node.id, &unit.content).with_message(format!("Ingest {}", unit.name));

    let node_id = node.id;
    let version_id = version.version_id.clone();

    // 4. Commit atomically
    stores.execute_batch(vec![
        TransactionOp::InsertNode(node),
        TransactionOp::InsertEmbedding {
            id: node_id,
            embedding: embedding.values,
        },
        TransactionOp::RecordVersion(version),
    ]).await?;

    Ok(IngestResult {
        node_id,
        version_id,
        edges_created: 0,
    })
}

/// Ingest multiple code units and automatically detect relationships between them.
///
/// After ingesting all units, this function scans for:
/// - Import relationships (based on module path references in content)
/// - Call relationships (based on function name references in content)
/// - Contains relationships (based on module path hierarchy)
pub async fn ingest_codebase(
    stores: &mut TransactionManager,
    units: &[CodeUnit],
    embedder: &BagOfWordsEmbedder,
) -> OnyxResult<Vec<IngestResult>> {
    let mut results = Vec::new();

    // Phase 1: Ingest all code units
    for unit in units {
        let result = ingest_code_unit(stores, unit, embedder).await?;
        results.push(result);
    }

    // Phase 2: Detect relationships
    let node_ids: Vec<Uuid> = results.iter().map(|r| r.node_id).collect();
    let mut edges_created = 0;

    // Build a lookup of name -> node_id for relationship detection
    let mut name_to_id: std::collections::HashMap<String, Uuid> = std::collections::HashMap::new();
    for &id in &node_ids {
        if let Some(node) = stores.graph_store.get_node(&id).await? {
            name_to_id.insert(node.name.clone(), id);
        }
    }

    // Detect calls and imports by scanning content for references to other entities
    for &id in &node_ids {
        let (content, _name) = {
            let node = stores
                .graph_store
                .get_node(&id)
                .await?
                .ok_or(OnyxError::NodeNotFound(id))?;
            (node.content.clone(), node.name.clone())
        };

        for (ref_name, ref_id) in &name_to_id {
            if *ref_id == id {
                continue; // Skip self-references
            }

            // Check if this node's content references another node by name
            // This is a simple heuristic; production would use AST analysis
            if content.contains(ref_name.as_str()) {
                // Determine if it's a call or import based on context
                let edge_type = if content.contains("use ") || content.contains("mod ") {
                    EdgeType::Imports
                } else {
                    EdgeType::Calls
                };

                let edge = Edge::new(edge_type, id, *ref_id)
                    .with_confidence(0.8) // Heuristic-based, not AST-confirmed
                    .with_metadata("detection", "content_scan");

                stores.execute(TransactionOp::InsertEdge(edge)).await?;
                edges_created += 1;
            }
        }
    }

    // Detect contains relationships based on module path hierarchy
    for i in 0..node_ids.len() {
        for j in 0..node_ids.len() {
            if i == j {
                continue;
            }

            let (path_i, path_j) = {
                let node_i = stores
                    .graph_store
                    .get_node(&node_ids[i])
                    .await?
                    .ok_or(OnyxError::NodeNotFound(node_ids[i]))?;
                let node_j = stores
                    .graph_store
                    .get_node(&node_ids[j])
                    .await?
                    .ok_or(OnyxError::NodeNotFound(node_ids[j]))?;

                let pi = match &node_i.extension {
                    NodeExtension::CodeEntity(ext) => ext.module_path.clone(),
                    _ => Vec::new(),
                };
                let pj = match &node_j.extension {
                    NodeExtension::CodeEntity(ext) => ext.module_path.clone(),
                    _ => Vec::new(),
                };
                (pi, pj)
            };

            // Check if node_i's module path is a prefix of node_j's
            if !path_i.is_empty() && path_j.len() == path_i.len() + 1 && path_j.starts_with(&path_i)
            {
                let edge = Edge::new(EdgeType::Contains, node_ids[i], node_ids[j])
                    .with_confidence(1.0)
                    .with_metadata("detection", "module_hierarchy");

                stores.execute(TransactionOp::InsertEdge(edge)).await?;
                edges_created += 1;
            }
        }
    }

    // Update edge counts in results
    for result in &mut results {
        result.edges_created = edges_created;
    }

    Ok(results)
}

/// A simplified Rust source parser that extracts basic function information.
///
/// ## Limitations
/// This is a regex/heuristic-based parser for the prototype. A production
/// implementation would use `syn` or `tree-sitter` for proper AST parsing.
///
/// ## TODO
/// - Use `syn` crate for proper Rust parsing
/// - Support struct, enum, trait, impl parsing
/// - Extract doc comments as Doc nodes
/// - Extract test functions as Test nodes
pub fn parse_rust_source(source: &str, file_path: &str) -> Vec<CodeUnit> {
    let mut units = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // Detect function definitions
        if line.contains("fn ")
            && (line.starts_with("pub")
                || line.starts_with("fn")
                || line.starts_with("    pub")
                || line.starts_with("    fn"))
        {
            let start_line = i + 1; // 1-indexed

            // Extract function name
            if let Some(fn_name) = extract_fn_name(line) {
                // Find the end of the function (matching braces)
                let end_line = find_block_end(&lines, i);
                let content = lines[i..=end_line].join("\n");

                let is_test = i > 0 && lines[i - 1].trim().contains("#[test]");
                let visibility = if line.contains("pub") {
                    Visibility::Public
                } else {
                    Visibility::Private
                };

                let kind = if is_test {
                    CodeEntityKind::Function // Tests are still functions
                } else {
                    CodeEntityKind::Function
                };

                units.push(CodeUnit {
                    name: fn_name.to_string(),
                    content,
                    kind,
                    language: Language::Rust,
                    file_path: file_path.to_string(),
                    line_range: Some((start_line, end_line + 1)),
                    signature: Some(extract_signature(line)),
                    visibility,
                    module_path: Vec::new(), // Caller can set this
                    commit_id: None,
                    branch: None,
                });

                i = end_line + 1;
                continue;
            }
        }

        i += 1;
    }

    units
}

/// Extract function name from a line like "pub fn my_func(args) -> RetType {"
fn extract_fn_name(line: &str) -> Option<&str> {
    let fn_idx = line.find("fn ")?;
    let after_fn = &line[fn_idx + 3..];
    let name_end = after_fn.find(|c: char| c == '(' || c == '<' || c.is_whitespace())?;
    let name = &after_fn[..name_end];
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Extract the full function signature (up to the opening brace).
fn extract_signature(line: &str) -> String {
    if let Some(brace_idx) = line.find('{') {
        line[..brace_idx].trim().to_string()
    } else {
        line.trim().to_string()
    }
}

/// Find the line where a brace-delimited block ends.
fn find_block_end(lines: &[&str], start: usize) -> usize {
    let mut depth = 0;
    for (i, line) in lines.iter().enumerate().skip(start) {
        for ch in line.chars() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
        }
    }
    lines.len().saturating_sub(1)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::history::HistoryStore;
    use crate::store::vector::VectorStore;

    #[test]
    fn test_parse_rust_source() {
        let source = r#"
pub fn calculate_total(items: &[f64]) -> f64 {
    items.iter().sum()
}

fn helper() -> bool {
    true
}
"#;

        let units = parse_rust_source(source, "src/lib.rs");
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].name, "calculate_total");
        assert_eq!(units[1].name, "helper");
        assert_eq!(units[0].visibility, Visibility::Public);
        assert_eq!(units[1].visibility, Visibility::Private);
    }

    #[test]
    fn test_ingest_code_unit() {
        let embedder = BagOfWordsEmbedder::from_corpus(&["fn pub struct use mod crate"], 20);
        let mut stores = TransactionManager::new();

        let unit = CodeUnit {
            name: "calculate_total".to_string(),
            content: "pub fn calculate_total(items: &[f64]) -> f64 { items.iter().sum() }"
                .to_string(),
            kind: CodeEntityKind::Function,
            language: Language::Rust,
            file_path: "src/billing.rs".to_string(),
            line_range: Some((1, 3)),
            signature: Some("pub fn calculate_total(items: &[f64]) -> f64".to_string()),
            visibility: Visibility::Public,
            module_path: vec!["billing".to_string()],
            commit_id: Some("abc123".to_string()),
            branch: Some("main".to_string()),
        };

        let result = ingest_code_unit(&mut stores, &unit, &embedder).unwrap();
        assert!(!result.version_id.is_empty());

        // Verify node was stored
        let node = stores
            .graph_store
            .get_node(&result.node_id)
            .unwrap()
            .unwrap();
        assert_eq!(node.name, "calculate_total");

        // Verify embedding was stored
        let emb = stores.vector_store.get(&result.node_id).unwrap();
        assert!(emb.is_some());

        // Verify version was stored
        let versions = stores.history_store.list_versions(&result.node_id).unwrap();
        assert_eq!(versions.len(), 1);
    }

    #[test]
    fn test_ingest_codebase_with_relationships() {
        let embedder = BagOfWordsEmbedder::from_corpus(
            &["fn pub calculate_total apply_discount items price"],
            20,
        );
        let mut stores = TransactionManager::new();

        let units = vec![
            CodeUnit {
                name: "calculate_total".to_string(),
                content: "pub fn calculate_total() { apply_discount(); }".to_string(),
                kind: CodeEntityKind::Function,
                language: Language::Rust,
                file_path: "src/billing.rs".to_string(),
                line_range: Some((1, 3)),
                signature: None,
                visibility: Visibility::Public,
                module_path: vec!["billing".to_string()],
                commit_id: None,
                branch: None,
            },
            CodeUnit {
                name: "apply_discount".to_string(),
                content: "pub fn apply_discount() { /* discount logic */ }".to_string(),
                kind: CodeEntityKind::Function,
                language: Language::Rust,
                file_path: "src/billing.rs".to_string(),
                line_range: Some((5, 7)),
                signature: None,
                visibility: Visibility::Public,
                module_path: vec!["billing".to_string()],
                commit_id: None,
                branch: None,
            },
        ];

        let results = ingest_codebase(&mut stores, &units, &embedder).unwrap();
        assert_eq!(results.len(), 2);

        // Should have detected the call relationship
        assert!(stores.graph_store.edge_count() > 0);
    }

    #[test]
    fn test_extract_fn_name() {
        assert_eq!(
            extract_fn_name("pub fn hello(x: i32) -> bool {"),
            Some("hello")
        );
        assert_eq!(extract_fn_name("fn helper() {"), Some("helper"));
        assert_eq!(
            extract_fn_name("pub fn generic<T>(v: T) {"),
            Some("generic")
        );
    }
}
