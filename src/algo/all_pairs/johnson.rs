use super::{AllPairsShortestPaths, indexed_nodes};
use crate::Vec;
use crate::{GraphError, IndexGraphView, Result};
use alloc::collections::BinaryHeap;
use core::cmp::Reverse;
#[cfg(feature = "rayon")]
use rayon::prelude::*;

/// Computes sparse all-pairs signed shortest paths with Johnson's algorithm.
///
/// # Errors
///
/// Returns an error for arithmetic overflow or any negative cycle.
pub fn johnson_all_pairs<G, F>(graph: &G, edge_cost: F) -> Result<AllPairsShortestPaths<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> i64,
{
    johnson_all_pairs_filtered(graph, |edge| Some(edge_cost(edge)))
}

/// Computes Johnson all-pairs paths while omitting edges whose cost is `None`.
///
/// # Errors
///
/// Returns an error for arithmetic overflow or any selected negative cycle.
pub fn johnson_all_pairs_filtered<G, F>(
    graph: &G,
    edge_cost: F,
) -> Result<AllPairsShortestPaths<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> Option<i64>,
{
    let (nodes, by_slot) = indexed_nodes(graph);
    let edges = graph
        .edge_references()
        .filter_map(|(edge, endpoints)| {
            edge_cost(edge).map(|weight| {
                (
                    G::node_slot(endpoints.source()),
                    G::node_slot(endpoints.target()),
                    weight,
                )
            })
        })
        .collect::<Vec<_>>();
    let potentials = potentials(nodes.len(), &edges, graph.node_bound())?;
    let bound = graph.node_bound();
    let length = bound
        .checked_mul(bound)
        .ok_or(GraphError::ArithmeticOverflow {
            operation: "Johnson matrix allocation",
        })?;
    let mut distances = vec![0_i64; length];
    let mut reachable = vec![false; length];
    let mut next = vec![None; length];
    let mut adjacency = vec![Vec::new(); bound];
    for &(source, target, weight) in &edges {
        adjacency[source].push((target, weight));
    }
    for list in &mut adjacency {
        list.sort_unstable_by_key(|&(target, _)| target);
    }
    for &source_node in &nodes {
        let source = G::node_slot(source_node);
        let offset = source * bound;
        dijkstra_row(
            source,
            &adjacency,
            &potentials,
            &mut distances[offset..offset + bound],
            &mut reachable[offset..offset + bound],
            &mut next[offset..offset + bound],
        )?;
    }
    Ok(AllPairsShortestPaths {
        nodes,
        by_slot,
        distances,
        reachable,
        next,
        bound,
        node_slot: G::node_slot,
    })
}

