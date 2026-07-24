#![allow(clippy::cast_precision_loss)]

use super::math::square_root;
use crate::{GraphError, IndexGraphView, Result, String, Vec};

/// Canonically ordered node-score pairs returned by HITS.
pub type HitsScores<Node> = Vec<(Node, f64)>;

/// HITS hub and authority scores in canonical node order.
#[derive(Debug, Clone, PartialEq)]
pub struct Hits<Node> {
    hubs: HitsScores<Node>,
    authorities: HitsScores<Node>,
    iterations: usize,
    converged: bool,
}

impl<Node> Hits<Node> {
    /// Returns L2-normalized hub scores.
    #[must_use]
    pub fn hubs(&self) -> &[(Node, f64)] {
        &self.hubs
    }

    /// Returns L2-normalized authority scores.
    #[must_use]
    pub fn authorities(&self) -> &[(Node, f64)] {
        &self.authorities
    }

    /// Returns the number of completed power iterations.
    #[must_use]
    pub fn iterations(&self) -> usize {
        self.iterations
    }

    /// Reports whether the requested tolerance was reached.
    #[must_use]
    pub fn converged(&self) -> bool {
        self.converged
    }

    /// Consumes the result into hub and authority vectors.
    #[must_use]
    pub fn into_scores(self) -> (HitsScores<Node>, HitsScores<Node>) {
        (self.hubs, self.authorities)
    }
}

impl<Node> Hits<Node>
where
    Node: Copy + Eq,
{
    /// Returns one node's hub score.
    #[must_use]
    pub fn hub(&self, node: Node) -> Option<f64> {
        score(&self.hubs, node)
    }

    /// Returns one node's authority score.
    #[must_use]
    pub fn authority(&self, node: Node) -> Option<f64> {
        score(&self.authorities, node)
    }
}

/// Computes HITS hub and authority scores on topological relationships.
///
/// Parallel edges with equal endpoints are treated as one relationship, so
/// duplicate provenance does not bias repository rankings.
///
/// # Errors
///
/// Returns an error for zero iterations or a non-positive, non-finite
/// tolerance.
pub fn hits<G>(graph: &G, max_iterations: usize, tolerance: f64) -> Result<Hits<G::Node>>
where
    G: IndexGraphView,
{
    hits_filtered(graph, max_iterations, tolerance, |_| true)
}

/// Computes HITS scores using accepted relationships only.
///
/// The predicate is evaluated exactly once per edge.
///
/// # Errors
///
/// Returns an error for zero iterations or a non-positive, non-finite
/// tolerance.
pub fn hits_filtered<G, F>(
    graph: &G,
    max_iterations: usize,
    tolerance: f64,
    allows_edge: F,
) -> Result<Hits<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    validate(max_iterations, tolerance)?;
    let mut nodes = graph.node_indices().collect::<Vec<_>>();
    nodes.sort_unstable_by_key(|node| G::node_slot(*node));
    let slots = nodes
        .iter()
        .map(|node| G::node_slot(*node))
        .collect::<Vec<_>>();
    let dense = slots
        .iter()
        .enumerate()
        .all(|(expected, &slot)| expected == slot);
    let edges = accepted_edges(graph, allows_edge);
    if nodes.is_empty() || edges.is_empty() {
        return Ok(zero_result(nodes));
    }
    let initial = 1.0 / square_root(nodes.len() as f64);
    let mut hubs = vec![initial; graph.node_bound()];
    let mut authorities = vec![initial; graph.node_bound()];
    let mut next_hubs = vec![0.0; graph.node_bound()];
    let mut next_authorities = vec![0.0; graph.node_bound()];
    for iteration in 1..=max_iterations {
        next_authorities.fill(0.0);
        for &(source, target) in &edges {
            next_authorities[target] += hubs[source];
        }
        normalize(&mut next_authorities, &slots, dense);
        next_hubs.fill(0.0);
        for &(source, target) in &edges {
            next_hubs[source] += next_authorities[target];
        }
        normalize(&mut next_hubs, &slots, dense);
        if vectors_converged(
            &hubs,
            &next_hubs,
            &authorities,
            &next_authorities,
            &slots,
            dense,
            tolerance,
        ) {
            return Ok(result(
                &nodes,
                &slots,
                &next_hubs,
                &next_authorities,
                iteration,
                true,
            ));
        }
        core::mem::swap(&mut hubs, &mut next_hubs);
        core::mem::swap(&mut authorities, &mut next_authorities);
    }
    Ok(result(
        &nodes,
        &slots,
        &hubs,
        &authorities,
        max_iterations,
        false,
    ))
}

