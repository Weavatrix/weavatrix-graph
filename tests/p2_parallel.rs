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
fn parallel_topology_build_matches_sequential_and_preserves_edge_indexes() {
    let edges = [
        EdgeEndpoints::new(NodeIndex::new(2), NodeIndex::new(0)),
        EdgeEndpoints::new(NodeIndex::new(0), NodeIndex::new(1)),
        EdgeEndpoints::new(NodeIndex::new(2), NodeIndex::new(1)),
    ];
    let sequential = Topology::try_from_edges(3, edges).unwrap();
    let parallel = Topology::try_from_edges_parallel(3, edges).unwrap();
    assert_eq!(parallel, sequential);

    let unordered = Topology::try_from_edges_parallel_unordered(3, edges).unwrap();
    assert_eq!(
        unordered.edge_references().collect::<Vec<_>>(),
        sequential.edge_references().collect::<Vec<_>>()
    );
    for node in sequential.node_indices() {
        let mut expected = sequential.outgoing_edges(node).collect::<Vec<_>>();
        let mut actual = unordered.outgoing_edges(node).collect::<Vec<_>>();
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(actual, expected);
    }
}

#[cfg(feature = "unsafe-fast")]
#[test]
fn unsafe_fast_parallel_builds_match_safe_semantics() {
    for node_count in [1, 2, 7, 31, 64] {
        let edges = generated_edges(node_count, node_count * 9 + 3);
        let sequential = Topology::try_from_edges(node_count, edges.iter().copied()).unwrap();
        let stable =
            Topology::try_from_edges_parallel_fast(node_count, edges.iter().copied()).unwrap();
        assert_eq!(stable, sequential);
        let unordered =
            Topology::try_from_edges_parallel_unordered_fast(node_count, edges).unwrap();
        assert_same_adjacencies(&sequential, &unordered);
    }
}

#[test]
fn safe_parallel_builds_match_across_sizes_and_reject_invalid_endpoints() {
    for node_count in [1, 2, 7, 31, 64] {
        let edges = generated_edges(node_count, node_count * 9 + 3);
        let sequential = Topology::try_from_edges(node_count, edges.iter().copied()).unwrap();
        let stable = Topology::try_from_edges_parallel(node_count, edges.iter().copied()).unwrap();
        assert_eq!(stable, sequential);
        let unordered = Topology::try_from_edges_parallel_unordered(node_count, edges).unwrap();
        assert_same_adjacencies(&sequential, &unordered);
    }
    let invalid = [EdgeEndpoints::new(NodeIndex::new(0), NodeIndex::new(2))];
    assert!(Topology::try_from_edges_parallel(2, invalid).is_err());
    assert!(Topology::try_from_edges_parallel_unordered(2, invalid).is_err());

    let small = generated_edges(31, 500);
    assert_eq!(
        Topology::try_from_edges_auto(31, small.iter().copied()).unwrap(),
        Topology::try_from_edges(31, small).unwrap()
    );
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

fn generated_edges(node_count: usize, edge_count: usize) -> Vec<EdgeEndpoints> {
    (0..edge_count)
        .map(|edge| {
            let source = edge % node_count;
            let target = (source * 17 + edge / node_count * 7 + 1) % node_count;
            EdgeEndpoints::new(
                NodeIndex::new(u32::try_from(source).unwrap()),
                NodeIndex::new(u32::try_from(target).unwrap()),
            )
        })
        .collect()
}

fn assert_same_adjacencies(expected: &Topology, actual: &Topology) {
    assert_eq!(
        actual.edge_references().collect::<Vec<_>>(),
        expected.edge_references().collect::<Vec<_>>()
    );
    for node in expected.node_indices() {
        let mut expected_out = expected.outgoing_edges(node).collect::<Vec<_>>();
        let mut actual_out = actual.outgoing_edges(node).collect::<Vec<_>>();
        expected_out.sort_unstable();
        actual_out.sort_unstable();
        assert_eq!(actual_out, expected_out);

        let mut expected_in = expected.incoming_edges(node).collect::<Vec<_>>();
        let mut actual_in = actual.incoming_edges(node).collect::<Vec<_>>();
        expected_in.sort_unstable();
        actual_in.sort_unstable();
        assert_eq!(actual_in, expected_in);
    }
}
