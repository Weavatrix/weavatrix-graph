use crate::Vec;
use crate::{GraphError, IndexUndirectedGraphView, Result};
use alloc::collections::BinaryHeap;
use core::cmp::Reverse;

mod disjoint;
mod select;

use disjoint::DisjointSet;
use select::candidate_tree;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteinerTree<Node, Edge> {
    terminals: Vec<Node>,
    edges: Vec<Edge>,
    total_cost: u64,
}

struct ShortestPaths<Edge> {
    distance: Vec<u64>,
    predecessor: Vec<Option<(usize, Edge)>>,
}

impl<Node, Edge> SteinerTree<Node, Edge> {
    #[must_use]
    pub fn terminals(&self) -> &[Node] {
        &self.terminals
    }

    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    #[must_use]
    pub const fn total_cost(&self) -> u64 {
        self.total_cost
    }
}

/// Builds a deterministic metric-closure Steiner-tree approximation.
///
/// # Errors
///
/// Returns an error when path or tree cost arithmetic overflows.
pub fn steiner_tree_approximation<G, F>(
    graph: &G,
    terminals: &[G::Node],
    edge_cost: F,
) -> Result<Option<SteinerTree<G::Node, G::Edge>>>
where
    G: IndexUndirectedGraphView,
    F: Fn(G::Edge) -> u64,
{
    let mut terminals = terminals.to_vec();
    terminals.sort_unstable_by_key(|node| G::node_slot(*node));
    terminals.dedup();
    if terminals.iter().any(|node| !graph.contains_node(*node)) {
        return Ok(None);
    }
    if terminals.len() < 2 {
        return Ok(Some(SteinerTree {
            terminals,
            edges: Vec::new(),
            total_cost: 0,
        }));
    }
    let mut nodes = vec![None; graph.node_bound()];
    for node in graph.node_indices() {
        nodes[G::node_slot(node)] = Some(node);
    }
    let mut metric = Vec::new();
    for left in 0..terminals.len() {
        let paths = shortest_paths(graph, &nodes, terminals[left], &edge_cost)?;
        for right in left + 1..terminals.len() {
            let target = G::node_slot(terminals[right]);
            let cost = paths.distance[target];
            if cost == u64::MAX {
                return Ok(None);
            }
            let Some(path) =
                reconstruct::<G>(terminals[left], terminals[right], &paths.predecessor)
            else {
                return Ok(None);
            };
            metric.push((cost, left, right, path));
        }
    }
    let mut expanded = vec![false; graph.edge_bound()];
    for (_, _, _, path) in &metric {
        for &edge in path {
            expanded[G::edge_slot(edge)] = true;
        }
    }
    let mut edges = candidate_tree(graph, &terminals, &expanded, &edge_cost);
    let mut total_cost = tree_cost(&edges, &edge_cost)?;
    for seed in 0_u64..32 {
        let selected =
            metric_tree_selection::<G>(&metric, graph.edge_bound(), terminals.len(), seed);
        let candidate = candidate_tree(graph, &terminals, &selected, &edge_cost);
        let cost = tree_cost(&candidate, &edge_cost)?;
        if cost < total_cost {
            edges = candidate;
            total_cost = cost;
        }
    }
    Ok(Some(SteinerTree {
        terminals,
        edges,
        total_cost,
    }))
}

fn metric_tree_selection<G>(
    metric: &[(u64, usize, usize, Vec<G::Edge>)],
    edge_bound: usize,
    terminal_count: usize,
    seed: u64,
) -> Vec<bool>
where
    G: IndexUndirectedGraphView,
{
    let mut order = (0..metric.len()).collect::<Vec<_>>();
    order.sort_unstable_by_key(|index| {
        let (cost, left, right, _) = &metric[*index];
        (*cost, metric_tie(*left, *right, terminal_count, seed))
    });
    let mut sets = DisjointSet::new(terminal_count);
    let mut selected = vec![false; edge_bound];
    for index in order {
        let (_, left, right, path) = &metric[index];
        if sets.union(*left, *right) {
            for &edge in path {
                selected[G::edge_slot(edge)] = true;
            }
        }
    }
    selected
}

fn metric_tie(left: usize, right: usize, terminal_count: usize, seed: u64) -> u64 {
    let canonical = left
        .checked_mul(terminal_count)
        .and_then(|value| value.checked_add(right))
        .and_then(|value| u64::try_from(value).ok())
        .unwrap_or(u64::MAX);
    if seed == 0 {
        return canonical;
    }
    let mut value = canonical ^ seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn tree_cost<Edge: Copy, F>(edges: &[Edge], edge_cost: &F) -> Result<u64>
where
    F: Fn(Edge) -> u64,
{
    edges.iter().try_fold(0_u64, |total, edge| {
        total
            .checked_add(edge_cost(*edge))
            .ok_or(GraphError::ArithmeticOverflow {
                operation: "Steiner tree cost",
            })
    })
}

fn shortest_paths<G, F>(
    graph: &G,
    nodes: &[Option<G::Node>],
    source: G::Node,
    edge_cost: &F,
) -> Result<ShortestPaths<G::Edge>>
where
    G: IndexUndirectedGraphView,
    F: Fn(G::Edge) -> u64,
{
    let mut distance = vec![u64::MAX; graph.node_bound()];
    let mut predecessor = vec![None; graph.node_bound()];
    let source_slot = G::node_slot(source);
    distance[source_slot] = 0;
    let mut queue = BinaryHeap::from([Reverse((0_u64, source_slot))]);
    while let Some(Reverse((cost, slot))) = queue.pop() {
        if cost != distance[slot] {
            continue;
        }
        let Some(node) = nodes[slot] else {
            continue;
        };
        for edge in graph.incident_edges(node) {
            let Some(neighbor) = graph.opposite(edge, node) else {
                continue;
            };
            let candidate =
                cost.checked_add(edge_cost(edge))
                    .ok_or(GraphError::ArithmeticOverflow {
                        operation: "Steiner Dijkstra relaxation",
                    })?;
            let neighbor_slot = G::node_slot(neighbor);
            if candidate < distance[neighbor_slot] {
                distance[neighbor_slot] = candidate;
                predecessor[neighbor_slot] = Some((slot, edge));
                queue.push(Reverse((candidate, neighbor_slot)));
            }
        }
    }
    Ok(ShortestPaths {
        distance,
        predecessor,
    })
}

fn reconstruct<G: IndexUndirectedGraphView>(
    source: G::Node,
    target: G::Node,
    predecessor: &[Option<(usize, G::Edge)>],
) -> Option<Vec<G::Edge>> {
    let source = G::node_slot(source);
    let mut cursor = G::node_slot(target);
    let mut edges = Vec::new();
    while cursor != source {
        let (parent, edge) = predecessor[cursor]?;
        edges.push(edge);
        cursor = parent;
    }
    edges.reverse();
    Some(edges)
}
