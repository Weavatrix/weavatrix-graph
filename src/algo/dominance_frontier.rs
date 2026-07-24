use super::{Dominators, dominators_filtered};
use crate::{IndexGraphView, Vec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DominanceFrontiers<Node> {
    root: Node,
    entries: Vec<(Node, Vec<Node>)>,
}

impl<Node> DominanceFrontiers<Node>
where
    Node: Copy + Eq,
{
    #[must_use]
    pub const fn root(&self) -> Node {
        self.root
    }

    #[must_use]
    pub fn frontier(&self, node: Node) -> Option<&[Node]> {
        self.entries
            .iter()
            .find_map(|(candidate, frontier)| (*candidate == node).then_some(frontier.as_slice()))
    }

    pub fn iter(&self) -> impl Iterator<Item = (Node, &[Node])> {
        self.entries
            .iter()
            .map(|(node, frontier)| (*node, frontier.as_slice()))
    }
}

#[must_use]
pub fn dominance_frontiers<G>(graph: &G, root: G::Node) -> Option<DominanceFrontiers<G::Node>>
where
    G: IndexGraphView,
{
    dominance_frontiers_filtered(graph, root, |_| true)
}

#[must_use]
pub fn dominance_frontiers_filtered<G, F>(
    graph: &G,
    root: G::Node,
    allows_edge: F,
) -> Option<DominanceFrontiers<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    let mut allowed = vec![false; graph.edge_bound()];
    for edge in graph.edge_indices() {
        allowed[G::edge_slot(edge)] = allows_edge(edge);
    }
    let dominators = dominators_filtered(graph, root, |edge| allowed[G::edge_slot(edge)])?;
    let predecessors = reachable_predecessors(graph, &dominators, &allowed);
    let mut immediate = vec![None; graph.node_bound()];
    immediate[G::node_slot(root)] = Some(root);
    for (node, parent) in dominators.immediate_dominators() {
        immediate[G::node_slot(node)] = Some(parent);
    }
    let mut frontiers = vec![Vec::new(); graph.node_bound()];
    for &join in dominators.reachable_nodes() {
        let join_slot = G::node_slot(join);
        if predecessors[join_slot].len() < 2 {
            continue;
        }
        let stop = immediate[join_slot]?;
        for &predecessor in &predecessors[join_slot] {
            propagate::<G>(predecessor, stop, join, &immediate, &mut frontiers);
        }
    }
    let entries = dominators
        .reachable_nodes()
        .iter()
        .copied()
        .map(|node| {
            let frontier = &mut frontiers[G::node_slot(node)];
            frontier.sort_unstable_by_key(|candidate| G::node_slot(*candidate));
            frontier.dedup();
            (node, core::mem::take(frontier))
        })
        .collect();
    Some(DominanceFrontiers { root, entries })
}

fn reachable_predecessors<G>(
    graph: &G,
    dominators: &Dominators<G::Node>,
    allowed: &[bool],
) -> Vec<Vec<G::Node>>
where
    G: IndexGraphView,
{
    let mut reachable = vec![false; graph.node_bound()];
    for &node in dominators.reachable_nodes() {
        reachable[G::node_slot(node)] = true;
    }
    let mut predecessors = vec![Vec::new(); graph.node_bound()];
    for (edge, endpoints) in graph.edge_references() {
        if !allowed[G::edge_slot(edge)] {
            continue;
        }
        let source_slot = G::node_slot(endpoints.source());
        let target_slot = G::node_slot(endpoints.target());
        if !reachable[source_slot] || !reachable[target_slot] {
            continue;
        }
        let incoming = &mut predecessors[target_slot];
        if !incoming.contains(&endpoints.source()) {
            incoming.push(endpoints.source());
        }
    }
    predecessors
}

fn propagate<G>(
    mut runner: G::Node,
    stop: G::Node,
    join: G::Node,
    immediate: &[Option<G::Node>],
    frontiers: &mut [Vec<G::Node>],
) where
    G: IndexGraphView,
{
    while runner != stop {
        let slot = G::node_slot(runner);
        frontiers[slot].push(join);
        let Some(parent) = immediate[slot] else {
            break;
        };
        if parent == runner {
            break;
        }
        runner = parent;
    }
}
