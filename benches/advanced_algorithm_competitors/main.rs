mod dag_intelligence;
#[path = "../support/mod.rs"]
mod support;

use petgraph::algo::{
    astar as pet_astar, bellman_ford as pet_bellman,
    dominators::simple_fast,
    page_rank as pet_page_rank,
    tred::{dag_to_toposorted_adjacency_list, dag_transitive_reduction_closure as pet_transitive},
};
use petgraph::graph::DiGraph;
use std::collections::BTreeSet;
use std::hint::black_box;
use support::{measure, print_measurement, topology_pairs};
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, Topology, astar, bellman_ford, dag_transitive_reduction_closure,
    dominators, page_rank,
};

fn main() {
    println!("statistic=median runs=11 warmups=2");
    compare_astar();
    compare_bellman_ford();
    compare_page_rank();
    compare_dominators();
    dag_intelligence::compare();
    compare_transitive();
}

fn compare_astar() {
    const NODES: usize = 10_000;
    let pairs = topology_pairs(NODES, 30_000);
    let weights = pairs
        .iter()
        .copied()
        .map(positive_weight)
        .collect::<Vec<_>>();
    let ours = ours_graph(NODES, &pairs);
    let (pet, nodes) = weighted_pet(NODES, &pairs, positive_weight);
    let target = node(NODES - 1);
    let ours_measurement = measure(|| {
        black_box(astar(
            &ours,
            node(0),
            target,
            |edge| weights[edge.index()],
            |_| 0,
        ))
    });
    let pet_measurement = measure(|| {
        black_box(pet_astar(
            &pet,
            nodes[0],
            |candidate| candidate == nodes[NODES - 1],
            |edge| *edge.weight(),
            |_| 0,
        ))
    });
    print_measurement(
        "astar-zero-heuristic",
        "library=weavatrix-graph",
        &ours_measurement,
    );
    print_measurement("astar-zero-heuristic", "library=petgraph", &pet_measurement);
}

fn compare_bellman_ford() {
    const NODES: usize = 1_000;
    let pairs = dag_pairs(NODES, 5_000);
    let weights = pairs.iter().copied().map(signed_weight).collect::<Vec<_>>();
    let ours = ours_graph(NODES, &pairs);
    let (pet, nodes) = weighted_pet(NODES, &pairs, |pair| {
        f64::from(i32::try_from(signed_weight(pair)).unwrap())
    });
    let ours_measurement =
        measure(|| black_box(bellman_ford(&ours, node(0), |edge| weights[edge.index()])));
    let pet_measurement = measure(|| black_box(pet_bellman(&pet, nodes[0])));
    print_measurement("bellman-ford", "library=weavatrix-graph", &ours_measurement);
    print_measurement("bellman-ford", "library=petgraph", &pet_measurement);
}

fn compare_page_rank() {
    const NODES: usize = 500;
    let pairs = unique_pairs(NODES, 2_000);
    let ours = ours_graph(NODES, &pairs);
    let (pet, _) = weighted_pet(NODES, &pairs, |_| ());
    let ours_measurement = measure(|| black_box(page_rank(&ours, 0.85, 20).unwrap()));
    let pet_measurement = measure(|| black_box(pet_page_rank(&pet, 0.85_f64, 20)));
    print_measurement("page-rank-20", "library=weavatrix-graph", &ours_measurement);
    print_measurement("page-rank-20", "library=petgraph", &pet_measurement);
}

fn compare_dominators() {
    const NODES: usize = 10_000;
    let pairs = topology_pairs(NODES, 30_000);
    let ours = ours_graph(NODES, &pairs);
    let (pet, nodes) = weighted_pet(NODES, &pairs, |_| ());
    let ours_measurement = measure(|| black_box(dominators(&ours, node(0))));
    let pet_measurement = measure(|| black_box(simple_fast(&pet, nodes[0])));
    print_measurement("dominators", "library=weavatrix-graph", &ours_measurement);
    print_measurement("dominators", "library=petgraph", &pet_measurement);
}

fn compare_transitive() {
    const NODES: usize = 512;
    let pairs = dag_pairs(NODES, 3_000);
    let ours = ours_graph(NODES, &pairs);
    let (pet, nodes) = weighted_pet(NODES, &pairs, |_| ());
    let ours_measurement = measure(|| black_box(dag_transitive_reduction_closure(&ours).unwrap()));
    let pet_measurement = measure(|| {
        let (ordered, _) = dag_to_toposorted_adjacency_list::<_, u32>(&pet, &nodes);
        black_box(pet_transitive(&ordered))
    });
    print_measurement(
        "dag-reduction-closure",
        "library=weavatrix-graph",
        &ours_measurement,
    );
    print_measurement(
        "dag-reduction-closure",
        "library=petgraph preprocessing=included",
        &pet_measurement,
    );
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

fn weighted_pet<Weight>(
    node_count: usize,
    pairs: &[(usize, usize)],
    weight: impl Fn((usize, usize)) -> Weight,
) -> (DiGraph<(), Weight>, Vec<petgraph::graph::NodeIndex>) {
    let mut graph = DiGraph::with_capacity(node_count, pairs.len());
    let nodes = (0..node_count)
        .map(|_| graph.add_node(()))
        .collect::<Vec<_>>();
    for &pair @ (source, target) in pairs {
        graph.add_edge(nodes[source], nodes[target], weight(pair));
    }
    (graph, nodes)
}

fn unique_pairs(node_count: usize, edge_count: usize) -> Vec<(usize, usize)> {
    topology_pairs(node_count, edge_count * 2)
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(edge_count)
        .collect()
}

fn dag_pairs(node_count: usize, edge_count: usize) -> Vec<(usize, usize)> {
    let mut pairs = BTreeSet::new();
    for source in 0..node_count.saturating_sub(1) {
        pairs.insert((source, source + 1));
    }
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    while pairs.len() < edge_count {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let source = usize::try_from(seed % (node_count - 1) as u64).unwrap();
        let span = node_count - source - 1;
        let target = source + 1 + usize::try_from((seed >> 32) % span as u64).unwrap();
        pairs.insert((source, target));
    }
    pairs.into_iter().collect()
}

fn positive_weight((source, target): (usize, usize)) -> u64 {
    u64::try_from((source * 31 + target * 17) % 97 + 1).unwrap()
}

fn signed_weight((source, target): (usize, usize)) -> i64 {
    i64::try_from((source * 31 + target * 17) % 41).unwrap() - 10
}

fn node(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap())
}
