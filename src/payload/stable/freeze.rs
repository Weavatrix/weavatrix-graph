use super::StablePayloadGraph;
use crate::Vec;
use crate::{
    EdgeEndpoints, EdgeIndex, NodeIndex, PayloadGraph, Result, StableEdgeKey, StableNodeKey,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadFreezeMap {
    nodes: Vec<Option<(u32, NodeIndex)>>,
    edges: Vec<Option<(u32, EdgeIndex)>>,
}

impl PayloadFreezeMap {
    pub(crate) const fn from_slots(
        nodes: Vec<Option<(u32, NodeIndex)>>,
        edges: Vec<Option<(u32, EdgeIndex)>>,
    ) -> Self {
        Self { nodes, edges }
    }

    #[must_use]
    pub fn node(&self, key: StableNodeKey) -> Option<NodeIndex> {
        let (generation, index) = self.nodes.get(key.index())?.as_ref()?;
        (*generation == key.generation()).then_some(*index)
    }

    #[must_use]
    pub fn edge(&self, key: StableEdgeKey) -> Option<EdgeIndex> {
        let (generation, index) = self.edges.get(key.index())?.as_ref()?;
        (*generation == key.generation()).then_some(*index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenPayloadGraph<NodePayload, EdgePayload> {
    graph: PayloadGraph<NodePayload, EdgePayload>,
    indices: PayloadFreezeMap,
}

impl<NodePayload, EdgePayload> FrozenPayloadGraph<NodePayload, EdgePayload> {
    #[must_use]
    pub const fn graph(&self) -> &PayloadGraph<NodePayload, EdgePayload> {
        &self.graph
    }

    #[must_use]
    pub const fn indices(&self) -> &PayloadFreezeMap {
        &self.indices
    }

    #[must_use]
    pub fn into_parts(self) -> (PayloadGraph<NodePayload, EdgePayload>, PayloadFreezeMap) {
        (self.graph, self.indices)
    }
}

impl<NodePayload, EdgePayload> StablePayloadGraph<NodePayload, EdgePayload> {
    /// Compacts live slots into an immutable CSR-backed payload graph.
    ///
    /// # Errors
    ///
    /// Returns an error if compact index capacity is exhausted.
    pub fn freeze(self) -> Result<FrozenPayloadGraph<NodePayload, EdgePayload>> {
        let mut node_map = vec![None; self.nodes.len()];
        let mut nodes = Vec::with_capacity(self.node_count);
        for (slot, entry) in self.nodes.into_iter().enumerate() {
            let Some(payload) = entry.value else {
                continue;
            };
            let compact = NodeIndex::new(u32::try_from(nodes.len()).map_err(|_| {
                crate::GraphError::IndexCapacityExceeded {
                    category: "frozen payload nodes",
                    count: nodes.len(),
                }
            })?);
            node_map[slot] = Some((entry.generation, compact));
            nodes.push(payload);
        }

        let mut edge_map = vec![None; self.edges.len()];
        let mut edges = Vec::with_capacity(self.edge_count);
        for (slot, entry) in self.edges.into_iter().enumerate() {
            let Some(payload) = entry.value else {
                continue;
            };
            let source = mapped_node(&node_map, entry.source)?;
            let target = mapped_node(&node_map, entry.target)?;
            let compact = EdgeIndex::new(u32::try_from(edges.len()).map_err(|_| {
                crate::GraphError::IndexCapacityExceeded {
                    category: "frozen payload edges",
                    count: edges.len(),
                }
            })?);
            edge_map[slot] = Some((entry.generation, compact));
            edges.push((EdgeEndpoints::new(source, target), payload));
        }
        let graph = PayloadGraph::try_from_edges(nodes, edges)?;
        Ok(FrozenPayloadGraph {
            graph,
            indices: PayloadFreezeMap::from_slots(node_map, edge_map),
        })
    }
}

pub(crate) fn mapped_node(nodes: &[Option<(u32, NodeIndex)>], slot: u32) -> Result<NodeIndex> {
    nodes
        .get(slot as usize)
        .and_then(Option::as_ref)
        .copied()
        .map(|(_, index)| index)
        .ok_or(crate::GraphError::InvalidNodeIndex {
            node: slot as usize,
            node_count: nodes.len(),
        })
}
