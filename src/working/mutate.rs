use super::{StableEdgeKey, StableNodeKey, WorkingGraph};
use crate::ToString;
use crate::graph::validate::{validate_edge, validate_node};
use crate::{Edge, GraphError, Node, Result};

impl WorkingGraph {
    pub fn remove_edge(&mut self, key: StableEdgeKey) -> Option<Edge> {
        let endpoints = self.edge_endpoints(key)?;
        let edge = self.edge_slot_mut(key)?.value.take()?.value;
        self.node_slot_mut(endpoints.source())?
            .outgoing
            .retain(|candidate| *candidate != key);
        self.node_slot_mut(endpoints.target())?
            .incoming
            .retain(|candidate| *candidate != key);
        self.edge_count -= 1;
        self.retire_edge_slot(key);
        Some(edge)
    }

    pub fn remove_node(&mut self, key: StableNodeKey) -> Option<Node> {
        let slot = self.node_slot(key)?;
        let id = slot.value.as_ref()?.id.clone();
        let mut incident = slot.outgoing.clone();
        incident.extend(slot.incoming.iter().copied());
        incident.sort_unstable();
        incident.dedup();
        for edge in incident {
            self.remove_edge(edge);
        }

        let node = self.node_slot_mut(key)?.value.take()?;
        self.node_by_id.remove(&id);
        self.node_count -= 1;
        self.retire_node_slot(key);
        Some(node)
    }

    /// Replaces a node while retaining its stable key.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid data or a conflicting new identifier.
    pub fn replace_node(&mut self, key: StableNodeKey, node: Node) -> Result<Option<Node>> {
        validate_node(&node)?;
        let Some(current) = self.node(key) else {
            return Ok(None);
        };
        let old_id = current.id.clone();
        if self
            .node_by_id
            .get(&node.id)
            .is_some_and(|existing| *existing != key)
        {
            return Err(GraphError::ConflictingNode {
                id: node.id.to_string(),
            });
        }

        if old_id != node.id {
            let new_id = node.id.clone();
            let Some(incident) = self.node_slot(key).map(|slot| {
                let mut keys = slot.outgoing.clone();
                keys.extend(slot.incoming.iter().copied());
                keys.sort_unstable();
                keys.dedup();
                keys
            }) else {
                return Ok(None);
            };
            for edge in incident {
                if let Some(working) = self
                    .edge_slot_mut(edge)
                    .and_then(|slot| slot.value.as_mut())
                {
                    if working.source == key {
                        working.value.source = new_id.clone();
                    }
                    if working.target == key {
                        working.value.target = new_id.clone();
                    }
                }
            }
            self.node_by_id.remove(&old_id);
            self.node_by_id.insert(new_id, key);
        }
        Ok(self
            .node_slot_mut(key)
            .and_then(|slot| slot.value.replace(node)))
    }

    /// Replaces an edge while retaining its stable key.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid evidence or missing new endpoints.
    pub fn replace_edge(&mut self, key: StableEdgeKey, edge: Edge) -> Result<Option<Edge>> {
        validate_edge(&edge)?;
        let Some(old_endpoints) = self.edge_endpoints(key) else {
            return Ok(None);
        };
        let source =
            self.node_key(edge.source.as_str())
                .ok_or_else(|| GraphError::MissingEdgeSource {
                    id: edge.source.to_string(),
                })?;
        let target =
            self.node_key(edge.target.as_str())
                .ok_or_else(|| GraphError::MissingEdgeTarget {
                    id: edge.target.to_string(),
                })?;

        if let Some(slot) = self.node_slot_mut(old_endpoints.source()) {
            slot.outgoing.retain(|candidate| *candidate != key);
        }
        if let Some(slot) = self.node_slot_mut(old_endpoints.target()) {
            slot.incoming.retain(|candidate| *candidate != key);
        }
        if let Some(slot) = self.node_slot_mut(source) {
            slot.outgoing.push(key);
        }
        if let Some(slot) = self.node_slot_mut(target) {
            slot.incoming.push(key);
        }

        let Some(working) = self.edge_slot_mut(key).and_then(|slot| slot.value.as_mut()) else {
            return Ok(None);
        };
        working.source = source;
        working.target = target;
        Ok(Some(core::mem::replace(&mut working.value, edge)))
    }

    fn retire_node_slot(&mut self, key: StableNodeKey) {
        let slot = &mut self.nodes[key.index()];
        slot.outgoing.clear();
        slot.incoming.clear();
        if let Some(generation) = slot.generation.checked_add(1) {
            slot.generation = generation;
            self.free_nodes.push(key.slot());
        }
    }

    fn retire_edge_slot(&mut self, key: StableEdgeKey) {
        let slot = &mut self.edges[key.index()];
        if let Some(generation) = slot.generation.checked_add(1) {
            slot.generation = generation;
            self.free_edges.push(key.slot());
        }
    }
}
