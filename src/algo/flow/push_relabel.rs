use super::MaxFlow;
use super::common::{finish, prepare, zero};
use super::cut::indexed_nodes;
use crate::{GraphError, IndexGraphView, Result};
use alloc::collections::VecDeque;

/// Computes maximum flow with a deterministic FIFO push-relabel algorithm.
///
/// # Errors
///
/// Returns an error when the maximum flow exceeds `u64::MAX`.
pub fn push_relabel<G, F>(
    graph: &G,
    source: G::Node,
    sink: G::Node,
    edge_capacity: F,
) -> Result<Option<MaxFlow<G::Node, G::Edge>>>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> u64,
{
    if !graph.contains_node(source) || !graph.contains_node(sink) {
        return Ok(None);
    }
    let input = prepare(graph, edge_capacity);
    if source == sink {
        return Ok(Some(zero(graph, source, input)));
    }
    let nodes = indexed_nodes(graph);
    let source_slot = G::node_slot(source);
    let sink_slot = G::node_slot(sink);
    let mut flows = vec![0_u64; graph.edge_bound()];
    let mut excess = vec![0_u128; graph.node_bound()];
    let mut heights = vec![0_usize; graph.node_bound()];
    let mut active = vec![false; graph.node_bound()];
    let mut queue = VecDeque::new();
    heights[source_slot] = graph.node_count();
    if let Some(source_node) = nodes[source_slot] {
        for edge in graph.outgoing_edges(source_node) {
            let slot = G::edge_slot(edge);
            let amount = input.capacities[slot];
            let Some(target) = graph
                .edge_endpoints(edge)
                .map(|ends| G::node_slot(ends.target()))
            else {
                continue;
            };
            flows[slot] = amount;
            excess[target] += u128::from(amount);
            activate(
                target,
                source_slot,
                sink_slot,
                &excess,
                &mut active,
                &mut queue,
            );
        }
    }
    while let Some(node) = queue.pop_front() {
        active[node] = false;
        discharge::<G>(
            graph,
            &nodes,
            node,
            source_slot,
            sink_slot,
            &input.capacities,
            &mut flows,
            &mut excess,
            &mut heights,
            &mut active,
            &mut queue,
        );
        activate(
            node,
            source_slot,
            sink_slot,
            &excess,
            &mut active,
            &mut queue,
        );
    }
    let value = u64::try_from(excess[sink_slot]).map_err(|_| GraphError::ArithmeticOverflow {
        operation: "push-relabel maximum flow",
    })?;
    Ok(Some(finish(graph, source, value, input, &flows)))
}

#[allow(clippy::too_many_arguments)]
fn discharge<G>(
    graph: &G,
    nodes: &[Option<G::Node>],
    node: usize,
    source: usize,
    sink: usize,
    capacities: &[u64],
    flows: &mut [u64],
    excess: &mut [u128],
    heights: &mut [usize],
    active: &mut [bool],
    queue: &mut VecDeque<usize>,
) where
    G: IndexGraphView,
{
    while excess[node] > 0 {
        let Some(node_key) = nodes[node] else {
            break;
        };
        let mut pushed = false;
        for edge in graph.outgoing_edges(node_key) {
            let slot = G::edge_slot(edge);
            let Some(target) = graph
                .edge_endpoints(edge)
                .map(|ends| G::node_slot(ends.target()))
            else {
                continue;
            };
            let residual = capacities[slot] - flows[slot];
            if residual > 0 && heights[node] == heights[target] + 1 {
                let amount = residual.min(u64::try_from(excess[node]).unwrap_or(u64::MAX));
                flows[slot] += amount;
                transfer(node, target, amount, source, sink, excess, active, queue);
                pushed = true;
                if excess[node] == 0 {
                    break;
                }
            }
        }
        if excess[node] == 0 {
            break;
        }
        for edge in graph.incoming_edges(node_key) {
            let slot = G::edge_slot(edge);
            let Some(target) = graph
                .edge_endpoints(edge)
                .map(|ends| G::node_slot(ends.source()))
            else {
                continue;
            };
            if flows[slot] > 0 && heights[node] == heights[target] + 1 {
                let amount = flows[slot].min(u64::try_from(excess[node]).unwrap_or(u64::MAX));
                flows[slot] -= amount;
                transfer(node, target, amount, source, sink, excess, active, queue);
                pushed = true;
                if excess[node] == 0 {
                    break;
                }
            }
        }
        if excess[node] > 0 && !pushed {
            heights[node] =
                minimum_residual_height::<G>(graph, node_key, capacities, flows, heights)
                    .map_or(graph.node_bound().saturating_mul(2), |height| height + 1);
        }
    }
}

fn minimum_residual_height<G>(
    graph: &G,
    node: G::Node,
    capacities: &[u64],
    flows: &[u64],
    heights: &[usize],
) -> Option<usize>
where
    G: IndexGraphView,
{
    let outgoing = graph.outgoing_edges(node).filter_map(|edge| {
        let slot = G::edge_slot(edge);
        (capacities[slot] > flows[slot])
            .then(|| {
                graph
                    .edge_endpoints(edge)
                    .map(|ends| heights[G::node_slot(ends.target())])
            })
            .flatten()
    });
    let incoming = graph.incoming_edges(node).filter_map(|edge| {
        (flows[G::edge_slot(edge)] > 0)
            .then(|| {
                graph
                    .edge_endpoints(edge)
                    .map(|ends| heights[G::node_slot(ends.source())])
            })
            .flatten()
    });
    outgoing.chain(incoming).min()
}

#[allow(clippy::too_many_arguments)]
fn transfer(
    source: usize,
    target: usize,
    amount: u64,
    flow_source: usize,
    sink: usize,
    excess: &mut [u128],
    active: &mut [bool],
    queue: &mut VecDeque<usize>,
) {
    excess[source] -= u128::from(amount);
    excess[target] += u128::from(amount);
    activate(target, flow_source, sink, excess, active, queue);
}

fn activate(
    node: usize,
    source: usize,
    sink: usize,
    excess: &[u128],
    active: &mut [bool],
    queue: &mut VecDeque<usize>,
) {
    if node != source && node != sink && excess[node] > 0 && !active[node] {
        active[node] = true;
        queue.push_back(node);
    }
}
