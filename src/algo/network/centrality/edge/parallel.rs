use super::{
    BrandesGraph, BrandesWorkspace, brandes_source, directed_adjacency, edge_score_capacity, scale,
    undirected_adjacency,
};
use crate::algo::traversal::Direction;
use crate::{IndexGraphView, IndexUndirectedGraphView, Vec};
use rayon::prelude::*;

/// Parallel unweighted Brandes edge betweenness for a directed graph view.
#[must_use]
pub fn edge_betweenness_centrality_parallel<G>(
    graph: &G,
    direction: Direction,
    normalized: bool,
) -> Vec<(G::Edge, f64)>
where
    G: IndexGraphView,
    G::Edge: Send + Sync,
{
    let adjacent = directed_adjacency(graph, direction, |_| true);
    calculate_parallel(&adjacent, direction == Direction::Both, normalized)
}

/// Parallel unweighted Brandes edge betweenness for an undirected graph view.
#[must_use]
pub fn undirected_edge_betweenness_centrality_parallel<G>(
    graph: &G,
    normalized: bool,
) -> Vec<(G::Edge, f64)>
where
    G: IndexUndirectedGraphView,
    G::Edge: Send + Sync,
{
    let adjacent = undirected_adjacency(graph, |_| true);
    calculate_parallel(&adjacent, true, normalized)
}

fn calculate_parallel<Edge>(
    adjacent: &BrandesGraph<Edge>,
    undirected: bool,
    normalized: bool,
) -> Vec<(Edge, f64)>
where
    Edge: Copy + Send + Sync,
{
    let workers = rayon::current_num_threads().max(1);
    let chunk_size = adjacent.source_slots.len().div_ceil(workers).max(1);
    let partials = adjacent
        .source_slots
        .par_chunks(chunk_size)
        .map(|sources| {
            let mut scores = vec![0.0; edge_score_capacity(adjacent)];
            let mut workspace = BrandesWorkspace::new(adjacent.arcs_by_node.len());
            for &source in sources {
                brandes_source(adjacent, source, &mut workspace, &mut scores);
            }
            scores
        })
        .collect::<Vec<_>>();
    let mut scores = vec![0.0; edge_score_capacity(adjacent)];
    for partial in partials {
        for (score, value) in scores.iter_mut().zip(partial) {
            *score += value;
        }
    }
    scale(
        &mut scores,
        adjacent.source_slots.len(),
        undirected,
        normalized,
    );
    adjacent
        .accepted_edges
        .iter()
        .copied()
        .map(|(edge, slot)| (edge, scores[slot]))
        .collect()
}
