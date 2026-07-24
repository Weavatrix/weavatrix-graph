use super::{AllPairsShortestPaths, cell, indexed_nodes};
use crate::{GraphError, IndexGraphView, Result};

/// Computes all-pairs signed shortest paths with Floyd-Warshall.
///
/// # Errors
///
/// Returns an error for arithmetic overflow or any negative cycle.
pub fn floyd_warshall<G, F>(graph: &G, edge_cost: F) -> Result<AllPairsShortestPaths<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> i64,
{
    floyd_warshall_filtered(graph, |edge| Some(edge_cost(edge)))
}

/// Computes Floyd-Warshall while omitting edges whose cost is `None`.
///
/// # Errors
///
/// Returns an error for arithmetic overflow or any selected negative cycle.
pub fn floyd_warshall_filtered<G, F>(
    graph: &G,
    edge_cost: F,
) -> Result<AllPairsShortestPaths<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> Option<i64>,
{
    let (nodes, by_slot) = indexed_nodes(graph);
    let bound = graph.node_bound();
    let length = bound
        .checked_mul(bound)
        .ok_or(GraphError::ArithmeticOverflow {
            operation: "Floyd-Warshall matrix allocation",
        })?;
    let mut distances = vec![0_i64; length];
    let mut reachable = vec![false; length];
    let mut next = vec![None; length];
    for &node in &nodes {
        let slot = G::node_slot(node);
        reachable[cell(bound, slot, slot)] = true;
        next[cell(bound, slot, slot)] = Some(slot);
    }
    for (edge, endpoints) in graph.edge_references() {
        let Some(weight) = edge_cost(edge) else {
            continue;
        };
        let source = G::node_slot(endpoints.source());
        let target = G::node_slot(endpoints.target());
        let index = cell(bound, source, target);
        if !reachable[index] || weight < distances[index] {
            distances[index] = weight;
            reachable[index] = true;
            next[index] = Some(target);
        }
    }
    for &via_node in &nodes {
        relax_via(
            &nodes,
            &mut distances,
            &mut reachable,
            &mut next,
            bound,
            G::node_slot(via_node),
            G::node_slot,
        )?;
    }
    reject_negative_diagonal(&nodes, &distances, &reachable, bound, G::node_slot)?;
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

fn relax_via<Node: Copy>(
    nodes: &[Node],
    distances: &mut [i64],
    reachable: &mut [bool],
    next: &mut [Option<usize>],
    bound: usize,
    via: usize,
    slot: fn(Node) -> usize,
) -> Result<()> {
    for &source_node in nodes {
        let source = slot(source_node);
        let left_index = cell(bound, source, via);
        if !reachable[left_index] {
            continue;
        }
        let left = distances[left_index];
        for &target_node in nodes {
            let target = slot(target_node);
            let right_index = cell(bound, via, target);
            if !reachable[right_index] {
                continue;
            }
            let right = distances[right_index];
            let candidate = left
                .checked_add(right)
                .ok_or(GraphError::ArithmeticOverflow {
                    operation: "Floyd-Warshall relaxation",
                })?;
            let index = cell(bound, source, target);
            if !reachable[index] || candidate < distances[index] {
                distances[index] = candidate;
                reachable[index] = true;
                next[index] = next[cell(bound, source, via)];
            }
        }
    }
    Ok(())
}

fn reject_negative_diagonal<Node: Copy>(
    nodes: &[Node],
    distances: &[i64],
    reachable: &[bool],
    bound: usize,
    slot: fn(Node) -> usize,
) -> Result<()> {
    if nodes.iter().map(|node| slot(*node)).any(|node| {
        let index = cell(bound, node, node);
        reachable[index] && distances[index] < 0
    }) {
        return Err(GraphError::NegativeCycle {
            algorithm: "Floyd-Warshall",
        });
    }
    Ok(())
}
