use super::{StableEdgeKey, StableNodeKey};
use crate::graph::validate::{validate_edge, validate_node};
use crate::{Edge, EdgeEndpoints, GraphError, Node, NodeId, Result};
use crate::{ToString, Vec};
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as NodeMap;
#[cfg(feature = "std")]
use std::collections::HashMap as NodeMap;

pub(super) struct WorkingEdge {
    pub(super) value: Edge,
    pub(super) source: StableNodeKey,
    pub(super) target: StableNodeKey,
}

pub(super) struct NodeSlot {
    pub(super) generation: u32,
    pub(super) value: Option<Node>,
    pub(super) outgoing: Vec<StableEdgeKey>,
    pub(super) incoming: Vec<StableEdgeKey>,
}

pub(super) struct EdgeSlot {
    pub(super) generation: u32,
    pub(super) value: Option<WorkingEdge>,
}

#[derive(Default)]
pub struct WorkingGraph {
    pub(super) nodes: Vec<NodeSlot>,
    pub(super) edges: Vec<EdgeSlot>,
    pub(super) node_by_id: NodeMap<NodeId, StableNodeKey>,
    pub(super) free_nodes: Vec<u32>,
    pub(super) free_edges: Vec<u32>,
    pub(super) node_count: usize,
    pub(super) edge_count: usize,
}

