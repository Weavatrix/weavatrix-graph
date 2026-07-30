use super::{IndexUndirectedGraphView, UndirectedGraphView};
use crate::Vec;
use crate::topology::csr::Csr;
use crate::{EdgeEndpoints, EdgeIndex, GraphError, NodeIndex, Result};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UndirectedTopology {
    node_count: u32,
    endpoints: Vec<EdgeEndpoints>,
    #[serde(skip)]
    incidence: Csr,
}

impl UndirectedTopology {
    /// Builds an undirected topology with compact incidence CSR.
    ///
    /// Self-loops occupy one incidence entry and report degree two.
    ///
    /// # Errors
    ///
    /// Returns an error for capacity overflow or an endpoint outside the graph.
    pub fn try_from_edges(
        node_count: usize,
        edges: impl IntoIterator<Item = EdgeEndpoints>,
    ) -> Result<Self> {
        let compact_node_count =
            u32::try_from(node_count).map_err(|_| GraphError::IndexCapacityExceeded {
                category: "nodes",
                count: node_count,
            })?;
        let endpoints = edges.into_iter().collect::<Vec<_>>();
        u32::try_from(endpoints.len()).map_err(|_| GraphError::IndexCapacityExceeded {
            category: "edges",
            count: endpoints.len(),
        })?;
        let incidence = Csr::try_build_undirected(node_count, &endpoints)?;
        Ok(Self {
            node_count: compact_node_count,
            endpoints,
            incidence,
        })
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
    pub fn incident_edges(
        &self,
        node: NodeIndex,
    ) -> impl DoubleEndedIterator<Item = EdgeIndex> + ExactSizeIterator + '_ {
        self.incidence.get(node.index()).iter().copied()
    }

    #[must_use]
    pub fn incident_edge_at(&self, node: NodeIndex, offset: usize) -> Option<EdgeIndex> {
        self.incidence.get(node.index()).get(offset).copied()
    }

    #[must_use]
    pub fn neighbors(
        &self,
        node: NodeIndex,
    ) -> impl DoubleEndedIterator<Item = NodeIndex> + ExactSizeIterator + '_ {
        self.incident_edges(node).map(move |edge| {
            let endpoints = self.endpoints[edge.index()];
            if endpoints.source() == node {
                endpoints.target()
            } else {
                endpoints.source()
            }
        })
    }

    #[must_use]
    pub fn degree(&self, node: NodeIndex) -> Option<usize> {
        self.contains_node(node).then(|| {
            self.incident_edges(node)
                .map(|edge| {
                    let endpoints = self.endpoints[edge.index()];
                    usize::from(endpoints.source() == node && endpoints.target() == node)
                })
                .sum::<usize>()
                + self.incident_edges(node).len()
        })
    }
}

impl UndirectedGraphView for UndirectedTopology {
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

    fn node_indices(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        (0..self.node_count).map(NodeIndex::new)
    }

    fn edge_indices(&self) -> impl Iterator<Item = EdgeIndex> + '_ {
        let edge_count = u32::try_from(self.edge_count()).unwrap_or(u32::MAX);
        (0..edge_count).map(EdgeIndex::new)
    }

    fn edge_endpoints(&self, edge: EdgeIndex) -> Option<EdgeEndpoints> {
        self.edge_endpoints(edge)
    }

    fn incident_edges(
        &self,
        node: NodeIndex,
    ) -> impl DoubleEndedIterator<Item = EdgeIndex> + ExactSizeIterator + '_ {
        self.incident_edges(node)
    }
}

impl IndexUndirectedGraphView for UndirectedTopology {
    fn node_bound(&self) -> usize {
        self.node_count()
    }

    fn edge_bound(&self) -> usize {
        self.edge_count()
    }

    fn node_slot(node: NodeIndex) -> usize {
        node.index()
    }

    fn edge_slot(edge: EdgeIndex) -> usize {
        edge.index()
    }

    fn incident_edge_at(&self, node: NodeIndex, offset: usize) -> Option<EdgeIndex> {
        self.incident_edge_at(node, offset)
    }
}

#[derive(Deserialize)]
struct UndirectedWire {
    node_count: u32,
    endpoints: Vec<EdgeEndpoints>,
}

impl<'de> Deserialize<'de> for UndirectedTopology {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = UndirectedWire::deserialize(deserializer)?;
        Self::try_from_edges(wire.node_count as usize, wire.endpoints).map_err(D::Error::custom)
    }
}
