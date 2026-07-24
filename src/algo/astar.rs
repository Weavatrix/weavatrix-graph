use super::shortest::{WeightedPath, reconstruct};
use super::traversal::{Direction, for_each_adjacent};
use crate::IndexGraphView;
use alloc::collections::BinaryHeap;
use core::cmp::Reverse;

pub fn astar<G, F, H>(
    graph: &G,
    source: G::Node,
    target: G::Node,
    mut edge_cost: F,
    estimate_cost: H,
) -> Option<WeightedPath<G::Node>>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> u64,
    H: FnMut(G::Node) -> u64,
{
    astar_filtered(
        graph,
        source,
        target,
        Direction::Outgoing,
        |edge| Some(edge_cost(edge)),
        estimate_cost,
    )
}

pub fn astar_filtered<G, F, H>(
    graph: &G,
    source: G::Node,
    target: G::Node,
    direction: Direction,
    mut edge_cost: F,
    mut estimate_cost: H,
) -> Option<WeightedPath<G::Node>>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> Option<u64>,
    H: FnMut(G::Node) -> u64,
{
    if !graph.contains_node(source) || !graph.contains_node(target) {
        return None;
    }
    let mut nodes = vec![None; graph.node_bound()];
    for node in graph.node_indices() {
        nodes[G::node_slot(node)] = Some(node);
    }
    let mut costs = vec![u64::MAX; graph.node_bound()];
    let mut predecessor = vec![None; graph.node_bound()];
    let source_slot = G::node_slot(source);
    costs[source_slot] = 0;
    let source_estimate = estimate_cost(source);
    let mut queue = BinaryHeap::new();
    queue.push(Reverse((
        source_estimate,
        source_estimate,
        0_u64,
        source_slot,
    )));

    while let Some(Reverse((_, _, cost, slot))) = queue.pop() {
        if cost != costs[slot] {
            continue;
        }
        let Some(node) = nodes[slot] else {
            continue;
        };
        if node == target {
            return Some(WeightedPath::from_parts(
                reconstruct::<G>(source, target, &predecessor)?,
                cost,
            ));
        }
        relax_neighbors(
            graph,
            node,
            direction,
            cost,
            &mut edge_cost,
            &mut estimate_cost,
            &mut costs,
            &mut predecessor,
            &mut queue,
        );
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn relax_neighbors<G, F, H>(
    graph: &G,
    node: G::Node,
    direction: Direction,
    cost: u64,
    edge_cost: &mut F,
    estimate_cost: &mut H,
    costs: &mut [u64],
    predecessor: &mut [Option<G::Node>],
    queue: &mut BinaryHeap<Reverse<(u64, u64, u64, usize)>>,
) where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> Option<u64>,
    H: FnMut(G::Node) -> u64,
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
            let heuristic = estimate_cost(neighbor);
            costs[slot] = candidate;
            predecessor[slot] = Some(node);
            queue.push(Reverse((
                candidate.saturating_add(heuristic),
                heuristic,
                candidate,
                slot,
            )));
        }
    });
}
