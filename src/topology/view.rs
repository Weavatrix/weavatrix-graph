use super::NodeIndex;
use core::hash::Hash;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeEndpoints<Node = NodeIndex> {
    source: Node,
    target: Node,
}

impl<Node: Copy> EdgeEndpoints<Node> {
    #[must_use]
    pub const fn new(source: Node, target: Node) -> Self {
        Self { source, target }
    }

    #[must_use]
    pub const fn source(self) -> Node {
        self.source
    }

    #[must_use]
    pub const fn target(self) -> Node {
        self.target
    }
}

pub trait GraphView {
    type Node: Copy + Eq + Hash;
    type Edge: Copy + Eq + Hash;

    fn node_count(&self) -> usize;

    fn edge_count(&self) -> usize;

    fn contains_node(&self, node: Self::Node) -> bool {
        self.node_indices().any(|candidate| candidate == node)
    }

    fn contains_edge(&self, edge: Self::Edge) -> bool {
        self.edge_indices().any(|candidate| candidate == edge)
    }

    fn node_indices(&self) -> impl Iterator<Item = Self::Node> + '_;

    fn edge_indices(&self) -> impl Iterator<Item = Self::Edge> + '_;

    fn edge_endpoints(&self, edge: Self::Edge) -> Option<EdgeEndpoints<Self::Node>>;

    fn edge_references(
        &self,
    ) -> impl Iterator<Item = (Self::Edge, EdgeEndpoints<Self::Node>)> + '_ {
        self.edge_indices()
            .filter_map(|edge| self.edge_endpoints(edge).map(|endpoints| (edge, endpoints)))
    }

    fn outgoing_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_;

    fn incoming_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_;
}

pub trait IndexGraphView: GraphView {
    fn node_bound(&self) -> usize;

    fn edge_bound(&self) -> usize;

    fn node_slot(node: Self::Node) -> usize;

    fn edge_slot(edge: Self::Edge) -> usize;
}
