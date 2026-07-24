use super::WeightedPath;
use crate::IndexGraphView;
use crate::Vec;
use alloc::collections::{BTreeSet, BinaryHeap};
use core::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathEnumeration<Node> {
    paths: Vec<Vec<Node>>,
    truncated: bool,
}
impl<Node> PathEnumeration<Node> {
    #[must_use]
    pub fn paths(&self) -> &[Vec<Node>] {
        &self.paths
    }
    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}
pub fn all_simple_paths<G>(
    graph: &G,
    source: G::Node,
    target: G::Node,
    max_depth: usize,
    max_paths: usize,
) -> PathEnumeration<G::Node>
where
    G: IndexGraphView,
{
    if max_paths == 0 || !graph.contains_node(source) || !graph.contains_node(target) {
        return PathEnumeration {
            paths: Vec::new(),
            truncated: false,
        };
    }
    let mut state = PathState::new(graph, max_paths);
    state.path.push(source);
    state.seen[G::node_slot(source)] = true;
    enumerate_paths(graph, source, target, max_depth, &mut state);
    PathEnumeration {
        paths: state.results,
        truncated: state.truncated,
    }
}
pub fn k_shortest_paths<G, F>(
    graph: &G,
    source: G::Node,
    target: G::Node,
    k: usize,
    edge_cost: F,
) -> Vec<WeightedPath<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> u64,
{
    if k == 0 || !graph.contains_node(source) || !graph.contains_node(target) {
        return Vec::new();
    }
    let source_slot = G::node_slot(source);
    let mut queue = BinaryHeap::new();
    queue.push(Candidate {
        cost: 0,
        slots: vec![source_slot],
        nodes: vec![source],
    });
    let mut queued = BTreeSet::from([vec![source_slot]]);
    let mut results = Vec::with_capacity(k);
    while let Some(candidate) = queue.pop() {
        if candidate.nodes.last() == Some(&target) {
            results.push(WeightedPath::from_parts(candidate.nodes, candidate.cost));
            if results.len() == k {
                break;
            }
            continue;
        }
        let Some(&node) = candidate.nodes.last() else {
            continue;
        };
        for (neighbor, weight) in weighted_outgoing(graph, node, &edge_cost) {
            let slot = G::node_slot(neighbor);
            if candidate.slots.contains(&slot) {
                continue;
            }
            let Some(cost) = candidate.cost.checked_add(weight) else {
                continue;
            };
            let mut slots = candidate.slots.clone();
            slots.push(slot);
            if !queued.insert(slots.clone()) {
                continue;
            }
            let mut nodes = candidate.nodes.clone();
            nodes.push(neighbor);
            queue.push(Candidate { cost, slots, nodes });
        }
    }
    results
}
struct PathState<Node> {
    path: Vec<Node>,
    seen: Vec<bool>,
    results: Vec<Vec<Node>>,
    limit: usize,
    truncated: bool,
}
impl<Node> PathState<Node> {
    fn new<G: IndexGraphView<Node = Node>>(graph: &G, limit: usize) -> Self {
        Self {
            path: Vec::new(),
            seen: vec![false; graph.node_bound()],
            results: Vec::new(),
            limit,
            truncated: false,
        }
    }
}

fn enumerate_paths<G>(
    graph: &G,
    node: G::Node,
    target: G::Node,
    depth_left: usize,
    state: &mut PathState<G::Node>,
) where
    G: IndexGraphView,
{
    if node == target {
        state.results.push(state.path.clone());
        return;
    }
    if depth_left == 0 {
        return;
    }
    for (_, neighbor) in outgoing(graph, node) {
        let slot = G::node_slot(neighbor);
        if state.seen[slot] {
            continue;
        }
        if state.results.len() == state.limit {
            state.truncated = true;
            return;
        }
        state.seen[slot] = true;
        state.path.push(neighbor);
        enumerate_paths(graph, neighbor, target, depth_left - 1, state);
        state.path.pop();
        state.seen[slot] = false;
        if state.truncated {
            return;
        }
    }
}

fn outgoing<G: IndexGraphView>(graph: &G, node: G::Node) -> Vec<(G::Edge, G::Node)> {
    let mut adjacent = graph
        .outgoing_edges(node)
        .filter_map(|edge| {
            graph
                .edge_endpoints(edge)
                .map(|endpoints| (edge, endpoints.target()))
        })
        .collect::<Vec<_>>();
    adjacent.sort_unstable_by_key(|(edge, target)| (G::node_slot(*target), G::edge_slot(*edge)));
    adjacent.dedup_by_key(|(_, target)| G::node_slot(*target));
    adjacent
}

fn weighted_outgoing<G, F>(graph: &G, node: G::Node, edge_cost: &F) -> Vec<(G::Node, u64)>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> u64,
{
    let mut adjacent = graph
        .outgoing_edges(node)
        .filter_map(|edge| {
            graph
                .edge_endpoints(edge)
                .map(|endpoints| (endpoints.target(), edge_cost(edge), G::edge_slot(edge)))
        })
        .collect::<Vec<_>>();
    adjacent.sort_unstable_by_key(|(target, cost, edge)| (G::node_slot(*target), *cost, *edge));
    adjacent.dedup_by_key(|(target, _, _)| G::node_slot(*target));
    adjacent
        .into_iter()
        .map(|(target, cost, _)| (target, cost))
        .collect()
}

struct Candidate<Node> {
    cost: u64,
    slots: Vec<usize>,
    nodes: Vec<Node>,
}

impl<Node> PartialEq for Candidate<Node> {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.slots == other.slots
    }
}

impl<Node> Eq for Candidate<Node> {}

impl<Node> PartialOrd for Candidate<Node> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Node> Ord for Candidate<Node> {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| other.slots.cmp(&self.slots))
    }
}
