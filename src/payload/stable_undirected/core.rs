use super::incidence::{IncidentEdges, NONE_SLOT};
use crate::Vec;
use crate::payload::stable::core::next_slot;
use crate::{EdgeEndpoints, GraphError, Result, StableEdgeKey, StableNodeKey};

#[derive(Debug, Clone)]
pub(super) struct NodeSlot<NodePayload> {
    pub(super) generation: u32,
    pub(super) value: Option<NodePayload>,
    pub(super) first_edge: u32,
    pub(super) last_edge: u32,
    pub(super) degree: usize,
}

#[derive(Debug, Clone)]
pub(super) struct EdgeSlot<EdgePayload> {
    pub(super) generation: u32,
    pub(super) value: Option<EdgePayload>,
    pub(super) source: u32,
    pub(super) target: u32,
    pub(super) source_previous: u32,
    pub(super) source_next: u32,
    pub(super) target_previous: u32,
    pub(super) target_next: u32,
}

/// A mutable undirected payload graph with generation-checked stable keys.
#[derive(Debug, Clone)]
pub struct StableUndirectedPayloadGraph<NodePayload, EdgePayload> {
    pub(super) nodes: Vec<NodeSlot<NodePayload>>,
    pub(super) edges: Vec<EdgeSlot<EdgePayload>>,
    pub(super) free_nodes: Vec<u32>,
    pub(super) free_edges: Vec<u32>,
    pub(super) node_count: usize,
    pub(super) edge_count: usize,
}

impl<NodePayload, EdgePayload> Default for StableUndirectedPayloadGraph<NodePayload, EdgePayload> {
    fn default() -> Self {
        Self::new()
    }
}

