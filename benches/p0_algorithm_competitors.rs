mod support;

use petgraph::algo::{
    bridges as pet_bridges, floyd_warshall as pet_floyd, is_isomorphic as pet_isomorphic,
    johnson as pet_johnson,
};
use petgraph::graph::{DiGraph, UnGraph};
use std::hint::black_box;
use support::{measure, measure_batched, print_measurement, topology_pairs};
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, Topology, UndirectedTopology, bridges_and_articulation_points,
    floyd_warshall, graph_isomorphic, johnson_all_pairs,
};

fn main() {
    println!("statistic=median runs=11 warmups=2");
    compare_floyd();
    compare_johnson();
    compare_bridges();
    compare_isomorphism();
}

fn compare_floyd() {
    const NODES: usize = 160;
    let pairs = topology_pairs(NODES, 1_200);
    let weights = weights(&pairs);
    let ours = ours_graph(NODES, &pairs);
    let (pet, _) = pet_graph(NODES, &pairs, &weights);
    let ours_measurement =
        measure(|| black_box(floyd_warshall(&ours, |edge| weights[edge.index()]).unwrap()));
    let pet_measurement = measure(|| black_box(pet_floyd(&pet, |edge| *edge.weight()).unwrap()));
    print_measurement(
        "floyd-warshall",
        "library=weavatrix-graph",
        &ours_measurement,
    );
    print_measurement("floyd-warshall", "library=petgraph", &pet_measurement);
}

fn compare_johnson() {
    const NODES: usize = 800;
    let pairs = topology_pairs(NODES, 4_000);
    let weights = weights(&pairs);
    let ours = ours_graph(NODES, &pairs);
    let (pet, _) = pet_graph(NODES, &pairs, &weights);
    let ours_measurement =
        measure(|| black_box(johnson_all_pairs(&ours, |edge| weights[edge.index()]).unwrap()));
    let pet_measurement = measure(|| black_box(pet_johnson(&pet, |edge| *edge.weight()).unwrap()));
    print_measurement("johnson-apsp", "library=weavatrix-graph", &ours_measurement);
    print_measurement("johnson-apsp", "library=petgraph", &pet_measurement);
}

fn compare_bridges() {
    const NODES: usize = 5_000;
    let pairs = simple_pairs(NODES, 15_000);
    let ours = UndirectedTopology::try_from_edges(
        NODES,
        pairs
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node(source), node(target))),
    )
    .unwrap();
    let mut pet = UnGraph::<(), ()>::new_undirected();
    let nodes = (0..NODES).map(|_| pet.add_node(())).collect::<Vec<_>>();
    for &(source, target) in &pairs {
        pet.add_edge(nodes[source], nodes[target], ());
    }
    let ours_measurement = measure(|| black_box(bridges_and_articulation_points(&ours)));
    let pet_measurement = measure(|| black_box(pet_bridges(&pet).count()));
    print_measurement(
        "bridges-plus-articulation",
        "library=weavatrix-graph",
        &ours_measurement,
    );
    print_measurement(
        "bridges-only",
        "library=petgraph contract=narrower",
        &pet_measurement,
    );
}

fn compare_isomorphism() {
    const NODES: usize = 64;
    let pairs = simple_pairs(NODES, 300);
    let permutation = (0..NODES)
        .map(|index| (index * 11) % NODES)
        .collect::<Vec<_>>();
    let permuted = pairs
        .iter()
        .map(|&(source, target)| (permutation[source], permutation[target]))
        .collect::<Vec<_>>();
    let ours_left = ours_graph(NODES, &pairs);
    let ours_right = ours_graph(NODES, &permuted);
    let (pet_left, _) = pet_graph(NODES, &pairs, &vec![1; pairs.len()]);
    let (pet_right, _) = pet_graph(NODES, &permuted, &vec![1; pairs.len()]);
    let ours_measurement = measure_batched(256, || {
        black_box(graph_isomorphic(
            &ours_left,
            &ours_right,
            |_, _| true,
            |_, _| true,
        ))
    });
    let pet_measurement = measure_batched(256, || black_box(pet_isomorphic(&pet_left, &pet_right)));
    print_measurement(
        "graph-isomorphism",
        "library=weavatrix-graph",
        &ours_measurement,
    );
    print_measurement("graph-isomorphism", "library=petgraph", &pet_measurement);
}

fn ours_graph(node_count: usize, pairs: &[(usize, usize)]) -> Topology {
    Topology::try_from_edges(
        node_count,
        pairs
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node(source), node(target))),
    )
    .unwrap()
}

fn pet_graph(
    node_count: usize,
    pairs: &[(usize, usize)],
    weights: &[i64],
) -> (DiGraph<(), i64>, Vec<petgraph::graph::NodeIndex>) {
    let mut graph = DiGraph::new();
    let nodes = (0..node_count)
        .map(|_| graph.add_node(()))
        .collect::<Vec<_>>();
    for (index, &(source, target)) in pairs.iter().enumerate() {
        graph.add_edge(nodes[source], nodes[target], weights[index]);
    }
    (graph, nodes)
}

fn weights(pairs: &[(usize, usize)]) -> Vec<i64> {
    pairs
        .iter()
        .map(|&(source, target)| i64::try_from((source * 17 + target * 31) % 19 + 1).unwrap())
        .collect()
}

fn simple_pairs(node_count: usize, edge_count: usize) -> Vec<(usize, usize)> {
    let mut pairs = topology_pairs(node_count, edge_count * 2);
    for pair in &mut pairs {
        if pair.0 > pair.1 {
            *pair = (pair.1, pair.0);
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    pairs.truncate(edge_count);
    pairs
}

fn node(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap())
}
