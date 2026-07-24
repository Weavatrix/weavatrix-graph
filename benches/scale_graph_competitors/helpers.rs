use super::{EDGES, NODES};
use graaf::{AddArc, AdjacencyList, Empty};
use petgraph::Directed;
use petgraph::csr::Csr;
use petgraph::graph::{DiGraph, NodeIndex as PetNodeIndex};
use petgraph::visit::Bfs as PetBfs;
use std::collections::HashMap;
use weavatrix_graph::{
    Confidence, Edge, EdgeEndpoints, EdgeKind, EvidenceKind, Node, NodeId, NodeIndex, NodeKind,
    Provenance,
};

type Endpoint = (usize, usize);
type Directions = (Vec<Endpoint>, Vec<Endpoint>);
type PetCsr = Csr<(), (), Directed, usize>;
type PetDualCsr = (PetCsr, PetCsr);

pub(super) fn pet_graph(pairs: &[Endpoint]) -> DiGraph<(), u64> {
    let mut graph = DiGraph::with_capacity(NODES, EDGES);
    let nodes = (0..NODES).map(|_| graph.add_node(())).collect::<Vec<_>>();
    for (index, &(source, target)) in pairs.iter().enumerate() {
        graph.add_edge(nodes[source], nodes[target], weight(index));
    }
    graph
}

pub(super) fn pet_dual_csr(pairs: &[Endpoint]) -> PetDualCsr {
    let (forward, reverse) = sorted_directions(pairs);
    pet_dual_csr_sorted(&forward, &reverse)
}

pub(super) fn pet_dual_csr_sorted(forward: &[Endpoint], reverse: &[Endpoint]) -> PetDualCsr {
    (
        Csr::from_sorted_edges(forward).unwrap(),
        Csr::from_sorted_edges(reverse).unwrap(),
    )
}

pub(super) fn sorted_directions(pairs: &[Endpoint]) -> Directions {
    let mut forward = pairs.to_vec();
    forward.sort_unstable();
    forward.dedup();
    let mut reverse = forward
        .iter()
        .map(|&(source, target)| (target, source))
        .collect::<Vec<_>>();
    reverse.sort_unstable();
    (forward, reverse)
}

pub(super) fn graaf_graph(pairs: &[Endpoint]) -> AdjacencyList {
    let mut graph = AdjacencyList::empty(NODES);
    for &(source, target) in pairs {
        graph.add_arc(source, target);
    }
    graph
}

pub(super) fn pet_bfs(graph: &DiGraph<(), u64>) -> Vec<PetNodeIndex> {
    let mut traversal = PetBfs::new(graph, PetNodeIndex::new(0));
    let mut nodes = Vec::new();
    while let Some(node) = traversal.next(graph) {
        nodes.push(node);
    }
    nodes
}

pub(super) fn rich_parts() -> (Vec<Node>, Vec<Edge>) {
    let nodes = (0..NODES)
        .map(|index| {
            Node::new(
                format!("file:{index:06}"),
                format!("file_{index}.rs"),
                NodeKind::File,
            )
            .unwrap()
            .with_language("rust")
        })
        .collect::<Vec<_>>();
    let provenance =
        Provenance::new("scale.graph", EvidenceKind::Resolved, Confidence::High).unwrap();
    let mut edges = super::topology_pairs(NODES, EDGES)
        .into_iter()
        .map(|(source, target)| {
            Edge::new(
                NodeId::new(format!("file:{source:06}")).unwrap(),
                NodeId::new(format!("file:{target:06}")).unwrap(),
                EdgeKind::Calls,
                provenance.clone(),
            )
        })
        .collect::<Vec<_>>();
    edges.sort_unstable();
    (nodes, edges)
}

pub(super) fn rich_petgraph(nodes: Vec<Node>, edges: Vec<Edge>) -> DiGraph<Node, Edge> {
    let mut graph = DiGraph::with_capacity(nodes.len(), edges.len());
    let mut positions = HashMap::with_capacity(nodes.len());
    for node in nodes {
        positions.insert(node.id.clone(), graph.add_node(node));
    }
    for edge in edges {
        graph.add_edge(positions[&edge.source], positions[&edge.target], edge);
    }
    graph
}

pub(super) fn compact_edges(pairs: &[Endpoint]) -> Vec<EdgeEndpoints> {
    pairs
        .iter()
        .map(|&(source, target)| EdgeEndpoints::new(node(source), node(target)))
        .collect()
}

pub(super) fn edge_weight(edge: weavatrix_graph::EdgeIndex) -> u64 {
    weight(edge.index())
}

fn weight(index: usize) -> u64 {
    u64::try_from(index % 97 + 1).unwrap()
}

pub(super) fn node(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap())
}