impl<NodePayload, EdgePayload> StableUndirectedPayloadGraph<NodePayload, EdgePayload> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            free_nodes: Vec::new(),
            free_edges: Vec::new(),
            node_count: 0,
            edge_count: 0,
        }
    }

    #[must_use]
    pub fn with_capacity(nodes: usize, edges: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(nodes),
            edges: Vec::with_capacity(edges),
            free_nodes: Vec::new(),
            free_edges: Vec::new(),
            node_count: 0,
            edge_count: 0,
        }
    }

    /// Adds a node payload and returns a generation-stable key.
    ///
    /// # Errors
    ///
    /// Returns an error when the stable index space is exhausted.
    pub fn add_node(&mut self, payload: NodePayload) -> Result<StableNodeKey> {
        let key = if let Some(slot) = self.free_nodes.pop() {
            let entry = &mut self.nodes[slot as usize];
            entry.value = Some(payload);
            entry.first_edge = NONE_SLOT;
            entry.last_edge = NONE_SLOT;
            entry.degree = 0;
            StableNodeKey::new(slot, entry.generation)
        } else {
            let slot = next_slot(self.nodes.len(), "stable undirected nodes")?;
            self.nodes.push(NodeSlot {
                generation: 0,
                value: Some(payload),
                first_edge: NONE_SLOT,
                last_edge: NONE_SLOT,
                degree: 0,
            });
            StableNodeKey::new(slot, 0)
        };
        self.node_count += 1;
        Ok(key)
    }

    /// Adds one undirected edge. A self-loop occupies one incidence entry.
    ///
    /// # Errors
    ///
    /// Returns an error for stale endpoints or exhausted edge indices.
    pub fn add_edge(
        &mut self,
        source: StableNodeKey,
        target: StableNodeKey,
        payload: EdgePayload,
    ) -> Result<StableEdgeKey> {
        self.require_node(source)?;
        self.require_node(target)?;
        let key = self.allocate_edge(source, target, payload)?;
        self.link(source, key);
        if source != target {
            self.link(target, key);
        }
        self.edge_count += 1;
        Ok(key)
    }

    #[must_use]
    pub fn node(&self, key: StableNodeKey) -> Option<&NodePayload> {
        self.node_slot(key)?.value.as_ref()
    }

    #[must_use]
    pub fn node_mut(&mut self, key: StableNodeKey) -> Option<&mut NodePayload> {
        self.node_slot_mut(key)?.value.as_mut()
    }

    #[must_use]
    pub fn edge(&self, key: StableEdgeKey) -> Option<&EdgePayload> {
        self.edge_slot(key)?.value.as_ref()
    }

    #[must_use]
    pub fn edge_mut(&mut self, key: StableEdgeKey) -> Option<&mut EdgePayload> {
        self.edge_slot_mut(key)?.value.as_mut()
    }

    #[must_use]
    pub fn edge_endpoints(&self, key: StableEdgeKey) -> Option<EdgeEndpoints<StableNodeKey>> {
        let edge = self.edge_slot(key)?;
        Some(EdgeEndpoints::new(
            self.node_key_at(edge.source)?,
            self.node_key_at(edge.target)?,
        ))
    }

    pub fn nodes(&self) -> impl Iterator<Item = (StableNodeKey, &NodePayload)> {
        self.nodes.iter().enumerate().filter_map(|(slot, entry)| {
            Some((
                StableNodeKey::new(u32::try_from(slot).ok()?, entry.generation),
                entry.value.as_ref()?,
            ))
        })
    }

    pub fn edges(&self) -> impl Iterator<Item = (StableEdgeKey, &EdgePayload)> {
        self.edges.iter().enumerate().filter_map(|(slot, entry)| {
            Some((
                StableEdgeKey::new(u32::try_from(slot).ok()?, entry.generation),
                entry.value.as_ref()?,
            ))
        })
    }

    #[must_use]
    pub fn incident_edges(
        &self,
        node: StableNodeKey,
    ) -> impl DoubleEndedIterator<Item = StableEdgeKey> + ExactSizeIterator + '_ {
        IncidentEdges::new(self, node)
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

    pub(super) fn node_slot(&self, key: StableNodeKey) -> Option<&NodeSlot<NodePayload>> {
        let slot = self.nodes.get(key.index())?;
        (slot.generation == key.generation() && slot.value.is_some()).then_some(slot)
    }

    pub(super) fn node_slot_mut(
        &mut self,
        key: StableNodeKey,
    ) -> Option<&mut NodeSlot<NodePayload>> {
        let slot = self.nodes.get_mut(key.index())?;
        (slot.generation == key.generation() && slot.value.is_some()).then_some(slot)
    }

    pub(super) fn edge_slot(&self, key: StableEdgeKey) -> Option<&EdgeSlot<EdgePayload>> {
        let slot = self.edges.get(key.index())?;
        (slot.generation == key.generation() && slot.value.is_some()).then_some(slot)
    }

    pub(super) fn edge_slot_mut(
        &mut self,
        key: StableEdgeKey,
    ) -> Option<&mut EdgeSlot<EdgePayload>> {
        let slot = self.edges.get_mut(key.index())?;
        (slot.generation == key.generation() && slot.value.is_some()).then_some(slot)
    }

    pub(super) fn require_node(&self, key: StableNodeKey) -> Result<()> {
        self.node_slot(key)
            .map(|_| ())
            .ok_or(GraphError::InvalidStableKey {
                category: "undirected node",
                slot: key.slot(),
                generation: key.generation(),
            })
    }

    fn allocate_edge(
        &mut self,
        source: StableNodeKey,
        target: StableNodeKey,
        payload: EdgePayload,
    ) -> Result<StableEdgeKey> {
        if let Some(slot) = self.free_edges.pop() {
            let entry = &mut self.edges[slot as usize];
            entry.value = Some(payload);
            entry.source = source.slot();
            entry.target = target.slot();
            entry.source_previous = NONE_SLOT;
            entry.source_next = NONE_SLOT;
            entry.target_previous = NONE_SLOT;
            entry.target_next = NONE_SLOT;
            return Ok(StableEdgeKey::new(slot, entry.generation));
        }
        let slot = next_slot(self.edges.len(), "stable undirected edges")?;
        self.edges.push(EdgeSlot {
            generation: 0,
            value: Some(payload),
            source: source.slot(),
            target: target.slot(),
            source_previous: NONE_SLOT,
            source_next: NONE_SLOT,
            target_previous: NONE_SLOT,
            target_next: NONE_SLOT,
        });
        Ok(StableEdgeKey::new(slot, 0))
    }

    pub(super) fn node_key_at(&self, slot: u32) -> Option<StableNodeKey> {
        let entry = self.nodes.get(slot as usize)?;
        entry
            .value
            .as_ref()
            .map(|_| StableNodeKey::new(slot, entry.generation))
    }
}
