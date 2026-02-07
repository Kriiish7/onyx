use chrono::{DateTime, Utc};
use std::collections::HashSet;
use uuid::Uuid;

use crate::error::OnyxResult;
use crate::model::edge::EdgeType;
use crate::store::graph::GraphStore;
use crate::store::history::HistoryStore;
use crate::store::transaction::TransactionManager;
use crate::store::vector::VectorStore;

// ---------------------------------------------------------------------------
// Query Engine: multi-hop, cross-store retrieval and reasoning
// ---------------------------------------------------------------------------

/// Options for controlling query behavior.
#[derive(Debug, Clone)]
pub struct QueryOptions {
    /// Maximum traversal depth for graph queries.
    pub max_depth: usize,
    /// Number of vector search results to retrieve.
    pub top_k: usize,
    /// Edge types to follow during graph traversal.
    pub edge_types: Option<Vec<EdgeType>>,
    /// Time range for temporal filtering (None = all time).
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Whether to include version history in results.
    pub include_history: bool,
    /// Minimum confidence score for edges to follow.
    pub min_confidence: f64,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            max_depth: 2,
            top_k: 10,
            edge_types: None,
            time_range: None,
            include_history: false,
            min_confidence: 0.0,
        }
    }
}

/// A single item in a query result.
#[derive(Debug, Clone)]
pub struct QueryResultItem {
    /// The node ID.
    pub node_id: Uuid,
    /// The node name.
    pub name: String,
    /// The node content.
    pub content: String,
    /// How this node was found (vector search, graph traversal, or both).
    pub source: ResultSource,
    /// Relevance score (0.0 to 1.0).
    pub score: f64,
    /// Depth from the query origin (0 = direct match).
    pub depth: usize,
    /// The path of edge types from the origin to this node.
    pub edge_path: Vec<EdgeType>,
    /// Version history entries if requested.
    pub versions: Vec<VersionInfo>,
}

/// How a result was discovered.
#[derive(Debug, Clone, PartialEq)]
pub enum ResultSource {
    VectorSearch,
    GraphTraversal,
    Combined,
}

/// Summary of a version for display in query results.
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version_id: String,
    pub timestamp: DateTime<Utc>,
    pub message: Option<String>,
    pub author: Option<String>,
    pub lines_changed: usize,
}

/// Complete result of a query operation.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// The items in the result, sorted by relevance.
    pub items: Vec<QueryResultItem>,
    /// Total nodes examined during the query.
    pub nodes_examined: usize,
    /// How long the query took.
    pub query_time_ms: u64,
}

// ---------------------------------------------------------------------------
// Query execution
// ---------------------------------------------------------------------------

/// Execute a semantic query against the Onyx stores.
///
/// The query engine follows this strategy:
/// 1. If an embedding is provided, find semantically similar nodes via vector search
/// 2. For each vector result, expand context via graph traversal
/// 3. Apply temporal filtering if a time range is specified
/// 4. Fuse results, deduplicate, and rank by combined relevance
pub async fn execute_query(
    stores: &TransactionManager,
    query_embedding: Option<&[f32]>,
    options: &QueryOptions,
) -> OnyxResult<QueryResult> {
    let start = std::time::Instant::now();
    let mut seen: HashSet<Uuid> = HashSet::new();
    let mut items: Vec<QueryResultItem> = Vec::new();
    let mut nodes_examined: usize = 0;

    // Step 1: Vector similarity search
    if let Some(embedding) = query_embedding {
        let vector_results = stores.vector_store.search(embedding, options.top_k).await?;
        nodes_examined += vector_results.len();

        for (node_id, score) in &vector_results {
            if let Some(node) = stores.graph_store.get_node(node_id).await? {
                seen.insert(*node_id);
                items.push(QueryResultItem {
                    node_id: *node_id,
                    name: node.name.clone(),
                    content: node.content.clone(),
                    source: ResultSource::VectorSearch,
                    score: *score as f64,
                    depth: 0,
                    edge_path: Vec::new(),
                    versions: Vec::new(),
                });
            }
        }
    }

    // Step 2: Graph traversal from each vector result
    let seed_ids: Vec<Uuid> = items.iter().map(|i| i.node_id).collect();
    for seed_id in &seed_ids {
        let traversal = stores
            .graph_store
            .traverse(seed_id, options.edge_types.as_deref(), options.max_depth)
            .await?;

        for (node_id, depth) in &traversal.nodes {
            if depth == &0 {
                continue; // Skip the seed node itself
            }
            nodes_examined += 1;

            if !seen.contains(node_id) {
                seen.insert(*node_id);
                if let Some(node) = stores.graph_store.get_node(node_id).await? {
                    // Score decays with depth
                    let depth_penalty = 1.0 / (1.0 + *depth as f64);
                    items.push(QueryResultItem {
                        node_id: *node_id,
                        name: node.name.clone(),
                        content: node.content.clone(),
                        source: ResultSource::GraphTraversal,
                        score: depth_penalty,
                        depth: *depth,
                        edge_path: Vec::new(), // TODO: track actual edge path
                        versions: Vec::new(),
                    });
                }
            } else {
                // Node found by both vector search and graph traversal
                if let Some(item) = items.iter_mut().find(|i| i.node_id == *node_id) {
                    item.source = ResultSource::Combined;
                    item.score = (item.score + 0.2).min(1.0); // Boost for multi-source
                }
            }
        }
    }

    // Step 3: Add version history if requested
    if options.include_history {
        for item in &mut items {
            let versions = stores.history_store.list_versions(&item.node_id).await?;
            for v in versions {
                item.versions.push(VersionInfo {
                    version_id: v.version_id.clone(),
                    timestamp: v.timestamp,
                    message: v.message.clone(),
                    author: v.author.clone(),
                    lines_changed: v.diff.lines_changed(),
                });
            }
        }
    }

    // Step 4: Sort by score (descending)
    items.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let elapsed = start.elapsed().as_millis() as u64;

    Ok(QueryResult {
        items,
        nodes_examined,
        query_time_ms: elapsed,
    })
}

