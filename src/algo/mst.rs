use crate::IndexUndirectedGraphView;
use crate::Vec;
use alloc::collections::BinaryHeap;
use core::cmp::Reverse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanningForest<Edge> {
    edges: Vec<Edge>,
    total_weight: u128,
    component_count: usize,
}

impl<Edge> SpanningForest<Edge> {
    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    #[must_use]
    pub const fn total_weight(&self) -> u128 {
        self.total_weight
    }

    #[must_use]
    pub const fn component_count(&self) -> usize {
        self.component_count
    }

    #[must_use]
    pub fn into_edges(self) -> Vec<Edge> {
        self.edges
    }
}

pub fn minimum_spanning_forest<G, F>(graph: &G, mut edge_weight: F) -> SpanningForest<G::Edge>
where
    G: IndexUndirectedGraphView,
    F: FnMut(G::Edge) -> u64,
{
    let mut weighted = graph
        .edge_indices()
        .map(|edge| (edge_weight(edge), G::edge_slot(edge), edge))
        .collect::<Vec<_>>();
    weighted.sort_unstable_by_key(|&(weight, slot, _)| (weight, slot));

    let mut sets = DisjointSets::new(graph.node_bound());
    let mut selected = Vec::with_capacity(graph.node_count().saturating_sub(1));
    let mut total_weight = 0_u128;
    let mut component_count = graph.node_count();
    for (weight, _, edge) in weighted {
        let Some(endpoints) = graph.edge_endpoints(edge) else {
            continue;
        };
        let source = G::node_slot(endpoints.source());
        let target = G::node_slot(endpoints.target());
        if sets.union(source, target) {
            selected.push(edge);
            total_weight += u128::from(weight);
            component_count -= 1;
        }
    }
    SpanningForest {
        edges: selected,
        total_weight,
        component_count,
    }
}

#[must_use]
pub fn prim_spanning_forest<G, F>(graph: &G, mut edge_weight: F) -> SpanningForest<G::Edge>
where
    G: IndexUndirectedGraphView,
    F: FnMut(G::Edge) -> u64,
{
    let mut edges = vec![None; graph.edge_bound()];
    let mut weights = vec![0_u64; graph.edge_bound()];
    for edge in graph.edge_indices() {
        let slot = G::edge_slot(edge);
        edges[slot] = Some(edge);
        weights[slot] = edge_weight(edge);
    }
    let mut seen = vec![false; graph.node_bound()];
    let mut selected = Vec::with_capacity(graph.node_count().saturating_sub(1));
    let mut total_weight = 0_u128;
    let mut component_count = 0;
    let mut queue = BinaryHeap::new();
    for root in graph.node_indices() {
        let root_slot = G::node_slot(root);
        if seen[root_slot] {
            continue;
        }
        component_count += 1;
        visit_prim::<G>(graph, root, &weights, &mut seen, &mut queue);
        while let Some(Reverse((weight, edge_slot, target_slot))) = queue.pop() {
            if seen[target_slot] {
                continue;
            }
            let Some(edge) = edges[edge_slot] else {
                continue;
            };
            let Some(endpoints) = graph.edge_endpoints(edge) else {
                continue;
            };
            let target = if G::node_slot(endpoints.source()) == target_slot {
                endpoints.source()
            } else {
                endpoints.target()
            };
            if seen[G::node_slot(target)] {
                continue;
            }
            selected.push(edge);
            total_weight += u128::from(weight);
            visit_prim::<G>(graph, target, &weights, &mut seen, &mut queue);
        }
    }
    SpanningForest {
        edges: selected,
        total_weight,
        component_count,
    }
}

fn visit_prim<G>(
    graph: &G,
    node: G::Node,
    weights: &[u64],
    seen: &mut [bool],
    queue: &mut BinaryHeap<Reverse<(u64, usize, usize)>>,
) where
    G: IndexUndirectedGraphView,
{
    seen[G::node_slot(node)] = true;
    for edge in graph.incident_edges(node) {
        let Some(target) = graph.opposite(edge, node) else {
            continue;
        };
        let target_slot = G::node_slot(target);
        if !seen[target_slot] {
            let edge_slot = G::edge_slot(edge);
            queue.push(Reverse((weights[edge_slot], edge_slot, target_slot)));
        }
    }
}

struct DisjointSets {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl DisjointSets {
    fn new(bound: usize) -> Self {
        Self {
            parent: (0..bound).collect(),
            rank: vec![0; bound],
        }
    }

    fn find(&mut self, mut node: usize) -> usize {
        let mut root = node;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        while self.parent[node] != node {
            let parent = self.parent[node];
            self.parent[node] = root;
            node = parent;
        }
        root
    }

    fn union(&mut self, left: usize, right: usize) -> bool {
        let mut left = self.find(left);
        let mut right = self.find(right);
        if left == right {
            return false;
        }
        if self.rank[left] < self.rank[right] {
            core::mem::swap(&mut left, &mut right);
        }
        self.parent[right] = left;
        if self.rank[left] == self.rank[right] {
            self.rank[left] = self.rank[left].saturating_add(1);
        }
        true
    }
}
