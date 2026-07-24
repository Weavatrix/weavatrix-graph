use super::measure::Measure;
use crate::Vec;
use crate::{GraphError, IndexGraphView, Result, String};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedPath<Node, Cost = i64> {
    nodes: Vec<Node>,
    total_cost: Cost,
}

impl<Node, Cost> SignedPath<Node, Cost> {
    pub(super) fn from_parts(nodes: Vec<Node>, total_cost: Cost) -> Self {
        Self { nodes, total_cost }
    }

    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    #[must_use]
    pub const fn total_cost(&self) -> Cost
    where
        Cost: Copy,
    {
        self.total_cost
    }

    #[must_use]
    pub fn into_nodes(self) -> Vec<Node> {
        self.nodes
    }
}

#[derive(Debug, Clone)]
pub struct BellmanFord<Node, Cost = i64> {
    source: Node,
    nodes: Vec<Node>,
    nodes_by_slot: Vec<Option<Node>>,
    distances: Vec<Cost>,
    reachable: Vec<bool>,
    predecessors: Vec<Option<usize>>,
    node_slot: fn(Node) -> usize,
}

impl<Node, Cost> BellmanFord<Node, Cost>
where
    Node: Copy + Eq,
    Cost: Copy,
{
    pub(super) fn from_parts(
        source: Node,
        nodes: Vec<Node>,
        nodes_by_slot: Vec<Option<Node>>,
        distances: Vec<Cost>,
        reachable: Vec<bool>,
        predecessors: Vec<Option<usize>>,
        node_slot: fn(Node) -> usize,
    ) -> Self {
        Self {
            source,
            nodes,
            nodes_by_slot,
            distances,
            reachable,
            predecessors,
            node_slot,
        }
    }

    #[must_use]
    pub const fn source(&self) -> Node {
        self.source
    }

    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    #[must_use]
    pub fn distance_to(&self, node: Node) -> Option<Cost> {
        let slot = (self.node_slot)(node);
        self.nodes_by_slot
            .get(slot)
            .is_some_and(|stored| *stored == Some(node))
            .then(|| self.reachable[slot])
            .filter(|reachable| *reachable)
            .map(|_| self.distances[slot])
    }

    #[must_use]
    pub fn predecessor(&self, node: Node) -> Option<Node> {
        let slot = (self.node_slot)(node);
        self.nodes_by_slot
            .get(slot)
            .is_some_and(|stored| *stored == Some(node))
            .then(|| self.predecessors[slot])
            .flatten()
            .and_then(|predecessor| self.nodes_by_slot[predecessor])
    }

    #[must_use]
    pub fn path_to(&self, target: Node) -> Option<SignedPath<Node, Cost>> {
        let total_cost = self.distance_to(target)?;
        let mut nodes = vec![target];
        let mut cursor = target;
        while cursor != self.source {
            cursor = self.predecessor(cursor)?;
            nodes.push(cursor);
            if nodes.len() > self.nodes.len() {
                return None;
            }
        }
        nodes.reverse();
        Some(SignedPath { nodes, total_cost })
    }
}

/// Computes signed shortest paths from `source`.
///
/// # Errors
///
/// Returns an error for arithmetic overflow or a reachable negative cycle.
pub fn bellman_ford<G, F>(
    graph: &G,
    source: G::Node,
    edge_cost: F,
) -> Result<Option<BellmanFord<G::Node>>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> i64,
{
    bellman_ford_filtered(graph, source, |edge| Some(edge_cost(edge)))
}

/// Computes signed shortest paths using only edges with a returned cost.
///
/// # Errors
///
/// Returns an error for arithmetic overflow or a reachable negative cycle.
pub fn bellman_ford_filtered<G, F>(
    graph: &G,
    source: G::Node,
    edge_cost: F,
) -> Result<Option<BellmanFord<G::Node>>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> Option<i64>,
{
    bellman_ford_measure_filtered(graph, source, edge_cost)
}

