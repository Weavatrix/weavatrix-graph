use super::measure::Measure;
use super::traversal::{Direction, for_each_adjacent};
use super::walk::{DijkstraWorkspace, dijkstra_iter_filtered};
use crate::Vec;
use crate::{IndexGraphView, Result};
use alloc::collections::BinaryHeap;
use core::cmp::Reverse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightedPath<Node, Cost = u64> {
    nodes: Vec<Node>,
    total_cost: Cost,
}

impl<Node, Cost> WeightedPath<Node, Cost> {
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

/// Computes a shortest path with an arbitrary checked measure.
///
/// # Errors
///
/// Returns an error for negative, non-finite, or overflowing path costs.
pub fn dijkstra_measure<G, Cost, F>(
    graph: &G,
    source: G::Node,
    target: G::Node,
    mut edge_cost: F,
) -> Result<Option<WeightedPath<G::Node, Cost>>>
where
    G: IndexGraphView,
    Cost: Measure,
    F: FnMut(G::Edge) -> Cost,
{
    dijkstra_measure_filtered(graph, source, target, Direction::Outgoing, |edge| {
        Some(edge_cost(edge))
    })
}

/// Computes a filtered shortest path with an arbitrary checked measure.
///
/// # Errors
///
/// Returns an error for negative, non-finite, or overflowing path costs.
pub fn dijkstra_measure_filtered<G, Cost, F>(
    graph: &G,
    source: G::Node,
    target: G::Node,
    direction: Direction,
    edge_cost: F,
) -> Result<Option<WeightedPath<G::Node, Cost>>>
where
    G: IndexGraphView,
    Cost: Measure,
    F: FnMut(G::Edge) -> Option<Cost>,
{
    if !graph.contains_node(source) || !graph.contains_node(target) {
        return Ok(None);
    }
    let mut workspace = DijkstraWorkspace::new();
    let mut target_cost = None;
    {
        let search = dijkstra_iter_filtered(graph, source, direction, &mut workspace, edge_cost);
        for settled in search {
            let (node, cost) = settled?;
            if node == target {
                target_cost = Some(cost);
                break;
            }
        }
    }
    let Some(total_cost) = target_cost else {
        return Ok(None);
    };
    Ok(workspace
        .path_to::<G>(source, target)
        .map(|nodes| WeightedPath::from_parts(nodes, total_cost)))
}

pub fn dijkstra<G, F>(
    graph: &G,
    source: G::Node,
    target: G::Node,
    mut edge_cost: F,
) -> Option<WeightedPath<G::Node>>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> u64,
{
    dijkstra_filtered(graph, source, target, Direction::Outgoing, |edge| {
        Some(edge_cost(edge))
    })
}

pub fn dijkstra_filtered<G, F>(
    graph: &G,
    source: G::Node,
    target: G::Node,
    direction: Direction,
    mut edge_cost: F,
) -> Option<WeightedPath<G::Node>>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> Option<u64>,
{
    if !graph.contains_node(source) || !graph.contains_node(target) {
        return None;
    }
    let bound = graph.node_bound();
    let mut nodes = vec![None; bound];
    for node in graph.node_indices() {
        nodes[G::node_slot(node)] = Some(node);
    }
    let mut costs = vec![u64::MAX; bound];
    let mut predecessor = vec![None; bound];
    let source_slot = G::node_slot(source);
    costs[source_slot] = 0;
    let mut queue = BinaryHeap::new();
    queue.push(Reverse((0_u64, source_slot)));

    while let Some(Reverse((cost, slot))) = queue.pop() {
        if cost != costs[slot] {
            continue;
        }
        let Some(node) = nodes[slot] else {
            continue;
        };
        if node == target {
            return Some(WeightedPath {
                nodes: reconstruct::<G>(source, target, &predecessor)?,
                total_cost: cost,
            });
        }
        relax_neighbors(
            graph,
            node,
            direction,
            cost,
            &mut edge_cost,
            &mut costs,
            &mut predecessor,
            &mut queue,
        );
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn relax_neighbors<G, F>(
    graph: &G,
    node: G::Node,
    direction: Direction,
    cost: u64,
    edge_cost: &mut F,
    costs: &mut [u64],
    predecessor: &mut [Option<G::Node>],
    queue: &mut BinaryHeap<Reverse<(u64, usize)>>,
) where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> Option<u64>,
{
    for_each_adjacent(graph, node, direction, &mut |_| true, |edge, neighbor| {
        let Some(weight) = edge_cost(edge) else {
            return;
        };
        let Some(candidate) = cost.checked_add(weight) else {
            return;
        };
        let slot = G::node_slot(neighbor);
        if candidate < costs[slot] {
            costs[slot] = candidate;
            predecessor[slot] = Some(node);
            queue.push(Reverse((candidate, slot)));
        }
    });
}

pub(super) fn reconstruct<G: IndexGraphView>(
    source: G::Node,
    target: G::Node,
    predecessor: &[Option<G::Node>],
) -> Option<Vec<G::Node>> {
    let mut path = vec![target];
    let mut cursor = target;
    while cursor != source {
        cursor = predecessor[G::node_slot(cursor)]?;
        path.push(cursor);
    }
    path.reverse();
    Some(path)
}
