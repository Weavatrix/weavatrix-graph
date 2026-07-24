use super::{AllPairsShortestPaths, floyd_warshall_filtered, johnson_all_pairs_filtered};
use crate::{IndexGraphView, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllPairsStrategy {
    FloydWarshall,
    Johnson,
}

#[derive(Debug, Clone)]
pub struct AutoAllPairs<Node> {
    strategy: AllPairsStrategy,
    paths: AllPairsShortestPaths<Node>,
}

impl<Node> AutoAllPairs<Node> {
    #[must_use]
    pub const fn strategy(&self) -> AllPairsStrategy {
        self.strategy
    }

    #[must_use]
    pub const fn paths(&self) -> &AllPairsShortestPaths<Node> {
        &self.paths
    }

    #[must_use]
    pub fn into_paths(self) -> AllPairsShortestPaths<Node> {
        self.paths
    }
}

/// Selects Floyd-Warshall for small/dense graphs and Johnson for sparse graphs.
///
/// The weight callback is evaluated exactly once for every edge.
///
/// # Errors
///
/// Returns an error for arithmetic overflow or any selected negative cycle.
pub fn all_pairs_auto<G, F>(graph: &G, edge_cost: F) -> Result<AutoAllPairs<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> i64,
{
    all_pairs_auto_filtered(graph, |edge| Some(edge_cost(edge)))
}

/// Automatically selects an all-pairs algorithm over a filtered edge set.
///
/// The selected strategy is exposed in the result. Filtered weights are
/// snapshotted once before selection.
///
/// # Errors
///
/// Returns an error for arithmetic overflow or any selected negative cycle.
pub fn all_pairs_auto_filtered<G, F>(graph: &G, edge_cost: F) -> Result<AutoAllPairs<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> Option<i64>,
{
    let mut weights = vec![None; graph.edge_bound()];
    let mut selected_edges = 0_usize;
    for edge in graph.edge_indices() {
        let weight = edge_cost(edge);
        selected_edges += usize::from(weight.is_some());
        weights[G::edge_slot(edge)] = weight;
    }
    let strategy = select_strategy(graph.node_count(), selected_edges);
    let paths = match strategy {
        AllPairsStrategy::FloydWarshall => {
            floyd_warshall_filtered(graph, |edge| weights[G::edge_slot(edge)])?
        }
        AllPairsStrategy::Johnson => {
            johnson_all_pairs_filtered(graph, |edge| weights[G::edge_slot(edge)])?
        }
    };
    Ok(AutoAllPairs { strategy, paths })
}

fn select_strategy(node_count: usize, edge_count: usize) -> AllPairsStrategy {
    if node_count <= 64 {
        return AllPairsStrategy::FloydWarshall;
    }
    let dense_threshold = node_count
        .checked_mul(node_count)
        .map_or(usize::MAX, |cells| cells / 8);
    if edge_count >= dense_threshold {
        AllPairsStrategy::FloydWarshall
    } else {
        AllPairsStrategy::Johnson
    }
}
