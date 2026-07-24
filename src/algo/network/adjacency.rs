use crate::algo::traversal::{Direction, for_each_neighbor};
use crate::{IndexGraphView, Vec};

pub(super) struct SlotAdjacency<Node> {
    pub(super) nodes: Vec<Node>,
    pub(super) nodes_by_slot: Vec<Option<Node>>,
    pub(super) neighbors: Vec<Vec<usize>>,
}

impl<Node: Copy> SlotAdjacency<Node> {
    pub(super) fn node(&self, slot: usize) -> Option<Node> {
        self.nodes_by_slot.get(slot).copied().flatten()
    }
}

pub(super) fn adjacency<G>(graph: &G, direction: Direction) -> SlotAdjacency<G::Node>
where
    G: IndexGraphView,
{
    let nodes = graph.node_indices().collect::<Vec<_>>();
    let mut nodes_by_slot = vec![None; graph.node_bound()];
    let mut neighbors = vec![Vec::new(); graph.node_bound()];
    for &node in &nodes {
        let slot = G::node_slot(node);
        nodes_by_slot[slot] = Some(node);
        for_each_neighbor(graph, node, direction, &mut |_| true, |neighbor| {
            let neighbor_slot = G::node_slot(neighbor);
            if neighbor_slot != slot {
                neighbors[slot].push(neighbor_slot);
            }
        });
        neighbors[slot].sort_unstable();
        neighbors[slot].dedup();
    }
    SlotAdjacency {
        nodes,
        nodes_by_slot,
        neighbors,
    }
}
