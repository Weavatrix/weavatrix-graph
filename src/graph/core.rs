use super::bulk;
use crate::Vec;
use crate::{
    Edge, EdgeEndpoints, EdgeIndex, GraphView, IndexGraphView, Node, NodeId, NodeIndex, Result,
    Topology,
};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// Backward-compatible name for a compact topology node index.
pub type GraphNodeIndex = NodeIndex;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    #[serde(skip)]
    topology: Topology,
}

impl Graph {
    pub(super) const fn from_indexed_parts(
        nodes: Vec<Node>,
        edges: Vec<Edge>,
        topology: Topology,
    ) -> Self {
        Self {
            nodes,
            edges,
            topology,
        }
    }

    pub(crate) fn from_validated_sorted_parts(nodes: Vec<Node>, edges: Vec<Edge>) -> Result<Self> {
        let topology = super::index::index_canonical_edges(&nodes, &edges)?;
        Ok(Self::from_indexed_parts(nodes, edges, topology))
    }

    /// Creates a validated graph and canonicalizes its ordering.
    ///
    /// # Errors
    ///
    /// Returns an error for conflicting nodes, dangling edges, empty extractor
    /// identities, or invalid source spans.
    pub fn try_from_parts(
        nodes: impl IntoIterator<Item = Node>,
        edges: impl IntoIterator<Item = Edge>,
    ) -> Result<Self> {
        bulk::canonical(nodes, edges)
    }

    /// Builds a canonical graph without re-sorting already sorted nodes.
    ///
    /// Unsorted nodes safely fall back to [`Self::try_from_parts`]. Edges may
    /// arrive in any order and are still validated, sorted, and deduplicated.
    ///
    /// # Errors
    ///
    /// Returns the same validation errors as [`Self::try_from_parts`].
    pub fn try_from_sorted_nodes(
        nodes: Vec<Node>,
        edges: impl IntoIterator<Item = Edge>,
    ) -> Result<Self> {
        bulk::from_sorted_nodes(nodes, edges)
    }

    /// Builds faster when nodes and edges are already in canonical order.
    ///
    /// Unordered input safely falls back to [`Self::try_from_parts`].
    ///
    /// # Errors
    ///
    /// Returns the same validation errors as [`Self::try_from_parts`].
    pub fn try_from_sorted_parts(nodes: Vec<Node>, edges: Vec<Edge>) -> Result<Self> {
        bulk::from_sorted_parts(nodes, edges)
    }

    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    #[must_use]
    pub const fn topology(&self) -> &Topology {
        &self.topology
    }

    #[must_use]
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.node_index(id).and_then(|index| self.node_at(index))
    }

    pub fn outgoing<'graph>(&'graph self, id: &NodeId) -> impl Iterator<Item = &'graph Edge> {
        self.node_index(id.as_str())
            .into_iter()
            .flat_map(|index| self.topology.outgoing_edges(index))
            .map(|edge| &self.edges[edge.index()])
    }

    pub fn incoming<'graph>(&'graph self, id: &NodeId) -> impl Iterator<Item = &'graph Edge> {
        self.node_index(id.as_str())
            .into_iter()
            .flat_map(|index| self.topology.incoming_edges(index))
            .map(|edge| &self.edges[edge.index()])
    }

    #[must_use]
    pub fn edge_at(&self, index: EdgeIndex) -> Option<&Edge> {
        self.edges.get(index.index())
    }

    #[must_use]
    pub fn node_index(&self, id: &str) -> Option<GraphNodeIndex> {
        self.nodes
            .binary_search_by(|node| node.id.as_str().cmp(id))
            .ok()
            .and_then(|index| u32::try_from(index).ok())
            .map(NodeIndex::new)
    }

    #[must_use]
    pub fn node_at(&self, index: GraphNodeIndex) -> Option<&Node> {
        self.nodes.get(index.index())
    }

    pub fn outgoing_at(&self, index: GraphNodeIndex) -> impl Iterator<Item = &Edge> {
        self.topology
            .outgoing_edges(index)
            .map(|edge| &self.edges[edge.index()])
    }

    pub fn incoming_at(&self, index: GraphNodeIndex) -> impl Iterator<Item = &Edge> {
        self.topology
            .incoming_edges(index)
            .map(|edge| &self.edges[edge.index()])
    }

    pub fn outgoing_neighbors_at(
        &self,
        index: GraphNodeIndex,
    ) -> impl Iterator<Item = GraphNodeIndex> + '_ {
        self.topology.outgoing_neighbors(index)
    }

    pub fn incoming_neighbors_at(
        &self,
        index: GraphNodeIndex,
    ) -> impl Iterator<Item = GraphNodeIndex> + '_ {
        self.topology.incoming_neighbors(index)
    }

    #[must_use]
    pub fn out_degree(&self, index: GraphNodeIndex) -> Option<usize> {
        self.topology.out_degree(index)
    }

    #[must_use]
    pub fn in_degree(&self, index: GraphNodeIndex) -> Option<usize> {
        self.topology.in_degree(index)
    }

    #[must_use]
    pub const fn node_count(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub const fn edge_count(&self) -> usize {
        self.edges.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.edges.is_empty()
    }

    #[must_use]
    pub fn into_parts(self) -> (Vec<Node>, Vec<Edge>) {
        (self.nodes, self.edges)
    }
}

impl GraphView for Graph {
    type Node = NodeIndex;
    type Edge = EdgeIndex;

    fn node_count(&self) -> usize {
        self.node_count()
    }

    fn edge_count(&self) -> usize {
        self.edge_count()
    }

    fn contains_node(&self, node: NodeIndex) -> bool {
        self.node_at(node).is_some()
    }

    fn contains_edge(&self, edge: EdgeIndex) -> bool {
        self.edge_at(edge).is_some()
    }

    fn node_indices(&self) -> impl Iterator<Item = Self::Node> + '_ {
        self.topology.node_indices()
    }

    fn edge_indices(&self) -> impl Iterator<Item = Self::Edge> + '_ {
        self.topology.edge_indices()
    }

    fn edge_endpoints(&self, edge: EdgeIndex) -> Option<EdgeEndpoints> {
        self.topology.edge_endpoints(edge)
    }

    fn edge_references(&self) -> impl Iterator<Item = (EdgeIndex, EdgeEndpoints)> + '_ {
        self.topology.edge_references()
    }

    fn outgoing_edges(&self, node: NodeIndex) -> impl Iterator<Item = EdgeIndex> + '_ {
        self.topology.outgoing_edges(node)
    }

    fn incoming_edges(&self, node: NodeIndex) -> impl Iterator<Item = EdgeIndex> + '_ {
        self.topology.incoming_edges(node)
    }
}

impl IndexGraphView for Graph {
    fn node_bound(&self) -> usize {
        self.topology.node_bound()
    }

    fn edge_bound(&self) -> usize {
        self.topology.edge_bound()
    }

    fn node_slot(node: Self::Node) -> usize {
        node.index()
    }

    fn edge_slot(edge: Self::Edge) -> usize {
        edge.index()
    }
}

#[derive(Deserialize)]
struct GraphWire {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl<'de> Deserialize<'de> for Graph {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = GraphWire::deserialize(deserializer)?;
        Self::try_from_parts(wire.nodes, wire.edges).map_err(D::Error::custom)
    }
}
