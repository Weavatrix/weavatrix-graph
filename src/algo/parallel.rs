use super::{WeightedPath, bfs, dijkstra};
use crate::{IndexGraphView, Vec};
use rayon::prelude::*;

/// Runs independent breadth-first traversals in parallel.
///
/// Results preserve the order of `starts`. Use this only when the batch is
/// large enough to amortize Rayon scheduling.
#[must_use]
pub fn bfs_batch_parallel<G>(graph: &G, starts: &[G::Node]) -> Vec<Vec<G::Node>>
where
    G: IndexGraphView + Sync,
    G::Node: Send + Sync,
{
    starts.par_iter().map(|&start| bfs(graph, start)).collect()
}

/// Runs independent Dijkstra queries in parallel.
///
/// Results preserve the order of `queries`; each query keeps the deterministic
/// tie-breaking of [`dijkstra`].
#[must_use]
pub fn dijkstra_batch_parallel<G, F>(
    graph: &G,
    queries: &[(G::Node, G::Node)],
    edge_cost: F,
) -> Vec<Option<WeightedPath<G::Node>>>
where
    G: IndexGraphView + Sync,
    G::Node: Send + Sync,
    G::Edge: Send,
    F: Fn(G::Edge) -> u64 + Send + Sync,
{
    queries
        .par_iter()
        .map(|&(source, target)| dijkstra(graph, source, target, &edge_cost))
        .collect()
}