// ---------------------------------------------------------------------------
// Impact analysis: reason over the graph to find affected nodes
// ---------------------------------------------------------------------------

/// Given a node, find all downstream nodes that would be affected by a change.
/// Follows `Calls`, `Imports`, `DependsOn`, and `Documents` edges.
pub async fn impact_analysis(
    stores: &TransactionManager,
    node_id: &Uuid,
    max_depth: usize,
) -> OnyxResult<Vec<(Uuid, String, usize)>> {
    let impact_edges = vec![
        EdgeType::Calls,
        EdgeType::Imports,
        EdgeType::DependsOn,
        EdgeType::Documents,
        EdgeType::TestsOf,
    ];

    // Get inbound edges -- nodes that DEPEND ON the changed node
    let mut affected: Vec<(Uuid, String, usize)> = Vec::new();
    let mut visited: HashSet<Uuid> = HashSet::new();
    visited.insert(*node_id);

    let mut frontier: Vec<(Uuid, usize)> = vec![(*node_id, 0)];

    while let Some((current, depth)) = frontier.pop() {
        if depth > 0 {
            if let Some(node) = stores.graph_store.get_node(&current).await? {
                affected.push((current, node.name.clone(), depth));
            }
        }

        if depth >= max_depth {
            continue;
        }

        // Find nodes that reference the current node
        let inbound = stores
            .graph_store
            .get_inbound(&current, Some(&impact_edges))
            .await?;

        for (_edge, node) in inbound {
            if !visited.contains(&node.id) {
                visited.insert(node.id);
                frontier.push((node.id, depth + 1));
            }
        }
    }

    Ok(affected)
}

