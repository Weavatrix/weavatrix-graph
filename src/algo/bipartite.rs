use crate::IndexUndirectedGraphView;
use crate::Vec;
use alloc::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BipartitePartition<Node> {
    left: Vec<Node>,
    right: Vec<Node>,
}

impl<Node> BipartitePartition<Node> {
    #[must_use]
    pub fn left(&self) -> &[Node] {
        &self.left
    }

    #[must_use]
    pub fn right(&self) -> &[Node] {
        &self.right
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BipartiteMatching<Node> {
    pairs: Vec<(Node, Node)>,
}

impl<Node> BipartiteMatching<Node> {
    #[must_use]
    pub fn pairs(&self) -> &[(Node, Node)] {
        &self.pairs
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.pairs.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

pub fn bipartite_partition<G>(graph: &G) -> Option<BipartitePartition<G::Node>>
where
    G: IndexUndirectedGraphView,
{
    let (nodes, adjacency) = indexed(graph);
    let colors = color(&nodes, &adjacency)?;
    let mut left = Vec::new();
    let mut right = Vec::new();
    for node in nodes.into_iter().flatten() {
        if colors[G::node_slot(node)] == Some(false) {
            left.push(node);
        } else {
            right.push(node);
        }
    }
    Some(BipartitePartition { left, right })
}

pub fn maximum_bipartite_matching<G>(graph: &G) -> Option<BipartiteMatching<G::Node>>
where
    G: IndexUndirectedGraphView,
{
    let (nodes, adjacency) = indexed(graph);
    let colors = color(&nodes, &adjacency)?;
    let left = nodes
        .iter()
        .enumerate()
        .filter_map(|(slot, node)| (node.is_some() && colors[slot] == Some(false)).then_some(slot))
        .collect::<Vec<_>>();
    let mut matching = vec![None; graph.node_bound()];
    loop {
        let distances = layers(&left, &adjacency, &matching);
        let mut changed = false;
        for &node in &left {
            if matching[node].is_none() && augment(node, &adjacency, &distances, &mut matching) {
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let pairs = left
        .into_iter()
        .filter_map(|left| Some((nodes[left]?, nodes[matching[left]?]?)))
        .collect();
    Some(BipartiteMatching { pairs })
}

fn indexed<G: IndexUndirectedGraphView>(graph: &G) -> (Vec<Option<G::Node>>, Vec<Vec<usize>>) {
    let mut nodes = vec![None; graph.node_bound()];
    let mut adjacency = vec![Vec::new(); graph.node_bound()];
    for node in graph.node_indices() {
        let slot = G::node_slot(node);
        nodes[slot] = Some(node);
        adjacency[slot] = graph
            .incident_edges(node)
            .filter_map(|edge| graph.opposite(edge, node))
            .map(G::node_slot)
            .collect();
        adjacency[slot].sort_unstable();
        adjacency[slot].dedup();
    }
    (nodes, adjacency)
}

fn color<Node>(nodes: &[Option<Node>], adjacency: &[Vec<usize>]) -> Option<Vec<Option<bool>>> {
    let mut colors = vec![None; nodes.len()];
    for start in 0..nodes.len() {
        if nodes[start].is_none() || colors[start].is_some() {
            continue;
        }
        colors[start] = Some(false);
        let mut queue = VecDeque::from([start]);
        while let Some(node) = queue.pop_front() {
            let current = colors[node]?;
            for &neighbor in &adjacency[node] {
                if neighbor == node {
                    return None;
                }
                if let Some(neighbor_color) = colors[neighbor] {
                    if neighbor_color == current {
                        return None;
                    }
                } else {
                    colors[neighbor] = Some(!current);
                    queue.push_back(neighbor);
                }
            }
        }
    }
    Some(colors)
}

fn layers(left: &[usize], adjacency: &[Vec<usize>], matching: &[Option<usize>]) -> Vec<usize> {
    let mut distances = vec![usize::MAX; adjacency.len()];
    let mut queue = VecDeque::new();
    for &node in left {
        if matching[node].is_none() {
            distances[node] = 0;
            queue.push_back(node);
        }
    }
    while let Some(node) = queue.pop_front() {
        for &right in &adjacency[node] {
            if let Some(next) = matching[right]
                && distances[next] == usize::MAX
            {
                distances[next] = distances[node] + 1;
                queue.push_back(next);
            }
        }
    }
    distances
}

fn augment(
    node: usize,
    adjacency: &[Vec<usize>],
    distances: &[usize],
    matching: &mut [Option<usize>],
) -> bool {
    for &right in &adjacency[node] {
        let can_use = matching[right].is_none_or(|next| {
            distances[next] == distances[node] + 1 && augment(next, adjacency, distances, matching)
        });
        if can_use {
            matching[node] = Some(right);
            matching[right] = Some(node);
            return true;
        }
    }
    false
}
