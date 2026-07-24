use crate::{EdgeEndpoints, GraphError, IndexGraphView, NodeIndex, Result, Topology, Vec};

#[derive(Debug, Clone)]
pub struct TopologyProjection<Node> {
    topology: Topology,
    original_nodes: Vec<Node>,
}

impl<Node> TopologyProjection<Node> {
    #[must_use]
    pub const fn topology(&self) -> &Topology {
        &self.topology
    }

    #[must_use]
    pub fn original_nodes(&self) -> &[Node] {
        &self.original_nodes
    }

    #[must_use]
    pub fn original_node(&self, projected: NodeIndex) -> Option<&Node> {
        self.original_nodes.get(projected.index())
    }

    #[must_use]
    pub fn into_parts(self) -> (Topology, Vec<Node>) {
        (self.topology, self.original_nodes)
    }
}

/// Materializes the directed simple-graph complement.
///
/// Parallel edges collapse to one adjacency relation.
///
/// # Errors
///
/// Returns an error when the dense pair space or compact topology overflows.
pub fn complement<G>(graph: &G, include_self_loops: bool) -> Result<TopologyProjection<G::Node>>
where
    G: IndexGraphView,
{
    let nodes = graph.node_indices().collect::<Vec<_>>();
    let pair_count =
        nodes
            .len()
            .checked_mul(nodes.len())
            .ok_or(GraphError::ArithmeticOverflow {
                operation: "graph complement pair count",
            })?;
    let mut positions = vec![None; graph.node_bound()];
    for (index, node) in nodes.iter().copied().enumerate() {
        positions[G::node_slot(node)] = Some(index);
    }
    let mut present = vec![false; pair_count];
    for (_, endpoints) in graph.edge_references() {
        let Some(source) = positions[G::node_slot(endpoints.source())] else {
            continue;
        };
        let Some(target) = positions[G::node_slot(endpoints.target())] else {
            continue;
        };
        present[source * nodes.len() + target] = true;
    }
    let mut edges = Vec::new();
    for source in 0..nodes.len() {
        for target in 0..nodes.len() {
            if (include_self_loops || source != target) && !present[source * nodes.len() + target] {
                edges.push(projected_endpoints(source, target)?);
            }
        }
    }
    Ok(TopologyProjection {
        topology: Topology::try_from_edges(nodes.len(), edges)?,
        original_nodes: nodes,
    })
}

/// Materializes the simple union of two graphs by node identity.
///
/// # Errors
///
/// Returns an error when the compact result exceeds topology capacity.
pub fn union<Left, Right>(left: &Left, right: &Right) -> Result<TopologyProjection<Left::Node>>
where
    Left: IndexGraphView,
    Right: IndexGraphView<Node = Left::Node>,
{
    let mut nodes = left.node_indices().collect::<Vec<_>>();
    for node in right.node_indices() {
        if !nodes.contains(&node) {
            nodes.push(node);
        }
    }
    let mut edges = Vec::with_capacity(left.edge_count() + right.edge_count());
    append_projected_edges(left, &nodes, &mut edges)?;
    append_projected_edges(right, &nodes, &mut edges)?;
    edges.sort_unstable_by_key(|edge| (edge.source(), edge.target()));
    edges.dedup();
    Ok(TopologyProjection {
        topology: Topology::try_from_edges(nodes.len(), edges)?,
        original_nodes: nodes,
    })
}

fn append_projected_edges<G>(
    graph: &G,
    nodes: &[G::Node],
    edges: &mut Vec<EdgeEndpoints>,
) -> Result<()>
where
    G: IndexGraphView,
{
    for (_, endpoints) in graph.edge_references() {
        let source = nodes
            .iter()
            .position(|node| *node == endpoints.source())
            .ok_or(GraphError::InvalidNodeIndex {
                node: G::node_slot(endpoints.source()),
                node_count: nodes.len(),
            })?;
        let target = nodes
            .iter()
            .position(|node| *node == endpoints.target())
            .ok_or(GraphError::InvalidNodeIndex {
                node: G::node_slot(endpoints.target()),
                node_count: nodes.len(),
            })?;
        edges.push(projected_endpoints(source, target)?);
    }
    Ok(())
}

fn projected_endpoints(source: usize, target: usize) -> Result<EdgeEndpoints> {
    let source = u32::try_from(source).map_err(|_| GraphError::IndexCapacityExceeded {
        category: "nodes",
        count: source,
    })?;
    let target = u32::try_from(target).map_err(|_| GraphError::IndexCapacityExceeded {
        category: "nodes",
        count: target,
    })?;
    Ok(EdgeEndpoints::new(
        NodeIndex::new(source),
        NodeIndex::new(target),
    ))
}