/// Computes Johnson all-pairs paths with source rows distributed over Rayon.
///
/// # Errors
///
/// Returns an error for arithmetic overflow or any negative cycle.
#[cfg(feature = "rayon")]
pub fn johnson_all_pairs_parallel<G, F>(
    graph: &G,
    edge_cost: F,
) -> Result<AllPairsShortestPaths<G::Node>>
where
    G: IndexGraphView,
    G::Node: Send + Sync,
    F: Fn(G::Edge) -> i64,
{
    let (nodes, by_slot) = indexed_nodes(graph);
    let edges = graph
        .edge_references()
        .map(|(edge, endpoints)| {
            (
                G::node_slot(endpoints.source()),
                G::node_slot(endpoints.target()),
                edge_cost(edge),
            )
        })
        .collect::<Vec<_>>();
    let potentials = potentials(nodes.len(), &edges, graph.node_bound())?;
    let bound = graph.node_bound();
    let length = bound
        .checked_mul(bound)
        .ok_or(GraphError::ArithmeticOverflow {
            operation: "parallel Johnson matrix allocation",
        })?;
    let mut adjacency = vec![Vec::new(); bound];
    for &(source, target, weight) in &edges {
        adjacency[source].push((target, weight));
    }
    for list in &mut adjacency {
        list.sort_unstable_by_key(|&(target, _)| target);
    }
    let rows = nodes
        .par_iter()
        .map(|source_node| {
            let source = G::node_slot(*source_node);
            let mut distances = vec![0_i64; bound];
            let mut reachable = vec![false; bound];
            let mut next = vec![None; bound];
            dijkstra_row(
                source,
                &adjacency,
                &potentials,
                &mut distances,
                &mut reachable,
                &mut next,
            )?;
            Ok((source, distances, reachable, next))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut distances = vec![0_i64; length];
    let mut reachable = vec![false; length];
    let mut next = vec![None; length];
    for (source, row_distances, row_reachable, row_next) in rows {
        let offset = source * bound;
        distances[offset..offset + bound].copy_from_slice(&row_distances);
        reachable[offset..offset + bound].copy_from_slice(&row_reachable);
        next[offset..offset + bound].copy_from_slice(&row_next);
    }
    Ok(AllPairsShortestPaths {
        nodes,
        by_slot,
        distances,
        reachable,
        next,
        bound,
        node_slot: G::node_slot,
    })
}

fn potentials(node_count: usize, edges: &[(usize, usize, i64)], bound: usize) -> Result<Vec<i64>> {
    let mut values = vec![0_i64; bound];
    for _ in 1..node_count {
        let mut changed = false;
        for &(source, target, weight) in edges {
            let candidate =
                values[source]
                    .checked_add(weight)
                    .ok_or(GraphError::ArithmeticOverflow {
                        operation: "Johnson potential relaxation",
                    })?;
            if candidate < values[target] {
                values[target] = candidate;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for &(source, target, weight) in edges {
        let candidate =
            values[source]
                .checked_add(weight)
                .ok_or(GraphError::ArithmeticOverflow {
                    operation: "Johnson cycle check",
                })?;
        if candidate < values[target] {
            return Err(GraphError::NegativeCycle {
                algorithm: "Johnson",
            });
        }
    }
    Ok(values)
}

fn dijkstra_row(
    source: usize,
    adjacency: &[Vec<(usize, i64)>],
    potentials: &[i64],
    distances: &mut [i64],
    reachable: &mut [bool],
    next: &mut [Option<usize>],
) -> Result<()> {
    let bound = adjacency.len();
    let mut reduced = vec![i64::MAX; bound];
    let mut first = vec![None; bound];
    reduced[source] = 0;
    first[source] = Some(source);
    let mut queue = BinaryHeap::new();
    queue.push(Reverse((0_i64, source)));
    while let Some(Reverse((cost, node))) = queue.pop() {
        if cost != reduced[node] {
            continue;
        }
        for &(target, weight) in &adjacency[node] {
            let reweighted = weight
                .checked_add(potentials[node])
                .and_then(|value| value.checked_sub(potentials[target]))
                .ok_or(GraphError::ArithmeticOverflow {
                    operation: "Johnson edge reweighting",
                })?;
            let candidate = cost
                .checked_add(reweighted)
                .ok_or(GraphError::ArithmeticOverflow {
                    operation: "Johnson Dijkstra relaxation",
                })?;
            let candidate_first = if node == source {
                target
            } else {
                first[node].unwrap_or(target)
            };
            if candidate < reduced[target]
                || (candidate == reduced[target]
                    && first[target].is_none_or(|current| candidate_first < current))
            {
                reduced[target] = candidate;
                first[target] = Some(candidate_first);
                queue.push(Reverse((candidate, target)));
            }
        }
    }
    restore_distances(
        source, potentials, distances, reachable, next, &reduced, &first,
    )
}

#[allow(clippy::too_many_arguments)]
fn restore_distances(
    source: usize,
    potentials: &[i64],
    distances: &mut [i64],
    reachable: &mut [bool],
    next: &mut [Option<usize>],
    reduced: &[i64],
    first: &[Option<usize>],
) -> Result<()> {
    let bound = reduced.len();
    for target in 0..bound {
        if reduced[target] == i64::MAX {
            continue;
        }
        let original = reduced[target]
            .checked_sub(potentials[source])
            .and_then(|value| value.checked_add(potentials[target]))
            .ok_or(GraphError::ArithmeticOverflow {
                operation: "Johnson distance restoration",
            })?;
        distances[target] = original;
        reachable[target] = true;
        next[target] = first[target];
    }
    Ok(())
}
