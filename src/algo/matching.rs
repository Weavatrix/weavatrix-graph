use crate::IndexUndirectedGraphView;
use crate::Vec;
use alloc::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaximumMatching<Node> {
    pairs: Vec<(Node, Node)>,
}

impl<Node> MaximumMatching<Node> {
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

/// Computes a maximum-cardinality matching in a general undirected graph.
pub fn maximum_matching<G>(graph: &G) -> MaximumMatching<G::Node>
where
    G: IndexUndirectedGraphView,
{
    let (nodes, adjacency) = indexed(graph);
    let matching = Blossom::new(adjacency).solve();
    let pairs = matching
        .iter()
        .enumerate()
        .filter_map(|(left, right)| {
            let right = (*right)?;
            (left < right).then(|| Some((nodes[left]?, nodes[right]?)))?
        })
        .collect();
    MaximumMatching { pairs }
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
            .filter(|neighbor| *neighbor != slot)
            .collect();
    }
    (nodes, adjacency)
}

struct Blossom {
    adjacency: Vec<Vec<usize>>,
    matching: Vec<Option<usize>>,
    parent: Vec<Option<usize>>,
    base: Vec<usize>,
    used: Vec<bool>,
    contracted: Vec<bool>,
}

impl Blossom {
    fn new(adjacency: Vec<Vec<usize>>) -> Self {
        let bound = adjacency.len();
        Self {
            adjacency,
            matching: vec![None; bound],
            parent: vec![None; bound],
            base: (0..bound).collect(),
            used: vec![false; bound],
            contracted: vec![false; bound],
        }
    }

    fn solve(mut self) -> Vec<Option<usize>> {
        self.seed_greedy();
        for root in 0..self.adjacency.len() {
            if self.matching[root].is_none() {
                self.find_augmenting_path(root);
            }
        }
        self.matching
    }

    fn seed_greedy(&mut self) {
        for left in 0..self.adjacency.len() {
            if self.matching[left].is_some() {
                continue;
            }
            let right = self.adjacency[left]
                .iter()
                .copied()
                .find(|right| self.matching[*right].is_none());
            if let Some(right) = right {
                self.matching[left] = Some(right);
                self.matching[right] = Some(left);
            }
        }
    }

    fn find_augmenting_path(&mut self, root: usize) -> bool {
        self.used.fill(false);
        self.parent.fill(None);
        for (slot, base) in self.base.iter_mut().enumerate() {
            *base = slot;
        }
        let mut queue = VecDeque::from([root]);
        self.used[root] = true;
        while let Some(node) = queue.pop_front() {
            for index in 0..self.adjacency[node].len() {
                let neighbor = self.adjacency[node][index];
                if self.base[node] == self.base[neighbor] || self.matching[node] == Some(neighbor) {
                    continue;
                }
                if neighbor == root
                    || self.matching[neighbor]
                        .and_then(|matched| self.parent[matched])
                        .is_some()
                {
                    self.contract(node, neighbor, &mut queue);
                } else if self.parent[neighbor].is_none() {
                    self.parent[neighbor] = Some(node);
                    if self.matching[neighbor].is_none() {
                        self.augment(neighbor);
                        return true;
                    }
                    if let Some(matched) = self.matching[neighbor] {
                        self.used[matched] = true;
                        queue.push_back(matched);
                    }
                }
            }
        }
        false
    }

    fn contract(&mut self, left: usize, right: usize, queue: &mut VecDeque<usize>) {
        let base = self.lowest_common_base(left, right);
        self.contracted.fill(false);
        self.mark_path(left, right, base);
        self.mark_path(right, left, base);
        for node in 0..self.adjacency.len() {
            if self.contracted[self.base[node]] {
                self.base[node] = base;
                if !self.used[node] {
                    self.used[node] = true;
                    queue.push_back(node);
                }
            }
        }
    }

    fn lowest_common_base(&self, mut left: usize, mut right: usize) -> usize {
        let mut path = vec![false; self.adjacency.len()];
        loop {
            left = self.base[left];
            path[left] = true;
            let Some(matched) = self.matching[left] else {
                break;
            };
            let Some(parent) = self.parent[matched] else {
                break;
            };
            left = parent;
        }
        loop {
            right = self.base[right];
            if path[right] {
                return right;
            }
            let Some(matched) = self.matching[right] else {
                return right;
            };
            let Some(parent) = self.parent[matched] else {
                return right;
            };
            right = parent;
        }
    }

    fn mark_path(&mut self, mut node: usize, mut child: usize, base: usize) {
        while self.base[node] != base {
            let Some(matched) = self.matching[node] else {
                break;
            };
            self.contracted[self.base[node]] = true;
            self.contracted[self.base[matched]] = true;
            self.parent[node] = Some(child);
            child = matched;
            let Some(parent) = self.parent[matched] else {
                break;
            };
            node = parent;
        }
    }

    fn augment(&mut self, mut node: usize) {
        while let Some(parent) = self.parent[node] {
            let next = self.matching[parent];
            self.matching[node] = Some(parent);
            self.matching[parent] = Some(node);
            let Some(next) = next else {
                break;
            };
            node = next;
        }
    }
}
