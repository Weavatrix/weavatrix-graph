use super::core::Graph;
use super::index::canonicalize_edges;
use super::validate::{validate_edge, validate_node};
use crate::{Edge, GraphError, Node, NodeId, Result};
use crate::{ToString, Vec};
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as NodeMap;
#[cfg(feature = "std")]
use std::collections::HashMap as NodeMap;

#[derive(Debug, Default)]
pub struct GraphBuilder {
    nodes: NodeMap<NodeId, Node>,
    edges: Vec<Edge>,
}

#[cfg(feature = "std")]
fn node_map_with_capacity(capacity: usize) -> NodeMap<NodeId, Node> {
    NodeMap::with_capacity(capacity)
}

#[cfg(not(feature = "std"))]
fn node_map_with_capacity(_: usize) -> NodeMap<NodeId, Node> {
    NodeMap::new()
}

impl GraphBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: NodeMap::new(),
            edges: Vec::new(),
        }
    }

    /// Creates a builder with storage sized for the expected graph.
    #[must_use]
    pub fn with_capacity(nodes: usize, edges: usize) -> Self {
        Self {
            nodes: node_map_with_capacity(nodes),
            edges: Vec::with_capacity(edges),
        }
    }

    /// Adds a node idempotently.
    ///
    /// # Errors
    ///
    /// Returns an error when the same identifier already has a different
    /// definition or the node contains an invalid source span.
    pub fn add_node(&mut self, node: Node) -> Result<&mut Self> {
        validate_node(&node)?;
        if let Some(existing) = self.nodes.get(&node.id) {
            if existing == &node {
                return Ok(self);
            }
            return Err(GraphError::ConflictingNode {
                id: node.id.to_string(),
            });
        }
        self.nodes.insert(node.id.clone(), node);
        Ok(self)
    }

    /// Adds an edge idempotently. Endpoint existence is validated by `build`,
    /// so callers may insert edges before nodes.
    ///
    /// # Errors
    ///
    /// Returns an error when provenance or its source span is invalid.
    pub fn add_edge(&mut self, edge: Edge) -> Result<&mut Self> {
        validate_edge(&edge)?;
        self.edges.push(edge);
        Ok(self)
    }

    /// Validates all endpoints and returns an immutable graph.
    ///
    /// # Errors
    ///
    /// Returns an error when an edge references a missing source or target.
    pub fn build(self) -> Result<Graph> {
        let mut nodes = self.nodes.into_values().collect::<Vec<_>>();
        nodes.sort_unstable_by(|left, right| left.id.cmp(&right.id));
        let (edges, topology) = canonicalize_edges(&nodes, self.edges)?;
        Ok(Graph::from_indexed_parts(nodes, edges, topology))
    }
}
