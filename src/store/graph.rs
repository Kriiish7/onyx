use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::db::OnyxDatabase;
use crate::error::{OnyxError, OnyxResult};
use crate::model::edge::{Edge, EdgeType};
use crate::model::node::Node;

// ---------------------------------------------------------------------------
// GraphStore trait: interface for structural relationship storage & traversal
// ---------------------------------------------------------------------------

/// Trait for graph storage backends that maintain relationships between nodes.
#[async_trait]
pub trait GraphStore: Send + Sync {
    /// Add a node to the graph.
    async fn add_node(&self, node: Node) -> OnyxResult<()>;

    /// Get a node by ID.
    async fn get_node(&self, id: &Uuid) -> OnyxResult<Option<Node>>;

    /// Update a node.
    async fn update_node(&self, node: Node) -> OnyxResult<()>;

    /// Remove a node and all its edges.
    async fn remove_node(&self, id: &Uuid) -> OnyxResult<()>;

    /// Add an edge to the graph.
    async fn add_edge(&self, edge: Edge) -> OnyxResult<()>;

    /// Get an edge by ID.
    async fn get_edge(&self, id: &Uuid) -> OnyxResult<Option<Edge>>;

    /// Remove an edge by ID.
    async fn remove_edge(&self, id: &Uuid) -> OnyxResult<()>;

    /// Get outbound neighbors of a node, optionally filtered by edge types.
    async fn get_neighbors(
        &self,
        id: &Uuid,
        edge_types: Option<&[EdgeType]>,
    ) -> OnyxResult<Vec<(Edge, Node)>>;

    /// Get inbound edges pointing to a node, optionally filtered by edge types.
    async fn get_inbound(
        &self,
        id: &Uuid,
        edge_types: Option<&[EdgeType]>,
    ) -> OnyxResult<Vec<(Edge, Node)>>;

    /// Get all node IDs in the graph.
    async fn get_all_node_ids(&self) -> OnyxResult<Vec<Uuid>>;

    /// Get all edge IDs in the graph.
    async fn get_all_edge_ids(&self) -> OnyxResult<Vec<Uuid>>;

    /// Insert a node (alias for add_node).
    async fn insert_node(&self, node: Node) -> OnyxResult<()> {
        self.add_node(node).await
    }

    /// Insert an edge (alias for add_edge).
    async fn insert_edge(&self, edge: Edge) -> OnyxResult<()> {
        self.add_edge(edge).await
    }

    /// Multi-hop traversal: get all nodes reachable from a start node within
    /// a given depth, following specified edge types.
    async fn traverse(
        &self,
        start_id: &Uuid,
        edge_types: Option<&[EdgeType]>,
        max_depth: usize,
    ) -> OnyxResult<TraversalResult>;

    /// Find all paths between two nodes up to a maximum depth.
    async fn find_paths(
        &self,
        from: &Uuid,
        to: &Uuid,
        max_depth: usize,
    ) -> OnyxResult<Vec<Vec<Uuid>>>;

    /// Get a subgraph rooted at a node to a given depth.
    async fn subgraph(&self, root_id: &Uuid, depth: usize) -> OnyxResult<SubgraphResult>;

    /// Get all nodes of a specific type.
    async fn nodes_by_type(&self, node_type: &crate::model::node::NodeType) -> Vec<Node>;

    /// Get all edges of a specific type.
    async fn edges_by_type(&self, edge_type: &EdgeType) -> Vec<Edge>;

    /// Get edges that are valid at a specific timestamp.
    async fn edges_at_time(&self, id: &Uuid, timestamp: &DateTime<Utc>) -> OnyxResult<Vec<Edge>>;

    /// Total number of nodes in the graph.
    async fn node_count(&self) -> usize;

    /// Total number of edges in the graph.
    async fn edge_count(&self) -> usize;

    /// Get all nodes in the graph.
    async fn all_nodes(&self) -> Vec<Node>;
}

