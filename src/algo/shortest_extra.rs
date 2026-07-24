use super::{BellmanFord, WeightedPath};
use crate::Vec;
use crate::{GraphError, IndexGraphView, Result};
use alloc::collections::{BinaryHeap, VecDeque};
use core::cmp::Reverse;

pub fn bidirectional_dijkstra<G, F>(
    graph: &G,
    source: G::Node,
    target: G::Node,
    edge_cost: F,
) -> Option<WeightedPath<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> u64,
{
    if !graph.contains_node(source) || !graph.contains_node(target) {
        return None;
    }
    if source == target {
        return Some(WeightedPath::from_parts(vec![source], 0));
    }
    let bound = graph.node_bound();
    let mut by_slot = vec![None; bound];
    for node in graph.node_indices() {
        by_slot[G::node_slot(node)] = Some(node);
    }
    let mut forward = SearchSide::new(bound, G::node_slot(source));
    let mut backward = SearchSide::new(bound, G::node_slot(target));
    let mut best = None;
    while !forward.queue.is_empty() && !backward.queue.is_empty() {
        let forward_min = forward.queue.peek().map_or(u64::MAX, |entry| entry.0.0);
        let backward_min = backward.queue.peek().map_or(u64::MAX, |entry| entry.0.0);
        if best.is_some_and(|(cost, _)| forward_min.saturating_add(backward_min) >= cost) {
            break;
        }
        if forward_min <= backward_min {
            expand(
                graph,
                true,
                &edge_cost,
                &by_slot,
                &mut forward,
                &backward,
                &mut best,
            );
        } else {
            expand(
                graph,
                false,
                &edge_cost,
                &by_slot,
                &mut backward,
                &forward,
                &mut best,
            );
        }
    }
    let (cost, meeting) = best?;
    let nodes = reconstruct::<G>(
        source,
        target,
        meeting,
        &by_slot,
        &forward.parent,
        &backward.parent,
    )?;
    Some(WeightedPath::from_parts(nodes, cost))
}

/// Computes signed single-source paths with the queue-based SPFA algorithm.
///
/// # Errors
///
/// Returns an error for arithmetic overflow or a reachable negative cycle.
pub fn spfa<G, F>(graph: &G, source: G::Node, edge_cost: F) -> Result<Option<BellmanFord<G::Node>>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> i64,
{
    spfa_filtered(graph, source, |edge| Some(edge_cost(edge)))
}

/// Computes SPFA while omitting edges whose cost is `None`.
///
/// # Errors
///
/// Returns an error for arithmetic overflow or a reachable negative cycle.
pub fn spfa_filtered<G, F>(
    graph: &G,
    source: G::Node,
    edge_cost: F,
) -> Result<Option<BellmanFord<G::Node>>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> Option<i64>,
{
    if !graph.contains_node(source) {
        return Ok(None);
    }
    let nodes = graph.node_indices().collect::<Vec<_>>();
    let mut by_slot = vec![None; graph.node_bound()];
    for &node in &nodes {
        by_slot[G::node_slot(node)] = Some(node);
    }
    let mut distance = vec![0_i64; graph.node_bound()];
    let mut reachable = vec![false; graph.node_bound()];
    let mut parent = vec![None; graph.node_bound()];
    let mut queued = vec![false; graph.node_bound()];
    let mut relaxations = vec![0_usize; graph.node_bound()];
    let source_slot = G::node_slot(source);
    reachable[source_slot] = true;
    queued[source_slot] = true;
    let mut queue = VecDeque::from([source]);
    while let Some(node) = queue.pop_front() {
        let node_slot = G::node_slot(node);
        queued[node_slot] = false;
        for edge in graph.outgoing_edges(node) {
            let Some(weight) = edge_cost(edge) else {
                continue;
            };
            let Some(endpoints) = graph.edge_endpoints(edge) else {
                continue;
            };
            let target = endpoints.target();
            let target_slot = G::node_slot(target);
            let candidate =
                distance[node_slot]
                    .checked_add(weight)
                    .ok_or(GraphError::ArithmeticOverflow {
                        operation: "SPFA relaxation",
                    })?;
            if !reachable[target_slot] || candidate < distance[target_slot] {
                reachable[target_slot] = true;
                distance[target_slot] = candidate;
                parent[target_slot] = Some(node_slot);
                relaxations[target_slot] += 1;
                if relaxations[target_slot] >= nodes.len() {
                    return Err(GraphError::NegativeCycle { algorithm: "SPFA" });
                }
                if !queued[target_slot] {
                    queued[target_slot] = true;
                    queue.push_back(target);
                }
            }
        }
    }
    Ok(Some(BellmanFord::from_parts(
        source,
        nodes,
        by_slot,
        distance,
        reachable,
        parent,
        G::node_slot,
    )))
}

