use super::{StableEdgeKey, StableNodeKey, WorkingGraph};
use crate::Vec;
use crate::{EdgeIndex, Graph, NodeIndex, Result};
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as StableMap;
#[cfg(feature = "std")]
use std::collections::HashMap as StableMap;

#[cfg(feature = "std")]
fn stable_map_with_capacity<K, V>(capacity: usize) -> StableMap<K, V> {
    StableMap::with_capacity(capacity)
}

#[cfg(not(feature = "std"))]
fn stable_map_with_capacity<K: Ord, V>(_: usize) -> StableMap<K, V> {
    StableMap::new()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreezeMap {
    nodes: StableMap<StableNodeKey, NodeIndex>,
    edges: StableMap<StableEdgeKey, EdgeIndex>,
}

impl FreezeMap {
    #[must_use]
    pub fn node(&self, key: StableNodeKey) -> Option<NodeIndex> {
        self.nodes.get(&key).copied()
    }

    #[must_use]
    pub fn edge(&self, key: StableEdgeKey) -> Option<EdgeIndex> {
        self.edges.get(&key).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenGraph {
    graph: Graph,
    indices: FreezeMap,
}

impl FrozenGraph {
    #[must_use]
    pub const fn graph(&self) -> &Graph {
        &self.graph
    }

    #[must_use]
    pub const fn indices(&self) -> &FreezeMap {
        &self.indices
    }

    #[must_use]
    pub fn into_parts(self) -> (Graph, FreezeMap) {
        (self.graph, self.indices)
    }
}

impl WorkingGraph {
    /// Canonicalizes the working graph and returns stable-to-compact remapping.
    ///
    /// # Errors
    ///
    /// Returns an error if an internal endpoint invariant is violated.
    pub fn freeze(self) -> Result<FrozenGraph> {
        let mut keyed_nodes = self
            .nodes
            .into_iter()
            .enumerate()
            .filter_map(|(slot, entry)| {
                Some((
                    StableNodeKey::new(u32::try_from(slot).ok()?, entry.generation),
                    entry.value?,
                ))
            })
            .collect::<Vec<_>>();
        keyed_nodes.sort_unstable_by(|left, right| left.1.id.cmp(&right.1.id));

        let mut node_map = stable_map_with_capacity(keyed_nodes.len());
        let mut nodes = Vec::with_capacity(keyed_nodes.len());
        for (index, (key, node)) in keyed_nodes.into_iter().enumerate() {
            let index =
                u32::try_from(index).map_err(|_| crate::GraphError::IndexCapacityExceeded {
                    category: "frozen nodes",
                    count: nodes.len(),
                })?;
            node_map.insert(key, NodeIndex::new(index));
            nodes.push(node);
        }

        let mut keyed_edges = self
            .edges
            .into_iter()
            .enumerate()
            .filter_map(|(slot, entry)| {
                Some((
                    StableEdgeKey::new(u32::try_from(slot).ok()?, entry.generation),
                    entry.value?.value,
                ))
            })
            .collect::<Vec<_>>();
        keyed_edges.sort_unstable_by(|left, right| left.1.cmp(&right.1));

        let mut edge_map = stable_map_with_capacity(keyed_edges.len());
        let mut edges = Vec::with_capacity(keyed_edges.len());
        for (key, edge) in keyed_edges {
            let index = if edges.last() == Some(&edge) {
                edges.len() - 1
            } else {
                edges.push(edge);
                edges.len() - 1
            };
            let index =
                u32::try_from(index).map_err(|_| crate::GraphError::IndexCapacityExceeded {
                    category: "frozen edges",
                    count: edges.len(),
                })?;
            edge_map.insert(key, EdgeIndex::new(index));
        }

        let graph = Graph::from_validated_sorted_parts(nodes, edges)?;
        Ok(FrozenGraph {
            graph,
            indices: FreezeMap {
                nodes: node_map,
                edges: edge_map,
            },
        })
    }
}
