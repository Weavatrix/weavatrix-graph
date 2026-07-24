use super::StablePayloadGraph;
use super::adjacency::NONE_SLOT;
use crate::{GraphError, Result, StableEdgeKey, StableNodeKey};

impl<NodePayload, EdgePayload> StablePayloadGraph<NodePayload, EdgePayload> {
    pub fn remove_edge(&mut self, key: StableEdgeKey) -> Option<EdgePayload> {
        let endpoints = self.edge_endpoints(key)?;
        self.unlink_outgoing(endpoints.source(), key);
        self.unlink_incoming(endpoints.target(), key);
        let payload = self.edge_slot_mut(key)?.value.take()?;
        self.edge_count -= 1;
        self.retire_edge(key);
        Some(payload)
    }

    pub fn remove_node(&mut self, key: StableNodeKey) -> Option<NodePayload> {
        self.node_slot(key)?;
        let mut incident = self.outgoing_edges(key).collect::<crate::Vec<_>>();
        incident.extend(self.incoming_edges(key));
        incident.sort_unstable();
        incident.dedup();
        for edge in incident {
            self.remove_edge(edge);
        }
        let payload = self.node_slot_mut(key)?.value.take()?;
        self.node_count -= 1;
        self.retire_node(key);
        Some(payload)
    }

    /// Moves a live edge without changing its stable key or payload.
    ///
    /// # Errors
    ///
    /// Returns an error when either new endpoint key is stale.
    pub fn set_edge_endpoints(
        &mut self,
        key: StableEdgeKey,
        source: StableNodeKey,
        target: StableNodeKey,
    ) -> Result<bool> {
        self.require_node(source)?;
        self.require_node(target)?;
        let Some(previous) = self.edge_endpoints(key) else {
            return Ok(false);
        };
        if previous.source() == source && previous.target() == target {
            return Ok(true);
        }
        self.unlink_outgoing(previous.source(), key);
        self.unlink_incoming(previous.target(), key);
        let edge = self
            .edge_slot_mut(key)
            .ok_or(GraphError::InvalidStableKey {
                category: "edge",
                slot: key.slot(),
                generation: key.generation(),
            })?;
        edge.source = source.slot();
        edge.target = target.slot();
        self.link_outgoing(source, key);
        self.link_incoming(target, key);
        Ok(true)
    }

    pub(super) fn link_outgoing(&mut self, node: StableNodeKey, key: StableEdgeKey) {
        self.edges[key.index()].next_outgoing = self.nodes[node.index()].outgoing;
        self.nodes[node.index()].outgoing = key.slot();
    }

    pub(super) fn link_incoming(&mut self, node: StableNodeKey, key: StableEdgeKey) {
        self.edges[key.index()].next_incoming = self.nodes[node.index()].incoming;
        self.nodes[node.index()].incoming = key.slot();
    }

    fn unlink_outgoing(&mut self, node: StableNodeKey, key: StableEdgeKey) {
        let next = self.edges[key.index()].next_outgoing;
        if self.nodes[node.index()].outgoing == key.slot() {
            self.nodes[node.index()].outgoing = next;
            return;
        }
        let mut cursor = self.nodes[node.index()].outgoing;
        while cursor != NONE_SLOT {
            if self.edges[cursor as usize].next_outgoing == key.slot() {
                self.edges[cursor as usize].next_outgoing = next;
                return;
            }
            cursor = self.edges[cursor as usize].next_outgoing;
        }
    }

    fn unlink_incoming(&mut self, node: StableNodeKey, key: StableEdgeKey) {
        let next = self.edges[key.index()].next_incoming;
        if self.nodes[node.index()].incoming == key.slot() {
            self.nodes[node.index()].incoming = next;
            return;
        }
        let mut cursor = self.nodes[node.index()].incoming;
        while cursor != NONE_SLOT {
            if self.edges[cursor as usize].next_incoming == key.slot() {
                self.edges[cursor as usize].next_incoming = next;
                return;
            }
            cursor = self.edges[cursor as usize].next_incoming;
        }
    }

    fn retire_node(&mut self, key: StableNodeKey) {
        let slot = &mut self.nodes[key.index()];
        slot.outgoing = NONE_SLOT;
        slot.incoming = NONE_SLOT;
        if let Some(generation) = slot.generation.checked_add(1) {
            slot.generation = generation;
            self.free_nodes.push(key.slot());
        }
    }

    fn retire_edge(&mut self, key: StableEdgeKey) {
        let slot = &mut self.edges[key.index()];
        slot.next_outgoing = NONE_SLOT;
        slot.next_incoming = NONE_SLOT;
        if let Some(generation) = slot.generation.checked_add(1) {
            slot.generation = generation;
            self.free_edges.push(key.slot());
        }
    }
}
