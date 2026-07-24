use super::core::EdgeSlot;
use crate::StableEdgeKey;
use core::iter::FusedIterator;

pub(super) const NONE_SLOT: u32 = u32::MAX;

pub struct EdgeKeys<'graph, EdgePayload> {
    edges: &'graph [EdgeSlot<EdgePayload>],
    next: u32,
    outgoing: bool,
}

impl<'graph, EdgePayload> EdgeKeys<'graph, EdgePayload> {
    pub(super) const fn new(
        edges: &'graph [EdgeSlot<EdgePayload>],
        next: u32,
        outgoing: bool,
    ) -> Self {
        Self {
            edges,
            next,
            outgoing,
        }
    }

    pub(super) const fn empty(edges: &'graph [EdgeSlot<EdgePayload>], outgoing: bool) -> Self {
        Self::new(edges, NONE_SLOT, outgoing)
    }
}

impl<EdgePayload> Iterator for EdgeKeys<'_, EdgePayload> {
    type Item = StableEdgeKey;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == NONE_SLOT {
            return None;
        }
        let slot_index = self.next;
        let slot = self.edges.get(slot_index as usize)?;
        if slot.value.is_none() {
            self.next = NONE_SLOT;
            return None;
        }
        self.next = if self.outgoing {
            slot.next_outgoing
        } else {
            slot.next_incoming
        };
        Some(StableEdgeKey::new(slot_index, slot.generation))
    }
}

impl<EdgePayload> FusedIterator for EdgeKeys<'_, EdgePayload> {}
