use super::super::SignedPath;
use crate::IndexGraphView;
use crate::Vec;

#[derive(Debug, Clone)]
pub struct AllPairsShortestPaths<Node> {
    pub(super) nodes: Vec<Node>,
    pub(super) by_slot: Vec<Option<Node>>,
    pub(super) distances: Vec<i64>,
    pub(super) reachable: Vec<bool>,
    pub(super) next: Vec<Option<usize>>,
    pub(super) bound: usize,
    pub(super) node_slot: fn(Node) -> usize,
}

impl<Node: Copy + Eq> AllPairsShortestPaths<Node> {
    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    #[must_use]
    pub fn distance(&self, source: Node, target: Node) -> Option<i64> {
        let (source, target) = self.slots(source, target)?;
        let index = cell(self.bound, source, target);
        self.reachable[index].then_some(self.distances[index])
    }

    #[must_use]
    pub fn path(&self, source: Node, target: Node) -> Option<SignedPath<Node>> {
        let total_cost = self.distance(source, target)?;
        let (source_slot, target_slot) = self.slots(source, target)?;
        let mut slot = source_slot;
        let mut nodes = vec![source];
        while slot != target_slot {
            slot = self.next[cell(self.bound, slot, target_slot)]?;
            nodes.push(self.by_slot[slot]?);
            if nodes.len() > self.nodes.len() {
                return None;
            }
        }
        Some(SignedPath::from_parts(nodes, total_cost))
    }

    fn slots(&self, source: Node, target: Node) -> Option<(usize, usize)> {
        let source_slot = (self.node_slot)(source);
        let target_slot = (self.node_slot)(target);
        (self.by_slot.get(source_slot) == Some(&Some(source))
            && self.by_slot.get(target_slot) == Some(&Some(target)))
        .then_some((source_slot, target_slot))
    }
}

pub(super) fn indexed_nodes<G: IndexGraphView>(graph: &G) -> (Vec<G::Node>, Vec<Option<G::Node>>) {
    let mut nodes = graph.node_indices().collect::<Vec<_>>();
    nodes.sort_unstable_by_key(|node| G::node_slot(*node));
    let mut by_slot = vec![None; graph.node_bound()];
    for &node in &nodes {
        by_slot[G::node_slot(node)] = Some(node);
    }
    (nodes, by_slot)
}

pub(super) const fn cell(bound: usize, source: usize, target: usize) -> usize {
    source * bound + target
}
