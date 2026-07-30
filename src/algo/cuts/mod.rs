mod stoer_wagner;

pub use stoer_wagner::{StoerWagnerCut, stoer_wagner_min_cut, stoer_wagner_min_cut_filtered};

use crate::IndexUndirectedGraphView;
use crate::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndirectedCuts<Node, Edge> {
    bridges: Vec<Edge>,
    articulation_points: Vec<Node>,
}

impl<Node, Edge> UndirectedCuts<Node, Edge> {
    #[must_use]
    pub fn bridges(&self) -> &[Edge] {
        &self.bridges
    }

    #[must_use]
    pub fn articulation_points(&self) -> &[Node] {
        &self.articulation_points
    }
}

pub fn bridges_and_articulation_points<G>(graph: &G) -> UndirectedCuts<G::Node, G::Edge>
where
    G: IndexUndirectedGraphView,
{
    let mut state = CutState::<G>::new(graph);
    let mut nodes = graph.node_indices().collect::<Vec<_>>();
    nodes.sort_unstable_by_key(|node| G::node_slot(*node));
    for node in nodes {
        if state.discovery[G::node_slot(node)].is_none() {
            visit(graph, node, None, &mut state);
        }
    }
    state
        .bridges
        .sort_unstable_by_key(|edge| G::edge_slot(*edge));
    let mut articulation_points = graph
        .node_indices()
        .filter(|node| state.articulation[G::node_slot(*node)])
        .collect::<Vec<_>>();
    articulation_points.sort_unstable_by_key(|node| G::node_slot(*node));
    UndirectedCuts {
        bridges: state.bridges,
        articulation_points,
    }
}

struct CutState<G: IndexUndirectedGraphView> {
    time: usize,
    discovery: Vec<Option<usize>>,
    low: Vec<usize>,
    articulation: Vec<bool>,
    bridges: Vec<G::Edge>,
}

impl<G: IndexUndirectedGraphView> CutState<G> {
    fn new(graph: &G) -> Self {
        Self {
            time: 0,
            discovery: vec![None; graph.node_bound()],
            low: vec![0; graph.node_bound()],
            articulation: vec![false; graph.node_bound()],
            bridges: Vec::new(),
        }
    }
}

fn visit<G>(graph: &G, node: G::Node, parent_edge: Option<G::Edge>, state: &mut CutState<G>)
where
    G: IndexUndirectedGraphView,
{
    let slot = G::node_slot(node);
    state.discovery[slot] = Some(state.time);
    state.low[slot] = state.time;
    state.time += 1;
    let mut children = 0;
    let mut incident = graph.incident_edges(node).collect::<Vec<_>>();
    incident.sort_unstable_by_key(|edge| G::edge_slot(*edge));
    for edge in incident {
        if Some(edge) == parent_edge {
            continue;
        }
        let Some(neighbor) = graph.opposite(edge, node) else {
            continue;
        };
        let neighbor_slot = G::node_slot(neighbor);
        if let Some(discovery) = state.discovery[neighbor_slot] {
            state.low[slot] = state.low[slot].min(discovery);
            continue;
        }
        children += 1;
        visit(graph, neighbor, Some(edge), state);
        state.low[slot] = state.low[slot].min(state.low[neighbor_slot]);
        let node_discovery = state.discovery[slot].expect("visited node has discovery time");
        if state.low[neighbor_slot] > node_discovery {
            state.bridges.push(edge);
        }
        if parent_edge.is_some() && state.low[neighbor_slot] >= node_discovery {
            state.articulation[slot] = true;
        }
    }
    if parent_edge.is_none() && children > 1 {
        state.articulation[slot] = true;
    }
}
