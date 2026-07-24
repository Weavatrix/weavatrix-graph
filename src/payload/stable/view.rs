use super::StablePayloadGraph;
use crate::{EdgeEndpoints, GraphView, IndexGraphView, StableEdgeKey, StableNodeKey};

impl<NodePayload, EdgePayload> GraphView for StablePayloadGraph<NodePayload, EdgePayload> {
    type Node = StableNodeKey;
    type Edge = StableEdgeKey;

    fn node_count(&self) -> usize {
        self.node_count()
    }

    fn edge_count(&self) -> usize {
        self.edge_count()
    }

    fn contains_node(&self, node: Self::Node) -> bool {
        self.node(node).is_some()
    }

    fn contains_edge(&self, edge: Self::Edge) -> bool {
        self.edge(edge).is_some()
    }

    fn node_indices(&self) -> impl Iterator<Item = Self::Node> + '_ {
        self.nodes().map(|(key, _)| key)
    }

    fn edge_indices(&self) -> impl Iterator<Item = Self::Edge> + '_ {
        self.edges().map(|(key, _)| key)
    }

    fn edge_endpoints(&self, edge: Self::Edge) -> Option<EdgeEndpoints<Self::Node>> {
        StablePayloadGraph::edge_endpoints(self, edge)
    }

    fn outgoing_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        StablePayloadGraph::outgoing_edges(self, node)
    }

    fn incoming_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        StablePayloadGraph::incoming_edges(self, node)
    }
}

impl<NodePayload, EdgePayload> IndexGraphView for StablePayloadGraph<NodePayload, EdgePayload> {
    fn node_bound(&self) -> usize {
        self.nodes.len()
    }

    fn edge_bound(&self) -> usize {
        self.edges.len()
    }

    fn node_slot(node: Self::Node) -> usize {
        node.index()
    }

    fn edge_slot(edge: Self::Edge) -> usize {
        edge.index()
    }
}
