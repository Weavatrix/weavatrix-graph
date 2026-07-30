use super::result::{MinCostFlow, finish};
use super::search::shortest_residual_path;
use super::{Predecessors, Step};
use crate::algo::flow::common::prepare;
use crate::algo::flow::cut::indexed_nodes;
use crate::{GraphError, IndexGraphView, Result, String};

/// Computes a maximum flow of minimum cost with successive shortest paths.
///
/// # Errors
///
/// Returns an error for arithmetic overflow or a reachable negative residual
/// cycle.
pub fn min_cost_max_flow<G, Capacity, Cost>(
    graph: &G,
    source: G::Node,
    sink: G::Node,
    edge_capacity: Capacity,
    edge_cost: Cost,
) -> Result<Option<MinCostFlow<G::Node, G::Edge>>>
where
    G: IndexGraphView,
    Capacity: FnMut(G::Edge) -> u64,
    Cost: Fn(G::Edge) -> i64,
{
    if !graph.contains_node(source) || !graph.contains_node(sink) {
        return Ok(None);
    }
    let input = prepare(graph, edge_capacity);
    let mut costs = vec![0_i64; graph.edge_bound()];
    for &edge in &input.edges {
        costs[G::edge_slot(edge)] = edge_cost(edge);
    }
    let mut flows = vec![0_u64; graph.edge_bound()];
    if source == sink {
        return Ok(Some(finish(graph, source, input, &flows, 0, 0)));
    }
    let nodes = indexed_nodes(graph);
    let source_slot = G::node_slot(source);
    let sink_slot = G::node_slot(sink);
    let mut value = 0_u64;
    let mut total_cost = 0_i128;
    while let Some(predecessors) = shortest_residual_path(
        graph,
        &nodes,
        source_slot,
        sink_slot,
        &input.capacities,
        &costs,
        &flows,
    )? {
        let amount = bottleneck::<G>(
            source_slot,
            sink_slot,
            &predecessors,
            &input.capacities,
            &flows,
        )?;
        let path_cost = apply::<G>(
            source_slot,
            sink_slot,
            amount,
            &predecessors,
            &costs,
            &mut flows,
        )?;
        value = value
            .checked_add(amount)
            .ok_or(GraphError::ArithmeticOverflow {
                operation: "minimum-cost maximum flow value",
            })?;
        total_cost = total_cost
            .checked_add(i128::from(amount).checked_mul(path_cost).ok_or(
                GraphError::ArithmeticOverflow {
                    operation: "minimum-cost path multiplication",
                },
            )?)
            .ok_or(GraphError::ArithmeticOverflow {
                operation: "minimum-cost maximum flow cost",
            })?;
    }
    Ok(Some(finish(
        graph, source, input, &flows, value, total_cost,
    )))
}

fn bottleneck<G>(
    source: usize,
    mut cursor: usize,
    predecessors: &Predecessors<G::Edge>,
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
    predecessors: &Predecessors<G::Edge>,
    costs: &[i64],
    flows: &mut [u64],
) -> Result<i128>
where
    G: IndexGraphView,
{
    let mut cost = 0_i128;
    let mut remaining = predecessors.len();
    while cursor != source {
        if remaining == 0 {
            return Err(invalid_residual_path());
        }
        remaining -= 1;
        let Step {
            previous,
            edge,
            forward,
        } = predecessors
            .get(cursor)
            .copied()
            .flatten()
            .ok_or_else(invalid_residual_path)?;
        let slot = G::edge_slot(edge);
        let edge_cost = *costs.get(slot).ok_or_else(invalid_residual_path)?;
        let flow = flows.get_mut(slot).ok_or_else(invalid_residual_path)?;
        cost = if forward {
            *flow = flow.checked_add(amount).ok_or_else(invalid_residual_path)?;
            cost.checked_add(i128::from(edge_cost))
        } else {
            *flow = flow.checked_sub(amount).ok_or_else(invalid_residual_path)?;
            cost.checked_sub(i128::from(edge_cost))
        }
        .ok_or(GraphError::ArithmeticOverflow {
            operation: "minimum-cost path sum",
        })?;
        cursor = previous;
    }
    Ok(cost)
}

fn invalid_residual_path() -> GraphError {
    GraphError::InvalidAlgorithmParameter {
        algorithm: "minimum-cost maximum flow",
        parameter: "residual predecessor path",
        value: String::from("path is incomplete or inconsistent"),
    }
}
