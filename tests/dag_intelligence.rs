use std::cell::Cell;
use weavatrix_graph::{
    EdgeEndpoints, GraphError, NodeIndex, Topology, dag_longest_path, dag_longest_path_filtered,
    dag_longest_path_length, dag_longest_path_length_filtered, dag_weighted_longest_path,
    dominance_frontiers, dominance_frontiers_filtered, topological_generations,
    topological_generations_filtered,
};

fn topology(node_count: usize, edges: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges(
        node_count,
        edges.iter().map(|&(source, target)| {
            EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
        }),
    )
    .unwrap()
}

fn indexes(nodes: &[NodeIndex]) -> Vec<usize> {
    nodes.iter().map(|node| node.index()).collect()
}

#[test]
fn dag_paths_and_generations_are_deterministic() {
    let graph = topology(
        7,
        &[
            (0, 1),
            (0, 2),
            (1, 3),
            (2, 3),
            (3, 4),
            (2, 5),
            (4, 6),
            (5, 6),
        ],
    );
    let generations = topological_generations(&graph).unwrap();
    assert_eq!(
        generations
            .iter()
            .map(|generation| indexes(generation))
            .collect::<Vec<_>>(),
        [vec![0], vec![1, 2], vec![3, 5], vec![4], vec![6]]
    );

    let unweighted = dag_longest_path(&graph).unwrap().unwrap();
    assert_eq!(indexes(unweighted.nodes()), [0, 1, 3, 4, 6]);
    assert_eq!(unweighted.total_cost(), 4);
    assert_eq!(dag_longest_path_length(&graph).unwrap(), Some(4));

    let weights = [2_i64, 5, 10, 1, 3, 20, 4, 1];
    let calls = Cell::new(0);
    let weighted = dag_weighted_longest_path(&graph, |edge| {
        calls.set(calls.get() + 1);
        Some(weights[edge.index()])
    })
    .unwrap()
    .unwrap();
    assert_eq!(calls.get(), graph.edge_count());
    assert_eq!(indexes(weighted.nodes()), [0, 2, 5, 6]);
    assert_eq!(weighted.total_cost(), 26);
}

#[test]
fn dag_paths_filter_edges_and_reject_invalid_inputs() {
    let graph = topology(4, &[(0, 1), (1, 2), (0, 2), (2, 3)]);
    let filtered = dag_weighted_longest_path(&graph, |edge| {
        (edge.index() != 1).then_some([2_i64, 50, 3, 4][edge.index()])
    })
    .unwrap()
    .unwrap();
    assert_eq!(indexes(filtered.nodes()), [0, 2, 3]);
    assert_eq!(filtered.total_cost(), 7);

    let generations = topological_generations_filtered(&graph, |edge| edge.index() != 0).unwrap();
    assert_eq!(indexes(&generations[0]), [0, 1]);
    let unweighted = dag_longest_path_filtered(&graph, |edge| edge.index() != 1)
        .unwrap()
        .unwrap();
    assert_eq!(indexes(unweighted.nodes()), [0, 2, 3]);
    assert_eq!(
        dag_longest_path_length_filtered(&graph, |edge| edge.index() != 1).unwrap(),
        Some(2)
    );

    let negative = topology(2, &[(0, 1)]);
    let zero = dag_weighted_longest_path(&negative, |_| Some(-1_i64))
        .unwrap()
        .unwrap();
    assert_eq!(indexes(zero.nodes()), [0]);
    assert_eq!(zero.total_cost(), 0);

    assert!(matches!(
        dag_weighted_longest_path(&negative, |_| Some(f64::NAN)),
        Err(GraphError::InvalidAlgorithmParameter { .. })
    ));
    assert!(matches!(
        dag_weighted_longest_path(&negative, |_| Some(u64::MAX)),
        Ok(Some(_))
    ));
    let overflow = topology(3, &[(0, 1), (1, 2)]);
    assert!(matches!(
        dag_weighted_longest_path(&overflow, |_| Some(u64::MAX)),
        Err(GraphError::ArithmeticOverflow { .. })
    ));

    let cycle = topology(2, &[(0, 1), (1, 0)]);
    assert!(topological_generations(&cycle).is_none());
    assert!(matches!(
        dag_longest_path(&cycle),
        Err(GraphError::CyclicGraph { .. })
    ));
    assert!(dag_longest_path(&topology(0, &[])).unwrap().is_none());
}

#[test]
fn dominance_frontiers_cover_joins_loops_filters_and_unreachable_nodes() {
    let graph = topology(
        8,
        &[
            (0, 1),
            (0, 2),
            (1, 3),
            (2, 3),
            (3, 4),
            (3, 5),
            (4, 6),
            (5, 6),
            (6, 3),
        ],
    );
    let result = dominance_frontiers(&graph, NodeIndex::new(0)).unwrap();
    assert_eq!(result.root(), NodeIndex::new(0));
    assert_eq!(indexes(result.frontier(NodeIndex::new(1)).unwrap()), [3]);
    assert_eq!(indexes(result.frontier(NodeIndex::new(2)).unwrap()), [3]);
    assert_eq!(indexes(result.frontier(NodeIndex::new(3)).unwrap()), [3]);
    assert_eq!(indexes(result.frontier(NodeIndex::new(4)).unwrap()), [6]);
    assert_eq!(indexes(result.frontier(NodeIndex::new(5)).unwrap()), [6]);
    assert_eq!(indexes(result.frontier(NodeIndex::new(6)).unwrap()), [3]);
    assert!(result.frontier(NodeIndex::new(0)).unwrap().is_empty());
    assert!(result.frontier(NodeIndex::new(7)).is_none());
    assert_eq!(result.iter().count(), 7);

    let filtered =
        dominance_frontiers_filtered(&graph, NodeIndex::new(0), |edge| edge.index() != 8).unwrap();
    assert!(filtered.frontier(NodeIndex::new(3)).unwrap().is_empty());
    assert!(filtered.frontier(NodeIndex::new(6)).unwrap().is_empty());
    assert!(dominance_frontiers(&graph, NodeIndex::new(99)).is_none());

    let parallel = topology(2, &[(0, 1), (0, 1)]);
    assert!(
        dominance_frontiers(&parallel, NodeIndex::new(0))
            .unwrap()
            .frontier(NodeIndex::new(0))
            .unwrap()
            .is_empty()
    );
}
