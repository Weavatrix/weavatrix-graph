use super::adjacency::{SlotAdjacency, adjacency};
use crate::algo::traversal::Direction;
use crate::{IndexGraphView, Vec};
use alloc::collections::{BinaryHeap, VecDeque};
use core::cmp::Reverse;

#[must_use]
pub fn k_core_numbers<G>(graph: &G) -> Vec<(G::Node, usize)>
where
    G: IndexGraphView,
{
    let adjacent = adjacency(graph, Direction::Both);
    let mut degree = adjacent.neighbors.iter().map(Vec::len).collect::<Vec<_>>();
    let mut removed = vec![false; graph.node_bound()];
    let mut core = vec![0; graph.node_bound()];
    let mut queue = BinaryHeap::new();
    for &node in &adjacent.nodes {
        let slot = G::node_slot(node);
        queue.push(Reverse((degree[slot], slot)));
    }
    let mut level = 0;
    while let Some(Reverse((candidate, slot))) = queue.pop() {
        if removed[slot] || degree[slot] != candidate {
            continue;
        }
        removed[slot] = true;
        level = level.max(candidate);
        core[slot] = level;
        for &neighbor in &adjacent.neighbors[slot] {
            if !removed[neighbor] {
                degree[neighbor] = degree[neighbor].saturating_sub(1);
                queue.push(Reverse((degree[neighbor], neighbor)));
            }
        }
    }
    adjacent
        .nodes
        .iter()
        .copied()
        .map(|node| (node, core[G::node_slot(node)]))
        .collect()
}

/// Returns a deterministic fundamental cycle basis of the undirected projection.
#[must_use]
pub fn cycle_basis<G>(graph: &G) -> Vec<Vec<G::Node>>
where
    G: IndexGraphView,
{
    let adjacent = adjacency(graph, Direction::Both);
    let bound = graph.node_bound();
    let mut parent = vec![None; bound];
    let mut depth = vec![usize::MAX; bound];
    let mut tree_edges = Vec::new();
    for &root_node in &adjacent.nodes {
        let root = G::node_slot(root_node);
        if depth[root] != usize::MAX {
            continue;
        }
        depth[root] = 0;
        let mut queue = VecDeque::from([root]);
        while let Some(node) = queue.pop_front() {
            for &neighbor in &adjacent.neighbors[node] {
                if depth[neighbor] == usize::MAX {
                    depth[neighbor] = depth[node] + 1;
                    parent[neighbor] = Some(node);
                    tree_edges.push(ordered_pair(node, neighbor));
                    queue.push_back(neighbor);
                }
            }
        }
    }
    tree_edges.sort_unstable();
    let mut edges = Vec::new();
    for &node in &adjacent.nodes {
        let source = G::node_slot(node);
        for &target in &adjacent.neighbors[source] {
            if source < target {
                edges.push((source, target));
            }
        }
    }
    edges.sort_unstable();
    edges.dedup();
    edges
        .into_iter()
        .filter(|edge| tree_edges.binary_search(edge).is_err())
        .filter_map(|(source, target)| {
            fundamental_cycle(&adjacent, source, target, &parent, &depth)
        })
        .collect()
}

fn fundamental_cycle<Node>(
    adjacent: &SlotAdjacency<Node>,
    mut left: usize,
    mut right: usize,
    parent: &[Option<usize>],
    depth: &[usize],
) -> Option<Vec<Node>>
where
    Node: Copy,
{
    let mut left_path = vec![left];
    let mut right_path = vec![right];
    while depth[left] > depth[right] {
        left = parent[left]?;
        left_path.push(left);
    }
    while depth[right] > depth[left] {
        right = parent[right]?;
        right_path.push(right);
    }
    while left != right {
        left = parent[left]?;
        right = parent[right]?;
        left_path.push(left);
        right_path.push(right);
    }
    right_path.pop();
    left_path.extend(right_path.into_iter().rev());
    left_path
        .into_iter()
        .map(|slot| adjacent.node(slot))
        .collect()
}

const fn ordered_pair(left: usize, right: usize) -> (usize, usize) {
    if left < right {
        (left, right)
    } else {
        (right, left)
    }
}
