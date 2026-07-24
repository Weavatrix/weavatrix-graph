use super::scc;
use crate::Vec;
use crate::{EdgeEndpoints, GraphError, IndexGraphView, NodeIndex, Result, Topology};

#[derive(Clone, Debug)]
pub struct Condensation<Node> {
    components: Vec<Vec<Node>>,
    component_by_node: Vec<(Node, NodeIndex)>,
    topology: Topology,
}

impl<Node> Condensation<Node>
where
    Node: Copy + Eq,
{
    #[must_use]
    pub fn components(&self) -> &[Vec<Node>] {
        &self.components
    }

    #[must_use]
    pub fn component(&self, index: NodeIndex) -> Option<&[Node]> {
        self.components.get(index.index()).map(Vec::as_slice)
    }

    #[must_use]
    pub fn component_of(&self, node: Node) -> Option<NodeIndex> {
        self.component_by_node
            .iter()
            .find_map(|&(candidate, index)| (candidate == node).then_some(index))
    }

    #[must_use]
    pub const fn topology(&self) -> &Topology {
        &self.topology
    }

    #[must_use]
    pub fn into_parts(self) -> (Vec<Vec<Node>>, Topology) {
        (self.components, self.topology)
    }
}

/// Builds the acyclic graph of strongly connected components.
///
/// # Errors
///
/// Returns an error when the compact component topology exceeds index capacity.
pub fn condensation<G>(graph: &G) -> Result<Condensation<G::Node>>
where
    G: IndexGraphView,
{
    let allows_edge = |_| true;
    let components = scc::with_filter(graph, &allows_edge);
    build(graph, components, &allows_edge)
}

/// Builds a condensation DAG using only edges accepted by `allows_edge`.
///
/// # Errors
///
/// Returns an error when the compact component topology exceeds index capacity.
pub fn condensation_filtered<G, F>(graph: &G, allows_edge: F) -> Result<Condensation<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    let components = scc::with_filter(graph, &allows_edge);
    build(graph, components, &allows_edge)
}

fn build<G, F>(
    graph: &G,
    components: Vec<Vec<G::Node>>,
    allows_edge: &F,
) -> Result<Condensation<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    let mut component_by_node = Vec::with_capacity(graph.node_count());
    let mut component_by_slot = vec![None; graph.node_bound()];
    for (position, component) in components.iter().enumerate() {
        let compact = u32::try_from(position).map_err(|_| GraphError::IndexCapacityExceeded {
            category: "components",
            count: components.len(),
        })?;
        for &node in component {
            let index = NodeIndex::new(compact);
            component_by_node.push((node, index));
            if let Some(slot) = component_by_slot.get_mut(G::node_slot(node)) {
                *slot = Some(index);
            }
        }
    }

    let mut edges = Vec::new();
    for (edge, endpoints) in graph.edge_references() {
        if !allows_edge(edge) {
            continue;
        }
        let (Some(Some(source)), Some(Some(target))) = (
            component_by_slot.get(G::node_slot(endpoints.source())),
            component_by_slot.get(G::node_slot(endpoints.target())),
        ) else {
            continue;
        };
        if source != target {
            edges.push(EdgeEndpoints::new(*source, *target));
        }
    }
    edges.sort_unstable_by_key(|edge| (edge.source().index(), edge.target().index()));
    edges.dedup();
    let topology = Topology::try_from_edges(components.len(), edges)?;
    Ok(Condensation {
        components,
        component_by_node,
        topology,
    })
}
