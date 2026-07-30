use super::MaxFlow;
use super::common::{finish, prepare, zero};
use super::cut::indexed_nodes;
use crate::{GraphError, IndexGraphView, Result, String, Vec};
use alloc::collections::VecDeque;

#[derive(Clone, Copy)]
struct Step<Edge> {
    previous: usize,
    edge: Edge,
    forward: bool,
}

/// Computes maximum flow with the Edmonds-Karp shortest augmenting-path rule.
///
/// # Errors
///
/// Returns an error when the total flow exceeds `u64::MAX`.
pub fn edmonds_karp<G, F>(
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
    let mut flows = vec![0; graph.edge_bound()];
    let mut total = 0_u64;
    while let Some(predecessors) = augmenting_path(
        graph,
        &nodes,
        source_slot,
        sink_slot,
        &input.capacities,
        &flows,
    ) {
        let amount = bottleneck::<G>(
            source_slot,
            sink_slot,
            &predecessors,
            &input.capacities,
            &flows,
        )?;
        apply::<G>(source_slot, sink_slot, amount, &predecessors, &mut flows)?;
        total = total
            .checked_add(amount)
            .ok_or(GraphError::ArithmeticOverflow {
                operation: "Edmonds-Karp maximum flow",
            })?;
    }
    Ok(Some(finish(graph, source, total, input, &flows)))
}

fn augmenting_path<G>(
    graph: &G,
    nodes: &[Option<G::Node>],
    source: usize,
    sink: usize,
    capacities: &[u64],
    flows: &[u64],
) -> Option<Vec<Option<Step<G::Edge>>>>
where
    G: IndexGraphView,
{
    let mut predecessors = vec![None; graph.node_bound()];
    let mut seen = vec![false; graph.node_bound()];
    let mut queue = VecDeque::from([source]);
    seen[source] = true;
    while let Some(slot) = queue.pop_front() {
        let node = nodes[slot]?;
        for edge in graph.outgoing_edges(node) {
            let edge_slot = G::edge_slot(edge);
            let target = graph
                .edge_endpoints(edge)
                .map(|ends| G::node_slot(ends.target()))?;
            if capacities[edge_slot] > flows[edge_slot]
                && discover(
                    slot,
                    target,
                    edge,
                    true,
                    &mut seen,
                    &mut predecessors,
                    &mut queue,
                )
                && target == sink
            {
                return Some(predecessors);
            }
        }
        for edge in graph.incoming_edges(node) {
            let edge_slot = G::edge_slot(edge);
            let target = graph
                .edge_endpoints(edge)
                .map(|ends| G::node_slot(ends.source()))?;
            if flows[edge_slot] > 0
                && discover(
                    slot,
                    target,
                    edge,
                    false,
                    &mut seen,
                    &mut predecessors,
                    &mut queue,
                )
                && target == sink
            {
                return Some(predecessors);
            }
        }
    }
    None
}

fn discover<Edge: Copy>(
    previous: usize,
    target: usize,
    edge: Edge,
    forward: bool,
    seen: &mut [bool],
    predecessors: &mut [Option<Step<Edge>>],
    queue: &mut VecDeque<usize>,
) -> bool {
    if seen[target] {
        return false;
    }
    seen[target] = true;
    predecessors[target] = Some(Step {
        previous,
        edge,
        forward,
    });
    queue.push_back(target);
    true
}

fn bottleneck<G>(
    source: usize,
    mut cursor: usize,
    predecessors: &[Option<Step<G::Edge>>],
    capacities: &[u64],
    flows: &[u64],
) -> Result<u64>
where
    G: IndexGraphView,
{
    let mut amount = u64::MAX;
    let mut remaining = predecessors.len();
    while cursor != source {
        if remaining == 0 {
            return Err(invalid_residual_path());
        }
        remaining -= 1;
        let step = predecessors
            .get(cursor)
            .copied()
            .flatten()
            .ok_or_else(invalid_residual_path)?;
        let slot = G::edge_slot(step.edge);
        amount = amount.min(if step.forward {
            capacities
                .get(slot)
                .zip(flows.get(slot))
                .and_then(|(capacity, flow)| capacity.checked_sub(*flow))
                .ok_or_else(invalid_residual_path)?
        } else {
            *flows.get(slot).ok_or_else(invalid_residual_path)?
        });
        cursor = step.previous;
    }
    Ok(amount)
}

fn apply<G>(
    source: usize,
    mut cursor: usize,
    amount: u64,
    predecessors: &[Option<Step<G::Edge>>],
    flows: &mut [u64],
) -> Result<()>
where
    G: IndexGraphView,
{
    let mut remaining = predecessors.len();
    while cursor != source {
        if remaining == 0 {
            return Err(invalid_residual_path());
        }
        remaining -= 1;
        let step = predecessors
            .get(cursor)
            .copied()
            .flatten()
            .ok_or_else(invalid_residual_path)?;
        let slot = G::edge_slot(step.edge);
        let flow = flows.get_mut(slot).ok_or_else(invalid_residual_path)?;
        if step.forward {
            *flow = flow.checked_add(amount).ok_or_else(invalid_residual_path)?;
        } else {
            *flow = flow.checked_sub(amount).ok_or_else(invalid_residual_path)?;
        }
        cursor = step.previous;
    }
    Ok(())
}

fn invalid_residual_path() -> GraphError {
    GraphError::InvalidAlgorithmParameter {
        algorithm: "Edmonds-Karp",
        parameter: "residual predecessor path",
        value: String::from("path is incomplete or inconsistent"),
    }
}