/// Computes shortest paths with an arbitrary signed or unsigned measure.
///
/// # Errors
///
/// Returns an error for non-finite costs, arithmetic overflow, or a reachable
/// negative cycle.
pub fn bellman_ford_measure<G, Cost, F>(
    graph: &G,
    source: G::Node,
    edge_cost: F,
) -> Result<Option<BellmanFord<G::Node, Cost>>>
where
    G: IndexGraphView,
    Cost: Measure,
    F: Fn(G::Edge) -> Cost,
{
    bellman_ford_measure_filtered(graph, source, |edge| Some(edge_cost(edge)))
}

/// Computes filtered shortest paths with an arbitrary checked measure.
///
/// # Errors
///
/// Returns an error for non-finite costs, arithmetic overflow, or a reachable
/// negative cycle.
pub fn bellman_ford_measure_filtered<G, Cost, F>(
    graph: &G,
    source: G::Node,
    edge_cost: F,
) -> Result<Option<BellmanFord<G::Node, Cost>>>
where
    G: IndexGraphView,
    Cost: Measure,
    F: Fn(G::Edge) -> Option<Cost>,
{
    if !graph.contains_node(source) {
        return Ok(None);
    }
    let mut nodes_by_slot = vec![None; graph.node_bound()];
    let nodes = graph.node_indices().collect::<Vec<_>>();
    for &node in &nodes {
        nodes_by_slot[G::node_slot(node)] = Some(node);
    }
    let mut edges = Vec::with_capacity(graph.edge_count());
    for (edge, endpoints) in graph.edge_references() {
        if let Some(weight) = edge_cost(edge) {
            if !weight.is_valid() {
                return Err(GraphError::InvalidAlgorithmParameter {
                    algorithm: "Bellman-Ford",
                    parameter: "edge_cost",
                    value: String::from("must be finite and totally ordered"),
                });
            }
            edges.push((
                G::node_slot(endpoints.source()),
                G::node_slot(endpoints.target()),
                weight,
            ));
        }
    }
    let mut distances = vec![Cost::zero(); graph.node_bound()];
    let mut reachable = vec![false; graph.node_bound()];
    let mut predecessors = vec![None; graph.node_bound()];
    reachable[G::node_slot(source)] = true;
    for _ in 1..nodes.len() {
        if !relax_all(&edges, &mut distances, &mut reachable, &mut predecessors)? {
            break;
        }
    }
    reject_negative_cycle(&edges, &distances, &reachable)?;

    Ok(Some(BellmanFord {
        source,
        nodes,
        nodes_by_slot,
        distances,
        reachable,
        predecessors,
        node_slot: G::node_slot,
    }))
}

fn relax_all<Cost: Measure>(
    edges: &[(usize, usize, Cost)],
    distances: &mut [Cost],
    reachable: &mut [bool],
    predecessors: &mut [Option<usize>],
) -> Result<bool> {
    let mut changed = false;
    for &(source, target, weight) in edges {
        if !reachable[source] {
            continue;
        }
        let candidate =
            distances[source]
                .checked_add(weight)
                .ok_or(GraphError::ArithmeticOverflow {
                    operation: "Bellman-Ford edge relaxation",
                })?;
        if !reachable[target]
            || candidate.compare(distances[target]) == Some(core::cmp::Ordering::Less)
        {
            distances[target] = candidate;
            reachable[target] = true;
            predecessors[target] = Some(source);
            changed = true;
        }
    }
    Ok(changed)
}

fn reject_negative_cycle<Cost: Measure>(
    edges: &[(usize, usize, Cost)],
    distances: &[Cost],
    reachable: &[bool],
) -> Result<()> {
    for &(source, target, weight) in edges {
        if !reachable[source] {
            continue;
        }
        let candidate =
            distances[source]
                .checked_add(weight)
                .ok_or(GraphError::ArithmeticOverflow {
                    operation: "Bellman-Ford cycle check",
                })?;
        if reachable[target]
            && candidate.compare(distances[target]) == Some(core::cmp::Ordering::Less)
        {
            return Err(GraphError::NegativeCycle {
                algorithm: "Bellman-Ford",
            });
        }
    }
    Ok(())
}
