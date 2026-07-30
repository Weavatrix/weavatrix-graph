use super::core::StableUndirectedPayloadGraph;
use super::incidence::NONE_SLOT;
use crate::{GraphError, Result, StableEdgeKey, StableNodeKey};

impl<NodePayload, EdgePayload> StableUndirectedPayloadGraph<NodePayload, EdgePayload> {
    pub fn remove_edge(&mut self, key: StableEdgeKey) -> Option<EdgePayload> {
        let endpoints = self.edge_endpoints(key)?;
        self.validate_unlink(endpoints.source(), key).ok()?;
        if endpoints.source() != endpoints.target() {
            self.validate_unlink(endpoints.target(), key).ok()?;
        }
        self.unlink(endpoints.source(), key).ok()?;
        if endpoints.source() != endpoints.target() {
            self.unlink(endpoints.target(), key).ok()?;
        }
        let payload = self.edge_slot_mut(key)?.value.take()?;
        self.edge_count -= 1;
        self.retire_edge(key);
        Some(payload)
    }

    pub fn remove_node(&mut self, key: StableNodeKey) -> Option<NodePayload> {
        let incident = self.incident_edges(key).collect::<crate::Vec<_>>();
        for edge in incident {
            self.remove_edge(edge);
        }
        let payload = self.node_slot_mut(key)?.value.take()?;
        self.node_count -= 1;
        self.retire_node(key);
        Some(payload)
    }

    /// Moves an edge without changing its stable key or payload.
    ///
    /// # Errors
    ///
    /// Returns an error when either new endpoint is stale.
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
        self.validate_unlink(previous.source(), key)?;
        if previous.source() != previous.target() {
            self.validate_unlink(previous.target(), key)?;
        }
        self.unlink(previous.source(), key)?;
        if previous.source() != previous.target() {
            self.unlink(previous.target(), key)?;
        }
        let edge = self
            .edge_slot_mut(key)
            .ok_or(GraphError::InvalidStableKey {
                category: "undirected edge",
                slot: key.slot(),
                generation: key.generation(),
            })?;
        edge.source = source.slot();
        edge.target = target.slot();
        edge.source_previous = NONE_SLOT;
        edge.source_next = NONE_SLOT;
        edge.target_previous = NONE_SLOT;
        edge.target_next = NONE_SLOT;
        self.link(source, key);
        if source != target {
            self.link(target, key);
        }
        Ok(true)
    }

    fn unlink(&mut self, node: StableNodeKey, edge: StableEdgeKey) -> Result<()> {
        self.validate_unlink(node, edge)?;
        let (previous, next) = {
            let Some(slot) = self.edge_slot(edge) else {
                return Err(invalid_incidence(edge));
            };
            (slot.previous(node.slot()), slot.next(node.slot()))
        };
        if previous == NONE_SLOT {
            self.nodes[node.index()].first_edge = next;
        } else {
            self.edges[previous as usize].set_next(node.slot(), next);
        }
        if next == NONE_SLOT {
            self.nodes[node.index()].last_edge = previous;
        } else {
            self.edges[next as usize].set_previous(node.slot(), previous);
        }
        self.nodes[node.index()].degree -= 1;
        Ok(())
    }

    fn validate_unlink(&self, node: StableNodeKey, edge: StableEdgeKey) -> Result<()> {
        let node_slot = self.node_slot(node).ok_or(GraphError::InvalidStableKey {
            category: "undirected node",
            slot: node.slot(),
            generation: node.generation(),
        })?;
        let edge_slot = self
            .edge_slot(edge)
            .ok_or_else(|| invalid_incidence(edge))?;
        if node_slot.degree == 0
            || (edge_slot.source != node.slot() && edge_slot.target != node.slot())
            || !self.live_incidence(edge_slot.previous(node.slot()), node.slot())
            || !self.live_incidence(edge_slot.next(node.slot()), node.slot())
        {
            return Err(invalid_incidence(edge));
        }
        Ok(())
    }

    fn live_incidence(&self, edge: u32, node: u32) -> bool {
        edge == NONE_SLOT
            || self.edges.get(edge as usize).is_some_and(|slot| {
                slot.value.is_some() && (slot.source == node || slot.target == node)
            })
    }

    fn retire_node(&mut self, key: StableNodeKey) {
        let slot = &mut self.nodes[key.index()];
        slot.first_edge = NONE_SLOT;
        slot.last_edge = NONE_SLOT;
        slot.degree = 0;
        if let Some(generation) = slot.generation.checked_add(1) {
            slot.generation = generation;
            self.free_nodes.push(key.slot());
        }
    }

    fn retire_edge(&mut self, key: StableEdgeKey) {
        let slot = &mut self.edges[key.index()];
        slot.source_previous = NONE_SLOT;
        slot.source_next = NONE_SLOT;
        slot.target_previous = NONE_SLOT;
        slot.target_next = NONE_SLOT;
        if let Some(generation) = slot.generation.checked_add(1) {
            slot.generation = generation;
            self.free_edges.push(key.slot());
        }
    }

    pub(super) fn link(&mut self, node: StableNodeKey, edge: StableEdgeKey) {
        let previous = self.nodes[node.index()].last_edge;
        self.edges[edge.index()].set_previous(node.slot(), previous);
        self.edges[edge.index()].set_next(node.slot(), NONE_SLOT);
        if previous == NONE_SLOT {
            self.nodes[node.index()].first_edge = edge.slot();
        } else {
            self.edges[previous as usize].set_next(node.slot(), edge.slot());
        }
        self.nodes[node.index()].last_edge = edge.slot();
        self.nodes[node.index()].degree += 1;
    }
}

fn invalid_incidence(edge: StableEdgeKey) -> GraphError {
    GraphError::InvalidStableKey {
        category: "undirected incidence edge",
        slot: edge.slot(),
        generation: edge.generation(),
    }
}
