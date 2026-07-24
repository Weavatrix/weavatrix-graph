#[path = "scale_graph_competitors/helpers.rs"]
mod helpers;
mod support;

use graaf::{Bfs as GraafBfs, Size};
use helpers::{
    compact_edges, edge_weight, graaf_graph, node, pet_bfs, pet_dual_csr, pet_dual_csr_sorted,
    pet_graph, rich_parts, rich_petgraph, sorted_directions,
};
use petgraph::algo::{dijkstra as pet_dijkstra, kosaraju_scc};
use petgraph::graph::NodeIndex as PetNodeIndex;
use std::hint::black_box;
use std::time::{Duration, Instant};
use support::topology_pairs;
use weavatrix_graph::{Graph, Topology, bfs, dijkstra, strongly_connected_components};

const NODES: usize = 200_000;
const EDGES: usize = 1_000_000;

fn main() {
    let mode = std::env::args().find(|argument| {
        matches!(
            argument.as_str(),
            "topology" | "petgraph" | "petgraph-csr" | "graaf" | "rich-ours" | "rich-petgraph"
        )
    });
    if let Some(mode) = mode {
        peak_mode(&mode);
        return;
    }
    println!("nodes={NODES} edges={EDGES} statistic=median runs=5 warmups=1");
    let pairs = topology_pairs(NODES, EDGES);
    compact_builds(&pairs);
    compact_algorithms(&pairs);
    rich_builds();
}

fn compact_builds(pairs: &[(usize, usize)]) {
    let compact = compact_edges(pairs);
    {
        let ours = Topology::try_from_edges(NODES, compact.iter().copied()).unwrap();
        assert_eq!((ours.node_count(), ours.edge_count()), (NODES, EDGES));
        let pet_csr = pet_dual_csr(pairs);
        assert_eq!(
            (pet_csr.0.edge_count(), pet_csr.1.edge_count()),
            (EDGES, EDGES)
        );
        let pet = pet_graph(pairs);
        assert_eq!((pet.node_count(), pet.edge_count()), (NODES, EDGES));
        assert_eq!(graaf_graph(pairs).size(), EDGES);
    }

    report(
        "dual-csr-build",
        "weavatrix-graph",
        measure(|| Topology::try_from_edges(NODES, compact.iter().copied()).unwrap()),
    );
    let (forward, reverse) = sorted_directions(pairs);
    report(
        "dual-csr-presorted",
        "petgraph",
        measure(|| pet_dual_csr_sorted(&forward, &reverse)),
    );
    report(
        "dual-csr-build",
        "petgraph-with-preprocessing",
        measure(|| pet_dual_csr(pairs)),
    );
    report("mutable-build", "petgraph", measure(|| pet_graph(pairs)));
    report("mutable-build", "graaf", measure(|| graaf_graph(pairs)));
}

fn compact_algorithms(pairs: &[(usize, usize)]) {
    let ours = Topology::try_from_edges(NODES, compact_edges(pairs)).unwrap();
    let pet = pet_graph(pairs);
    let graaf = graaf_graph(pairs);
    let ours_bfs = bfs(&ours, node(0));
    let pet_visited = pet_bfs(&pet);
    let graaf_bfs = GraafBfs::new(&graaf, core::iter::once(0)).collect::<Vec<_>>();
    assert_eq!(ours_bfs.len(), pet_visited.len());
    assert_eq!(ours_bfs.len(), graaf_bfs.len());
    report("bfs", "weavatrix-graph", measure(|| bfs(&ours, node(0))));
    report("bfs", "petgraph", measure(|| pet_bfs(&pet)));
    report(
        "bfs",
        "graaf",
        measure(|| GraafBfs::new(&graaf, core::iter::once(0)).collect::<Vec<_>>()),
    );

    let mut ours_sizes = strongly_connected_components(&ours)
        .into_iter()
        .map(|component| component.len())
        .collect::<Vec<_>>();
    let mut pet_sizes = kosaraju_scc(&pet)
        .into_iter()
        .map(|component| component.len())
        .collect::<Vec<_>>();
    ours_sizes.sort_unstable();
    pet_sizes.sort_unstable();
    assert_eq!(ours_sizes, pet_sizes);
    report(
        "scc",
        "weavatrix-graph",
        measure(|| strongly_connected_components(&ours)),
    );
    report("scc", "petgraph", measure(|| kosaraju_scc(&pet)));

    let target = node(NODES - 1);
    let ours_path = dijkstra(&ours, node(0), target, edge_weight);
    let pet_costs = pet_dijkstra(
        &pet,
        PetNodeIndex::new(0),
        Some(PetNodeIndex::new(NODES - 1)),
        |edge| *edge.weight(),
    );
    assert_eq!(
        ours_path
            .as_ref()
            .map(weavatrix_graph::WeightedPath::total_cost),
        pet_costs.get(&PetNodeIndex::new(NODES - 1)).copied()
    );
    report(
        "dijkstra-target",
        "weavatrix-graph",
        measure(|| dijkstra(&ours, node(0), target, edge_weight)),
    );
    report(
        "dijkstra-target",
        "petgraph",
        measure(|| {
            pet_dijkstra(
                &pet,
                PetNodeIndex::new(0),
                Some(PetNodeIndex::new(NODES - 1)),
                |edge| *edge.weight(),
            )
        }),
    );
}

