mod support;

use petgraph::algo::dijkstra as pet_dijkstra;
use petgraph::graph::DiGraph;
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::{Bfs as PetBfs, Walker};
use std::hint::black_box;
use support::{measure, print_measurement, topology_pairs};
use weavatrix_graph::{
    DijkstraWorkspace, EdgeEndpoints, NodeIndex, StablePayloadGraph, Topology, TraversalWorkspace,
    bfs_iter, dijkstra_iter,
};

const NODES: usize = 50_000;
const EDGES: usize = 150_000;

fn main() {
    println!("statistic=median runs=11 warmups=2");
    let pairs = topology_pairs(NODES, EDGES);
    let ours = topology(NODES, &pairs);
    let (pet, pet_nodes) = pet_graph(NODES, &pairs);
    compare_early_bfs(&ours, &pet, &pet_nodes);
    compare_generic_dijkstra(&ours, &pet, &pet_nodes);
    compare_stable_mutation(&pairs);
}

fn compare_early_bfs(
    ours: &Topology,
    pet: &DiGraph<(), f64>,
    pet_nodes: &[petgraph::graph::NodeIndex],
) {
    const LIMIT: usize = 128;
    let mut workspace = TraversalWorkspace::new();
    let ours_count = bfs_iter(ours, node(0), &mut workspace).take(LIMIT).count();
    let pet_count = PetBfs::new(pet, pet_nodes[0]).iter(pet).take(LIMIT).count();
    assert_eq!(ours_count, pet_count);
    let ours_measurement =
        measure(|| black_box(bfs_iter(ours, node(0), &mut workspace).take(LIMIT).count()));
    let pet_measurement =
        measure(|| black_box(PetBfs::new(pet, pet_nodes[0]).iter(pet).take(LIMIT).count()));
    print_measurement(
        "lazy-bfs-first-128",
        "library=weavatrix-graph workspace=reused",
        &ours_measurement,
    );
    print_measurement("lazy-bfs-first-128", "library=petgraph", &pet_measurement);
}

fn compare_generic_dijkstra(
    ours: &Topology,
    pet: &DiGraph<(), f64>,
    pet_nodes: &[petgraph::graph::NodeIndex],
) {
    let target = node(NODES - 1);
    let mut workspace = DijkstraWorkspace::new();
    let ours_cost = dijkstra_iter(ours, node(0), &mut workspace, edge_weight).find_map(|result| {
        let (current, cost) = result.unwrap();
        (current == target).then_some(cost)
    });
    let pet_cost = pet_dijkstra(pet, pet_nodes[0], Some(pet_nodes[NODES - 1]), |edge| {
        *edge.weight()
    })
    .get(&pet_nodes[NODES - 1])
    .copied();
    assert_eq!(ours_cost, pet_cost);
    let ours_measurement = measure(|| {
        black_box(
            dijkstra_iter(ours, node(0), &mut workspace, edge_weight).find_map(|result| {
                let (current, cost) = result.unwrap();
                (current == target).then_some(cost)
            }),
        )
    });
    let pet_measurement = measure(|| {
        black_box(pet_dijkstra(
            pet,
            pet_nodes[0],
            Some(pet_nodes[NODES - 1]),
            |edge| *edge.weight(),
        ))
    });
    print_measurement(
        "generic-f64-dijkstra-to-target",
        "library=weavatrix-graph workspace=reused",
        &ours_measurement,
    );
    print_measurement(
        "generic-f64-dijkstra-to-target",
        "library=petgraph",
        &pet_measurement,
    );
}

fn compare_stable_mutation(pairs: &[(usize, usize)]) {
    let ours_result = ours_stable(pairs);
    let pet_result = pet_stable(pairs);
    assert_eq!(ours_result, pet_result);
    print_measurement(
        "stable-build-remove-reinsert",
        "library=weavatrix-graph",
        &measure(|| black_box(ours_stable(pairs))),
    );
    print_measurement(
        "stable-build-remove-reinsert",
        "library=petgraph-stable",
        &measure(|| black_box(pet_stable(pairs))),
    );
}

fn ours_stable(pairs: &[(usize, usize)]) -> (usize, usize) {
    let mut graph = StablePayloadGraph::with_capacity(NODES, EDGES);
    let nodes = (0..NODES)
        .map(|_| graph.add_node(()).unwrap())
        .collect::<Vec<_>>();
    for &(source, target) in pairs {
        graph.add_edge(nodes[source], nodes[target], ()).unwrap();
    }
    for index in (0..1_000).step_by(10) {
        graph.remove_node(nodes[index]);
        graph.add_node(()).unwrap();
    }
    (graph.node_count(), graph.edge_count())
}

fn pet_stable(pairs: &[(usize, usize)]) -> (usize, usize) {
    let mut graph = StableDiGraph::<(), ()>::with_capacity(NODES, EDGES);
    let nodes = (0..NODES).map(|_| graph.add_node(())).collect::<Vec<_>>();
    for &(source, target) in pairs {
        graph.add_edge(nodes[source], nodes[target], ());
    }
    for index in (0..1_000).step_by(10) {
        graph.remove_node(nodes[index]);
        graph.add_node(());
    }
    (graph.node_count(), graph.edge_count())
}

fn topology(node_count: usize, pairs: &[(usize, usize)]) -> Topology {
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
) -> (DiGraph<(), f64>, Vec<petgraph::graph::NodeIndex>) {
    let mut graph = DiGraph::with_capacity(node_count, pairs.len());
    let nodes = (0..node_count)
        .map(|_| graph.add_node(()))
        .collect::<Vec<_>>();
    for (index, &(source, target)) in pairs.iter().enumerate() {
        graph.add_edge(nodes[source], nodes[target], edge_weight_slot(index));
    }
    (graph, nodes)
}

fn edge_weight(edge: weavatrix_graph::EdgeIndex) -> f64 {
    edge_weight_slot(edge.index())
}

fn edge_weight_slot(index: usize) -> f64 {
    f64::from(u32::try_from(index % 17 + 1).unwrap()) / 3.0
}

fn node(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap())
}
