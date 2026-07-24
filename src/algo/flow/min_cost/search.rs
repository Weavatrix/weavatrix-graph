use super::{Predecessors, Step};
use crate::{GraphError, IndexGraphView, Result};

#[allow(clippy::too_many_arguments)]
pub(super) fn shortest_residual_path<G>(
    graph: &G,
    nodes: &[Option<G::Node>],
    source: usize,
    sink: usize,
    capacities: &[u64],
    costs: &[i64],
    flows: &[u64],
) -> Result<Option<Predecessors<G::Edge>>>
where
    G: IndexGraphView,
{
    let mut distances = vec![None; graph.node_bound()];
    let mut predecessors = vec![None; graph.node_bound()];
    distances[source] = Some(0_i128);
    for _ in 1..graph.node_count() {
        if !relax_residual(
            graph,
            nodes,
            capacities,
            costs,
            flows,
            &mut distances,
            &mut predecessors,
        )? {
            break;
        }
    }
    let mut probe = predecessors.clone();
    if relax_residual(
        graph,
        nodes,
        capacities,
        costs,
        flows,
        &mut distances,
        &mut probe,
    )? {
        return Err(GraphError::NegativeCycle {
            algorithm: "minimum-cost maximum flow",
        });
    }
    Ok(distances[sink].map(|_| predecessors))
}

#[allow(clippy::too_many_arguments)]
fn relax_residual<G>(
    graph: &G,
    nodes: &[Option<G::Node>],
    capacities: &[u64],
    costs: &[i64],
    flows: &[u64],
    distances: &mut [Option<i128>],
    predecessors: &mut Predecessors<G::Edge>,
) -> Result<bool>
where
    G: IndexGraphView,
{
    let mut changed = false;
    for source in 0..nodes.len() {
        let Some(distance) = distances[source] else {
            continue;
        };
        let Some(node) = nodes[source] else {
            continue;
        };
        for edge in graph.outgoing_edges(node) {
            let slot = G::edge_slot(edge);
            if capacities[slot] > flows[slot] {
                let target = graph
                    .edge_endpoints(edge)
                    .map_or(source, |ends| G::node_slot(ends.target()));
                changed |= relax(
                    distance,
                    i128::from(costs[slot]),
                    source,
                    target,
                    edge,
                    true,
                    distances,
                    predecessors,
                )?;
            }
        }
        for edge in graph.incoming_edges(node) {
            let slot = G::edge_slot(edge);
            if flows[slot] > 0 {
                let target = graph
                    .edge_endpoints(edge)
                    .map_or(source, |ends| G::node_slot(ends.source()));
                changed |= relax(
                    distance,
                    -i128::from(costs[slot]),
                    source,
                    target,
                    edge,
                    false,
                    distances,
                    predecessors,
                )?;
            }
        }
    }
    Ok(changed)
}

#[allow(clippy::too_many_arguments)]
fn relax<Edge: Copy>(
    distance: i128,
    cost: i128,
    source: usize,
    target: usize,
    edge: Edge,
    forward: bool,
    distances: &mut [Option<i128>],
    predecessors: &mut Predecessors<Edge>,
) -> Result<bool> {
    let candidate = distance
        .checked_add(cost)
        .ok_or(GraphError::ArithmeticOverflow {
            operation: "minimum-cost residual relaxation",
        })?;
    if distances[target].is_some_and(|known| candidate >= known) {
        return Ok(false);
    }
    distances[target] = Some(candidate);
    predecessors[target] = Some(Step {
        previous: source,
        edge,
        forward,
    });
    Ok(true)
}
