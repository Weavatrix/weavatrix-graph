use super::StablePayloadGraph;
use crate::{
    Direction, EdgeEndpoints, GraphError, GraphView, IndexGraphView, Result, StableEdgeKey,
    StableNodeKey, has_cycle, reachable, reachable_filtered,
};

/// A stable mutable payload graph that rejects cycle-creating mutations.
#[derive(Debug, Clone, Default)]
pub struct AcyclicPayloadGraph<NodePayload, EdgePayload> {
    graph: StablePayloadGraph<NodePayload, EdgePayload>,
}

impl<NodePayload, EdgePayload> AcyclicPayloadGraph<NodePayload, EdgePayload> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            graph: StablePayloadGraph::new(),
        }
    }

    /// Wraps an existing graph after validating the DAG invariant.
    ///
    /// # Errors
    ///
    /// Returns an error if the input contains a directed cycle.
    pub fn try_from_graph(graph: StablePayloadGraph<NodePayload, EdgePayload>) -> Result<Self> {
        if has_cycle(&graph) {
            Err(GraphError::CycleWouldBeCreated)
        } else {
            Ok(Self { graph })
        }
    }

    /// Adds an arbitrary node payload.
    ///
    /// # Errors
    ///
    /// Returns an error when stable index capacity is exhausted.
    pub fn add_node(&mut self, payload: NodePayload) -> Result<StableNodeKey> {
        self.graph.add_node(payload)
    }

    /// Adds an edge only when the graph remains acyclic.
    ///
    /// # Errors
    ///
    /// Returns an error for stale endpoints, exhausted capacity, or a cycle.
    pub fn add_edge(
        &mut self,
        source: StableNodeKey,
        target: StableNodeKey,
        payload: EdgePayload,
    ) -> Result<StableEdgeKey> {
        self.graph.require_node(source)?;
        self.graph.require_node(target)?;
        if source == target || reachable(&self.graph, target, source) {
            return Err(GraphError::CycleWouldBeCreated);
        }
        self.graph.add_edge(source, target, payload)
    }

    /// Retargets an edge only when the graph remains acyclic.
    ///
    /// # Errors
    ///
    /// Returns an error for stale endpoints or a cycle.
    pub fn set_edge_endpoints(
        &mut self,
        edge: StableEdgeKey,
        source: StableNodeKey,
        target: StableNodeKey,
    ) -> Result<bool> {
        self.graph.require_node(source)?;
        self.graph.require_node(target)?;
        if source == target
            || reachable_filtered(
                &self.graph,
                target,
                source,
                Direction::Outgoing,
                |candidate| candidate != edge,
            )
        {
            return Err(GraphError::CycleWouldBeCreated);
        }
        self.graph.set_edge_endpoints(edge, source, target)
    }

    pub fn remove_node(&mut self, key: StableNodeKey) -> Option<NodePayload> {
        self.graph.remove_node(key)
    }

    pub fn remove_edge(&mut self, key: StableEdgeKey) -> Option<EdgePayload> {
        self.graph.remove_edge(key)
    }

    #[must_use]
    pub fn node(&self, key: StableNodeKey) -> Option<&NodePayload> {
        self.graph.node(key)
    }

    #[must_use]
    pub fn node_mut(&mut self, key: StableNodeKey) -> Option<&mut NodePayload> {
        self.graph.node_mut(key)
    }

    #[must_use]
    pub fn edge(&self, key: StableEdgeKey) -> Option<&EdgePayload> {
        self.graph.edge(key)
    }

    #[must_use]
    pub fn edge_mut(&mut self, key: StableEdgeKey) -> Option<&mut EdgePayload> {
        self.graph.edge_mut(key)
    }

    #[must_use]
    pub const fn graph(&self) -> &StablePayloadGraph<NodePayload, EdgePayload> {
        &self.graph
    }

    #[must_use]
    pub fn into_inner(self) -> StablePayloadGraph<NodePayload, EdgePayload> {
        self.graph
    }
}

impl<NodePayload, EdgePayload> GraphView for AcyclicPayloadGraph<NodePayload, EdgePayload> {
    type Node = StableNodeKey;
    type Edge = StableEdgeKey;

    fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    fn contains_node(&self, node: Self::Node) -> bool {
        self.graph.contains_node(node)
    }

    fn contains_edge(&self, edge: Self::Edge) -> bool {
        self.graph.contains_edge(edge)
    }

    fn node_indices(&self) -> impl Iterator<Item = Self::Node> + '_ {
        self.graph.node_indices()
    }

    fn edge_indices(&self) -> impl Iterator<Item = Self::Edge> + '_ {
        self.graph.edge_indices()
    }

    fn edge_endpoints(&self, edge: Self::Edge) -> Option<EdgeEndpoints<Self::Node>> {
        self.graph.edge_endpoints(edge)
    }

    fn outgoing_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        self.graph.outgoing_edges(node)
    }

    fn incoming_edges(&self, node: Self::Node) -> impl Iterator<Item = Self::Edge> + '_ {
        self.graph.incoming_edges(node)
    }
}

impl<NodePayload, EdgePayload> IndexGraphView for AcyclicPayloadGraph<NodePayload, EdgePayload> {
    fn node_bound(&self) -> usize {
        self.graph.node_bound()
    }

    fn edge_bound(&self) -> usize {
        self.graph.edge_bound()
    }

    fn node_slot(node: Self::Node) -> usize {
        node.index()
    }

    fn edge_slot(edge: Self::Edge) -> usize {
        edge.index()
    }
}
