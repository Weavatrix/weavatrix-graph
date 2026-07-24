use weavatrix_graph::{
    EdgeEndpoints, GraphError, NodeIndex, Topology, bellman_ford, bidirectional_dijkstra, dijkstra,
    spfa,
};

fn topology(edges: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges(
        6,
        edges.iter().map(|&(source, target)| {
            EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
        }),
    )
    .unwrap()
}

#[test]
fn bidirectional_dijkstra_matches_single_frontier_dijkstra() {
    let graph = topology(&[(0, 1), (0, 2), (1, 3), (2, 3), (3, 4), (1, 5), (5, 4)]);
    let weights = [2_u64, 8, 2, 1, 3, 1, 9];
    let source = NodeIndex::new(0);
    let target = NodeIndex::new(4);
    let expected = dijkstra(&graph, source, target, |edge| weights[edge.index()]).unwrap();
    let actual =
        bidirectional_dijkstra(&graph, source, target, |edge| weights[edge.index()]).unwrap();
    assert_eq!(actual, expected);
    assert!(bidirectional_dijkstra(&graph, NodeIndex::new(4), NodeIndex::new(0), |_| 1).is_none());
}

#[test]
fn spfa_matches_bellman_ford_and_rejects_reachable_negative_cycles() {
    let graph = topology(&[(0, 1), (0, 2), (1, 2), (1, 3), (2, 3), (3, 4)]);
    let weights = [3_i64, 8, -2, 5, 1, 2];
    let source = NodeIndex::new(0);
    let expected = bellman_ford(&graph, source, |edge| weights[edge.index()])
        .unwrap()
        .unwrap();
    let actual = spfa(&graph, source, |edge| weights[edge.index()])
        .unwrap()
        .unwrap();
    for target in 0..6 {
        let target = NodeIndex::new(target);
        assert_eq!(actual.distance_to(target), expected.distance_to(target));
    }

    let cycle = topology(&[(0, 1), (1, 2), (2, 1)]);
    assert!(matches!(
        spfa(&cycle, NodeIndex::new(0), |edge| [-1_i64, -1, 0]
            [edge.index()]),
        Err(GraphError::NegativeCycle { .. })
    ));
}
