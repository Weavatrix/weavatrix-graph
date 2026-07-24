use crate::IndexUndirectedGraphView;
use crate::Vec;
use alloc::collections::{BTreeSet, BinaryHeap};
use core::cmp::Reverse;

type Indexed<Node> = (Vec<Option<Node>>, Vec<Vec<usize>>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coloring<Node> {
    assignments: Vec<(Node, usize)>,
    color_count: usize,
}

impl<Node> Coloring<Node> {
    #[must_use]
    pub fn assignments(&self) -> &[(Node, usize)] {
        &self.assignments
    }

    #[must_use]
    pub const fn color_count(&self) -> usize {
        self.color_count
    }
}

/// Produces a deterministic DSATUR coloring.
///
/// DSATUR is exact for several common graph classes but is a heuristic for the
/// general NP-hard minimum-coloring problem.
pub fn dsatur_coloring<G>(graph: &G) -> Option<Coloring<G::Node>>
where
    G: IndexUndirectedGraphView,
{
    let (nodes, adjacency) = indexed(graph)?;
    let mut colors = vec![None; graph.node_bound()];
    let mut adjacent_colors = vec![BTreeSet::new(); graph.node_bound()];
    let mut queue = BinaryHeap::new();
    for (slot, node) in nodes.iter().enumerate() {
        if node.is_some() {
            queue.push((0, adjacency[slot].len(), Reverse(slot)));
        }
    }
    while let Some((saturation, _, Reverse(node))) = queue.pop() {
        if colors[node].is_some() || saturation != adjacent_colors[node].len() {
            continue;
        }
        let color = (0..=adjacency.len())
            .find(|color| !adjacent_colors[node].contains(color))
            .unwrap_or(0);
        colors[node] = Some(color);
        for &neighbor in &adjacency[node] {
            if colors[neighbor].is_none() && adjacent_colors[neighbor].insert(color) {
                queue.push((
                    adjacent_colors[neighbor].len(),
                    adjacency[neighbor].len(),
                    Reverse(neighbor),
                ));
            }
        }
    }
    let assignments = nodes
        .into_iter()
        .enumerate()
        .filter_map(|(slot, node)| Some((node?, colors[slot]?)))
        .collect::<Vec<_>>();
    let color_count = assignments
        .iter()
        .map(|(_, color)| color + 1)
        .max()
        .unwrap_or(0);
    Some(Coloring {
        assignments,
        color_count,
    })
}

fn indexed<G: IndexUndirectedGraphView>(graph: &G) -> Option<Indexed<G::Node>> {
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
        if adjacency[slot].contains(&slot) {
            return None;
        }
        adjacency[slot].sort_unstable();
        adjacency[slot].dedup();
    }
    Some((nodes, adjacency))
}
