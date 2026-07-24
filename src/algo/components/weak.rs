use crate::IndexGraphView;
use crate::Vec;

#[must_use]
pub fn weakly_connected_components<G>(graph: &G) -> Vec<Vec<G::Node>>
where
    G: IndexGraphView,
{
    with_filter(graph, &|_| true)
}

#[must_use]
pub fn weakly_connected_components_filtered<G, F>(graph: &G, allows_edge: F) -> Vec<Vec<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    with_filter(graph, &allows_edge)
}

fn with_filter<G, F>(graph: &G, allows_edge: &F) -> Vec<Vec<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    let node_bound = graph.node_bound();
    let mut sets = DisjointSet::new(node_bound);
    for (edge, endpoints) in graph.edge_references() {
        if !allows_edge(edge) {
            continue;
        }
        let source = G::node_slot(endpoints.source());
        let target = G::node_slot(endpoints.target());
        if source < node_bound && target < node_bound {
            sets.union(source, target);
        }
    }

    let mut grouped = vec![Vec::new(); node_bound];
    for node in graph.node_indices() {
        let slot = G::node_slot(node);
        if slot < node_bound {
            grouped[sets.root(slot)].push(node);
        }
    }
    let mut components = grouped
        .into_iter()
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>();
    components.sort_unstable_by_key(|component| G::node_slot(component[0]));
    components
}

struct DisjointSet {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl DisjointSet {
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
            rank: vec![0; len],
        }
    }

    fn find(&mut self, mut node: usize) -> usize {
        loop {
            let parent = self.parent[node];
            if parent == node {
                return node;
            }
            self.parent[node] = self.parent[parent];
            node = parent;
        }
    }

    fn root(&self, mut node: usize) -> usize {
        loop {
            let parent = self.parent[node];
            if parent == node {
                return node;
            }
            node = parent;
        }
    }

    fn union(&mut self, left: usize, right: usize) {
        let mut left = self.find(left);
        let mut right = self.find(right);
        if left == right {
            return;
        }
        if self.rank[left] < self.rank[right] {
            core::mem::swap(&mut left, &mut right);
        }
        self.parent[right] = left;
        if self.rank[left] == self.rank[right] {
            self.rank[left] += 1;
        }
    }
}
