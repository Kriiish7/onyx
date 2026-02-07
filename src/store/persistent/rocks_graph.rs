//! RocksDB-backed graph store implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rocksdb::DB;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{OnyxError, OnyxResult};
use crate::model::edge::{Edge, EdgeType};
use crate::model::node::{Node, NodeType};
use crate::store::graph::{GraphStore, SubgraphResult, TraversalResult};

use super::{CF_EDGES, CF_NODES, CF_NODE_INBOUND, CF_NODE_OUTBOUND};

/// RocksDB-backed graph store with persistent node and edge storage.
#[derive(Clone)]
pub struct RocksGraphStore {
    db: Arc<DB>,
}

impl RocksGraphStore {
    /// Create a new RocksDB graph store.
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    /// Serialize a node to bytes.
    fn serialize_node(&self, node: &Node) -> OnyxResult<Vec<u8>> {
        bincode::serialize(node)
            .map_err(|e| OnyxError::Internal(format!("Failed to serialize node: {}", e)))
    }

    /// Deserialize a node from bytes.
    fn deserialize_node(&self, bytes: &[u8]) -> OnyxResult<Node> {
        bincode::deserialize(bytes)
            .map_err(|e| OnyxError::Internal(format!("Failed to deserialize node: {}", e)))
    }

    /// Serialize an edge to bytes.
    fn serialize_edge(&self, edge: &Edge) -> OnyxResult<Vec<u8>> {
        bincode::serialize(edge)
            .map_err(|e| OnyxError::Internal(format!("Failed to serialize edge: {}", e)))
    }

    /// Deserialize an edge from bytes.
    fn deserialize_edge(&self, bytes: &[u8]) -> OnyxResult<Edge> {
        bincode::deserialize(bytes)
            .map_err(|e| OnyxError::Internal(format!("Failed to deserialize edge: {}", e)))
    }

    /// Get the nodes column family handle.
    fn cf_nodes(&self) -> OnyxResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_NODES)
            .ok_or_else(|| OnyxError::Internal("Missing nodes column family".to_string()))
    }

    /// Get the edges column family handle.
    fn cf_edges(&self) -> OnyxResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_EDGES)
            .ok_or_else(|| OnyxError::Internal("Missing edges column family".to_string()))
    }

    /// Get the node outbound edges column family handle.
    fn cf_node_outbound(&self) -> OnyxResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_NODE_OUTBOUND)
            .ok_or_else(|| OnyxError::Internal("Missing node_outbound column family".to_string()))
    }

    /// Get the node inbound edges column family handle.
    fn cf_node_inbound(&self) -> OnyxResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_NODE_INBOUND)
            .ok_or_else(|| OnyxError::Internal("Missing node_inbound column family".to_string()))
    }

    /// Build adjacency index key for node outbound edges.
    fn outbound_key(&self, node_id: &Uuid, edge_id: &Uuid) -> Vec<u8> {
        let mut key = node_id.as_bytes().to_vec();
        key.extend_from_slice(edge_id.as_bytes());
        key
    }

    /// Build adjacency index key for node inbound edges.
    fn inbound_key(&self, node_id: &Uuid, edge_id: &Uuid) -> Vec<u8> {
        let mut key = node_id.as_bytes().to_vec();
        key.extend_from_slice(edge_id.as_bytes());
        key
    }

    /// Get all edge IDs for a node from an adjacency index.
    fn get_edge_ids_from_adjacency(
        &self,
        cf: &rocksdb::ColumnFamily,
        node_id: &Uuid,
    ) -> OnyxResult<Vec<Uuid>> {
        let prefix = node_id.as_bytes();
        let iter = self.db.prefix_iterator_cf(cf, prefix);

        let mut edge_ids = Vec::new();
        for item in iter {
            let (key, _) = item.map_err(|e| {
                OnyxError::Internal(format!("Failed to iterate adjacency index: {}", e))
            })?;

            // Key format: [node_id (16 bytes)][edge_id (16 bytes)]
            if key.len() == 32 {
                let edge_id_bytes = &key[16..32];
                let edge_id = Uuid::from_slice(edge_id_bytes)
                    .map_err(|e| OnyxError::Internal(format!("Invalid edge UUID: {}", e)))?;
                edge_ids.push(edge_id);
            }
        }

        Ok(edge_ids)
    }
}