/// Given a node, find all tests that cover it (directly or transitively).
pub async fn find_covering_tests(
    stores: &TransactionManager,
    node_id: &Uuid,
    max_depth: usize,
) -> OnyxResult<Vec<QueryResultItem>> {
    let mut tests: Vec<QueryResultItem> = Vec::new();
    let mut visited: HashSet<Uuid> = HashSet::new();

    // Direct tests
    let direct = stores
        .graph_store
        .get_inbound(node_id, Some(&[EdgeType::TestsOf]))
        .await?;

    for (_, test_node) in &direct {
        if !visited.contains(&test_node.id) {
            visited.insert(test_node.id);
            tests.push(QueryResultItem {
                node_id: test_node.id,
                name: test_node.name.clone(),
                content: test_node.content.clone(),
                source: ResultSource::GraphTraversal,
                score: 1.0,
                depth: 1,
                edge_path: vec![EdgeType::TestsOf],
                versions: Vec::new(),
            });
        }
    }

    // Transitive: tests of callers
    if max_depth > 1 {
        let callers = stores
            .graph_store
            .get_inbound(node_id, Some(&[EdgeType::Calls]))
            .await?;

        for (_, caller_node) in &callers {
            let caller_tests = stores
                .graph_store
                .get_inbound(&caller_node.id, Some(&[EdgeType::TestsOf]))
                .await?;

            for (_, test_node) in &caller_tests {
                if !visited.contains(&test_node.id) {
                    visited.insert(test_node.id);
                    tests.push(QueryResultItem {
                        node_id: test_node.id,
                        name: test_node.name.clone(),
                        content: test_node.content.clone(),
                        source: ResultSource::GraphTraversal,
                        score: 0.7,
                        depth: 2,
                        edge_path: vec![EdgeType::Calls, EdgeType::TestsOf],
                        versions: Vec::new(),
                    });
                }
            }
        }
    }

    Ok(tests)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::edge::Edge;
    use crate::model::node::{CodeEntityKind, Node, NodeType};
    use crate::store::transaction::TransactionOp;

    fn build_test_stores() -> TransactionManager {
        let mut tm = TransactionManager::new();

        // Create a small graph: func_a -> func_b -> func_c
        // test_b tests func_b
        let func_a = Node::new(
            NodeType::CodeEntity(CodeEntityKind::Function),
            "func_a",
            "fn func_a() { func_b(); }",
        );
        let func_b = Node::new(
            NodeType::CodeEntity(CodeEntityKind::Function),
            "func_b",
            "fn func_b() { func_c(); }",
        );
        let func_c = Node::new(
            NodeType::CodeEntity(CodeEntityKind::Function),
            "func_c",
            "fn func_c() -> i32 { 42 }",
        );
        let test_b = Node::new(
            NodeType::Test,
            "test_func_b",
            "#[test] fn test_func_b() { assert!(func_b()); }",
        );

        let id_a = func_a.id;
        let id_b = func_b.id;
        let id_c = func_c.id;
        let id_test = test_b.id;

        tm.execute_batch(vec![
            TransactionOp::InsertNode(func_a),
            TransactionOp::InsertNode(func_b),
            TransactionOp::InsertNode(func_c),
            TransactionOp::InsertNode(test_b),
        ])
        .unwrap();

        // Edges
        tm.execute(TransactionOp::InsertEdge(Edge::new(
            EdgeType::Calls,
            id_a,
            id_b,
        )))
        .unwrap();
        tm.execute(TransactionOp::InsertEdge(Edge::new(
            EdgeType::Calls,
            id_b,
            id_c,
        )))
        .unwrap();
        tm.execute(TransactionOp::InsertEdge(Edge::new(
            EdgeType::TestsOf,
            id_test,
            id_b,
        )))
        .unwrap();

        // Embeddings
        tm.execute(TransactionOp::InsertEmbedding {
            id: id_a,
            embedding: vec![1.0, 0.0, 0.0],
        })
        .unwrap();
        tm.execute(TransactionOp::InsertEmbedding {
            id: id_b,
            embedding: vec![0.8, 0.2, 0.0],
        })
        .unwrap();
        tm.execute(TransactionOp::InsertEmbedding {
            id: id_c,
            embedding: vec![0.0, 0.0, 1.0],
        })
        .unwrap();

        tm
    }

    #[test]
    fn test_vector_search_query() {
        let stores = build_test_stores();
        let options = QueryOptions {
            top_k: 2,
            max_depth: 0,
            ..Default::default()
        };

        let result = execute_query(&stores, Some(&[1.0, 0.0, 0.0]), &options).unwrap();
        assert!(!result.items.is_empty());
        assert_eq!(result.items[0].name, "func_a"); // Most similar to [1,0,0]
    }

    #[test]
    fn test_graph_expanded_query() {
        let stores = build_test_stores();
        let options = QueryOptions {
            top_k: 1,
            max_depth: 2,
            edge_types: Some(vec![EdgeType::Calls]),
            ..Default::default()
        };

        let result = execute_query(&stores, Some(&[1.0, 0.0, 0.0]), &options).unwrap();
        // Should find func_a via vector search, then func_b and func_c via graph traversal
        assert!(result.items.len() >= 2);
    }

    #[test]
    fn test_impact_analysis() {
        let stores = build_test_stores();

        // Find what's affected if func_c changes
        // func_b calls func_c, func_a calls func_b -> both affected
        let func_c_id = stores
            .graph_store
            .nodes_by_type(&NodeType::CodeEntity(CodeEntityKind::Function))
            .iter()
            .find(|n| n.name == "func_c")
            .unwrap()
            .id;

        let affected = impact_analysis(&stores, &func_c_id, 3).unwrap();
        assert!(!affected.is_empty());

        let names: Vec<&str> = affected.iter().map(|(_, n, _)| n.as_str()).collect();
        assert!(names.contains(&"func_b"));
    }

    #[test]
    fn test_find_covering_tests() {
        let stores = build_test_stores();

        let func_b_id = stores
            .graph_store
            .nodes_by_type(&NodeType::CodeEntity(CodeEntityKind::Function))
            .iter()
            .find(|n| n.name == "func_b")
            .unwrap()
            .id;

        let tests = find_covering_tests(&stores, &func_b_id, 2).unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "test_func_b");
    }
}
