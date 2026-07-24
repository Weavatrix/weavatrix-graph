#![allow(clippy::cast_precision_loss)]

use core::cell::Cell;
use petgraph::{Directed, Undirected, graph::Graph};
use rustworkx_core::centrality::edge_betweenness_centrality as rustworkx_edges;
use weavatrix_graph::{
    Direction, EdgeEndpoints, EdgeIndex, NodeIndex, Topology, UndirectedTopology,
    edge_betweenness_centrality, edge_betweenness_centrality_filtered,
    undirected_edge_betweenness_centrality,
};
#[cfg(feature = "rayon")]
use weavatrix_graph::{
    edge_betweenness_centrality_parallel, undirected_edge_betweenness_centrality_parallel,
};

fn directed(node_count: usize, pairs: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges(
        node_count,
        pairs
            .iter()
            .map(|&(source, target)| endpoints(source, target)),
    )
    .unwrap()
}

fn undirected(node_count: usize, pairs: &[(u32, u32)]) -> UndirectedTopology {
    UndirectedTopology::try_from_edges(
        node_count,
        pairs
            .iter()
            .map(|&(source, target)| endpoints(source, target)),
    )
    .unwrap()
}

fn endpoints(source: u32, target: u32) -> EdgeEndpoints {
    EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
}

fn values(scores: &[(EdgeIndex, f64)]) -> Vec<f64> {
    scores.iter().map(|pair| pair.1).collect()
}

fn close(left: &[f64], right: &[f64]) {
    assert_eq!(left.len(), right.len());
    for (left, right) in left.iter().zip(right) {
        assert!((left - right).abs() < 1e-10, "{left} != {right}");
    }
}

#[test]
fn directed_path_has_expected_raw_and_normalized_scores() {
    let graph = directed(3, &[(0, 1), (1, 2)]);
    close(
        &values(&edge_betweenness_centrality(
            &graph,
            Direction::Outgoing,
            false,
        )),
        &[2.0, 2.0],
    );
    close(
        &values(&edge_betweenness_centrality(
            &graph,
            Direction::Outgoing,
            true,
        )),
        &[1.0 / 3.0, 1.0 / 3.0],
    );
}

#[test]
fn undirected_path_normalization_matches_brandes_contract() {
    let graph = undirected(3, &[(0, 1), (1, 2)]);
    close(
        &values(&undirected_edge_betweenness_centrality(&graph, false)),
        &[2.0, 2.0],
    );
    close(
        &values(&undirected_edge_betweenness_centrality(&graph, true)),
        &[2.0 / 3.0, 2.0 / 3.0],
    );
}

#[test]
fn parallel_edges_split_paths_and_self_loops_score_zero() {
    let graph = directed(3, &[(0, 1), (0, 1), (1, 2), (1, 1)]);
    close(
        &values(&edge_betweenness_centrality(
            &graph,
            Direction::Outgoing,
            false,
        )),
        &[1.0, 1.0, 2.0, 0.0],
    );
}

#[test]
fn filter_is_evaluated_once_and_only_returns_accepted_edges() {
    let graph = directed(3, &[(0, 1), (1, 2), (0, 2)]);
    let calls = Cell::new(0);
    let scores = edge_betweenness_centrality_filtered(&graph, Direction::Outgoing, false, |edge| {
        calls.set(calls.get() + 1);
        edge != EdgeIndex::new(2)
    });
    assert_eq!(calls.get(), 3);
    assert_eq!(
        scores.iter().map(|pair| pair.0).collect::<Vec<_>>(),
        vec![EdgeIndex::new(0), EdgeIndex::new(1)]
    );
    close(&values(&scores), &[2.0, 2.0]);
}

#[test]
fn directed_results_match_rustworkx_on_seeded_multigraphs() {
    for seed in 0..12_u32 {
        let pairs = (0..24)
            .map(|index| {
                let source = (index * 7 + seed * 3) % 9;
                let target = (index * 11 + seed + 1) % 9;
                (source, target)
            })
            .collect::<Vec<_>>();
        let ours = directed(9, &pairs);
        let mut reference = Graph::<(), (), Directed>::new();
        let nodes = (0..9).map(|_| reference.add_node(())).collect::<Vec<_>>();
        for &(source, target) in &pairs {
            reference.add_edge(nodes[source as usize], nodes[target as usize], ());
        }
        let expected = rustworkx_edges(&reference, false, usize::MAX)
            .into_iter()
            .map(Option::unwrap)
            .collect::<Vec<_>>();
        close(
            &values(&edge_betweenness_centrality(
                &ours,
                Direction::Outgoing,
                false,
            )),
            &expected,
        );
    }
}

#[test]
fn undirected_results_match_rustworkx() {
    let pairs = [(0, 1), (0, 1), (1, 2), (2, 3), (3, 0), (1, 3), (2, 2)];
    let ours = undirected(4, &pairs);
    let mut reference = Graph::<(), (), Undirected>::new_undirected();
    let nodes = (0..4).map(|_| reference.add_node(())).collect::<Vec<_>>();
    for &(source, target) in &pairs {
        reference.add_edge(nodes[source as usize], nodes[target as usize], ());
    }
    let expected = rustworkx_edges(&reference, true, usize::MAX)
        .into_iter()
        .map(Option::unwrap)
        .collect::<Vec<_>>();
    close(
        &values(&undirected_edge_betweenness_centrality(&ours, true)),
        &expected,
    );
}

#[cfg(feature = "rayon")]
#[test]
fn parallel_variants_match_sequential_results() {
    let directed_graph = directed(5, &[(0, 1), (0, 2), (1, 3), (2, 3), (3, 4)]);
    let sequential = edge_betweenness_centrality(&directed_graph, Direction::Outgoing, true);
    let parallel = edge_betweenness_centrality_parallel(&directed_graph, Direction::Outgoing, true);
    close(&values(&parallel), &values(&sequential));

    let undirected_graph = undirected(5, &[(0, 1), (0, 2), (1, 3), (2, 3), (3, 4)]);
    let sequential = undirected_edge_betweenness_centrality(&undirected_graph, true);
    let parallel = undirected_edge_betweenness_centrality_parallel(&undirected_graph, true);
    close(&values(&parallel), &values(&sequential));
}
