use crate::{GraphError, IndexGraphView, Result};
use crate::{ToString, Vec};

/// Computes `PageRank` in graph node order.
///
/// # Errors
///
/// Returns an error when `damping` is not finite or outside `0.0..=1.0`.
pub fn page_rank<G>(graph: &G, damping: f64, iterations: usize) -> Result<Vec<(G::Node, f64)>>
where
    G: IndexGraphView,
{
    page_rank_filtered(graph, damping, iterations, |_| true)
}

/// Computes `PageRank` over the selected edges in graph node order.
///
/// Dangling-node mass is distributed uniformly. Parallel edges contribute
/// independently, matching the multigraph contract used by traversal.
///
/// # Errors
///
/// Returns an error when `damping` is not finite or outside `0.0..=1.0`.
pub fn page_rank_filtered<G, F>(
    graph: &G,
    damping: f64,
    iterations: usize,
    allows_edge: F,
) -> Result<Vec<(G::Node, f64)>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    validate_damping(damping)?;
    let nodes = graph.node_indices().collect::<Vec<_>>();
    if nodes.is_empty() {
        return Ok(Vec::new());
    }
    let mut positions = vec![None; graph.node_bound()];
    for (position, &node) in nodes.iter().enumerate() {
        positions[G::node_slot(node)] = Some(position);
    }
    let edges = graph
        .edge_references()
        .filter(|(edge, _)| allows_edge(*edge))
        .filter_map(|(_, endpoints)| {
            let source = positions[G::node_slot(endpoints.source())]?;
            let target = positions[G::node_slot(endpoints.target())]?;
            Some((source, target))
        })
        .collect::<Vec<_>>();
    let mut out_degree = vec![0_usize; nodes.len()];
    for &(source, _) in &edges {
        out_degree[source] += 1;
    }

    let node_count = usize_as_f64(nodes.len());
    let mut ranks = vec![1.0 / node_count; nodes.len()];
    let mut next = vec![0.0; nodes.len()];
    for _ in 0..iterations {
        let dangling = ranks
            .iter()
            .zip(&out_degree)
            .filter(|(_, degree)| **degree == 0)
            .map(|(rank, _)| rank)
            .sum::<f64>();
        next.fill((1.0 - damping + damping * dangling) / node_count);
        for &(source, target) in &edges {
            next[target] += damping * ranks[source] / usize_as_f64(out_degree[source]);
        }
        normalize(&mut next);
        core::mem::swap(&mut ranks, &mut next);
    }
    Ok(nodes.into_iter().zip(ranks).collect())
}

fn validate_damping(damping: f64) -> Result<()> {
    if damping.is_finite() && (0.0..=1.0).contains(&damping) {
        return Ok(());
    }
    Err(GraphError::InvalidAlgorithmParameter {
        algorithm: "PageRank",
        parameter: "damping",
        value: damping.to_string(),
    })
}

fn normalize(values: &mut [f64]) {
    let sum = values.iter().sum::<f64>();
    if sum > 0.0 {
        for value in values {
            *value /= sum;
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn usize_as_f64(value: usize) -> f64 {
    value as f64
}
