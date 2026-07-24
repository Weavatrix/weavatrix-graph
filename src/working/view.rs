use super::{StableEdgeKey, StableNodeKey, WorkingGraph};
use crate::{EdgeEndpoints, GraphView, IndexGraphView};

impl GraphView for WorkingGraph {
    type Node = StableNodeKey;
    type Edge = StableEdgeKey;

    fn node_count(&self) -> usize {
        self.node_count()
    }

    fn edge_count(&self) -> usize {
        self.edge_count()
    }

    fn contains_node(&self, node: StableNodeKey) -> bool {
        self.node(node).is_some()
    }

    fn contains_edge(&self, edge: StableEdgeKey) -> bool {
        self.edge(edge).is_some()
    }

    fn node_indices(&self) -> impl Iterator<Item = Self::Node> + '_ {
        self.nodes().map(|(key, _)| key)
    }

    fn edge_indices(&self) -> impl Iterator<Item = Self::Edge> + '_ {
        self.edges().map(|(key, _)| key)
    }

    fn edge_endpoints(&self, edge: Self::Edge) -> Option<EdgeEndpoints<Self::Node>> {
        WorkingGraph::edge_endpoints(self, edge)
    }

    fn outgoing_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        WorkingGraph::outgoing_edges(self, node)
    }

    fn incoming_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        WorkingGraph::incoming_edges(self, node)
    }
}

impl IndexGraphView for WorkingGraph {
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
