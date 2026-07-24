mod support;

use petgraph::{Undirected, graph::Graph};
use rustworkx_core::{
    centrality::edge_betweenness_centrality as rustworkx_edge_betweenness,
    connectivity::stoer_wagner_min_cut as rustworkx_min_cut,
};
use std::hint::black_box;
use support::{measure, print_measurement, undirected_pairs};
#[cfg(feature = "rayon")]
use weavatrix_graph::undirected_edge_betweenness_centrality_parallel;
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, UndirectedTopology, stoer_wagner_min_cut,
    undirected_edge_betweenness_centrality,
};

const CENTRALITY_NODES: usize = 1_200;
const CENTRALITY_EDGES: usize = 4_800;
const MIN_CUT_NODES: usize = 350;
const MIN_CUT_EDGES: usize = 1_400;

fn main() {
    println!("statistic=median runs=11 warmups=2");
    compare_edge_betweenness();
    #[cfg(feature = "rayon")]
    compare_parallel_edge_betweenness();
    compare_min_cut();
}

fn compare_edge_betweenness() {
    let pairs = undirected_pairs(CENTRALITY_NODES, CENTRALITY_EDGES);
    let (ours, reference) = graphs(CENTRALITY_NODES, &pairs);
    let ours_result = undirected_edge_betweenness_centrality(&ours, true);
    let reference_result = rustworkx_edge_betweenness(&reference, true, usize::MAX);
    assert_scores_match(&ours_result, &reference_result);
    print_measurement(
        "edge-betweenness",
        "library=weavatrix-graph output=all-edge-scores normalized=true",
        &measure(|| {
            black_box(undirected_edge_betweenness_centrality(
                black_box(&ours),
                true,
            ))
        }),
    );
    print_measurement(
        "edge-betweenness",
        "library=rustworkx-core output=all-edge-scores normalized=true serial",
        &measure(|| {
            black_box(rustworkx_edge_betweenness(
                black_box(&reference),
                true,
                usize::MAX,
            ))
        }),
    );
}

#[cfg(feature = "rayon")]
fn compare_parallel_edge_betweenness() {
    let pairs = undirected_pairs(CENTRALITY_NODES, CENTRALITY_EDGES);
    let (ours, reference) = graphs(CENTRALITY_NODES, &pairs);
    let ours_result = undirected_edge_betweenness_centrality_parallel(&ours, true);
    let reference_result = rustworkx_edge_betweenness(&reference, true, 0);
    assert_scores_match(&ours_result, &reference_result);
    print_measurement(
        "edge-betweenness-parallel",
        "library=weavatrix-graph output=all-edge-scores normalized=true",
        &measure(|| {
            black_box(undirected_edge_betweenness_centrality_parallel(
                black_box(&ours),
                true,
            ))
        }),
    );
    print_measurement(
        "edge-betweenness-parallel",
        "library=rustworkx-core output=all-edge-scores normalized=true",
        &measure(|| black_box(rustworkx_edge_betweenness(black_box(&reference), true, 0))),
    );
}

fn compare_min_cut() {
    let pairs = undirected_pairs(MIN_CUT_NODES, MIN_CUT_EDGES);
    let (ours, reference) = weighted_graphs(MIN_CUT_NODES, &pairs);
    let ours_result = stoer_wagner_min_cut(&ours, |edge| weight(edge.index()))
        .unwrap()
        .unwrap();
    let reference_result = rustworkx_min_cut(&reference, |edge| Ok::<u64, ()>(*edge.weight()))
        .unwrap()
        .unwrap();
    assert_eq!(ours_result.weight(), reference_result.0);
    print_measurement(
        "stoer-wagner-min-cut",
        "library=weavatrix-graph output=weight+canonical-bipartition checked=true",
        &measure(|| {
            black_box(stoer_wagner_min_cut(black_box(&ours), |edge| weight(edge.index())).unwrap())
        }),
    );
    print_measurement(
        "stoer-wagner-min-cut",
        "library=rustworkx-core output=weight+one-partition checked=false",
        &measure(|| {
            black_box(
                rustworkx_min_cut(black_box(&reference), |edge| Ok::<u64, ()>(*edge.weight()))
                    .unwrap(),
            )
        }),
    );
}

fn graphs(
    node_count: usize,
    pairs: &[(usize, usize)],
) -> (UndirectedTopology, Graph<(), (), Undirected>) {
    let ours = topology(node_count, pairs);
    let mut reference = Graph::new_undirected();
    let nodes = (0..node_count)
        .map(|_| reference.add_node(()))
        .collect::<Vec<_>>();
    for &(source, target) in pairs {
        reference.add_edge(nodes[source], nodes[target], ());
    }
    (ours, reference)
}

fn weighted_graphs(
    node_count: usize,
    pairs: &[(usize, usize)],
) -> (UndirectedTopology, Graph<(), u64, Undirected>) {
    let ours = topology(node_count, pairs);
    let mut reference = Graph::new_undirected();
    let nodes = (0..node_count)
        .map(|_| reference.add_node(()))
        .collect::<Vec<_>>();
    for (index, &(source, target)) in pairs.iter().enumerate() {
        reference.add_edge(nodes[source], nodes[target], weight(index));
    }
    (ours, reference)
}

fn topology(node_count: usize, pairs: &[(usize, usize)]) -> UndirectedTopology {
    UndirectedTopology::try_from_edges(
        node_count,
        pairs
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node(source), node(target))),
    )
    .unwrap()
}

fn node(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap())
}

fn weight(index: usize) -> u64 {
    (index as u64 * 17 + 3) % 29 + 1
}

fn assert_scores_match(ours: &[(weavatrix_graph::EdgeIndex, f64)], reference: &[Option<f64>]) {
    assert_eq!(ours.len(), reference.len());
    for ((_, ours), reference) in ours.iter().zip(reference) {
        assert!((ours - reference.unwrap()).abs() < 1e-8);
    }
}
