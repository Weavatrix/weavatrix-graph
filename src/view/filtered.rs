use crate::{EdgeEndpoints, GraphView, IndexGraphView};

#[derive(Debug, Clone, Copy)]
pub struct EdgeFiltered<'graph, G, Predicate> {
    graph: &'graph G,
    predicate: Predicate,
}

impl<'graph, G, Predicate> EdgeFiltered<'graph, G, Predicate> {
    #[must_use]
    pub const fn new(graph: &'graph G, predicate: Predicate) -> Self {
        Self { graph, predicate }
    }

    #[must_use]
    pub const fn inner(&self) -> &'graph G {
        self.graph
    }
}

impl<G, Predicate> GraphView for EdgeFiltered<'_, G, Predicate>
where
    G: GraphView,
    Predicate: Fn(G::Edge) -> bool,
{
    type Node = G::Node;
    type Edge = G::Edge;

    fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    fn edge_count(&self) -> usize {
        self.edge_indices().count()
    }

    fn contains_node(&self, node: Self::Node) -> bool {
        self.graph.contains_node(node)
    }

    fn contains_edge(&self, edge: Self::Edge) -> bool {
        self.graph.contains_edge(edge) && (self.predicate)(edge)
    }

    fn node_indices(&self) -> impl Iterator<Item = Self::Node> + '_ {
        self.graph.node_indices()
    }

    fn edge_indices(&self) -> impl Iterator<Item = Self::Edge> + '_ {
        self.graph
            .edge_indices()
            .filter(|edge| (self.predicate)(*edge))
    }

    fn edge_endpoints(&self, edge: Self::Edge) -> Option<EdgeEndpoints<Self::Node>> {
        self.contains_edge(edge)
            .then(|| self.graph.edge_endpoints(edge))
            .flatten()
    }

    fn outgoing_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        self.graph
            .outgoing_edges(node)
            .filter(|edge| (self.predicate)(*edge))
    }

    fn incoming_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        self.graph
            .incoming_edges(node)
            .filter(|edge| (self.predicate)(*edge))
    }
}

impl<G, Predicate> IndexGraphView for EdgeFiltered<'_, G, Predicate>
where
    G: IndexGraphView,
    Predicate: Fn(G::Edge) -> bool,
{
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

#[derive(Debug, Clone, Copy)]
pub struct NodeFiltered<'graph, G, Predicate> {
    graph: &'graph G,
    predicate: Predicate,
}

impl<'graph, G, Predicate> NodeFiltered<'graph, G, Predicate> {
    #[must_use]
    pub const fn new(graph: &'graph G, predicate: Predicate) -> Self {
        Self { graph, predicate }
    }

    #[must_use]
    pub const fn inner(&self) -> &'graph G {
        self.graph
    }
}

impl<G, Predicate> NodeFiltered<'_, G, Predicate>
where
    G: GraphView,
    Predicate: Fn(G::Node) -> bool,
{
    fn allows_edge(&self, edge: G::Edge) -> bool {
        self.graph.edge_endpoints(edge).is_some_and(|endpoints| {
            (self.predicate)(endpoints.source()) && (self.predicate)(endpoints.target())
        })
    }
}

impl<G, Predicate> GraphView for NodeFiltered<'_, G, Predicate>
where
    G: GraphView,
    Predicate: Fn(G::Node) -> bool,
{
    type Node = G::Node;
    type Edge = G::Edge;

    fn node_count(&self) -> usize {
        self.node_indices().count()
    }

    fn edge_count(&self) -> usize {
        self.edge_indices().count()
    }

    fn contains_node(&self, node: Self::Node) -> bool {
        self.graph.contains_node(node) && (self.predicate)(node)
    }

    fn contains_edge(&self, edge: Self::Edge) -> bool {
        self.graph.contains_edge(edge) && self.allows_edge(edge)
    }

    fn node_indices(&self) -> impl Iterator<Item = Self::Node> + '_ {
        self.graph
            .node_indices()
            .filter(|node| (self.predicate)(*node))
    }

    fn edge_indices(&self) -> impl Iterator<Item = Self::Edge> + '_ {
        self.graph
            .edge_indices()
            .filter(|edge| self.allows_edge(*edge))
    }

    fn edge_endpoints(&self, edge: Self::Edge) -> Option<EdgeEndpoints<Self::Node>> {
        self.contains_edge(edge)
            .then(|| self.graph.edge_endpoints(edge))
            .flatten()
    }

    fn outgoing_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        self.graph
            .outgoing_edges(node)
            .filter(move |edge| self.contains_node(node) && self.allows_edge(*edge))
    }

    fn incoming_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        self.graph
            .incoming_edges(node)
            .filter(move |edge| self.contains_node(node) && self.allows_edge(*edge))
    }
}

impl<G, Predicate> IndexGraphView for NodeFiltered<'_, G, Predicate>
where
    G: IndexGraphView,
    Predicate: Fn(G::Node) -> bool,
{
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
pub const fn edge_filtered<G, Predicate>(
    graph: &G,
    predicate: Predicate,
) -> EdgeFiltered<'_, G, Predicate>
where
    G: GraphView,
    Predicate: Fn(G::Edge) -> bool,
{
    EdgeFiltered::new(graph, predicate)
}

#[must_use]
pub const fn induced_subgraph_view<G, Predicate>(
    graph: &G,
    predicate: Predicate,
) -> NodeFiltered<'_, G, Predicate>
where
    G: GraphView,
    Predicate: Fn(G::Node) -> bool,
{
    NodeFiltered::new(graph, predicate)
}
