use crate::{IndexUndirectedGraphView, Vec};

pub(super) struct UndirectedNeighbors<G>
where
    G: IndexUndirectedGraphView,
{
    nodes: Vec<G::Node>,
    offsets: Vec<usize>,
    neighbors: Vec<G::Node>,
}

impl<G> UndirectedNeighbors<G>
where
    G: IndexUndirectedGraphView,
{
    pub(super) fn new<F>(graph: &G, allows_edge: F) -> Self
    where
        F: Fn(G::Edge) -> bool,
    {
        let mut nodes = graph.node_indices().collect::<Vec<_>>();
        nodes.sort_unstable_by_key(|node| G::node_slot(*node));
        let mut allowed = vec![false; graph.edge_bound()];
        for edge in graph.edge_indices() {
            allowed[G::edge_slot(edge)] = allows_edge(edge);
        }
        let mut offsets = vec![0; graph.node_bound() + 1];
        for edge in graph.edge_indices() {
            if !allowed[G::edge_slot(edge)] {
                continue;
            }
            let Some(endpoints) = graph.edge_endpoints(edge) else {
                continue;
            };
            let source = G::node_slot(endpoints.source());
            let target = G::node_slot(endpoints.target());
            offsets[source + 1] += 1;
            if source != target {
                offsets[target + 1] += 1;
            }
        }
        for slot in 1..offsets.len() {
            offsets[slot] += offsets[slot - 1];
        }
        let mut cursors = offsets[..graph.node_bound()].to_vec();
        let mut neighbors = vec![None; offsets.last().copied().unwrap_or_default()];
        for edge in graph.edge_indices() {
            if !allowed[G::edge_slot(edge)] {
                continue;
            }
            let Some(endpoints) = graph.edge_endpoints(edge) else {
                continue;
            };
            let source = G::node_slot(endpoints.source());
            let target = G::node_slot(endpoints.target());
            neighbors[cursors[source]] = Some(endpoints.target());
            cursors[source] += 1;
            if source != target {
                neighbors[cursors[target]] = Some(endpoints.source());
                cursors[target] += 1;
            }
        }
        let Some(neighbors) = neighbors.into_iter().collect::<Option<Vec<_>>>() else {
            return Self {
                nodes,
                offsets: vec![0; graph.node_bound() + 1],
                neighbors: Vec::new(),
            };
        };
        Self {
            nodes,
            offsets,
            neighbors,
        }
    }

    pub(super) fn nodes(&self) -> &[G::Node] {
        &self.nodes
    }

    pub(super) fn neighbors(&self, node: G::Node) -> &[G::Node] {
        let slot = G::node_slot(node);
        &self.neighbors[self.offsets[slot]..self.offsets[slot + 1]]
    }
}
