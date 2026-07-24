#![allow(clippy::cast_precision_loss)]

mod support;

use biconnected_components::Bcc;
use petgraph::graph::{DiGraph, UnGraph};
use petgraph::visit::EdgeRef;
use std::collections::BTreeSet;
use std::hint::black_box;
use support::{measure, print_measurement, topology_pairs, undirected_pairs};
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, Topology, UndirectedTopology, biconnected_components, hits,
};

const NODES: usize = 10_000;
const EDGES: usize = 30_000;
const BCC_NODES: usize = 2_000;
const BCC_EDGES: usize = 6_000;

fn main() {
    println!("statistic=median runs=11 warmups=2");
    compare_biconnected();
    compare_hits();
}

fn compare_biconnected() {
    let pairs = undirected_pairs(BCC_NODES, BCC_EDGES);
    let ours = UndirectedTopology::try_from_edges(
        BCC_NODES,
        pairs
            .iter()
            .map(|&(source, target)| endpoints(source, target)),
    )
    .unwrap();
    let mut pet = UnGraph::<(), ()>::with_capacity(BCC_NODES, BCC_EDGES);
    let nodes = (0..BCC_NODES).map(|_| pet.add_node(())).collect::<Vec<_>>();
    for &(source, target) in &pairs {
        pet.add_edge(nodes[source], nodes[target], ());
    }
    let ours_nodes = ours_biconnected_nodes(&ours, &pairs);
    let mut pet_nodes = pet
        .bcc()
        .into_iter()
        .map(|component| {
            component
                .into_iter()
                .map(petgraph::graph::NodeIndex::index)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    canonicalize(&mut pet_nodes);
    assert_eq!(ours_nodes, pet_nodes);
    let ours_full = ours_biconnected_full(&ours);
    let pet_full = pet_biconnected_full(&pet, &pairs);
    assert_eq!(ours_full, pet_full);
    print_measurement(
        "biconnected-components",
        "library=weavatrix-graph output=edge-blocks+articulation-points",
        &measure(|| black_box(biconnected_components(&ours))),
    );
    print_measurement(
        "biconnected-components",
        "library=biconnected-components-adapter output=edge-blocks+articulation-points",
        &measure(|| black_box(pet_biconnected_full(&pet, &pairs))),
    );
    print_measurement(
        "biconnected-components",
        "library=biconnected-components output=node-blocks narrower-lower-bound=true",
        &measure(|| black_box(pet.bcc())),
    );
}

fn compare_hits() {
    let pairs = directed_pairs(NODES, EDGES);
    let ours = Topology::try_from_edges(
        NODES,
        pairs
            .iter()
            .map(|&(source, target)| endpoints(source, target)),
    )
    .unwrap();
    let mut pet = DiGraph::<(), ()>::with_capacity(NODES, EDGES);
    let nodes = (0..NODES).map(|_| pet.add_node(())).collect::<Vec<_>>();
    for &(source, target) in &pairs {
        pet.add_edge(nodes[source], nodes[target], ());
    }
    let ours_result = hits(&ours, 100, 1e-10).unwrap();
    let pet_result = petgraph_hits(&pet, 100, 1e-10);
    assert_eq!(ours_result.converged(), pet_result.3);
    for index in 0..NODES {
        assert!((ours_result.hub(node(index)).unwrap() - pet_result.0[index]).abs() < 1e-8);
        assert!((ours_result.authority(node(index)).unwrap() - pet_result.1[index]).abs() < 1e-8);
    }
    print_measurement(
        "hits-hubs-authorities",
        "library=weavatrix-graph normalization=l2",
        &measure(|| black_box(hits(&ours, 100, 1e-10).unwrap())),
    );
    print_measurement(
        "hits-hubs-authorities",
        "library=petgraph-adapter normalization=l2",
        &measure(|| black_box(petgraph_hits(&pet, 100, 1e-10))),
    );
}

fn petgraph_hits(
    graph: &DiGraph<(), ()>,
    max_iterations: usize,
    tolerance: f64,
) -> (Vec<f64>, Vec<f64>, usize, bool) {
    let node_count = graph.node_count();
    let mut edges = graph
        .edge_references()
        .filter_map(|edge| {
            let source = edge.source().index();
            let target = edge.target().index();
            (source != target).then_some((source, target))
        })
        .collect::<Vec<_>>();
    edges.sort_unstable();
    edges.dedup();
    let initial = 1.0 / (node_count as f64).sqrt();
    let mut hubs = vec![initial; node_count];
    let mut authorities = vec![initial; node_count];
    for iteration in 1..=max_iterations {
        let mut next_authorities = vec![0.0; node_count];
        for &(source, target) in &edges {
            next_authorities[target] += hubs[source];
        }
        normalize(&mut next_authorities);
        let mut next_hubs = vec![0.0; node_count];
        for &(source, target) in &edges {
            next_hubs[source] += next_authorities[target];
        }
        normalize(&mut next_hubs);
        let converged = hubs
            .iter()
            .zip(&next_hubs)
            .chain(authorities.iter().zip(&next_authorities))
            .all(|(left, right)| (*left - *right).abs() <= tolerance);
        hubs = next_hubs;
        authorities = next_authorities;
        if converged {
            return (hubs, authorities, iteration, true);
        }
    }
    (hubs, authorities, max_iterations, false)
}

fn ours_biconnected_nodes(graph: &UndirectedTopology, pairs: &[(usize, usize)]) -> Vec<Vec<usize>> {
    let mut components = biconnected_components(graph)
        .components()
        .iter()
        .map(|component| {
            component
                .iter()
                .flat_map(|edge| {
                    let (source, target) = pairs[edge.index()];
                    [source, target]
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    canonicalize(&mut components);
    components
}

fn ours_biconnected_full(graph: &UndirectedTopology) -> (Vec<Vec<usize>>, Vec<usize>) {
    let result = biconnected_components(graph);
    (
        result
            .components()
            .iter()
            .map(|block| block.iter().map(|edge| edge.index()).collect())
            .collect(),
        result
            .articulation_points()
            .iter()
            .map(|node| node.index())
            .collect(),
    )
}

fn pet_biconnected_full(
    graph: &UnGraph<(), ()>,
    pairs: &[(usize, usize)],
) -> (Vec<Vec<usize>>, Vec<usize>) {
    let node_blocks = graph
        .bcc()
        .into_iter()
        .map(|block| {
            block
                .into_iter()
                .map(petgraph::graph::NodeIndex::index)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut memberships = vec![Vec::new(); graph.node_count()];
    for (block_index, block) in node_blocks.iter().enumerate() {
        for &node in block {
            memberships[node].push(block_index);
        }
    }
    let articulation_points = memberships
        .iter()
        .enumerate()
        .filter_map(|(node, blocks)| (blocks.len() > 1).then_some(node))
        .collect();
    let mut edge_blocks = vec![Vec::new(); node_blocks.len()];
    for (edge, &(source, target)) in pairs.iter().enumerate() {
        let block = memberships[source]
            .iter()
            .copied()
            .find(|candidate| memberships[target].contains(candidate))
            .expect("each edge belongs to a biconnected block");
        edge_blocks[block].push(edge);
    }
    edge_blocks.retain(|block| !block.is_empty());
    edge_blocks.sort_unstable_by_key(|block| block[0]);
    (edge_blocks, articulation_points)
}

fn canonicalize(components: &mut Vec<Vec<usize>>) {
    for component in &mut *components {
        component.sort_unstable();
        component.dedup();
    }
    components.sort_unstable();
}

fn normalize(values: &mut [f64]) {
    let norm = values.iter().map(|value| value * value).sum::<f64>().sqrt();
    if norm > 0.0 {
        for value in values {
            *value /= norm;
        }
    }
}

fn directed_pairs(node_count: usize, edge_count: usize) -> Vec<(usize, usize)> {
    topology_pairs(node_count, edge_count * 3)
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(edge_count)
        .collect()
}

fn endpoints(source: usize, target: usize) -> EdgeEndpoints {
    EdgeEndpoints::new(node(source), node(target))
}

fn node(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap())
}
