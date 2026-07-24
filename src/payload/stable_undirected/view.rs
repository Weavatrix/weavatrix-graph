use super::core::StableUndirectedPayloadGraph;
use crate::{
    EdgeEndpoints, IndexUndirectedGraphView, StableEdgeKey, StableNodeKey, UndirectedGraphView,
};

impl<NodePayload, EdgePayload> UndirectedGraphView
    for StableUndirectedPayloadGraph<NodePayload, EdgePayload>
{
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
        self.nodes().map(|pair| pair.0)
    }

    fn edge_indices(&self) -> impl Iterator<Item = Self::Edge> + '_ {
        self.edges().map(|pair| pair.0)
    }

    fn edge_endpoints(&self, edge: Self::Edge) -> Option<EdgeEndpoints<Self::Node>> {
        self.edge_endpoints(edge)
    }

    fn incident_edges(
        &self,
        node: Self::Node,
    ) -> impl DoubleEndedIterator<Item = Self::Edge> + ExactSizeIterator + '_ {
        self.incident_edges(node)
    }
}

impl<NodePayload, EdgePayload> IndexUndirectedGraphView
    for StableUndirectedPayloadGraph<NodePayload, EdgePayload>
{
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

    fn incident_edge_at(&self, node: Self::Node, offset: usize) -> Option<Self::Edge> {
        let degree = self.node_slot(node)?.degree;
        if offset >= degree {
            return None;
        }
        if offset <= degree / 2 {
            self.incident_edges(node).nth(offset)
        } else {
            self.incident_edges(node).nth_back(degree - offset - 1)
        }
    }
}
