use super::adjacency::adjacency;
use crate::algo::traversal::Direction;
use crate::{IndexGraphView, Vec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Communities<Node> {
    groups: Vec<Vec<Node>>,
    memberships: Vec<(Node, usize)>,
    iterations: usize,
    converged: bool,
}

impl<Node> Communities<Node> {
    #[must_use]
    pub fn groups(&self) -> &[Vec<Node>] {
        &self.groups
    }

    #[must_use]
    pub fn memberships(&self) -> &[(Node, usize)] {
        &self.memberships
    }

    #[must_use]
    pub const fn iterations(&self) -> usize {
        self.iterations
    }

    #[must_use]
    pub const fn converged(&self) -> bool {
        self.converged
    }
}

/// Deterministic asynchronous label propagation over the undirected projection.
#[must_use]
pub fn label_propagation_communities<G>(graph: &G, max_iterations: usize) -> Communities<G::Node>
where
    G: IndexGraphView,
{
    let adjacent = adjacency(graph, Direction::Both);
    let mut labels = (0..graph.node_bound()).collect::<Vec<_>>();
    let mut converged = adjacent.nodes.is_empty();
    let mut iterations = 0;
    for iteration in 1..=max_iterations {
        iterations = iteration;
        let mut changed = false;
        for &node in &adjacent.nodes {
            let slot = G::node_slot(node);
            let mut counts = Vec::<(usize, usize)>::new();
            for &neighbor in &adjacent.neighbors[slot] {
                let label = labels[neighbor];
                if let Some((_, count)) = counts.iter_mut().find(|entry| entry.0 == label) {
                    *count += 1;
                } else {
                    counts.push((label, 1));
                }
            }
            let selected = counts
                .into_iter()
                .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
                .map_or(labels[slot], |entry| entry.0);
            if selected != labels[slot] {
                labels[slot] = selected;
                changed = true;
            }
        }
        if !changed {
            converged = true;
            break;
        }
    }
    canonicalize(
        &adjacent.nodes,
        &labels,
        G::node_slot,
        iterations,
        converged,
    )
}

fn canonicalize<Node>(
    nodes: &[Node],
    labels: &[usize],
    node_slot: fn(Node) -> usize,
    iterations: usize,
    converged: bool,
) -> Communities<Node>
where
    Node: Copy,
{
    let mut unique = nodes
        .iter()
        .map(|node| labels[node_slot(*node)])
        .collect::<Vec<_>>();
    unique.sort_unstable();
    unique.dedup();
    let memberships = nodes
        .iter()
        .copied()
        .map(|node| {
            let label = labels[node_slot(node)];
            let community = unique.binary_search(&label).unwrap_or(0);
            (node, community)
        })
        .collect::<Vec<_>>();
    let mut groups = vec![Vec::new(); unique.len()];
    for &(node, community) in &memberships {
        groups[community].push(node);
    }
    Communities {
        groups,
        memberships,
        iterations,
        converged,
    }
}
