use crate::{IndexUndirectedGraphView, Vec};

pub(super) struct UndirectedSnapshot<G>
where
    G: IndexUndirectedGraphView,
{
    nodes: Vec<G::Node>,
    offsets: Vec<usize>,
    edges: Vec<G::Edge>,
}

impl<G> UndirectedSnapshot<G>
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
        let mut incident = vec![None; *offsets.last().expect("offset sentinel")];
        for edge in graph.edge_indices() {
            if !allowed[G::edge_slot(edge)] {
                continue;
            }
            let Some(endpoints) = graph.edge_endpoints(edge) else {
                continue;
            };
            let source = G::node_slot(endpoints.source());
            let target = G::node_slot(endpoints.target());
            incident[cursors[source]] = Some(edge);
            cursors[source] += 1;
            if source != target {
                incident[cursors[target]] = Some(edge);
                cursors[target] += 1;
            }
        }
        let mut edges = incident
            .into_iter()
            .map(|edge| edge.expect("counted incident edge is populated"))
            .collect::<Vec<_>>();
        for range in offsets.windows(2) {
            edges[range[0]..range[1]].sort_unstable_by_key(|edge| G::edge_slot(*edge));
        }
        Self {
            nodes,
            offsets,
            edges,
        }
    }

    pub(super) fn nodes(&self) -> &[G::Node] {
        &self.nodes
    }

    pub(super) fn incident(&self, node: G::Node) -> &[G::Edge] {
        let slot = G::node_slot(node);
        &self.edges[self.offsets[slot]..self.offsets[slot + 1]]
    }
}