#[async_trait]
impl GraphStore for RocksGraphStore {
    async fn add_node(&self, node: Node) -> OnyxResult<()> {
        let cf = self.cf_nodes()?;
        let key = node.id.as_bytes();
        let value = self.serialize_node(&node)?;

        self.db
            .put_cf(cf, key, value)
            .map_err(|e| OnyxError::Internal(format!("Failed to add node: {}", e)))?;

        Ok(())
    }

    async fn get_node(&self, id: &Uuid) -> OnyxResult<Option<Node>> {
        let cf = self.cf_nodes()?;
        let key = id.as_bytes();

        match self.db.get_cf(cf, key) {
            Ok(Some(bytes)) => Ok(Some(self.deserialize_node(&bytes)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(OnyxError::Internal(format!("Failed to get node: {}", e))),
        }
    }

    async fn update_node(&self, node: Node) -> OnyxResult<()> {
        // Same as add_node for RocksDB (upsert)
        self.add_node(node).await
    }

    async fn remove_node(&self, id: &Uuid) -> OnyxResult<()> {
        let cf_nodes = self.cf_nodes()?;
        let cf_outbound = self.cf_node_outbound()?;
        let cf_inbound = self.cf_node_inbound()?;

        // Get all edges connected to this node
        let outbound_edges = self.get_edge_ids_from_adjacency(cf_outbound, id)?;
        let inbound_edges = self.get_edge_ids_from_adjacency(cf_inbound, id)?;

        // Remove all connected edges
        for edge_id in outbound_edges.iter().chain(inbound_edges.iter()) {
            self.remove_edge(edge_id).await?;
        }

        // Remove the node
        let key = id.as_bytes();
        self.db
            .delete_cf(cf_nodes, key)
            .map_err(|e| OnyxError::Internal(format!("Failed to remove node: {}", e)))?;

        Ok(())
    }

    async fn add_edge(&self, edge: Edge) -> OnyxResult<()> {
        let cf_edges = self.cf_edges()?;
        let cf_outbound = self.cf_node_outbound()?;
        let cf_inbound = self.cf_node_inbound()?;

        // Store the edge
        let key = edge.id.as_bytes();
        let value = self.serialize_edge(&edge)?;
        self.db
            .put_cf(cf_edges, key, value)
            .map_err(|e| OnyxError::Internal(format!("Failed to add edge: {}", e)))?;

        // Update adjacency indices
        let outbound_key = self.outbound_key(&edge.source_id, &edge.id);
        let inbound_key = self.inbound_key(&edge.target_id, &edge.id);

        self.db
            .put_cf(cf_outbound, outbound_key, &[])
            .map_err(|e| OnyxError::Internal(format!("Failed to update outbound index: {}", e)))?;

        self.db
            .put_cf(cf_inbound, inbound_key, &[])
            .map_err(|e| OnyxError::Internal(format!("Failed to update inbound index: {}", e)))?;

        Ok(())
    }

    async fn get_edge(&self, id: &Uuid) -> OnyxResult<Option<Edge>> {
        let cf = self.cf_edges()?;
        let key = id.as_bytes();

        match self.db.get_cf(cf, key) {
            Ok(Some(bytes)) => Ok(Some(self.deserialize_edge(&bytes)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(OnyxError::Internal(format!("Failed to get edge: {}", e))),
        }
    }

    async fn remove_edge(&self, id: &Uuid) -> OnyxResult<()> {
        // First get the edge to know source and target
        let edge = match self.get_edge(id).await? {
            Some(e) => e,
            None => return Ok(()), // Already deleted
        };

        let cf_edges = self.cf_edges()?;
        let cf_outbound = self.cf_node_outbound()?;
        let cf_inbound = self.cf_node_inbound()?;

        // Remove from adjacency indices
        let outbound_key = self.outbound_key(&edge.source_id, id);
        let inbound_key = self.inbound_key(&edge.target_id, id);

        self.db
            .delete_cf(cf_outbound, outbound_key)
            .map_err(|e| OnyxError::Internal(format!("Failed to remove from outbound index: {}", e)))?;

        self.db
            .delete_cf(cf_inbound, inbound_key)
            .map_err(|e| OnyxError::Internal(format!("Failed to remove from inbound index: {}", e)))?;

        // Remove the edge
        let key = id.as_bytes();
        self.db
            .delete_cf(cf_edges, key)
            .map_err(|e| OnyxError::Internal(format!("Failed to remove edge: {}", e)))?;

        Ok(())
    }

    async fn get_neighbors(
        &self,
        id: &Uuid,
        edge_types: Option<&[EdgeType]>,
    ) -> OnyxResult<Vec<(Edge, Node)>> {
        let cf_outbound = self.cf_node_outbound()?;
        let edge_ids = self.get_edge_ids_from_adjacency(cf_outbound, id)?;

        let mut neighbors = Vec::new();
        for edge_id in edge_ids {
            if let Some(edge) = self.get_edge(&edge_id).await? {
                // Filter by edge type if specified
                if let Some(types) = edge_types {
                    if !types.contains(&edge.edge_type) {
                        continue;
                    }
                }

                // Get the target node
                if let Some(node) = self.get_node(&edge.target_id).await? {
                    neighbors.push((edge, node));
                }
            }
        }

        Ok(neighbors)
    }

    async fn get_inbound(
        &self,
        id: &Uuid,
        edge_types: Option<&[EdgeType]>,
    ) -> OnyxResult<Vec<(Edge, Node)>> {
        let cf_inbound = self.cf_node_inbound()?;
        let edge_ids = self.get_edge_ids_from_adjacency(cf_inbound, id)?;

        let mut inbound = Vec::new();
        for edge_id in edge_ids {
            if let Some(edge) = self.get_edge(&edge_id).await? {
                // Filter by edge type if specified
                if let Some(types) = edge_types {
                    if !types.contains(&edge.edge_type) {
                        continue;
                    }
                }

                // Get the source node
                if let Some(node) = self.get_node(&edge.source_id).await? {
                    inbound.push((edge, node));
                }
            }
        }

        Ok(inbound)
    }

    async fn traverse(
        &self,
        start_id: &Uuid,
        edge_types: Option<&[EdgeType]>,
        max_depth: usize,
    ) -> OnyxResult<TraversalResult> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        queue.push_back((*start_id, 0));
        visited.insert(*start_id);

        while let Some((node_id, depth)) = queue.pop_front() {
            nodes.push((node_id, depth));

            if depth < max_depth {
                let neighbors = self.get_neighbors(&node_id, edge_types).await?;
                for (edge, neighbor) in neighbors {
                    edges.push(edge.id);

                    if !visited.contains(&neighbor.id) {
                        visited.insert(neighbor.id);
                        queue.push_back((neighbor.id, depth + 1));
                    }
                }
            }
        }

        Ok(TraversalResult {
            nodes,
            edges,
            total_visited: visited.len(),
        })
    }

    async fn find_paths(
        &self,
        from: &Uuid,
        to: &Uuid,
        max_depth: usize,
    ) -> OnyxResult<Vec<Vec<Uuid>>> {
        let mut paths = Vec::new();
        let mut current_path = vec![*from];
        let mut visited = HashSet::new();
        visited.insert(*from);

        self.dfs_find_paths(from, to, max_depth, 0, &mut current_path, &mut visited, &mut paths)
            .await?;

        Ok(paths)
    }

    async fn subgraph(&self, root_id: &Uuid, depth: usize) -> OnyxResult<SubgraphResult> {
        let traversal = self.traverse(root_id, None, depth).await?;

        let node_ids: HashSet<Uuid> = traversal.nodes.iter().map(|(id, _)| *id).collect();
        let edge_ids: HashSet<Uuid> = traversal.edges.iter().copied().collect();

        Ok(SubgraphResult { node_ids, edge_ids })
    }

    async fn nodes_by_type(&self, node_type: &NodeType) -> Vec<Node> {
        let cf = match self.cf_nodes() {
            Ok(cf) => cf,
            Err(_) => return vec![],
        };

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        let mut nodes = Vec::new();

        for item in iter {
            if let Ok((_, value)) = item {
                if let Ok(node) = self.deserialize_node(&value) {
                    if &node.node_type == node_type {
                        nodes.push(node);
                    }
                }
            }
        }

        nodes
    }

    async fn edges_by_type(&self, edge_type: &EdgeType) -> Vec<Edge> {
        let cf = match self.cf_edges() {
            Ok(cf) => cf,
            Err(_) => return vec![],
        };

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        let mut edges = Vec::new();

        for item in iter {
            if let Ok((_, value)) = item {
                if let Ok(edge) = self.deserialize_edge(&value) {
                    if &edge.edge_type == edge_type {
                        edges.push(edge);
                    }
                }
            }
        }

        edges
    }

    async fn edges_at_time(&self, id: &Uuid, timestamp: &DateTime<Utc>) -> OnyxResult<Vec<Edge>> {
        let neighbors = self.get_neighbors(id, None).await?;
        let inbound = self.get_inbound(id, None).await?;

        let mut valid_edges = Vec::new();

        for (edge, _) in neighbors.into_iter().chain(inbound.into_iter()) {
            if let Some(temporal) = &edge.temporal_context {
                if temporal.valid_from <= *timestamp
                    && temporal.valid_to.map_or(true, |vt| vt >= *timestamp)
                {
                    valid_edges.push(edge);
                }
            } else {
                valid_edges.push(edge);
            }
        }

        Ok(valid_edges)
    }

    async fn node_count(&self) -> usize {
        let cf = match self.cf_nodes() {
            Ok(cf) => cf,
            Err(_) => return 0,
        };

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        iter.count()
    }

    async fn edge_count(&self) -> usize {
        let cf = match self.cf_edges() {
            Ok(cf) => cf,
            Err(_) => return 0,
        };

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        iter.count()
    }

    async fn all_nodes(&self) -> Vec<Node> {
        let cf = match self.cf_nodes() {
            Ok(cf) => cf,
            Err(_) => return vec![],
        };

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        let mut nodes = Vec::new();

        for item in iter {
            if let Ok((_, value)) = item {
                if let Ok(node) = self.deserialize_node(&value) {
                    nodes.push(node);
                }
            }
        }

        nodes
    }

    async fn get_all_node_ids(&self) -> OnyxResult<Vec<Uuid>> {
        let cf = self.cf_nodes()?;
        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        let mut ids = Vec::new();

        for item in iter {
            if let Ok((key, _)) = item {
                if let Ok(id_str) = std::str::from_utf8(&key) {
                    if let Ok(id) = Uuid::parse_str(id_str) {
                        ids.push(id);
                    }
                }
            }
        }

        Ok(ids)
    }

    async fn get_all_edge_ids(&self) -> OnyxResult<Vec<Uuid>> {
        let cf = self.cf_edges()?;
        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        let mut ids = Vec::new();

        for item in iter {
            if let Ok((key, _)) = item {
                if let Ok(id_str) = std::str::from_utf8(&key) {
                    if let Ok(id) = Uuid::parse_str(id_str) {
                        ids.push(id);
                    }
                }
            }
        }

        Ok(ids)
    }
}

impl RocksGraphStore {
    /// Helper for DFS path finding.
    #[async_recursion::async_recursion]
    async fn dfs_find_paths(
        &self,
        current: &Uuid,
        target: &Uuid,
        max_depth: usize,
        depth: usize,
        current_path: &mut Vec<Uuid>,
        visited: &mut HashSet<Uuid>,
        paths: &mut Vec<Vec<Uuid>>,
    ) -> OnyxResult<()> {
        if current == target {
            paths.push(current_path.clone());
            return Ok(());
        }

        if depth >= max_depth {
            return Ok(());
        }

        let neighbors = self.get_neighbors(current, None).await?;
        for (_, neighbor) in neighbors {
            if !visited.contains(&neighbor.id) {
                visited.insert(neighbor.id);
                current_path.push(neighbor.id);

                self.dfs_find_paths(
                    &neighbor.id,
                    target,
                    max_depth,
                    depth + 1,
                    current_path,
                    visited,
                    paths,
                )
                .await?;

                current_path.pop();
                visited.remove(&neighbor.id);
            }
        }

        Ok(())
    }
}
