mod support;

use petgraph::graphmap::DiGraphMap;
use petgraph::stable_graph::StableUnGraph;
use std::hint::black_box;
use support::{measure, measure_batched_with_setup, print_measurement, topology_pairs};
use weavatrix_graph::{KeyedPayloadGraph, StableUndirectedPayloadGraph};

const NODES: usize = 10_000;
const EDGES: usize = 30_000;
const CHURN: usize = 1_000;

fn main() {
    println!("statistic=median runs=11 warmups=2");
    compare_keyed_build();
    compare_stable_undirected_build();
    compare_stable_undirected_churn();
}

fn compare_keyed_build() {
    let pairs = topology_pairs(NODES, EDGES);
    let ours = measure(|| black_box(build_keyed_ours(&pairs)));
    let pet = measure(|| black_box(build_keyed_petgraph(&pairs)));
    print_measurement(
        "keyed-directed-build",
        "library=weavatrix-graph stable-generations=true node-payloads=true",
        &ours,
    );
    print_measurement(
        "keyed-directed-build",
        "library=petgraph-graphmap stable-generations=false node-payloads=false",
        &pet,
    );
}

fn compare_stable_undirected_build() {
    let pairs = topology_pairs(NODES, EDGES);
    let ours = measure(|| black_box(build_stable_ours(&pairs)));
    let pet = measure(|| black_box(build_stable_petgraph(&pairs)));
    print_measurement(
        "stable-undirected-build",
        "library=weavatrix-graph generation-checked=true",
        &ours,
    );
    print_measurement(
        "stable-undirected-build",
        "library=petgraph-stable-ungraph generation-checked=false",
        &pet,
    );
}

fn compare_stable_undirected_churn() {
    let pairs = topology_pairs(NODES, EDGES);
    let ours = build_stable_ours(&pairs);
    let pet = build_stable_petgraph(&pairs);
    let ours_measurement = measure_batched_with_setup(
        1,
        || ours.clone(),
        |mut graph| {
            let edges = graph
                .edges()
                .take(CHURN)
                .map(|pair| pair.0)
                .collect::<Vec<_>>();
            let nodes = graph.nodes().map(|pair| pair.0).collect::<Vec<_>>();
            for edge in edges {
                black_box(graph.remove_edge(edge));
            }
            for index in 0..CHURN {
                black_box(
                    graph
                        .add_edge(nodes[index], nodes[(index * 37 + 17) % NODES], index)
                        .unwrap(),
                );
            }
            graph
        },
    );
    let pet_measurement = measure_batched_with_setup(
        1,
        || pet.clone(),
        |mut graph| {
            let edges = graph.edge_indices().take(CHURN).collect::<Vec<_>>();
            let nodes = graph.node_indices().collect::<Vec<_>>();
            for edge in edges {
                black_box(graph.remove_edge(edge));
            }
            for index in 0..CHURN {
                black_box(graph.add_edge(nodes[index], nodes[(index * 37 + 17) % NODES], index));
            }
            graph
        },
    );
    print_measurement(
        "stable-undirected-edge-churn",
        "library=weavatrix-graph remove=1000 insert=1000",
        &ours_measurement,
    );
    print_measurement(
        "stable-undirected-edge-churn",
        "library=petgraph-stable-ungraph remove=1000 insert=1000",
        &pet_measurement,
    );
}

fn build_keyed_ours(pairs: &[(usize, usize)]) -> KeyedPayloadGraph<u32, (), ()> {
    let mut graph = KeyedPayloadGraph::with_capacity(NODES, EDGES);
    for node in 0..NODES {
        graph.insert_node(u32::try_from(node).unwrap(), ()).unwrap();
    }
    for &(source, target) in pairs {
        graph
            .add_edge(
                &u32::try_from(source).unwrap(),
                &u32::try_from(target).unwrap(),
                (),
            )
            .unwrap();
    }
    assert_eq!(graph.node_count(), NODES);
    assert_eq!(graph.edge_count(), EDGES);
    graph
}

fn build_keyed_petgraph(pairs: &[(usize, usize)]) -> DiGraphMap<u32, ()> {
    let mut graph = DiGraphMap::with_capacity(NODES, EDGES);
    for node in 0..NODES {
        graph.add_node(u32::try_from(node).unwrap());
    }
    for &(source, target) in pairs {
        graph.add_edge(
            u32::try_from(source).unwrap(),
            u32::try_from(target).unwrap(),
            (),
        );
    }
    assert_eq!(graph.node_count(), NODES);
    assert_eq!(graph.edge_count(), EDGES);
    graph
}

fn build_stable_ours(pairs: &[(usize, usize)]) -> StableUndirectedPayloadGraph<(), usize> {
    let mut graph = StableUndirectedPayloadGraph::with_capacity(NODES, EDGES);
    let nodes = (0..NODES)
        .map(|_| graph.add_node(()).unwrap())
        .collect::<Vec<_>>();
    for (edge, &(source, target)) in pairs.iter().enumerate() {
        graph.add_edge(nodes[source], nodes[target], edge).unwrap();
    }
    graph
}

fn build_stable_petgraph(pairs: &[(usize, usize)]) -> StableUnGraph<(), usize> {
    let mut graph = StableUnGraph::with_capacity(NODES, EDGES);
    let nodes = (0..NODES).map(|_| graph.add_node(())).collect::<Vec<_>>();
    for (edge, &(source, target)) in pairs.iter().enumerate() {
        graph.add_edge(nodes[source], nodes[target], edge);
    }
    graph
}
