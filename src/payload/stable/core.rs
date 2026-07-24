use super::adjacency::{EdgeKeys, NONE_SLOT};
use crate::Vec;
use crate::{EdgeEndpoints, GraphError, Result, StableEdgeKey, StableNodeKey};

#[derive(Debug, Clone)]
pub(super) struct NodeSlot<NodePayload> {
    pub(super) generation: u32,
    pub(super) value: Option<NodePayload>,
    pub(super) outgoing: u32,
    pub(super) incoming: u32,
}

#[derive(Debug, Clone)]
pub(super) struct EdgeSlot<EdgePayload> {
    pub(super) generation: u32,
    pub(super) value: Option<EdgePayload>,
    pub(super) source: u32,
    pub(super) target: u32,
    pub(super) next_outgoing: u32,
    pub(super) next_incoming: u32,
}

/// A mutable directed payload graph with generation-checked stable keys.
#[derive(Debug, Clone)]
pub struct StablePayloadGraph<NodePayload, EdgePayload> {
    pub(super) nodes: Vec<NodeSlot<NodePayload>>,
    pub(super) edges: Vec<EdgeSlot<EdgePayload>>,
    pub(super) free_nodes: Vec<u32>,
    pub(super) free_edges: Vec<u32>,
    pub(super) node_count: usize,
    pub(super) edge_count: usize,
}

impl<NodePayload, EdgePayload> Default for StablePayloadGraph<NodePayload, EdgePayload> {
    fn default() -> Self {
        Self::new()
    }
}

impl<NodePayload, EdgePayload> StablePayloadGraph<NodePayload, EdgePayload> {
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

    /// Adds a payload and returns a key that detects later slot reuse.
    ///
    /// # Errors
    ///
    /// Returns an error when the stable index space is exhausted.
    pub fn add_node(&mut self, payload: NodePayload) -> Result<StableNodeKey> {
        let key = if let Some(slot) = self.free_nodes.pop() {
            let entry = &mut self.nodes[slot as usize];
            entry.value = Some(payload);
            entry.outgoing = NONE_SLOT;
            entry.incoming = NONE_SLOT;
            StableNodeKey::new(slot, entry.generation)
        } else {
            let slot = next_slot(self.nodes.len(), "stable payload nodes")?;
            self.nodes.push(NodeSlot {
                generation: 0,
                value: Some(payload),
                outgoing: NONE_SLOT,
                incoming: NONE_SLOT,
            });
            StableNodeKey::new(slot, 0)
        };
        self.node_count += 1;
        Ok(key)
    }

    /// Adds a directed edge between two live stable node keys.
    ///
    /// # Errors
    ///
    /// Returns an error for stale endpoints or exhausted stable indices.
    pub fn add_edge(
        &mut self,
        source: StableNodeKey,
        target: StableNodeKey,
        payload: EdgePayload,
    ) -> Result<StableEdgeKey> {
        self.require_node(source)?;
        self.require_node(target)?;
        let key = self.allocate_edge(source, target, payload)?;
        self.link_outgoing(source, key);
        self.link_incoming(target, key);
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
        let slot = self.edge_slot(key)?;
        Some(EdgeEndpoints::new(
            self.node_key_at(slot.source)?,
            self.node_key_at(slot.target)?,
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

    pub fn outgoing_edges(&self, node: StableNodeKey) -> impl Iterator<Item = StableEdgeKey> + '_ {
        self.node_slot(node).map_or_else(
            || EdgeKeys::empty(&self.edges, true),
            |slot| EdgeKeys::new(&self.edges, slot.outgoing, true),
        )
    }

    pub fn incoming_edges(&self, node: StableNodeKey) -> impl Iterator<Item = StableEdgeKey> + '_ {
        self.node_slot(node).map_or_else(
            || EdgeKeys::empty(&self.edges, false),
            |slot| EdgeKeys::new(&self.edges, slot.incoming, false),
        )
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
        if self.node_slot(key).is_some() {
            Ok(())
        } else {
            Err(GraphError::InvalidStableKey {
                category: "node",
                slot: key.slot(),
                generation: key.generation(),
            })
        }
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
            entry.next_outgoing = NONE_SLOT;
            entry.next_incoming = NONE_SLOT;
            return Ok(StableEdgeKey::new(slot, entry.generation));
        }
        let slot = next_slot(self.edges.len(), "stable payload edges")?;
        self.edges.push(EdgeSlot {
            generation: 0,
            value: Some(payload),
            source: source.slot(),
            target: target.slot(),
            next_outgoing: NONE_SLOT,
            next_incoming: NONE_SLOT,
        });
        Ok(StableEdgeKey::new(slot, 0))
    }

    fn node_key_at(&self, slot: u32) -> Option<StableNodeKey> {
        let entry = self.nodes.get(slot as usize)?;
        entry
            .value
            .as_ref()
            .map(|_| StableNodeKey::new(slot, entry.generation))
    }
}

pub(crate) fn next_slot(count: usize, category: &'static str) -> Result<u32> {
    u32::try_from(count).map_err(|_| GraphError::IndexCapacityExceeded { category, count })
}
