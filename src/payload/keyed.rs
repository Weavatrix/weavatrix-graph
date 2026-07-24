use super::StablePayloadGraph;
use crate::{GraphError, Result, StableEdgeKey, StableNodeKey};
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as KeyMap;
use core::hash::Hash;
#[cfg(feature = "std")]
use std::collections::HashMap as KeyMap;

/// A GraphMap-style key index backed by generation-stable graph handles.
///
/// Keys provide domain lookup while algorithms continue to use compact stable
/// handles from [`StablePayloadGraph`].
#[derive(Debug, Clone)]
pub struct KeyedPayloadGraph<Key, NodePayload, EdgePayload> {
    graph: StablePayloadGraph<NodePayload, EdgePayload>,
    keys: KeyMap<Key, StableNodeKey>,
}

impl<Key, NodePayload, EdgePayload> Default for KeyedPayloadGraph<Key, NodePayload, EdgePayload>
where
    Key: Clone + Eq + Hash + Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Key, NodePayload, EdgePayload> KeyedPayloadGraph<Key, NodePayload, EdgePayload>
where
    Key: Clone + Eq + Hash + Ord,
{
    #[must_use]
    pub fn new() -> Self {
        Self {
            graph: StablePayloadGraph::new(),
            keys: key_map_with_capacity(0),
        }
    }

    #[must_use]
    pub fn with_capacity(nodes: usize, edges: usize) -> Self {
        Self {
            graph: StablePayloadGraph::with_capacity(nodes, edges),
            keys: key_map_with_capacity(nodes),
        }
    }

    /// Inserts a key or replaces its node payload without changing its handle.
    ///
    /// Returns the stable handle and the previous payload, when one existed.
    ///
    /// # Errors
    ///
    /// Returns an error when the stable node index space is exhausted.
    pub fn insert_node(
        &mut self,
        key: Key,
        payload: NodePayload,
    ) -> Result<(StableNodeKey, Option<NodePayload>)> {
        if let Some(handle) = self.keys.get(&key).copied() {
            if let Some(node) = self.graph.node_mut(handle) {
                return Ok((handle, Some(core::mem::replace(node, payload))));
            }
            self.keys.remove(&key);
        }
        let handle = self.graph.add_node(payload)?;
        self.keys.insert(key, handle);
        Ok((handle, None))
    }

    #[must_use]
    pub fn node_key(&self, key: &Key) -> Option<StableNodeKey> {
        self.keys.get(key).copied()
    }

    #[must_use]
    pub fn node(&self, key: &Key) -> Option<&NodePayload> {
        self.graph.node(self.node_key(key)?)
    }

    #[must_use]
    pub fn node_mut(&mut self, key: &Key) -> Option<&mut NodePayload> {
        let handle = self.keys.get(key).copied()?;
        self.graph.node_mut(handle)
    }

    /// Adds a directed edge between two existing domain keys.
    ///
    /// # Errors
    ///
    /// Returns an error for a missing endpoint or exhausted edge indices.
    pub fn add_edge(
        &mut self,
        source: &Key,
        target: &Key,
        payload: EdgePayload,
    ) -> Result<StableEdgeKey> {
        let source = self
            .node_key(source)
            .ok_or(GraphError::MissingKeyedNode { endpoint: "source" })?;
        let target = self
            .node_key(target)
            .ok_or(GraphError::MissingKeyedNode { endpoint: "target" })?;
        self.graph.add_edge(source, target, payload)
    }

    pub fn remove_node(&mut self, key: &Key) -> Option<NodePayload> {
        let handle = self.keys.remove(key)?;
        self.graph.remove_node(handle)
    }

    #[must_use]
    pub const fn graph(&self) -> &StablePayloadGraph<NodePayload, EdgePayload> {
        &self.graph
    }

    #[must_use]
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn into_parts(
        self,
    ) -> (
        StablePayloadGraph<NodePayload, EdgePayload>,
        impl Iterator<Item = (Key, StableNodeKey)>,
    ) {
        (self.graph, self.keys.into_iter())
    }
}

#[cfg(feature = "std")]
fn key_map_with_capacity<Key, Value>(capacity: usize) -> KeyMap<Key, Value> {
    KeyMap::with_capacity(capacity)
}

#[cfg(not(feature = "std"))]
fn key_map_with_capacity<Key, Value>(_capacity: usize) -> KeyMap<Key, Value> {
    KeyMap::new()
}
