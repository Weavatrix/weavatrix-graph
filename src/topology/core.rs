use super::csr::Csr;
use super::{EdgeEndpoints, EdgeIndex, GraphView, IndexGraphView, NodeIndex};
use crate::Vec;
use crate::{GraphError, Result};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Topology {
    node_count: u32,
    endpoints: Vec<EdgeEndpoints>,
    #[serde(skip)]
    outgoing: Csr,
    #[serde(skip)]
    incoming: Csr,
}

impl Topology {
    /// Builds a validated directed topology with incoming and outgoing CSR.
    ///
    /// # Errors
    ///
    /// Returns an error when the graph exceeds the compact index capacity or
    /// an endpoint is outside `node_count`.
    pub fn try_from_edges(
        node_count: usize,
        edges: impl IntoIterator<Item = EdgeEndpoints>,
    ) -> Result<Self> {
        Self::try_from_collected(node_count, edges.into_iter().collect())
    }

    pub(super) fn try_from_collected(
        node_count: usize,
        endpoints: Vec<EdgeEndpoints>,
    ) -> Result<Self> {
        let compact_node_count = Self::validate_capacity(node_count, endpoints.len())?;
        let (outgoing, incoming) = Csr::try_build_pair(node_count, &endpoints)?;
        Ok(Self::from_parts(
            compact_node_count,
            endpoints,
            outgoing,
            incoming,
        ))
    }

    pub(super) fn validate_capacity(node_count: usize, edge_count: usize) -> Result<u32> {
        let compact_node_count =
            u32::try_from(node_count).map_err(|_| GraphError::IndexCapacityExceeded {
                category: "nodes",
                count: node_count,
            })?;
        u32::try_from(edge_count).map_err(|_| GraphError::IndexCapacityExceeded {
            category: "edges",
            count: edge_count,
        })?;
        Ok(compact_node_count)
    }

    pub(super) fn from_parts(
        node_count: u32,
        endpoints: Vec<EdgeEndpoints>,
        outgoing: Csr,
        incoming: Csr,
    ) -> Self {
        Self {
            node_count,
            endpoints,
            outgoing,
            incoming,
        }
    }

    pub(crate) fn try_from_usize_edges(
        node_count: usize,
        edges: impl IntoIterator<Item = (usize, usize)>,
    ) -> Result<Self> {
        let edges = edges.into_iter();
        let mut endpoints = Vec::with_capacity(edges.size_hint().0);
        for (source, target) in edges {
            endpoints.push(EdgeEndpoints::new(
                compact_node(source)?,
                compact_node(target)?,
            ));
        }
        Self::try_from_edges(node_count, endpoints)
    }

    #[must_use]
    pub const fn node_count(&self) -> usize {
        self.node_count as usize
    }

    #[must_use]
    pub const fn edge_count(&self) -> usize {
        self.endpoints.len()
    }

    #[must_use]
    pub fn contains_node(&self, node: NodeIndex) -> bool {
        node.index() < self.node_count()
    }

    #[must_use]
    pub fn contains_edge(&self, edge: EdgeIndex) -> bool {
        edge.index() < self.edge_count()
    }

    #[must_use]
    pub fn edge_endpoints(&self, edge: EdgeIndex) -> Option<EdgeEndpoints> {
        self.endpoints.get(edge.index()).copied()
    }

    #[must_use]
    pub fn outgoing_edges(
        &self,
        node: NodeIndex,
    ) -> impl DoubleEndedIterator<Item = EdgeIndex> + ExactSizeIterator + '_ {
        self.outgoing.get(node.index()).iter().copied()
    }

    #[must_use]
    pub fn incoming_edges(
        &self,
        node: NodeIndex,
    ) -> impl DoubleEndedIterator<Item = EdgeIndex> + ExactSizeIterator + '_ {
        self.incoming.get(node.index()).iter().copied()
    }

    #[must_use]
    pub fn outgoing_neighbors(
        &self,
        node: NodeIndex,
    ) -> impl DoubleEndedIterator<Item = NodeIndex> + ExactSizeIterator + '_ {
        self.outgoing_edges(node)
            .map(|edge| self.endpoints[edge.index()].target())
    }

    #[must_use]
    pub fn incoming_neighbors(
        &self,
        node: NodeIndex,
    ) -> impl DoubleEndedIterator<Item = NodeIndex> + ExactSizeIterator + '_ {
        self.incoming_edges(node)
            .map(|edge| self.endpoints[edge.index()].source())
    }

    #[must_use]
    pub fn out_degree(&self, node: NodeIndex) -> Option<usize> {
        self.contains_node(node)
            .then(|| self.outgoing.get(node.index()).len())
    }

    #[must_use]
    pub fn in_degree(&self, node: NodeIndex) -> Option<usize> {
        self.contains_node(node)
            .then(|| self.incoming.get(node.index()).len())
    }

    pub(crate) fn traversal_parts(&self) -> (&[EdgeEndpoints], &Csr, &Csr) {
        (&self.endpoints, &self.outgoing, &self.incoming)
    }
}

impl GraphView for Topology {
    type Node = NodeIndex;
    type Edge = EdgeIndex;

    fn node_count(&self) -> usize {
        self.node_count()
    }

    fn edge_count(&self) -> usize {
        self.edge_count()
    }

    fn contains_node(&self, node: NodeIndex) -> bool {
        self.contains_node(node)
    }

    fn contains_edge(&self, edge: EdgeIndex) -> bool {
        self.contains_edge(edge)
    }

    fn node_indices(&self) -> impl Iterator<Item = Self::Node> + '_ {
        (0..self.node_count).map(NodeIndex::new)
    }

    fn edge_indices(&self) -> impl Iterator<Item = Self::Edge> + '_ {
        (0..u32::try_from(self.edge_count()).expect("edge count was checked")).map(EdgeIndex::new)
    }

    fn edge_endpoints(&self, edge: EdgeIndex) -> Option<EdgeEndpoints> {
        self.edge_endpoints(edge)
    }

    fn edge_references(&self) -> impl Iterator<Item = (EdgeIndex, EdgeEndpoints)> + '_ {
        self.edge_indices().zip(self.endpoints.iter().copied())
    }

    fn outgoing_edges(&self, node: NodeIndex) -> impl Iterator<Item = EdgeIndex> + '_ {
        self.outgoing_edges(node)
    }

    fn incoming_edges(&self, node: NodeIndex) -> impl Iterator<Item = EdgeIndex> + '_ {
        self.incoming_edges(node)
    }
}

impl IndexGraphView for Topology {
    fn node_bound(&self) -> usize {
        self.node_count()
    }

    fn edge_bound(&self) -> usize {
        self.edge_count()
    }

    fn node_slot(node: Self::Node) -> usize {
        node.index()
    }

    fn edge_slot(edge: Self::Edge) -> usize {
        edge.index()
    }
}

#[derive(Deserialize)]
struct TopologyWire {
    node_count: u32,
    endpoints: Vec<EdgeEndpoints>,
}

impl<'de> Deserialize<'de> for Topology {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = TopologyWire::deserialize(deserializer)?;
        Self::try_from_edges(wire.node_count as usize, wire.endpoints).map_err(D::Error::custom)
    }
}

fn compact_node(index: usize) -> Result<NodeIndex> {
    u32::try_from(index)
        .map(NodeIndex::new)
        .map_err(|_| GraphError::IndexCapacityExceeded {
            category: "node index",
            count: index,
        })
}
