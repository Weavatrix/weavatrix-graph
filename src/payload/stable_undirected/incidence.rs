use super::core::{EdgeSlot, StableUndirectedPayloadGraph};
use crate::{StableEdgeKey, StableNodeKey};

pub(super) const NONE_SLOT: u32 = u32::MAX;

pub(super) struct IncidentEdges<'a, NodePayload, EdgePayload> {
    graph: &'a StableUndirectedPayloadGraph<NodePayload, EdgePayload>,
    node: u32,
    front: u32,
    back: u32,
    remaining: usize,
}

impl<'a, NodePayload, EdgePayload> IncidentEdges<'a, NodePayload, EdgePayload> {
    pub(super) fn new(
        graph: &'a StableUndirectedPayloadGraph<NodePayload, EdgePayload>,
        node: StableNodeKey,
    ) -> Self {
        let Some(slot) = graph.node_slot(node) else {
            return Self {
                graph,
                node: node.slot(),
                front: NONE_SLOT,
                back: NONE_SLOT,
                remaining: 0,
            };
        };
        Self {
            graph,
            node: node.slot(),
            front: slot.first_edge,
            back: slot.last_edge,
            remaining: slot.degree,
        }
    }
}

impl<NodePayload, EdgePayload> Iterator for IncidentEdges<'_, NodePayload, EdgePayload> {
    type Item = StableEdgeKey;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let slot = self.front;
        let edge = &self.graph.edges[slot as usize];
        self.front = edge.next(self.node);
        self.remaining -= 1;
        Some(StableEdgeKey::new(slot, edge.generation))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<NodePayload, EdgePayload> DoubleEndedIterator for IncidentEdges<'_, NodePayload, EdgePayload> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let slot = self.back;
        let edge = &self.graph.edges[slot as usize];
        self.back = edge.previous(self.node);
        self.remaining -= 1;
        Some(StableEdgeKey::new(slot, edge.generation))
    }
}

impl<NodePayload, EdgePayload> ExactSizeIterator for IncidentEdges<'_, NodePayload, EdgePayload> {}

impl<EdgePayload> EdgeSlot<EdgePayload> {
    pub(super) fn next(&self, node: u32) -> u32 {
        if self.source == node {
            self.source_next
        } else {
            self.target_next
        }
    }

    pub(super) fn previous(&self, node: u32) -> u32 {
        if self.source == node {
            self.source_previous
        } else {
            self.target_previous
        }
    }

    pub(super) fn set_next(&mut self, node: u32, edge: u32) {
        if self.source == node {
            self.source_next = edge;
        } else {
            self.target_next = edge;
        }
    }

    pub(super) fn set_previous(&mut self, node: u32, edge: u32) {
        if self.source == node {
            self.source_previous = edge;
        } else {
            self.target_previous = edge;
        }
    }
}
