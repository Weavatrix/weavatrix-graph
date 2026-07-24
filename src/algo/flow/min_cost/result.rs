use super::super::common::FlowInput;
use super::super::cut::residual_reachable;
use crate::{IndexGraphView, Vec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinCostFlow<Node, Edge> {
    value: u64,
    cost: i128,
    edge_flows: Vec<(Edge, u64)>,
    source_side: Vec<Node>,
}

impl<Node, Edge> MinCostFlow<Node, Edge> {
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.value
    }

    #[must_use]
    pub const fn cost(&self) -> i128 {
        self.cost
    }

    #[must_use]
    pub fn edge_flows(&self) -> &[(Edge, u64)] {
        &self.edge_flows
    }

    #[must_use]
    pub fn source_side(&self) -> &[Node] {
        &self.source_side
    }
}

pub(super) fn finish<G>(
    graph: &G,
    source: G::Node,
    input: FlowInput<G::Edge>,
    flows: &[u64],
    value: u64,
    cost: i128,
) -> MinCostFlow<G::Node, G::Edge>
where
    G: IndexGraphView,
{
    let reachable = residual_reachable(graph, source, &input.capacities, flows);
    MinCostFlow {
        value,
        cost,
        edge_flows: input
            .edges
            .into_iter()
            .map(|edge| (edge, flows[G::edge_slot(edge)]))
            .collect(),
        source_side: graph
            .node_indices()
            .filter(|node| reachable[G::node_slot(*node)])
            .collect(),
    }
}