fn rich_builds() {
    let (nodes, edges) = rich_parts();
    {
        let ours = Graph::try_from_sorted_parts(nodes.clone(), edges.clone()).unwrap();
        assert_eq!((ours.node_count(), ours.edge_count()), (NODES, EDGES));
        let pet = rich_petgraph(nodes.clone(), edges.clone());
        assert_eq!((pet.node_count(), pet.edge_count()), (NODES, EDGES));
    }

    report(
        "rich-evidence-build",
        "weavatrix-graph",
        measure(|| Graph::try_from_sorted_parts(nodes.clone(), edges.clone()).unwrap()),
    );
    report(
        "rich-evidence-build",
        "petgraph-adapter",
        measure(|| rich_petgraph(nodes.clone(), edges.clone())),
    );
}

fn peak_mode(mode: &str) {
    match mode {
        "topology" => {
            let pairs = topology_pairs(NODES, EDGES);
            let graph = Topology::try_from_edges(NODES, compact_edges(&pairs)).unwrap();
            let checksum = graph.node_count() + graph.edge_count();
            hold(graph, checksum);
        }
        "petgraph" => {
            let pairs = topology_pairs(NODES, EDGES);
            let graph = pet_graph(&pairs);
            let checksum = graph.node_count() + graph.edge_count();
            hold(graph, checksum);
        }
        "petgraph-csr" => {
            let pairs = topology_pairs(NODES, EDGES);
            let graph = pet_dual_csr(&pairs);
            let checksum = graph.0.edge_count() + graph.1.edge_count();
            hold(graph, checksum);
        }
        "graaf" => {
            let pairs = topology_pairs(NODES, EDGES);
            let graph = graaf_graph(&pairs);
            let checksum = GraafBfs::new(&graph, core::iter::once(0)).count();
            hold(graph, checksum);
        }
        "rich-ours" => {
            let (nodes, edges) = rich_parts();
            let graph = Graph::try_from_sorted_parts(nodes, edges).unwrap();
            let checksum = graph.node_count() + graph.edge_count();
            hold(graph, checksum);
        }
        "rich-petgraph" => {
            let (nodes, edges) = rich_parts();
            let graph = rich_petgraph(nodes, edges);
            let checksum = graph.node_count() + graph.edge_count();
            hold(graph, checksum);
        }
        _ => panic!("unknown peak mode"),
    }
    black_box(());
}

fn hold<T>(value: T, checksum: usize) {
    println!("checksum={checksum}");
    black_box(&value);
    std::thread::sleep(Duration::from_millis(100));
    black_box(value);
}

fn measure<T>(mut operation: impl FnMut() -> T) -> Duration {
    black_box(operation());
    let mut samples = Vec::with_capacity(5);
    for _ in 0..5 {
        let start = Instant::now();
        black_box(operation());
        samples.push(start.elapsed());
    }
    samples.sort_unstable();
    samples[2]
}

fn report(mode: &str, library: &str, duration: Duration) {
    println!(
        "mode={mode} library={library} median_ms={:.3}",
        duration.as_secs_f64() * 1_000.0
    );
}
