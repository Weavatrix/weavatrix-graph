use crate::{EdgeEndpoints, GraphView, IndexGraphView};

#[derive(Debug, Clone, Copy)]
pub struct Reversed<'graph, G> {
    graph: &'graph G,
}

impl<'graph, G> Reversed<'graph, G> {
    #[must_use]
    pub const fn new(graph: &'graph G) -> Self {
        Self { graph }
    }

    #[must_use]
    pub const fn inner(&self) -> &'graph G {
        self.graph
    }
}

impl<G: GraphView> GraphView for Reversed<'_, G> {
    type Node = G::Node;
    type Edge = G::Edge;

    fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    fn contains_node(&self, node: Self::Node) -> bool {
        self.graph.contains_node(node)
    }

    fn contains_edge(&self, edge: Self::Edge) -> bool {
        self.graph.contains_edge(edge)
    }

    fn node_indices(&self) -> impl Iterator<Item = Self::Node> + '_ {
        self.graph.node_indices()
    }

    fn edge_indices(&self) -> impl Iterator<Item = Self::Edge> + '_ {
        self.graph.edge_indices()
    }

    fn edge_endpoints(&self, edge: Self::Edge) -> Option<EdgeEndpoints<Self::Node>> {
        let endpoints = self.graph.edge_endpoints(edge)?;
        Some(EdgeEndpoints::new(endpoints.target(), endpoints.source()))
    }

    fn outgoing_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        self.graph.incoming_edges(node)
    }

    fn incoming_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        self.graph.outgoing_edges(node)
    }
}

impl<G: IndexGraphView> IndexGraphView for Reversed<'_, G> {
    fn node_bound(&self) -> usize {
        self.graph.node_bound()
    }

    fn edge_bound(&self) -> usize {
        self.graph.edge_bound()
    }

    fn node_slot(node: Self::Node) -> usize {
        G::node_slot(node)
    }

    fn edge_slot(edge: Self::Edge) -> usize {
        G::edge_slot(edge)
    }
}

#[must_use]
pub const fn reversed<G>(graph: &G) -> Reversed<'_, G> {
    Reversed::new(graph)
}
