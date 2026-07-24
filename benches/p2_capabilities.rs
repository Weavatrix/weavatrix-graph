#![allow(unsafe_code)]

mod support;

use petgraph::{
    Directed, Graph as PetGraph, graph::NodeIndex as PetNodeIndex, visit::GetAdjacencyMatrix,
};
use support::{measure, print_measurement, topology_pairs};
use weavatrix_graph::{
    BitMatrix, Direction, EdgeEndpoints, NodeIndex, Topology, betweenness_centrality,
    betweenness_centrality_parallel, bfs, bfs_batch_parallel, closeness_centrality,
    closeness_centrality_parallel, dijkstra, dijkstra_batch_parallel, johnson_all_pairs,
    johnson_all_pairs_parallel,
};

const NODE_COUNT: usize = 10_000;
const EDGE_COUNT: usize = 30_000;

fn main() {
    println!("statistic=median runs=11 warmups=2");
    let pairs = topology_pairs(NODE_COUNT, EDGE_COUNT);
    compare_bit_matrix(&pairs);
    compare_parallel_batches(&pairs);
    compare_parallel_apsp();
    compare_parallel_centrality();
}

fn compare_bit_matrix(pairs: &[(usize, usize)]) {
    let mut ours = BitMatrix::try_new(NODE_COUNT).unwrap();
    let mut pet = PetGraph::<(), (), Directed>::with_capacity(NODE_COUNT, EDGE_COUNT);
    let pet_nodes = (0..NODE_COUNT)
        .map(|_| pet.add_node(()))
        .collect::<Vec<_>>();
    for &(source, target) in pairs {
        ours.insert(node(source), node(target)).unwrap();
        pet.add_edge(pet_nodes[source], pet_nodes[target], ());
    }
    let pet_matrix = pet.adjacency_matrix();
    let queries = (0..1_000_000)
        .map(|index| {
            let source = (index * 37 + 11) % NODE_COUNT;
            let target = (index * 7_919 + 17) % NODE_COUNT;
            (
                node(source),
                node(target),
                PetNodeIndex::new(source),
                PetNodeIndex::new(target),
            )
        })
        .collect::<Vec<_>>();
    let ours_query = measure(|| {
        queries
            .iter()
            .filter(|&&(source, target, _, _)| ours.contains(source, target))
            .count()
    });
    let ours_fast_query = measure(|| {
        queries
            .iter()
            .filter(|&&(source, target, _, _)| ours.contains_fast(source, target))
            .count()
    });
    let ours_unchecked_query = measure(|| {
        queries
            .iter()
            .filter(|&&(source, target, _, _)| {
                // SAFETY: Every generated query is reduced modulo NODE_COUNT.
                unsafe { ours.contains_unchecked(source, target) }
            })
            .count()
    });
    let pet_query = measure(|| {
        queries
            .iter()
            .filter(|&&(_, _, source, target)| pet.is_adjacent(&pet_matrix, source, target))
            .count()
    });
    print_measurement(
        "bit-matrix-1m-lookups",
        &format!("library=weavatrix-graph bytes={}", ours.storage_bytes()),
        &ours_query,
    );
    print_measurement(
        "bit-matrix-1m-lookups",
        "library=weavatrix-graph mode=checked-unsafe-fast",
        &ours_fast_query,
    );
    print_measurement(
        "bit-matrix-1m-lookups",
        "library=weavatrix-graph mode=caller-validated-unchecked",
        &ours_unchecked_query,
    );
    print_measurement(
        "bit-matrix-1m-lookups",
        "library=petgraph-fixedbitset",
        &pet_query,
    );
}

fn compare_parallel_batches(pairs: &[(usize, usize)]) {
    let graph = Topology::try_from_edges(
        NODE_COUNT,
        pairs
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node(source), node(target))),
    )
    .unwrap();
    let starts = (0..128).map(|index| node(index * 73)).collect::<Vec<_>>();
    let queries = (0..64)
        .map(|index| (node(index * 131), node((index * 977 + 313) % NODE_COUNT)))
        .collect::<Vec<_>>();
    let sequential_bfs = measure(|| {
        starts
            .iter()
            .map(|&start| bfs(&graph, start))
            .collect::<Vec<_>>()
    });
    let parallel_bfs = measure(|| bfs_batch_parallel(&graph, &starts));
    let weight = |edge: weavatrix_graph::EdgeIndex| 1 + edge.index() as u64 % 19;
    let sequential_dijkstra = measure(|| {
        queries
            .iter()
            .map(|&(source, target)| dijkstra(&graph, source, target, weight))
            .collect::<Vec<_>>()
    });
    let parallel_dijkstra = measure(|| dijkstra_batch_parallel(&graph, &queries, weight));
    print_measurement("bfs-batch-128", "mode=sequential", &sequential_bfs);
    print_measurement("bfs-batch-128", "mode=rayon", &parallel_bfs);
    print_measurement("dijkstra-batch-64", "mode=sequential", &sequential_dijkstra);
    print_measurement("dijkstra-batch-64", "mode=rayon", &parallel_dijkstra);
}

fn compare_parallel_apsp() {
    let pairs = topology_pairs(1_200, 6_000);
    let graph = topology(1_200, &pairs);
    let weight = |edge: weavatrix_graph::EdgeIndex| 1 + i64::try_from(edge.index()).unwrap() % 19;
    let sequential = measure(|| johnson_all_pairs(&graph, weight).unwrap());
    let parallel = measure(|| johnson_all_pairs_parallel(&graph, weight).unwrap());
    print_measurement("johnson-apsp-1200", "mode=sequential", &sequential);
    print_measurement("johnson-apsp-1200", "mode=rayon", &parallel);
}

fn compare_parallel_centrality() {
    let pairs = topology_pairs(1_000, 4_000);
    let graph = topology(1_000, &pairs);
    let sequential_closeness = measure(|| closeness_centrality(&graph, Direction::Outgoing));
    let parallel_closeness = measure(|| closeness_centrality_parallel(&graph, Direction::Outgoing));
    let sequential_betweenness =
        measure(|| betweenness_centrality(&graph, Direction::Outgoing, true));
    let parallel_betweenness =
        measure(|| betweenness_centrality_parallel(&graph, Direction::Outgoing, true));
    print_measurement(
        "closeness-centrality-1000",
        "mode=sequential",
        &sequential_closeness,
    );
    print_measurement(
        "closeness-centrality-1000",
        "mode=rayon",
        &parallel_closeness,
    );
    print_measurement(
        "betweenness-centrality-1000",
        "mode=sequential",
        &sequential_betweenness,
    );
    print_measurement(
        "betweenness-centrality-1000",
        "mode=rayon",
        &parallel_betweenness,
    );
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

fn node(index: usize) -> NodeIndex {
    NodeIndex::new(index.try_into().unwrap())
}
