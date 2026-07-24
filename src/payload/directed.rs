use crate::Vec;
use crate::{
    EdgeEndpoints, EdgeIndex, GraphError, GraphView, IndexGraphView, NodeIndex, Result, Topology,
};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PayloadGraph<NodePayload, EdgePayload> {
    nodes: Vec<NodePayload>,
    edges: Vec<EdgePayload>,
    topology: Topology,
}

impl<NodePayload, EdgePayload> PayloadGraph<NodePayload, EdgePayload> {
    /// Builds a directed graph with arbitrary node and edge payloads.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid compact endpoints or capacity overflow.
    pub fn try_from_edges(
        nodes: Vec<NodePayload>,
        edges: impl IntoIterator<Item = (EdgeEndpoints, EdgePayload)>,
    ) -> Result<Self> {
        let (endpoints, edges): (Vec<_>, Vec<_>) = edges.into_iter().unzip();
        let topology = Topology::try_from_edges(nodes.len(), endpoints)?;
        Ok(Self {
            nodes,
            edges,
            topology,
        })
    }

    /// Reattaches payload vectors to a prebuilt topology.
    ///
    /// # Errors
    ///
    /// Returns an error when either payload count differs from the topology.
    pub fn try_from_parts(
        topology: Topology,
        nodes: Vec<NodePayload>,
        edges: Vec<EdgePayload>,
    ) -> Result<Self> {
        validate_count("node", topology.node_count(), nodes.len())?;
        validate_count("edge", topology.edge_count(), edges.len())?;
        Ok(Self {
            nodes,
            edges,
            topology,
        })
    }

    #[must_use]
    pub const fn topology(&self) -> &Topology {
        &self.topology
    }

    #[must_use]
    pub fn nodes(&self) -> &[NodePayload] {
        &self.nodes
    }

    #[must_use]
    pub fn edges(&self) -> &[EdgePayload] {
        &self.edges
    }

    #[must_use]
    pub fn node(&self, index: NodeIndex) -> Option<&NodePayload> {
        self.nodes.get(index.index())
    }

    #[must_use]
    pub fn node_mut(&mut self, index: NodeIndex) -> Option<&mut NodePayload> {
        self.nodes.get_mut(index.index())
    }

    #[must_use]
    pub fn edge(&self, index: EdgeIndex) -> Option<&EdgePayload> {
        self.edges.get(index.index())
    }

    #[must_use]
    pub fn edge_mut(&mut self, index: EdgeIndex) -> Option<&mut EdgePayload> {
        self.edges.get_mut(index.index())
    }

    #[must_use]
    pub fn into_parts(self) -> (Topology, Vec<NodePayload>, Vec<EdgePayload>) {
        (self.topology, self.nodes, self.edges)
    }
}

impl<NodePayload, EdgePayload> GraphView for PayloadGraph<NodePayload, EdgePayload> {
    type Node = NodeIndex;
    type Edge = EdgeIndex;

    fn node_count(&self) -> usize {
        self.topology.node_count()
    }

    fn edge_count(&self) -> usize {
        self.topology.edge_count()
    }

    fn contains_node(&self, node: Self::Node) -> bool {
        self.topology.contains_node(node)
    }

    fn contains_edge(&self, edge: Self::Edge) -> bool {
        self.topology.contains_edge(edge)
    }

    fn node_indices(&self) -> impl Iterator<Item = Self::Node> + '_ {
        self.topology.node_indices()
    }

    fn edge_indices(&self) -> impl Iterator<Item = Self::Edge> + '_ {
        self.topology.edge_indices()
    }

    fn edge_endpoints(&self, edge: Self::Edge) -> Option<EdgeEndpoints<Self::Node>> {
        self.topology.edge_endpoints(edge)
    }

    fn outgoing_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        self.topology.outgoing_edges(node)
    }

    fn incoming_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        self.topology.incoming_edges(node)
    }
}

impl<NodePayload, EdgePayload> IndexGraphView for PayloadGraph<NodePayload, EdgePayload> {
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
struct PayloadWire<NodePayload, EdgePayload> {
    nodes: Vec<NodePayload>,
    edges: Vec<EdgePayload>,
    topology: Topology,
}

impl<'de, NodePayload, EdgePayload> Deserialize<'de> for PayloadGraph<NodePayload, EdgePayload>
where
    NodePayload: Deserialize<'de>,
    EdgePayload: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PayloadWire::deserialize(deserializer)?;
        Self::try_from_parts(wire.topology, wire.nodes, wire.edges).map_err(D::Error::custom)
    }
}

fn validate_count(category: &'static str, expected: usize, actual: usize) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(GraphError::PayloadCountMismatch {
            category,
            expected,
            actual,
        })
    }
}