struct SearchSide {
    distance: Vec<u64>,
    parent: Vec<Option<usize>>,
    settled: Vec<bool>,
    queue: BinaryHeap<Reverse<(u64, usize)>>,
}

impl SearchSide {
    fn new(bound: usize, start: usize) -> Self {
        let mut distance = vec![u64::MAX; bound];
        distance[start] = 0;
        Self {
            distance,
            parent: vec![None; bound],
            settled: vec![false; bound],
            queue: BinaryHeap::from([Reverse((0, start))]),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn expand<G, F>(
    graph: &G,
    outgoing: bool,
    edge_cost: &F,
    by_slot: &[Option<G::Node>],
    side: &mut SearchSide,
    other: &SearchSide,
    best: &mut Option<(u64, usize)>,
) where
    G: IndexGraphView,
    F: Fn(G::Edge) -> u64,
{
    let Some(Reverse((cost, slot))) = side.queue.pop() else {
        return;
    };
    if cost != side.distance[slot] || side.settled[slot] {
        return;
    }
    side.settled[slot] = true;
    update_best(slot, side, other, best);
    let Some(node) = by_slot[slot] else {
        return;
    };
    let edges = if outgoing {
        graph.outgoing_edges(node).collect::<Vec<_>>()
    } else {
        graph.incoming_edges(node).collect::<Vec<_>>()
    };
    for edge in edges {
        let Some(endpoints) = graph.edge_endpoints(edge) else {
            continue;
        };
        let neighbor = if outgoing {
            endpoints.target()
        } else {
            endpoints.source()
        };
        let neighbor_slot = G::node_slot(neighbor);
        let Some(candidate) = cost.checked_add(edge_cost(edge)) else {
            continue;
        };
        if candidate < side.distance[neighbor_slot] {
            side.distance[neighbor_slot] = candidate;
            side.parent[neighbor_slot] = Some(slot);
            side.queue.push(Reverse((candidate, neighbor_slot)));
            update_best(neighbor_slot, side, other, best);
        }
    }
}

fn update_best(
    slot: usize,
    side: &SearchSide,
    other: &SearchSide,
    best: &mut Option<(u64, usize)>,
) {
    if side.distance[slot] == u64::MAX || other.distance[slot] == u64::MAX {
        return;
    }
    let Some(cost) = side.distance[slot].checked_add(other.distance[slot]) else {
        return;
    };
    if best.is_none_or(|current| cost < current.0 || (cost == current.0 && slot < current.1)) {
        *best = Some((cost, slot));
    }
}

fn reconstruct<G: IndexGraphView>(
    source: G::Node,
    target: G::Node,
    meeting: usize,
    by_slot: &[Option<G::Node>],
    forward: &[Option<usize>],
    backward: &[Option<usize>],
) -> Option<Vec<G::Node>> {
    let mut slots = vec![meeting];
    while *slots.last()? != G::node_slot(source) {
        slots.push(forward[*slots.last()?]?);
    }
    slots.reverse();
    while *slots.last()? != G::node_slot(target) {
        slots.push(backward[*slots.last()?]?);
    }
    slots
        .into_iter()
        .map(|slot| by_slot.get(slot).copied().flatten())
        .collect::<Option<Vec<_>>>()
}
