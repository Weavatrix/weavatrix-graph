#![allow(clippy::cast_precision_loss)]

mod support;

use petgraph::graph::UnGraph;
use rustworkx_core::connectivity::chain_decomposition as rustworkx_chains;
use std::collections::{BTreeSet, VecDeque};
use std::hint::black_box;
use support::{measure, print_measurement, undirected_pairs};
use weavatrix_graph::{
    ChainDecomposition, EdgeEndpoints, EdgeIndex, NodeIndex, UndirectedTopology,
    chain_decomposition, distance_analytics,
};

const DISTANCE_NODES: usize = 1_500;
const DISTANCE_EDGES: usize = 4_500;
const CHAIN_NODES: usize = 20_000;
const CHAIN_EDGES: usize = 60_000;

fn main() {
    println!("statistic=median runs=11 warmups=2");
    compare_distance_analytics();
    compare_chain_decomposition();
}

fn compare_distance_analytics() {
    let pairs = undirected_pairs(DISTANCE_NODES, DISTANCE_EDGES);
    let (ours, pet) = graphs(DISTANCE_NODES, &pairs);
    let ours_result = distance_analytics(&ours).unwrap();
    let pet_result = petgraph_distance_analytics(&pet).unwrap();
    assert_eq!(
        ours_result
            .eccentricities()
            .iter()
            .map(|pair| pair.1)
            .collect::<Vec<_>>(),
        pet_result.eccentricities
    );
    assert_eq!(ours_result.radius(), pet_result.radius);
    assert_eq!(ours_result.diameter(), pet_result.diameter);
    assert_eq!(
        ours_result
            .center()
            .iter()
            .map(|node| node.index())
            .collect::<Vec<_>>(),
        pet_result.center
    );
    assert_eq!(
        ours_result
            .periphery()
            .iter()
            .map(|node| node.index())
            .collect::<Vec<_>>(),
        pet_result.periphery
    );
    print_measurement(
        "distance-analytics",
        "library=weavatrix-graph output=eccentricity+radius+diameter+center+periphery",
        &measure(|| black_box(distance_analytics(&ours).unwrap())),
    );
    print_measurement(
        "distance-analytics",
        "library=petgraph-adapter output=eccentricity+radius+diameter+center+periphery",
        &measure(|| black_box(petgraph_distance_analytics(&pet).unwrap())),
    );
}

fn compare_chain_decomposition() {
    let pairs = undirected_pairs(CHAIN_NODES, CHAIN_EDGES);
    let (ours, pet) = graphs(CHAIN_NODES, &pairs);
    let ours_result = chain_decomposition(&ours);
    let pet_result = rustworkx_chains(&pet, None);
    assert_eq!(
        ours_chain_edges(&ours_result, &pairs),
        rustworkx_chain_edges(&pet_result)
    );
    assert_eq!(ours_result.chain_count(), pet_result.len());
    print_measurement(
        "chain-decomposition",
        "library=weavatrix-graph output=edge-ids multigraph-safe",
        &measure(|| black_box(chain_decomposition(&ours))),
    );
    print_measurement(
        "chain-decomposition",
        "library=rustworkx-core output=node-pairs simple-graph-only",
        &measure(|| black_box(rustworkx_chains(&pet, None))),
    );
}

struct ReferenceMetrics {
    eccentricities: Vec<usize>,
    radius: usize,
    diameter: usize,
    center: Vec<usize>,
    periphery: Vec<usize>,
}

fn petgraph_distance_analytics(graph: &UnGraph<(), ()>) -> Option<ReferenceMetrics> {
    if graph.node_count() == 0 {
        return None;
    }
    let mut eccentricities = Vec::with_capacity(graph.node_count());
    let mut distances = vec![usize::MAX; graph.node_count()];
    let mut queue = VecDeque::new();
    for source in graph.node_indices() {
        distances.fill(usize::MAX);
        distances[source.index()] = 0;
        queue.push_back(source);
        while let Some(node) = queue.pop_front() {
            for neighbor in graph.neighbors(node) {
                if distances[neighbor.index()] == usize::MAX {
                    distances[neighbor.index()] = distances[node.index()] + 1;
                    queue.push_back(neighbor);
                }
            }
        }
        let value = distances.iter().copied().max()?;
        if value == usize::MAX {
            return None;
        }
        eccentricities.push(value);
    }
    let radius = eccentricities.iter().copied().min()?;
    let diameter = eccentricities.iter().copied().max()?;
    let center = select(&eccentricities, radius);
    let periphery = select(&eccentricities, diameter);
    Some(ReferenceMetrics {
        eccentricities,
        radius,
        diameter,
        center,
        periphery,
    })
}

fn select(values: &[usize], target: usize) -> Vec<usize> {
    values
        .iter()
        .enumerate()
        .filter_map(|(node, &value)| (value == target).then_some(node))
        .collect()
}

fn graphs(node_count: usize, pairs: &[(usize, usize)]) -> (UndirectedTopology, UnGraph<(), ()>) {
    let ours = UndirectedTopology::try_from_edges(
        node_count,
        pairs
            .iter()
            .map(|&(source, target)| endpoints(source, target)),
    )
    .unwrap();
    let mut pet = UnGraph::with_capacity(node_count, pairs.len());
    let nodes = (0..node_count)
        .map(|_| pet.add_node(()))
        .collect::<Vec<_>>();
    for &(source, target) in pairs {
        pet.add_edge(nodes[source], nodes[target], ());
    }
    (ours, pet)
}

fn ours_chain_edges(
    result: &ChainDecomposition<NodeIndex, EdgeIndex>,
    pairs: &[(usize, usize)],
) -> BTreeSet<(usize, usize)> {
    result
        .chains()
        .iter()
        .flatten()
        .map(|step| {
            let (source, target) = pairs[step.edge().index()];
            (source.min(target), source.max(target))
        })
        .collect()
}

fn rustworkx_chain_edges(
    chains: &[Vec<(petgraph::graph::NodeIndex, petgraph::graph::NodeIndex)>],
) -> BTreeSet<(usize, usize)> {
    chains
        .iter()
        .flatten()
        .map(|&(source, target)| {
            let source = source.index();
            let target = target.index();
            (source.min(target), source.max(target))
        })
        .collect()
}

fn endpoints(source: usize, target: usize) -> EdgeEndpoints {
    EdgeEndpoints::new(node(source), node(target))
}

fn node(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap())
}
