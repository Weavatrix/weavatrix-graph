use super::measure::Measure;
use super::{WeightedPath, topological_sort_filtered};
use crate::{GraphError, IndexGraphView, Result, String, Vec};
use core::cmp::Ordering;

/// Returns a deterministic longest path in a directed acyclic graph.
///
/// Every accepted edge has unit cost. An empty graph returns `None`.
///
/// # Errors
///
/// Returns an error when the graph is cyclic or the length overflows.
pub fn dag_longest_path<G>(graph: &G) -> Result<Option<WeightedPath<G::Node>>>
where
    G: IndexGraphView,
{
    dag_weighted_longest_path(graph, |_| Some(1_u64))
}

/// Returns a deterministic longest path over accepted DAG edges.
///
/// # Errors
///
/// Returns an error when the accepted subgraph is cyclic or the length
/// overflows.
pub fn dag_longest_path_filtered<G, F>(
    graph: &G,
    mut allows_edge: F,
) -> Result<Option<WeightedPath<G::Node>>>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    dag_weighted_longest_path(graph, |edge| allows_edge(edge).then_some(1_u64))
}

/// Returns the number of edges in a deterministic longest DAG path.
///
/// # Errors
///
/// Returns an error when the graph is cyclic or the length overflows.
pub fn dag_longest_path_length<G>(graph: &G) -> Result<Option<u64>>
where
    G: IndexGraphView,
{
    Ok(dag_longest_path(graph)?.map(|path| path.total_cost()))
}

/// Returns the number of accepted edges in a deterministic longest DAG path.
///
/// # Errors
///
/// Returns the same errors as [`dag_longest_path_filtered`].
pub fn dag_longest_path_length_filtered<G, F>(graph: &G, allows_edge: F) -> Result<Option<u64>>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    Ok(dag_longest_path_filtered(graph, allows_edge)?.map(|path| path.total_cost()))
}

/// Returns a deterministic maximum-cost path in a directed acyclic graph.
///
/// Returning `None` from `edge_cost` excludes that edge. Costs may be negative;
/// a single node is the zero-cost result when every edge would reduce the cost.
///
/// # Errors
///
/// Returns an error for a cycle, a non-finite cost, or arithmetic overflow.
pub fn dag_weighted_longest_path<G, Cost, F>(
    graph: &G,
    mut edge_cost: F,
) -> Result<Option<WeightedPath<G::Node, Cost>>>
where
    G: IndexGraphView,
    Cost: Measure,
    F: FnMut(G::Edge) -> Option<Cost>,
{
    let weights = snapshot_weights(graph, &mut edge_cost)?;
    let order = topological_sort_filtered(graph, |edge| weights[G::edge_slot(edge)].is_some())
        .ok_or(GraphError::CyclicGraph {
            algorithm: "DAG longest path",
        })?;
    let Some(&first) = order.first() else {
        return Ok(None);
    };
    let mut distances = vec![None; graph.node_bound()];
    let mut predecessors = vec![None; graph.node_bound()];
    for &node in &order {
        distances[G::node_slot(node)] = Some(Cost::zero());
    }
    for &source in &order {
        let Some(source_cost) = distances[G::node_slot(source)] else {
            continue;
        };
        for edge in graph.outgoing_edges(source) {
            let Some(weight) = weights[G::edge_slot(edge)] else {
                continue;
            };
            let Some(endpoints) = graph.edge_endpoints(edge) else {
                continue;
            };
            let candidate =
                source_cost
                    .checked_add(weight)
                    .ok_or(GraphError::ArithmeticOverflow {
                        operation: "DAG longest path",
                    })?;
            let target_slot = G::node_slot(endpoints.target());
            let current = distances[target_slot].unwrap_or_else(Cost::zero);
            if candidate.compare(current) == Some(Ordering::Greater) {
                distances[target_slot] = Some(candidate);
                predecessors[target_slot] = Some(source);
            }
        }
    }
    let end = order.into_iter().skip(1).fold(first, |best, candidate| {
        let best_cost = distances[G::node_slot(best)].unwrap_or_else(Cost::zero);
        let candidate_cost = distances[G::node_slot(candidate)].unwrap_or_else(Cost::zero);
        if candidate_cost.compare(best_cost) == Some(Ordering::Greater) {
            candidate
        } else {
            best
        }
    });
    let total_cost = distances[G::node_slot(end)].unwrap_or_else(Cost::zero);
    Ok(Some(WeightedPath::from_parts(
        reconstruct::<G>(end, &predecessors),
        total_cost,
    )))
}

/// Returns only the maximum path cost.
///
/// # Errors
///
/// Returns the same errors as [`dag_weighted_longest_path`].
pub fn dag_weighted_longest_path_length<G, Cost, F>(graph: &G, edge_cost: F) -> Result<Option<Cost>>
where
    G: IndexGraphView,
    Cost: Measure,
    F: FnMut(G::Edge) -> Option<Cost>,
{
    Ok(dag_weighted_longest_path(graph, edge_cost)?.map(|path| path.total_cost()))
}

fn snapshot_weights<G, Cost, F>(graph: &G, edge_cost: &mut F) -> Result<Vec<Option<Cost>>>
where
    G: IndexGraphView,
    Cost: Measure,
    F: FnMut(G::Edge) -> Option<Cost>,
{
    let mut weights = vec![None; graph.edge_bound()];
    for edge in graph.edge_indices() {
        let Some(weight) = edge_cost(edge) else {
            continue;
        };
        if !weight.is_valid() {
            return Err(GraphError::InvalidAlgorithmParameter {
                algorithm: "DAG longest path",
                parameter: "edge_cost",
                value: String::from("must be finite"),
            });
        }
        weights[G::edge_slot(edge)] = Some(weight);
    }
    Ok(weights)
}

fn reconstruct<G>(mut node: G::Node, predecessors: &[Option<G::Node>]) -> Vec<G::Node>
where
    G: IndexGraphView,
{
    let mut path = vec![node];
    while let Some(parent) = predecessors[G::node_slot(node)] {
        node = parent;
        path.push(node);
    }
    path.reverse();
    path
}
