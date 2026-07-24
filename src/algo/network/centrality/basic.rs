#![allow(clippy::cast_precision_loss)]

use super::super::adjacency::{SlotAdjacency, adjacency};
use crate::algo::traversal::Direction;
use crate::{IndexGraphView, Vec};
use alloc::collections::VecDeque;
#[cfg(feature = "rayon")]
use rayon::prelude::*;

#[must_use]
pub fn degree_centrality<G>(graph: &G, direction: Direction) -> Vec<(G::Node, f64)>
where
    G: IndexGraphView,
{
    let adjacent = adjacency(graph, direction);
    let denominator = adjacent.nodes.len().saturating_sub(1) as f64;
    adjacent
        .nodes
        .iter()
        .copied()
        .map(|node| {
            let degree = adjacent.neighbors[G::node_slot(node)].len() as f64;
            (
                node,
                if denominator == 0.0 {
                    0.0
                } else {
                    degree / denominator
                },
            )
        })
        .collect()
}

#[must_use]
pub fn closeness_centrality<G>(graph: &G, direction: Direction) -> Vec<(G::Node, f64)>
where
    G: IndexGraphView,
{
    let adjacent = adjacency(graph, direction);
    adjacent
        .nodes
        .iter()
        .copied()
        .map(|node| {
            (
                node,
                closeness_from(&adjacent, G::node_slot(node), adjacent.nodes.len()),
            )
        })
        .collect()
}

#[must_use]
pub fn betweenness_centrality<G>(
    graph: &G,
    direction: Direction,
    normalized: bool,
) -> Vec<(G::Node, f64)>
where
    G: IndexGraphView,
{
    let adjacent = adjacency(graph, direction);
    let mut scores = vec![0.0; graph.node_bound()];
    for source in adjacent.nodes.iter().copied().map(G::node_slot) {
        brandes_source(&adjacent, source, &mut scores);
    }
    scale_betweenness(&mut scores, adjacent.nodes.len(), direction, normalized);
    adjacent
        .nodes
        .iter()
        .copied()
        .map(|node| (node, scores[G::node_slot(node)]))
        .collect()
}

fn scale_betweenness(
    scores: &mut [f64],
    node_count: usize,
    direction: Direction,
    normalized: bool,
) {
    if direction == Direction::Both {
        for score in &mut *scores {
            *score /= 2.0;
        }
    }
    if normalized && node_count > 2 {
        let denominator = ((node_count - 1) * (node_count - 2)) as f64;
        let scale = if direction == Direction::Both {
            2.0 / denominator
        } else {
            1.0 / denominator
        };
        for score in scores {
            *score *= scale;
        }
    }
}

#[cfg(feature = "rayon")]
#[must_use]
pub fn closeness_centrality_parallel<G>(graph: &G, direction: Direction) -> Vec<(G::Node, f64)>
where
    G: IndexGraphView,
    G::Node: Send + Sync,
{
    let adjacent = adjacency(graph, direction);
    adjacent
        .nodes
        .par_iter()
        .map(|&node| {
            (
                node,
                closeness_from(&adjacent, G::node_slot(node), adjacent.nodes.len()),
            )
        })
        .collect()
}

#[cfg(feature = "rayon")]
#[must_use]
pub fn betweenness_centrality_parallel<G>(
    graph: &G,
    direction: Direction,
    normalized: bool,
) -> Vec<(G::Node, f64)>
where
    G: IndexGraphView,
    G::Node: Send + Sync,
{
    let adjacent = adjacency(graph, direction);
    let bound = graph.node_bound();
    let sources = adjacent
        .nodes
        .iter()
        .copied()
        .map(G::node_slot)
        .collect::<Vec<_>>();
    let workers = rayon::current_num_threads().max(1);
    let chunk_size = sources.len().div_ceil(workers).max(1);
    let partials = sources
        .par_chunks(chunk_size)
        .map(|chunk| {
            let mut scores = vec![0.0; bound];
            for &source in chunk {
                brandes_source(&adjacent, source, &mut scores);
            }
            scores
        })
        .collect::<Vec<_>>();
    let mut scores = vec![0.0; bound];
    for partial in partials {
        for (score, value) in scores.iter_mut().zip(partial) {
            *score += value;
        }
    }
    scale_betweenness(&mut scores, adjacent.nodes.len(), direction, normalized);
    adjacent
        .nodes
        .iter()
        .copied()
        .map(|node| (node, scores[G::node_slot(node)]))
        .collect()
}

fn closeness_from<Node>(adjacent: &SlotAdjacency<Node>, source: usize, node_count: usize) -> f64 {
    let mut distances = vec![usize::MAX; adjacent.neighbors.len()];
    let mut queue = VecDeque::new();
    distances[source] = 0;
    queue.push_back(source);
    while let Some(node) = queue.pop_front() {
        for &neighbor in &adjacent.neighbors[node] {
            if distances[neighbor] == usize::MAX {
                distances[neighbor] = distances[node] + 1;
                queue.push_back(neighbor);
            }
        }
    }
    let reachable = distances
        .iter()
        .filter(|distance| **distance != usize::MAX)
        .count();
    let total = distances
        .iter()
        .filter(|distance| **distance != usize::MAX)
        .sum::<usize>();
    if reachable <= 1 || total == 0 || node_count <= 1 {
        return 0.0;
    }
    let reached = (reachable - 1) as f64;
    reached / total as f64 * reached / (node_count - 1) as f64
}

fn brandes_source<Node>(adjacent: &SlotAdjacency<Node>, source: usize, scores: &mut [f64]) {
    let bound = adjacent.neighbors.len();
    let mut stack = Vec::new();
    let mut predecessors = vec![Vec::new(); bound];
    let mut paths = vec![0.0; bound];
    let mut distances = vec![usize::MAX; bound];
    let mut queue = VecDeque::new();
    paths[source] = 1.0;
    distances[source] = 0;
    queue.push_back(source);
    while let Some(node) = queue.pop_front() {
        stack.push(node);
        for &neighbor in &adjacent.neighbors[node] {
            if distances[neighbor] == usize::MAX {
                distances[neighbor] = distances[node] + 1;
                queue.push_back(neighbor);
            }
            if distances[neighbor] == distances[node] + 1 {
                paths[neighbor] += paths[node];
                predecessors[neighbor].push(node);
            }
        }
    }
    let mut dependency = vec![0.0; bound];
    while let Some(node) = stack.pop() {
        for &predecessor in &predecessors[node] {
            dependency[predecessor] += paths[predecessor] / paths[node] * (1.0 + dependency[node]);
        }
        if node != source {
            scores[node] += dependency[node];
        }
    }
}
