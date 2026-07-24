use crate::EdgeEndpoints;
use core::hash::Hash;

pub trait UndirectedGraphView {
    type Node: Copy + Eq + Hash;
    type Edge: Copy + Eq + Hash;

    fn node_count(&self) -> usize;

    fn edge_count(&self) -> usize;

    fn contains_node(&self, node: Self::Node) -> bool;

    fn contains_edge(&self, edge: Self::Edge) -> bool;

    fn node_indices(&self) -> impl Iterator<Item = Self::Node> + '_;

    fn edge_indices(&self) -> impl Iterator<Item = Self::Edge> + '_;

    fn edge_endpoints(&self, edge: Self::Edge) -> Option<EdgeEndpoints<Self::Node>>;

    fn incident_edges(
        &self,
        node: Self::Node,
    ) -> impl DoubleEndedIterator<Item = Self::Edge> + ExactSizeIterator + '_;

    fn opposite(&self, edge: Self::Edge, node: Self::Node) -> Option<Self::Node> {
        let endpoints = self.edge_endpoints(edge)?;
        if endpoints.source() == node {
            Some(endpoints.target())
        } else if endpoints.target() == node {
            Some(endpoints.source())
        } else {
            None
        }
    }
}

pub trait IndexUndirectedGraphView: UndirectedGraphView {
    fn node_bound(&self) -> usize;

    fn edge_bound(&self) -> usize;

    fn node_slot(node: Self::Node) -> usize;

    fn edge_slot(edge: Self::Edge) -> usize;

    fn incident_edge_at(&self, node: Self::Node, offset: usize) -> Option<Self::Edge> {
        self.incident_edges(node).nth(offset)
    }
}
