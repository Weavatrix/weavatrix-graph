use weavatrix_graph::{
    EdgeEndpoints, GraphError, NodeIndex, SubgraphMode, Topology, UndirectedTopology,
    all_simple_paths, bridges_and_articulation_points, floyd_warshall, graph_isomorphic,
    johnson_all_pairs, johnson_cycles, k_shortest_paths, subgraph_isomorphisms,
};

fn directed(node_count: usize, edges: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges(
        node_count,
        edges
            .iter()
            .map(|&(source, target)| endpoints(source, target)),
    )
    .unwrap()
}

const fn endpoints(source: u32, target: u32) -> EdgeEndpoints {
    EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
}

#[test]
fn floyd_warshall_and_johnson_agree_with_negative_edges() {
    let graph = directed(5, &[(0, 1), (0, 2), (1, 2), (1, 3), (2, 3), (3, 4)]);
    let weights = [3_i64, 8, -2, 5, 1, 2];
    let floyd = floyd_warshall(&graph, |edge| weights[edge.index()]).unwrap();
    let johnson = johnson_all_pairs(&graph, |edge| weights[edge.index()]).unwrap();

    for source in 0..5 {
        for target in 0..5 {
            let source = NodeIndex::new(source);
            let target = NodeIndex::new(target);
            assert_eq!(
                floyd.distance(source, target),
                johnson.distance(source, target)
            );
        }
    }
    assert_eq!(
        floyd
            .path(NodeIndex::new(0), NodeIndex::new(4))
            .unwrap()
            .nodes(),
        [0, 1, 2, 3, 4].map(NodeIndex::new)
    );
    assert_eq!(
        johnson.distance(NodeIndex::new(0), NodeIndex::new(4)),
        Some(4)
    );
    assert_eq!(johnson.distance(NodeIndex::new(4), NodeIndex::new(0)), None);
}

#[test]
fn all_pairs_algorithms_reject_negative_cycles() {
    let graph = directed(3, &[(0, 1), (1, 2), (2, 0)]);
    let weights = [-2_i64, 0, 1];
    assert!(matches!(
        floyd_warshall(&graph, |edge| weights[edge.index()]),
        Err(GraphError::NegativeCycle { .. })
    ));
    assert!(matches!(
        johnson_all_pairs(&graph, |edge| weights[edge.index()]),
        Err(GraphError::NegativeCycle { .. })
    ));
}

#[test]
fn simple_and_k_shortest_paths_are_bounded_and_ordered() {
    let graph = directed(4, &[(0, 1), (0, 2), (1, 2), (1, 3), (2, 3)]);
    let paths = all_simple_paths(&graph, NodeIndex::new(0), NodeIndex::new(3), 4, 10);
    assert_eq!(paths.paths().len(), 3);
    assert!(!paths.truncated());

    let weights = [1_u64, 4, 1, 5, 1];
    let shortest = k_shortest_paths(&graph, NodeIndex::new(0), NodeIndex::new(3), 3, |edge| {
        weights[edge.index()]
    });
    assert_eq!(
        shortest
            .iter()
            .map(weavatrix_graph::WeightedPath::total_cost)
            .collect::<Vec<_>>(),
        [3, 5, 6]
    );

    let limited = all_simple_paths(&graph, NodeIndex::new(0), NodeIndex::new(3), 4, 1);
    assert_eq!(limited.paths().len(), 1);
    assert!(limited.truncated());
}

#[test]
fn circuit_enumeration_is_canonical_and_bounded() {
    let graph = directed(4, &[(0, 1), (1, 2), (2, 0), (1, 3), (3, 1)]);
    let cycles = johnson_cycles(&graph, 10);
    assert_eq!(
        cycles.paths(),
        &[
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1),
                NodeIndex::new(2),
                NodeIndex::new(0),
            ],
            vec![NodeIndex::new(1), NodeIndex::new(3), NodeIndex::new(1)],
        ]
    );
    assert!(!cycles.truncated());

    let limited = johnson_cycles(&graph, 1);
    assert_eq!(limited.paths().len(), 1);
    assert!(limited.truncated());
}

#[test]
fn tarjan_finds_bridges_and_articulation_points() {
    let graph = UndirectedTopology::try_from_edges(
        4,
        [
            endpoints(0, 1),
            endpoints(1, 2),
            endpoints(2, 0),
            endpoints(1, 3),
        ],
    )
    .unwrap();
    let cuts = bridges_and_articulation_points(&graph);
    assert_eq!(
        cuts.bridges()
            .iter()
            .map(|edge| edge.index())
            .collect::<Vec<_>>(),
        [3]
    );
    assert_eq!(cuts.articulation_points(), &[NodeIndex::new(1)]);
}

#[test]
fn graph_and_subgraph_isomorphism_respect_direction_and_extra_edges() {
    let left = directed(3, &[(0, 1), (1, 2), (2, 0)]);
    let right = directed(3, &[(2, 0), (0, 1), (1, 2)]);
    assert!(graph_isomorphic(&left, &right, |_, _| true, |_, _| true));

    let pattern = directed(3, &[(0, 1), (1, 2)]);
    let target = directed(5, &[(0, 1), (1, 2), (2, 3), (4, 0)]);
    let matches = subgraph_isomorphisms(
        &pattern,
        &target,
        SubgraphMode::NonInduced,
        20,
        |_, _| true,
        |_, _| true,
    );
    assert!(!matches.mappings().is_empty());

    let reversed = directed(3, &[(1, 0), (2, 1)]);
    assert!(!graph_isomorphic(
        &pattern,
        &reversed,
        |pattern, target| pattern == target,
        |_, _| true
    ));
}
