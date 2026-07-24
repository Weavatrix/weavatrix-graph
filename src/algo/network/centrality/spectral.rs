#![allow(clippy::cast_precision_loss)]

use super::super::adjacency::{SlotAdjacency, adjacency};
use super::math::square_root;
use crate::algo::traversal::Direction;
use crate::{GraphError, IndexGraphView, Result, String, Vec};

#[derive(Debug, Clone, PartialEq)]
pub struct IterativeCentrality<Node> {
    scores: Vec<(Node, f64)>,
    iterations: usize,
    converged: bool,
}

impl<Node> IterativeCentrality<Node> {
    #[must_use]
    pub fn scores(&self) -> &[(Node, f64)] {
        &self.scores
    }

    #[must_use]
    pub const fn iterations(&self) -> usize {
        self.iterations
    }

    #[must_use]
    pub const fn converged(&self) -> bool {
        self.converged
    }

    #[must_use]
    pub fn into_scores(self) -> Vec<(Node, f64)> {
        self.scores
    }
}

/// Computes eigenvector centrality by shifted power iteration.
///
/// # Errors
///
/// Returns an error for zero iterations or an invalid tolerance.
pub fn eigenvector_centrality<G>(
    graph: &G,
    max_iterations: usize,
    tolerance: f64,
) -> Result<IterativeCentrality<G::Node>>
where
    G: IndexGraphView,
{
    validate(max_iterations, tolerance)?;
    let adjacent = adjacency(graph, Direction::Outgoing);
    let initial = if adjacent.nodes.is_empty() {
        0.0
    } else {
        1.0 / square_root(adjacent.nodes.len() as f64)
    };
    let mut current = vec![initial; graph.node_bound()];
    let mut next = vec![0.0; graph.node_bound()];
    for iteration in 1..=max_iterations {
        next.fill(0.0);
        for &node in &adjacent.nodes {
            let source = G::node_slot(node);
            next[source] += current[source];
            for &target in &adjacent.neighbors[source] {
                next[target] += current[source];
            }
        }
        normalize::<G>(&mut next, &adjacent);
        if converged::<G>(&current, &next, &adjacent, tolerance) {
            return Ok(result::<G>(&adjacent, &next, iteration, true));
        }
        core::mem::swap(&mut current, &mut next);
    }
    Ok(result::<G>(&adjacent, &current, max_iterations, false))
}

/// Computes Katz centrality by fixed-point iteration.
///
/// # Errors
///
/// Returns an error for invalid iteration, tolerance, alpha, or beta values.
pub fn katz_centrality<G>(
    graph: &G,
    alpha: f64,
    beta: f64,
    max_iterations: usize,
    tolerance: f64,
) -> Result<IterativeCentrality<G::Node>>
where
    G: IndexGraphView,
{
    validate(max_iterations, tolerance)?;
    if !alpha.is_finite() || alpha < 0.0 || !beta.is_finite() {
        return Err(invalid_parameter("alpha/beta"));
    }
    let adjacent = adjacency(graph, Direction::Outgoing);
    let mut current = vec![1.0; graph.node_bound()];
    let mut next = vec![beta; graph.node_bound()];
    for iteration in 1..=max_iterations {
        next.fill(beta);
        for &node in &adjacent.nodes {
            let source = G::node_slot(node);
            for &target in &adjacent.neighbors[source] {
                next[target] += alpha * current[source];
            }
        }
        if next.iter().any(|score| !score.is_finite()) {
            return Err(invalid_parameter("alpha"));
        }
        if converged::<G>(&current, &next, &adjacent, tolerance) {
            return Ok(result::<G>(&adjacent, &next, iteration, true));
        }
        core::mem::swap(&mut current, &mut next);
    }
    Ok(result::<G>(&adjacent, &current, max_iterations, false))
}

fn normalize<G>(scores: &mut [f64], adjacent: &SlotAdjacency<G::Node>)
where
    G: IndexGraphView,
{
    let norm = adjacent
        .nodes
        .iter()
        .map(|node| {
            let value = scores[G::node_slot(*node)];
            value * value
        })
        .sum::<f64>();
    let norm = square_root(norm);
    if norm > 0.0 {
        for node in &adjacent.nodes {
            scores[G::node_slot(*node)] /= norm;
        }
    }
}

fn converged<G>(
    current: &[f64],
    next: &[f64],
    adjacent: &SlotAdjacency<G::Node>,
    tolerance: f64,
) -> bool
where
    G: IndexGraphView,
{
    adjacent
        .nodes
        .iter()
        .map(|node| (next[G::node_slot(*node)] - current[G::node_slot(*node)]).abs())
        .sum::<f64>()
        <= tolerance * adjacent.nodes.len() as f64
}

fn result<G>(
    adjacent: &SlotAdjacency<G::Node>,
    scores: &[f64],
    iterations: usize,
    converged: bool,
) -> IterativeCentrality<G::Node>
where
    G: IndexGraphView,
{
    IterativeCentrality {
        scores: adjacent
            .nodes
            .iter()
            .copied()
            .map(|node| (node, scores[G::node_slot(node)]))
            .collect(),
        iterations,
        converged,
    }
}

fn validate(max_iterations: usize, tolerance: f64) -> Result<()> {
    if max_iterations == 0 || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(invalid_parameter("max_iterations/tolerance"));
    }
    Ok(())
}

fn invalid_parameter(parameter: &'static str) -> GraphError {
    GraphError::InvalidAlgorithmParameter {
        algorithm: "centrality",
        parameter,
        value: String::from("must be finite, positive, and non-zero where required"),
    }
}
