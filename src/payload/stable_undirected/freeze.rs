use super::core::StableUndirectedPayloadGraph;
use crate::payload::stable::mapped_node;
use crate::{
    EdgeEndpoints, EdgeIndex, NodeIndex, PayloadFreezeMap, Result, UndirectedPayloadGraph, Vec,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenUndirectedPayloadGraph<NodePayload, EdgePayload> {
    graph: UndirectedPayloadGraph<NodePayload, EdgePayload>,
    indices: PayloadFreezeMap,
}

impl<NodePayload, EdgePayload> FrozenUndirectedPayloadGraph<NodePayload, EdgePayload> {
    #[must_use]
    pub const fn graph(&self) -> &UndirectedPayloadGraph<NodePayload, EdgePayload> {
        &self.graph
    }

    #[must_use]
    pub const fn indices(&self) -> &PayloadFreezeMap {
        &self.indices
    }

    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        UndirectedPayloadGraph<NodePayload, EdgePayload>,
        PayloadFreezeMap,
    ) {
        (self.graph, self.indices)
    }
}

impl<NodePayload, EdgePayload> StableUndirectedPayloadGraph<NodePayload, EdgePayload> {
    /// Compacts live stable slots into immutable undirected incidence CSR.
    ///
    /// # Errors
    ///
    /// Returns an error if compact index capacity is exhausted.
    pub fn freeze(self) -> Result<FrozenUndirectedPayloadGraph<NodePayload, EdgePayload>> {
        let mut node_map = vec![None; self.nodes.len()];
        let mut nodes = Vec::with_capacity(self.node_count);
        for (slot, entry) in self.nodes.into_iter().enumerate() {
            let Some(payload) = entry.value else {
                continue;
            };
            let compact = compact_node(nodes.len())?;
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
            let compact = compact_edge(edges.len())?;
            edge_map[slot] = Some((entry.generation, compact));
            edges.push((EdgeEndpoints::new(source, target), payload));
        }
        let graph = UndirectedPayloadGraph::try_from_edges(nodes, edges)?;
        Ok(FrozenUndirectedPayloadGraph {
            graph,
            indices: PayloadFreezeMap::from_slots(node_map, edge_map),
        })
    }
}

fn compact_node(count: usize) -> Result<NodeIndex> {
    Ok(NodeIndex::new(u32::try_from(count).map_err(|_| {
        crate::GraphError::IndexCapacityExceeded {
            category: "frozen undirected nodes",
            count,
        }
    })?))
}

fn compact_edge(count: usize) -> Result<EdgeIndex> {
    Ok(EdgeIndex::new(u32::try_from(count).map_err(|_| {
        crate::GraphError::IndexCapacityExceeded {
            category: "frozen undirected edges",
            count,
        }
    })?))
}