impl WorkingGraph {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_capacity(nodes: usize, edges: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(nodes),
            edges: Vec::with_capacity(edges),
            node_by_id: node_map_with_capacity(nodes),
            free_nodes: Vec::new(),
            free_edges: Vec::new(),
            node_count: 0,
            edge_count: 0,
        }
    }

    /// Inserts a node idempotently and returns its generation-stable key.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid nodes or conflicting definitions.
    pub fn insert_node(&mut self, node: Node) -> Result<StableNodeKey> {
        validate_node(&node)?;
        if let Some(&key) = self.node_by_id.get(&node.id) {
            if self.node(key) == Some(&node) {
                return Ok(key);
            }
            return Err(GraphError::ConflictingNode {
                id: node.id.to_string(),
            });
        }
        let id = node.id.clone();
        let key = self.allocate_node(node)?;
        self.node_by_id.insert(id, key);
        self.node_count += 1;
        Ok(key)
    }

    /// Inserts an edge after resolving its endpoint ids.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid evidence or missing endpoints.
    pub fn insert_edge(&mut self, edge: Edge) -> Result<StableEdgeKey> {
        validate_edge(&edge)?;
        let source = self.node_by_id.get(&edge.source).copied().ok_or_else(|| {
            GraphError::MissingEdgeSource {
                id: edge.source.to_string(),
            }
        })?;
        let target = self.node_by_id.get(&edge.target).copied().ok_or_else(|| {
            GraphError::MissingEdgeTarget {
                id: edge.target.to_string(),
            }
        })?;
        if self.node_slot(source).is_none() {
            return Err(GraphError::MissingEdgeSource {
                id: edge.source.to_string(),
            });
        }
        if self.node_slot(target).is_none() {
            return Err(GraphError::MissingEdgeTarget {
                id: edge.target.to_string(),
            });
        }
        let key = self.allocate_edge(WorkingEdge {
            value: edge,
            source,
            target,
        })?;
        if let Some(slot) = self.node_slot_mut(source) {
            slot.outgoing.push(key);
        }
        if let Some(slot) = self.node_slot_mut(target) {
            slot.incoming.push(key);
        }
        self.edge_count += 1;
        Ok(key)
    }

    #[must_use]
    pub fn node(&self, key: StableNodeKey) -> Option<&Node> {
        self.node_slot(key)?.value.as_ref()
    }

    #[must_use]
    pub fn edge(&self, key: StableEdgeKey) -> Option<&Edge> {
        Some(&self.edge_slot(key)?.value.as_ref()?.value)
    }

    #[must_use]
    pub fn node_key(&self, id: &str) -> Option<StableNodeKey> {
        self.node_by_id.get(id).copied()
    }

    #[must_use]
    pub fn edge_endpoints(&self, key: StableEdgeKey) -> Option<EdgeEndpoints<StableNodeKey>> {
        let edge = self.edge_slot(key)?.value.as_ref()?;
        Some(EdgeEndpoints::new(edge.source, edge.target))
    }

    pub fn nodes(&self) -> impl Iterator<Item = (StableNodeKey, &Node)> {
        self.nodes.iter().enumerate().filter_map(|(slot, entry)| {
            let node = entry.value.as_ref()?;
            Some((
                StableNodeKey::new(u32::try_from(slot).ok()?, entry.generation),
                node,
            ))
        })
    }

    pub fn edges(&self) -> impl Iterator<Item = (StableEdgeKey, &Edge)> {
        self.edges.iter().enumerate().filter_map(|(slot, entry)| {
            let edge = entry.value.as_ref()?;
            Some((
                StableEdgeKey::new(u32::try_from(slot).ok()?, entry.generation),
                &edge.value,
            ))
        })
    }

    #[must_use]
    pub fn outgoing_edges(
        &self,
        node: StableNodeKey,
    ) -> impl DoubleEndedIterator<Item = StableEdgeKey> + ExactSizeIterator + '_ {
        self.node_slot(node)
            .map_or(&[][..], |slot| slot.outgoing.as_slice())
            .iter()
            .copied()
    }

    #[must_use]
    pub fn incoming_edges(
        &self,
        node: StableNodeKey,
    ) -> impl DoubleEndedIterator<Item = StableEdgeKey> + ExactSizeIterator + '_ {
        self.node_slot(node)
            .map_or(&[][..], |slot| slot.incoming.as_slice())
            .iter()
            .copied()
    }

    #[must_use]
    pub const fn node_count(&self) -> usize {
        self.node_count
    }

    #[must_use]
    pub const fn edge_count(&self) -> usize {
        self.edge_count
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.node_count == 0 && self.edge_count == 0
    }

    pub(super) fn node_slot(&self, key: StableNodeKey) -> Option<&NodeSlot> {
        let slot = self.nodes.get(key.index())?;
        (slot.generation == key.generation() && slot.value.is_some()).then_some(slot)
    }

    pub(super) fn node_slot_mut(&mut self, key: StableNodeKey) -> Option<&mut NodeSlot> {
        let slot = self.nodes.get_mut(key.index())?;
        (slot.generation == key.generation() && slot.value.is_some()).then_some(slot)
    }

    pub(super) fn edge_slot(&self, key: StableEdgeKey) -> Option<&EdgeSlot> {
        let slot = self.edges.get(key.index())?;
        (slot.generation == key.generation() && slot.value.is_some()).then_some(slot)
    }

    pub(super) fn edge_slot_mut(&mut self, key: StableEdgeKey) -> Option<&mut EdgeSlot> {
        let slot = self.edges.get_mut(key.index())?;
        (slot.generation == key.generation() && slot.value.is_some()).then_some(slot)
    }

    fn allocate_node(&mut self, node: Node) -> Result<StableNodeKey> {
        if let Some(slot) = self.free_nodes.pop() {
            let entry = &mut self.nodes[slot as usize];
            entry.value = Some(node);
            entry.outgoing.clear();
            entry.incoming.clear();
            return Ok(StableNodeKey::new(slot, entry.generation));
        }
        let slot =
            u32::try_from(self.nodes.len()).map_err(|_| GraphError::IndexCapacityExceeded {
                category: "working nodes",
                count: self.nodes.len(),
            })?;
        self.nodes.push(NodeSlot {
            generation: 0,
            value: Some(node),
            outgoing: Vec::new(),
            incoming: Vec::new(),
        });
        Ok(StableNodeKey::new(slot, 0))
    }

    fn allocate_edge(&mut self, edge: WorkingEdge) -> Result<StableEdgeKey> {
        if let Some(slot) = self.free_edges.pop() {
            let entry = &mut self.edges[slot as usize];
            entry.value = Some(edge);
            return Ok(StableEdgeKey::new(slot, entry.generation));
        }
        let slot =
            u32::try_from(self.edges.len()).map_err(|_| GraphError::IndexCapacityExceeded {
                category: "working edges",
                count: self.edges.len(),
            })?;
        self.edges.push(EdgeSlot {
            generation: 0,
            value: Some(edge),
        });
        Ok(StableEdgeKey::new(slot, 0))
    }
}

#[cfg(feature = "std")]
fn node_map_with_capacity(capacity: usize) -> NodeMap<NodeId, StableNodeKey> {
    NodeMap::with_capacity(capacity)
}

#[cfg(not(feature = "std"))]
fn node_map_with_capacity(_: usize) -> NodeMap<NodeId, StableNodeKey> {
    NodeMap::new()
}
