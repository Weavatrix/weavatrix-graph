#![cfg(feature = "rayon")]

use weavatrix_graph::{
    Direction, EdgeEndpoints, GraphView, NodeIndex, Topology, betweenness_centrality,
    betweenness_centrality_parallel, bfs, bfs_batch_parallel, closeness_centrality,
    closeness_centrality_parallel, dijkstra, dijkstra_batch_parallel, johnson_all_pairs,
    johnson_all_pairs_parallel,
};

fn fixture() -> Topology {
    Topology::try_from_edges(
        5,
        [
            EdgeEndpoints::new(NodeIndex::new(0), NodeIndex::new(1)),
            EdgeEndpoints::new(NodeIndex::new(1), NodeIndex::new(2)),
            EdgeEndpoints::new(NodeIndex::new(0), NodeIndex::new(3)),
            EdgeEndpoints::new(NodeIndex::new(3), NodeIndex::new(4)),
            EdgeEndpoints::new(NodeIndex::new(4), NodeIndex::new(2)),
        ],
    )
    .unwrap()
}

#[test]
fn parallel_bfs_matches_sequential_and_preserves_order() {
    let graph = fixture();
    let starts = [NodeIndex::new(3), NodeIndex::new(0), NodeIndex::new(2)];
    let expected = starts.map(|start| bfs(&graph, start)).to_vec();
    assert_eq!(bfs_batch_parallel(&graph, &starts), expected);
}

#[test]
fn parallel_dijkstra_matches_sequential_and_preserves_order() {
    let graph = fixture();
    let queries = [
        (NodeIndex::new(0), NodeIndex::new(2)),
        (NodeIndex::new(3), NodeIndex::new(2)),
        (NodeIndex::new(2), NodeIndex::new(0)),
    ];
    let expected = queries
        .map(|(source, target)| dijkstra(&graph, source, target, |_| 1))
        .to_vec();
    assert_eq!(dijkstra_batch_parallel(&graph, &queries, |_| 1), expected);
}

#[test]
fn parallel_apsp_and_centrality_match_sequential_results() {
    let graph = fixture();
    let sequential = johnson_all_pairs(&graph, |_| 1).unwrap();
    let parallel = johnson_all_pairs_parallel(&graph, |_| 1).unwrap();
    for source in graph.node_indices() {
        for target in graph.node_indices() {
            assert_eq!(
                parallel.distance(source, target),
                sequential.distance(source, target)
            );
        }
    }

    let sequential = closeness_centrality(&graph, Direction::Both);
    let parallel = closeness_centrality_parallel(&graph, Direction::Both);
    assert_eq!(parallel, sequential);

    let sequential = betweenness_centrality(&graph, Direction::Both, true);
    let parallel = betweenness_centrality_parallel(&graph, Direction::Both, true);
    for ((left_node, left), (right_node, right)) in sequential.into_iter().zip(parallel) {
        assert_eq!(left_node, right_node);
        assert!((left - right).abs() < f64::EPSILON);
    }
}
