use super::cut::{indexed_nodes, residual_reachable};
use crate::Vec;
use crate::{GraphError, IndexGraphView, Result};
use alloc::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaxFlow<Node, Edge> {
    value: u64,
    edge_flows: Vec<(Edge, u64)>,
    source_side: Vec<Node>,
}

impl<Node, Edge> MaxFlow<Node, Edge> {
    pub(super) fn from_parts(
        value: u64,
        edge_flows: Vec<(Edge, u64)>,
        source_side: Vec<Node>,
    ) -> Self {
        Self {
            value,
            edge_flows,
            source_side,
        }
    }

    #[must_use]
    pub const fn value(&self) -> u64 {
        self.value
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

/// Computes a directed maximum flow with Dinic's blocking-flow algorithm.
///
/// The returned source-side residual partition also describes a minimum cut.
///
/// # Errors
///
/// Returns an error if the total flow is larger than `u64::MAX`.
pub fn maximum_flow<G, F>(
    graph: &G,
    source: G::Node,
    sink: G::Node,
    mut edge_capacity: F,
) -> Result<Option<MaxFlow<G::Node, G::Edge>>>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> u64,
{
    if !graph.contains_node(source) || !graph.contains_node(sink) {
        return Ok(None);
    }
    let mut capacities = vec![0_u64; graph.edge_bound()];
    let mut edge_order = Vec::with_capacity(graph.edge_count());
    for edge in graph.edge_indices() {
        capacities[G::edge_slot(edge)] = edge_capacity(edge);
        edge_order.push(edge);
    }
    if source == sink {
        return Ok(Some(MaxFlow {
            value: 0,
            edge_flows: edge_order.into_iter().map(|edge| (edge, 0)).collect(),
            source_side: vec![source],
        }));
    }

    let mut flows = vec![0_u64; graph.edge_bound()];
    let mut levels = vec![-1_i32; graph.node_bound()];
    let mut level_edges = vec![Vec::new(); graph.node_bound()];
    let mut total = 0_u64;
    while build_levels(
        graph,
        source,
        sink,
        &capacities,
        &flows,
        &mut levels,
        &mut level_edges,
    ) {
        let mut next = vec![0_usize; graph.node_bound()];
        loop {
            let pushed = augment_level_path::<G>(
                G::node_slot(source),
                G::node_slot(sink),
                &level_edges,
                &mut next,
                &capacities,
                &mut flows,
            );
            if pushed == 0 {
                break;
            }
            total = total
                .checked_add(pushed)
                .ok_or(GraphError::ArithmeticOverflow {
                    operation: "maximum flow",
                })?;
        }
    }

    let edge_flows = edge_order
        .into_iter()
        .map(|edge| (edge, flows[G::edge_slot(edge)]))
        .collect();
    let reachable = residual_reachable(graph, source, &capacities, &flows);
    let source_side = graph
        .node_indices()
        .filter(|node| reachable[G::node_slot(*node)])
        .collect();
    Ok(Some(MaxFlow {
        value: total,
        edge_flows,
        source_side,
    }))
}

#[derive(Clone, Copy)]
struct ResidualEdge<Edge> {
    edge: Edge,
    target: usize,
    forward: bool,
}

fn build_levels<G>(
    graph: &G,
    source: G::Node,
    sink: G::Node,
    capacities: &[u64],
    flows: &[u64],
    levels: &mut [i32],
    level_edges: &mut [Vec<ResidualEdge<G::Edge>>],
) -> bool
where
    G: IndexGraphView,
{
    levels.fill(-1);
    for edges in &mut *level_edges {
        edges.clear();
    }
    let source = G::node_slot(source);
    let sink = G::node_slot(sink);
    levels[source] = 0;
    let mut queue = VecDeque::from([source]);
    let nodes = indexed_nodes(graph);
    while let Some(node) = queue.pop_front() {
        let Some(node_key) = nodes[node] else {
            continue;
        };
        for edge in graph.outgoing_edges(node_key) {
            let slot = G::edge_slot(edge);
            if capacities[slot] > flows[slot] {
                let target = graph
                    .edge_endpoints(edge)
                    .map(|endpoints| G::node_slot(endpoints.target()));
                add_level_edge(edge, target, true, node, levels, level_edges, &mut queue);
            }
        }
        for edge in graph.incoming_edges(node_key) {
            let slot = G::edge_slot(edge);
            if flows[slot] > 0 {
                let target = graph
                    .edge_endpoints(edge)
                    .map(|endpoints| G::node_slot(endpoints.source()));
                add_level_edge(edge, target, false, node, levels, level_edges, &mut queue);
            }
        }
    }
    levels[sink] >= 0
}

fn add_level_edge<Edge: Copy>(
    edge: Edge,
    target: Option<usize>,
    forward: bool,
    source: usize,
    levels: &mut [i32],
    level_edges: &mut [Vec<ResidualEdge<Edge>>],
    queue: &mut VecDeque<usize>,
) {
    let Some(target) = target else {
        return;
    };
    if levels[target] < 0 {
        levels[target] = levels[source] + 1;
        queue.push_back(target);
    }
    if levels[target] == levels[source] + 1 {
        level_edges[source].push(ResidualEdge {
            edge,
            target,
            forward,
        });
    }
}

fn augment_level_path<G>(
    source: usize,
    sink: usize,
    level_edges: &[Vec<ResidualEdge<G::Edge>>],
    next: &mut [usize],
    capacities: &[u64],
    flows: &mut [u64],
) -> u64
where
    G: IndexGraphView,
{
    let mut nodes = vec![source];
    let mut path: Vec<ResidualEdge<G::Edge>> = Vec::new();
    loop {
        let node = *nodes.last().unwrap_or(&source);
        if node == sink {
            let amount = path
                .iter()
                .map(|step| residual::<G>(*step, capacities, flows))
                .min()
                .unwrap_or(0);
            for step in &path {
                let slot = G::edge_slot(step.edge);
                if step.forward {
                    flows[slot] += amount;
                } else {
                    flows[slot] -= amount;
                }
            }
            return amount;
        }
        let Some(&step) = level_edges[node].get(next[node]) else {
            if node == source {
                return 0;
            }
            nodes.pop();
            path.pop();
            let parent = *nodes.last().unwrap_or(&source);
            next[parent] += 1;
            continue;
        };
        if residual::<G>(step, capacities, flows) > 0 {
            nodes.push(step.target);
            path.push(step);
        } else {
            next[node] += 1;
        }
    }
}

fn residual<G>(step: ResidualEdge<G::Edge>, capacities: &[u64], flows: &[u64]) -> u64
where
    G: IndexGraphView,
{
    let slot = G::edge_slot(step.edge);
    if step.forward {
        capacities[slot] - flows[slot]
    } else {
        flows[slot]
    }
}