// ---------------------------------------------------------------------------
// Traversal and subgraph result types
// ---------------------------------------------------------------------------

/// Result of a multi-hop graph traversal.
#[derive(Debug, Clone)]
pub struct TraversalResult {
    /// Nodes discovered during traversal, with their depth from the start node.
    pub nodes: Vec<(Uuid, usize)>,
    /// Edges traversed during the search.
    pub edges: Vec<Uuid>,
    /// Total nodes visited.
    pub total_visited: usize,
}

/// Result of extracting a subgraph.
#[derive(Debug, Clone)]
pub struct SubgraphResult {
    /// Node IDs in the subgraph.
    pub node_ids: HashSet<Uuid>,
    /// Edge IDs in the subgraph.
    pub edge_ids: HashSet<Uuid>,
}

// ---------------------------------------------------------------------------
// SurrealDB Graph Store
// ---------------------------------------------------------------------------

/// SurrealDB-backed graph store using native graph capabilities.
#[derive(Clone)]
pub struct SurrealGraphStore {
    db: Arc<OnyxDatabase>,
}

impl SurrealGraphStore {
    /// Create a new SurrealDB graph store.
    pub fn new(db: Arc<OnyxDatabase>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl GraphStore for SurrealGraphStore {
    async fn add_node(&self, node: Node) -> OnyxResult<()> {
        let id = node.id.to_string();
        self.db
            .create_with_id("node", &id, node)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to create node: {}", e)))?;
        Ok(())
    }

    async fn get_node(&self, id: &Uuid) -> OnyxResult<Option<Node>> {
        let node: Option<Node> = self
            .db
            .select("node", id.to_string())
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to get node: {}", e)))?;
        Ok(node)
    }

    async fn update_node(&self, node: Node) -> OnyxResult<()> {
        let id = node.id.to_string();
        self.db
            .update("node", &id, node)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to update node: {}", e)))?;
        Ok(())
    }

    async fn remove_node(&self, id: &Uuid) -> OnyxResult<()> {
        let id_str = id.to_string();
        let query = format!(
            "DELETE edge WHERE source_id = '{}' OR target_id = '{}'",
            id_str, id_str
        );

        // Remove all edges where this node is source or target
        let _ = self.db.query(&query).await;

        self.db
            .delete("node", &id_str)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to delete node: {}", e)))?;
        Ok(())
    }

    async fn add_edge(&self, edge: Edge) -> OnyxResult<()> {
        let id = edge.id.to_string();
        self.db
            .create_with_id("edge", &id, edge)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to create edge: {}", e)))?;
        Ok(())
    }

    async fn get_edge(&self, id: &Uuid) -> OnyxResult<Option<Edge>> {
        let edge: Option<Edge> = self
            .db
            .select("edge", id.to_string())
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to get edge: {}", e)))?;
        Ok(edge)
    }

    async fn remove_edge(&self, id: &Uuid) -> OnyxResult<()> {
        self.db
            .delete("edge", &id.to_string())
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to delete edge: {}", e)))?;
        Ok(())
    }

    async fn get_neighbors(
        &self,
        id: &Uuid,
        edge_types: Option<&[EdgeType]>,
    ) -> OnyxResult<Vec<(Edge, Node)>> {
        let id_str = id.to_string();

        // Build query based on edge type filter
        let query = if let Some(types) = edge_types {
            let type_list: Vec<String> = types
                .iter()
                .map(|t| {
                    format!(
                        "'{}'",
                        serde_json::to_string(t)
                            .unwrap_or_default()
                            .trim_matches('"')
                    )
                })
                .collect();
            format!(
                "SELECT * FROM edge WHERE source_id = '{}' AND edge_type IN [{}]",
                id_str,
                type_list.join(", ")
            )
        } else {
            format!("SELECT * FROM edge WHERE source_id = '{}'", id_str)
        };

        let mut response = self
            .db
            .query(&query)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to get neighbors: {}", e)))?;

        let edges: Vec<Edge> = response
            .take(0)
            .map_err(|e| OnyxError::Internal(format!("Failed to parse edge results: {}", e)))?;

        let mut results = Vec::new();
        for edge in edges {
            if let Some(node) = self.get_node(&edge.target_id).await? {
                results.push((edge, node));
            }
        }

        Ok(results)
    }

    async fn get_inbound(
        &self,
        id: &Uuid,
        edge_types: Option<&[EdgeType]>,
    ) -> OnyxResult<Vec<(Edge, Node)>> {
        let id_str = id.to_string();

        let query = if let Some(types) = edge_types {
            let type_list: Vec<String> = types
                .iter()
                .map(|t| {
                    format!(
                        "'{}'",
                        serde_json::to_string(t)
                            .unwrap_or_default()
                            .trim_matches('"')
                    )
                })
                .collect();
            format!(
                "SELECT * FROM edge WHERE target_id = '{}' AND edge_type IN [{}]",
                id_str,
                type_list.join(", ")
            )
        } else {
            format!("SELECT * FROM edge WHERE target_id = '{}'", id_str)
        };

        let mut response = self
            .db
            .query(&query)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to get inbound: {}", e)))?;

        let edges: Vec<Edge> = response
            .take(0)
            .map_err(|e| OnyxError::Internal(format!("Failed to parse edge results: {}", e)))?;

        let mut results = Vec::new();
        for edge in edges {
            if let Some(node) = self.get_node(&edge.source_id).await? {
                results.push((edge, node));
            }
        }

        Ok(results)
    }

    async fn traverse(
        &self,
        start_id: &Uuid,
        edge_types: Option<&[EdgeType]>,
        max_depth: usize,
    ) -> OnyxResult<TraversalResult> {
        // Verify start node exists
        if self.get_node(start_id).await?.is_none() {
            return Err(OnyxError::NodeNotFound(*start_id));
        }

        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut result_nodes: Vec<(Uuid, usize)> = Vec::new();
        let mut result_edges: Vec<Uuid> = Vec::new();
        let mut queue: VecDeque<(Uuid, usize)> = VecDeque::new();

        queue.push_back((*start_id, 0));
        visited.insert(*start_id);

        while let Some((current_id, depth)) = queue.pop_front() {
            result_nodes.push((current_id, depth));

            if depth >= max_depth {
                continue;
            }

            let neighbors = self.get_neighbors(&current_id, edge_types).await?;
            for (edge, node) in neighbors {
                result_edges.push(edge.id);

                if !visited.contains(&node.id) {
                    visited.insert(node.id);
                    queue.push_back((node.id, depth + 1));
                }
            }
        }

        Ok(TraversalResult {
            total_visited: visited.len(),
            nodes: result_nodes,
            edges: result_edges,
        })
    }

    async fn find_paths(
        &self,
        from: &Uuid,
        to: &Uuid,
        max_depth: usize,
    ) -> OnyxResult<Vec<Vec<Uuid>>> {
        // Verify nodes exist
        if self.get_node(from).await?.is_none() {
            return Err(OnyxError::NodeNotFound(*from));
        }
        if self.get_node(to).await?.is_none() {
            return Err(OnyxError::NodeNotFound(*to));
        }

        let mut paths: Vec<Vec<Uuid>> = Vec::new();
        let mut current_path: Vec<Uuid> = vec![*from];
        let mut visited: HashSet<Uuid> = HashSet::new();
        visited.insert(*from);

        self.dfs_paths(
            from,
            to,
            max_depth,
            &mut current_path,
            &mut visited,
            &mut paths,
        )
        .await;

        Ok(paths)
    }

    async fn subgraph(&self, root_id: &Uuid, depth: usize) -> OnyxResult<SubgraphResult> {
        let traversal = self.traverse(root_id, None, depth).await?;

        let node_ids: HashSet<Uuid> = traversal.nodes.iter().map(|(id, _)| *id).collect();
        let edge_ids: HashSet<Uuid> = traversal.edges.iter().copied().collect();

        Ok(SubgraphResult { node_ids, edge_ids })
    }

    async fn nodes_by_type(&self, node_type: &crate::model::node::NodeType) -> Vec<Node> {
        let query = format!(
            "SELECT * FROM node WHERE node_type = {}",
            serde_json::to_string(node_type).unwrap_or_default()
        );

        match self.db.query(&query).await {
            Ok(mut response) => response.take(0).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    async fn edges_by_type(&self, edge_type: &EdgeType) -> Vec<Edge> {
        let query = format!(
            "SELECT * FROM edge WHERE edge_type = {}",
            serde_json::to_string(edge_type).unwrap_or_default()
        );

        match self.db.query(&query).await {
            Ok(mut response) => response.take(0).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    async fn edges_at_time(&self, id: &Uuid, timestamp: &DateTime<Utc>) -> OnyxResult<Vec<Edge>> {
        // Get all edges connected to this node
        let id_str = id.to_string();
        let query = format!(
            "SELECT * FROM edge WHERE source_id = '{}' OR target_id = '{}'",
            id_str, id_str
        );

        let mut response = self
            .db
            .query(&query)
            .await
            .map_err(|e| OnyxError::Internal(format!("Failed to get edges: {}", e)))?;

        let edges: Vec<Edge> = response
            .take(0)
            .map_err(|e| OnyxError::Internal(format!("Failed to parse edges: {}", e)))?;

        // Filter by temporal validity
        let filtered: Vec<Edge> = edges
            .into_iter()
            .filter(|edge| edge.temporal.is_valid_at(timestamp))
            .collect();

        Ok(filtered)
    }

    async fn node_count(&self) -> usize {
        match self
            .db
            .query("SELECT count() FROM node GROUP BY count")
            .await
        {
            Ok(mut response) => {
                let count: Option<i64> = response.take(0).ok().flatten();
                count.unwrap_or(0) as usize
            }
            Err(_) => 0,
        }
    }

    async fn edge_count(&self) -> usize {
        match self
            .db
            .query("SELECT count() FROM edge GROUP BY count")
            .await
        {
            Ok(mut response) => {
                let count: Option<i64> = response.take(0).ok().flatten();
                count.unwrap_or(0) as usize
            }
            Err(_) => 0,
        }
    }

    async fn all_nodes(&self) -> Vec<Node> {
        match self.db.query("SELECT * FROM node").await {
            Ok(mut response) => response.take(0).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    async fn get_all_node_ids(&self) -> OnyxResult<Vec<Uuid>> {
        let query = "SELECT record_id FROM node";
        let mut response = self.db.query(query).await
            .map_err(|e| OnyxError::Internal(format!("Failed to query node IDs: {}", e)))?;
        
        let records: Vec<serde_json::Value> = response.take(0).unwrap_or_default();
        let mut ids = Vec::new();
        
        for record in records {
            if let Some(id_str) = record.get("record_id").and_then(|v| v.as_str()) {
                if let Ok(id) = Uuid::parse_str(id_str) {
                    ids.push(id);
                }
            }
        }
        
        Ok(ids)
    }

    async fn get_all_edge_ids(&self) -> OnyxResult<Vec<Uuid>> {
        let query = "SELECT record_id FROM edge";
        let mut response = self.db.query(query).await
            .map_err(|e| OnyxError::Internal(format!("Failed to query edge IDs: {}", e)))?;
        
        let records: Vec<serde_json::Value> = response.take(0).unwrap_or_default();
        let mut ids = Vec::new();
        
        for record in records {
            if let Some(id_str) = record.get("record_id").and_then(|v| v.as_str()) {
                if let Ok(id) = Uuid::parse_str(id_str) {
                    ids.push(id);
                }
            }
        }
        
        Ok(ids)
    }
}

type DfsPathsFn = fn(
    &SurrealGraphStore,
    &Uuid,
    &Uuid,
    usize,
    &mut Vec<Uuid>,
    &mut HashSet<Uuid>,
    &mut Vec<Vec<Uuid>>,
);

impl SurrealGraphStore {
    /// DFS helper for finding all paths between two nodes.
    async fn dfs_paths(
        &self,
        current: &Uuid,
        target: &Uuid,
        remaining_depth: usize,
        path: &mut Vec<Uuid>,
        visited: &mut HashSet<Uuid>,
        results: &mut Vec<Vec<Uuid>>,
    ) {
        if current == target {
            results.push(path.clone());
            return;
        }

        if remaining_depth == 0 {
            return;
        }

        if let Ok(neighbors) = self.get_neighbors(current, None).await {
            for (edge, node) in neighbors {
                if !visited.contains(&node.id) {
                    visited.insert(node.id);
                    path.push(node.id);
                    self.dfs_paths(
                        &node.id,
                        target,
                        remaining_depth - 1,
                        path,
                        visited,
                        results,
                    )
                    .await;
                    path.pop();
                    visited.remove(&node.id);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// InMemoryGraphStore: for testing and fallback
// ---------------------------------------------------------------------------

/// In-memory graph store using adjacency lists for fast traversal.
pub struct InMemoryGraphStore {
    nodes: RwLock<HashMap<Uuid, Node>>,
    edges: RwLock<HashMap<Uuid, Edge>>,
    outbound: RwLock<HashMap<Uuid, Vec<Uuid>>>,
    inbound: RwLock<HashMap<Uuid, Vec<Uuid>>>,
}

impl InMemoryGraphStore {
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            edges: RwLock::new(HashMap::new()),
            outbound: RwLock::new(HashMap::new()),
            inbound: RwLock::new(HashMap::new()),
        }
    }

    pub async fn all_nodes(&self) -> Vec<Node> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }
}

impl Default for InMemoryGraphStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GraphStore for InMemoryGraphStore {
    async fn add_node(&self, node: Node) -> OnyxResult<()> {
        let mut nodes = self.nodes.write().await;

        let id = node.id;
        if nodes.contains_key(&id) {
            return Err(OnyxError::DuplicateNode(id));
        }
        nodes.insert(id, node);

        let mut outbound = self.outbound.write().await;
        let mut inbound = self.inbound.write().await;
        outbound.entry(id).or_default();
        inbound.entry(id).or_default();

        Ok(())
    }

    async fn get_node(&self, id: &Uuid) -> OnyxResult<Option<Node>> {
        let nodes = self.nodes.read().await;
        Ok(nodes.get(id).cloned())
    }

    async fn update_node(&self, node: Node) -> OnyxResult<()> {
        let mut nodes = self.nodes.write().await;
        nodes.insert(node.id, node);
        Ok(())
    }

    async fn remove_node(&self, id: &Uuid) -> OnyxResult<()> {
        let outbound_edges: Vec<Uuid> = {
            let outbound = self.outbound.read().await;
            outbound.get(id).cloned().unwrap_or_default()
        };

        let inbound_edges: Vec<Uuid> = {
            let inbound = self.inbound.read().await;
            inbound.get(id).cloned().unwrap_or_default()
        };

        let mut edges = self.edges.write().await;
        let mut outbound = self.outbound.write().await;
        let mut inbound = self.inbound.write().await;
        let mut nodes = self.nodes.write().await;

        for edge_id in outbound_edges.iter().chain(inbound_edges.iter()) {
            if let Some(edge) = edges.remove(edge_id) {
                if &edge.source_id == id {
                    if let Some(list) = inbound.get_mut(&edge.target_id) {
                        list.retain(|e| e != edge_id);
                    }
                } else {
                    if let Some(list) = outbound.get_mut(&edge.source_id) {
                        list.retain(|e| e != edge_id);
                    }
                }
            }
        }

        outbound.remove(id);
        inbound.remove(id);
        nodes.remove(id);
        Ok(())
    }

    async fn add_edge(&self, edge: Edge) -> OnyxResult<()> {
        let nodes = self.nodes.read().await;
        if !nodes.contains_key(&edge.source_id) {
            return Err(OnyxError::NodeNotFound(edge.source_id));
        }
        if !nodes.contains_key(&edge.target_id) {
            return Err(OnyxError::NodeNotFound(edge.target_id));
        }
        drop(nodes);

        let edge_id = edge.id;
        let source_id = edge.source_id;
        let target_id = edge.target_id;

        let mut edges = self.edges.write().await;
        let mut outbound = self.outbound.write().await;
        let mut inbound = self.inbound.write().await;

        edges.insert(edge_id, edge);
        outbound.entry(source_id).or_default().push(edge_id);
        inbound.entry(target_id).or_default().push(edge_id);

        Ok(())
    }

    async fn get_edge(&self, id: &Uuid) -> OnyxResult<Option<Edge>> {
        let edges = self.edges.read().await;
        Ok(edges.get(id).cloned())
    }

    async fn remove_edge(&self, id: &Uuid) -> OnyxResult<()> {
        let mut edges = self.edges.write().await;
        let mut outbound = self.outbound.write().await;
        let mut inbound = self.inbound.write().await;

        if let Some(edge) = edges.remove(id) {
            if let Some(list) = outbound.get_mut(&edge.source_id) {
                list.retain(|e| e != id);
            }
            if let Some(list) = inbound.get_mut(&edge.target_id) {
                list.retain(|e| e != id);
            }
        }
        Ok(())
    }

    async fn get_neighbors(
        &self,
        id: &Uuid,
        edge_types: Option<&[EdgeType]>,
    ) -> OnyxResult<Vec<(Edge, Node)>> {
        let edge_ids = {
            let outbound = self.outbound.read().await;
            outbound.get(id).cloned().unwrap_or_default()
        };

        let edges = self.edges.read().await;
        let nodes = self.nodes.read().await;
        let mut results = Vec::new();

        for edge_id in &edge_ids {
            if let Some(edge) = edges.get(edge_id) {
                if let Some(types) = edge_types {
                    if !types.contains(&edge.edge_type) {
                        continue;
                    }
                }
                if let Some(node) = nodes.get(&edge.target_id) {
                    results.push((edge.clone(), node.clone()));
                }
            }
        }

        Ok(results)
    }

    async fn get_inbound(
        &self,
        id: &Uuid,
        edge_types: Option<&[EdgeType]>,
    ) -> OnyxResult<Vec<(Edge, Node)>> {
        let edge_ids = {
            let inbound = self.inbound.read().await;
            inbound.get(id).cloned().unwrap_or_default()
        };

        let edges = self.edges.read().await;
        let nodes = self.nodes.read().await;
        let mut results = Vec::new();

        for edge_id in &edge_ids {
            if let Some(edge) = edges.get(edge_id) {
                if let Some(types) = edge_types {
                    if !types.contains(&edge.edge_type) {
                        continue;
                    }
                }
                if let Some(node) = nodes.get(&edge.source_id) {
                    results.push((edge.clone(), node.clone()));
                }
            }
        }

        Ok(results)
    }

    async fn traverse(
        &self,
        start_id: &Uuid,
        edge_types: Option<&[EdgeType]>,
        max_depth: usize,
    ) -> OnyxResult<TraversalResult> {
        let nodes = self.nodes.read().await;
        if !nodes.contains_key(start_id) {
            return Err(OnyxError::NodeNotFound(*start_id));
        }
        drop(nodes);

        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut result_nodes: Vec<(Uuid, usize)> = Vec::new();
        let mut result_edges: Vec<Uuid> = Vec::new();
        let mut queue: VecDeque<(Uuid, usize)> = VecDeque::new();

        queue.push_back((*start_id, 0));
        visited.insert(*start_id);

        let edges = self.edges.read().await;
        let outbound = self.outbound.read().await;

        while let Some((current_id, depth)) = queue.pop_front() {
            result_nodes.push((current_id, depth));

            if depth >= max_depth {
                continue;
            }

            let edge_ids = outbound.get(&current_id).cloned().unwrap_or_default();
            for edge_id in &edge_ids {
                if let Some(edge) = edges.get(edge_id) {
                    if let Some(types) = edge_types {
                        if !types.contains(&edge.edge_type) {
                            continue;
                        }
                    }

                    result_edges.push(*edge_id);

                    if !visited.contains(&edge.target_id) {
                        visited.insert(edge.target_id);
                        queue.push_back((edge.target_id, depth + 1));
                    }
                }
            }
        }

        Ok(TraversalResult {
            total_visited: visited.len(),
            nodes: result_nodes,
            edges: result_edges,
        })
    }

    async fn find_paths(
        &self,
        from: &Uuid,
        to: &Uuid,
        max_depth: usize,
    ) -> OnyxResult<Vec<Vec<Uuid>>> {
        let nodes = self.nodes.read().await;
        if !nodes.contains_key(from) {
            return Err(OnyxError::NodeNotFound(*from));
        }
        if !nodes.contains_key(to) {
            return Err(OnyxError::NodeNotFound(*to));
        }
        drop(nodes);

        let mut paths: Vec<Vec<Uuid>> = Vec::new();
        let mut current_path: Vec<Uuid> = vec![*from];
        let mut visited: HashSet<Uuid> = HashSet::new();
        visited.insert(*from);

        self.dfs_paths_sync(
            from,
            to,
            max_depth,
            &mut current_path,
            &mut visited,
            &mut paths,
        );

        Ok(paths)
    }

    async fn subgraph(&self, root_id: &Uuid, depth: usize) -> OnyxResult<SubgraphResult> {
        let traversal = self.traverse(root_id, None, depth).await?;

        let node_ids: HashSet<Uuid> = traversal.nodes.iter().map(|(id, _)| *id).collect();
        let edge_ids: HashSet<Uuid> = traversal.edges.iter().copied().collect();

        Ok(SubgraphResult { node_ids, edge_ids })
    }

    async fn nodes_by_type(&self, node_type: &crate::model::node::NodeType) -> Vec<Node> {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .filter(|n| &n.node_type == node_type)
            .cloned()
            .collect()
    }

    async fn edges_by_type(&self, edge_type: &EdgeType) -> Vec<Edge> {
        let edges = self.edges.read().await;
        edges
            .values()
            .filter(|e| &e.edge_type == edge_type)
            .cloned()
            .collect()
    }

    async fn edges_at_time(&self, id: &Uuid, timestamp: &DateTime<Utc>) -> OnyxResult<Vec<Edge>> {
        let outbound = self.outbound.read().await;
        let inbound = self.inbound.read().await;
        let edges = self.edges.read().await;

        let outbound_ids = outbound.get(id).cloned().unwrap_or_default();
        let inbound_ids = inbound.get(id).cloned().unwrap_or_default();

        let mut results = Vec::new();
        for edge_id in outbound_ids.iter().chain(inbound_ids.iter()) {
            if let Some(edge) = edges.get(edge_id) {
                if edge.temporal.is_valid_at(timestamp) {
                    results.push(edge.clone());
                }
            }
        }

        Ok(results)
    }

    async fn node_count(&self) -> usize {
        let nodes = self.nodes.read().await;
        nodes.len()
    }

    async fn edge_count(&self) -> usize {
        let edges = self.edges.read().await;
        edges.len()
    }

    async fn all_nodes(&self) -> Vec<Node> {
        self.all_nodes().await
    }
}

impl InMemoryGraphStore {
    fn dfs_paths_sync(
        &self,
        current: &Uuid,
        target: &Uuid,
        remaining_depth: usize,
        path: &mut Vec<Uuid>,
        visited: &mut HashSet<Uuid>,
        results: &mut Vec<Vec<Uuid>>,
    ) {
        if current == target {
            results.push(path.clone());
            return;
        }

        if remaining_depth == 0 {
            return;
        }

        let outbound = match self.outbound.try_read() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let edges = match self.edges.try_read() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        let edge_ids = outbound.get(current).cloned().unwrap_or_default();
        for edge_id in &edge_ids {
            if let Some(edge) = edges.get(edge_id) {
                if !visited.contains(&edge.target_id) {
                    visited.insert(edge.target_id);
                    path.push(edge.target_id);
                    self.dfs_paths_sync(
                        &edge.target_id,
                        target,
                        remaining_depth - 1,
                        path,
                        visited,
                        results,
                    );
                    path.pop();
                    visited.remove(&edge.target_id);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::node::{CodeEntityKind, NodeType};

    async fn make_graph() -> (InMemoryGraphStore, Uuid, Uuid, Uuid) {
        let g = InMemoryGraphStore::new();

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
            "fn func_c() {}",
        );

        let id_a = func_a.id;
        let id_b = func_b.id;
        let id_c = func_c.id;

        g.add_node(func_a).await.unwrap();
        g.add_node(func_b).await.unwrap();
        g.add_node(func_c).await.unwrap();

        g.add_edge(Edge::new(EdgeType::Calls, id_a, id_b))
            .await
            .unwrap();
        g.add_edge(Edge::new(EdgeType::Calls, id_b, id_c))
            .await
            .unwrap();

        (g, id_a, id_b, id_c)
    }

    #[tokio::test]
    async fn test_add_and_get_node() {
        let (g, id_a, _, _) = make_graph().await;
        let node = g.get_node(&id_a).await.unwrap().unwrap();
        assert_eq!(node.name, "func_a");
    }

    #[tokio::test]
    async fn test_get_neighbors() {
        let (g, id_a, _, _) = make_graph().await;
        let neighbors = g
            .get_neighbors(&id_a, Some(&[EdgeType::Calls]))
            .await
            .unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].1.name, "func_b");
    }

    #[tokio::test]
    async fn test_traverse_depth_2() {
        let (g, id_a, _, _) = make_graph().await;
        let result = g
            .traverse(&id_a, Some(&[EdgeType::Calls]), 2)
            .await
            .unwrap();
        assert_eq!(result.nodes.len(), 3);
        assert_eq!(result.total_visited, 3);
    }

    #[tokio::test]
    async fn test_traverse_depth_1() {
        let (g, id_a, _, _) = make_graph().await;
        let result = g
            .traverse(&id_a, Some(&[EdgeType::Calls]), 1)
            .await
            .unwrap();
        assert_eq!(result.nodes.len(), 2);
    }

    #[tokio::test]
    async fn test_find_paths() {
        let (g, id_a, _, id_c) = make_graph().await;
        let paths = g.find_paths(&id_a, &id_c, 3).await.unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].len(), 3);
    }

    #[tokio::test]
    async fn test_get_inbound() {
        let (g, _, _, id_c) = make_graph().await;
        let inbound = g
            .get_inbound(&id_c, Some(&[EdgeType::Calls]))
            .await
            .unwrap();
        assert_eq!(inbound.len(), 1);
        assert_eq!(inbound[0].1.name, "func_b");
    }

    #[tokio::test]
    async fn test_remove_node_cascades() {
        let (g, _, id_b, _) = make_graph().await;
        assert_eq!(g.edge_count().await, 2);
        g.remove_node(&id_b).await.unwrap();
        assert_eq!(g.node_count().await, 2);
        assert_eq!(g.edge_count().await, 0);
    }

    #[tokio::test]
    async fn test_duplicate_node_error() {
        let g = InMemoryGraphStore::new();
        let node = Node::new(NodeType::Doc, "readme", "# Hello");
        let id = node.id;
        g.add_node(node.clone()).await.unwrap();
        let mut dup = node;
        dup.id = id;
        assert!(g.add_node(dup).await.is_err());
    }
}
