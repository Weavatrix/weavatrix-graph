use super::MaxFlow;
use super::cut::residual_reachable;
use crate::{IndexGraphView, Vec};

pub(super) struct FlowInput<Edge> {
    pub(super) capacities: Vec<u64>,
    pub(super) edges: Vec<Edge>,
}

pub(super) fn prepare<G, F>(graph: &G, mut edge_capacity: F) -> FlowInput<G::Edge>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> u64,
{
    let mut capacities = vec![0; graph.edge_bound()];
    let mut edges = Vec::with_capacity(graph.edge_count());
    for edge in graph.edge_indices() {
        capacities[G::edge_slot(edge)] = edge_capacity(edge);
        edges.push(edge);
    }
    FlowInput { capacities, edges }
}

pub(super) fn finish<G>(
    graph: &G,
    source: G::Node,
    value: u64,
    input: FlowInput<G::Edge>,
    flows: &[u64],
) -> MaxFlow<G::Node, G::Edge>
where
    G: IndexGraphView,
{
    let reachable = residual_reachable(graph, source, &input.capacities, flows);
    let source_side = graph
        .node_indices()
        .filter(|node| reachable[G::node_slot(*node)])
        .collect();
    let edge_flows = input
        .edges
        .into_iter()
        .map(|edge| (edge, flows[G::edge_slot(edge)]))
        .collect();
    MaxFlow::from_parts(value, edge_flows, source_side)
}

pub(super) fn zero<G>(
    graph: &G,
    source: G::Node,
    input: FlowInput<G::Edge>,
) -> MaxFlow<G::Node, G::Edge>
where
    G: IndexGraphView,
{
    let flows = vec![0; graph.edge_bound()];
    finish(graph, source, 0, input, &flows)
}