fn accepted_edges<G, F>(graph: &G, allows_edge: F) -> Vec<(usize, usize)>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    let mut edges = graph
        .edge_references()
        .filter(|(edge, _)| allows_edge(*edge))
        .filter_map(|(_, endpoints)| {
            let source = G::node_slot(endpoints.source());
            let target = G::node_slot(endpoints.target());
            (source != target).then_some((source, target))
        })
        .collect::<Vec<_>>();
    if !edges.is_sorted() {
        edges.sort_unstable();
    }
    edges.dedup();
    edges
}

fn normalize(values: &mut [f64], slots: &[usize], dense: bool) {
    let norm = if dense {
        square_root(
            values[..slots.len()]
                .iter()
                .map(|value| value * value)
                .sum(),
        )
    } else {
        square_root(slots.iter().map(|&slot| values[slot] * values[slot]).sum())
    };
    if norm > 0.0 {
        if dense {
            for value in &mut values[..slots.len()] {
                *value /= norm;
            }
        } else {
            for &slot in slots {
                values[slot] /= norm;
            }
        }
    }
}

fn vectors_converged(
    hubs: &[f64],
    next_hubs: &[f64],
    authorities: &[f64],
    next_authorities: &[f64],
    slots: &[usize],
    dense: bool,
    tolerance: f64,
) -> bool {
    if dense {
        hubs.iter()
            .zip(next_hubs)
            .zip(authorities.iter().zip(next_authorities))
            .take(slots.len())
            .all(|((hub, next_hub), (authority, next_authority))| {
                (hub - next_hub).abs() <= tolerance
                    && (authority - next_authority).abs() <= tolerance
            })
    } else {
        slots.iter().all(|&slot| {
            (hubs[slot] - next_hubs[slot]).abs() <= tolerance
                && (authorities[slot] - next_authorities[slot]).abs() <= tolerance
        })
    }
}

fn result<Node>(
    nodes: &[Node],
    slots: &[usize],
    hubs: &[f64],
    authorities: &[f64],
    iterations: usize,
    converged: bool,
) -> Hits<Node>
where
    Node: Copy,
{
    Hits {
        hubs: nodes
            .iter()
            .zip(slots)
            .map(|(&node, &slot)| (node, hubs[slot]))
            .collect(),
        authorities: nodes
            .iter()
            .zip(slots)
            .map(|(&node, &slot)| (node, authorities[slot]))
            .collect(),
        iterations,
        converged,
    }
}

fn zero_result<Node>(nodes: Vec<Node>) -> Hits<Node>
where
    Node: Copy,
{
    Hits {
        hubs: nodes.iter().map(|&node| (node, 0.0)).collect(),
        authorities: nodes.into_iter().map(|node| (node, 0.0)).collect(),
        iterations: 0,
        converged: true,
    }
}

fn score<Node>(scores: &[(Node, f64)], node: Node) -> Option<f64>
where
    Node: Copy + Eq,
{
    scores
        .iter()
        .find_map(|(candidate, value)| (*candidate == node).then_some(*value))
}

fn validate(max_iterations: usize, tolerance: f64) -> Result<()> {
    if max_iterations == 0 || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(GraphError::InvalidAlgorithmParameter {
            algorithm: "hits",
            parameter: "max_iterations/tolerance",
            value: String::from("must be finite, positive, and non-zero"),
        });
    }
    Ok(())
}
