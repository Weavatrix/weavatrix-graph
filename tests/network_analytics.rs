use weavatrix_graph::{
    Direction, EdgeEndpoints, NodeIndex, Topology, betweenness_centrality, closeness_centrality,
    cycle_basis, eigenvector_centrality, k_core_numbers, katz_centrality,
    label_propagation_communities,
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

fn score(scores: &[(NodeIndex, f64)], node: u32) -> f64 {
    scores
        .iter()
        .find(|(candidate, _)| *candidate == NodeIndex::new(node))
        .unwrap()
        .1
}

#[test]
fn path_centralities_rank_the_bridge_node_highest() {
    let graph = topology(3, &[(0, 1), (1, 0), (1, 2), (2, 1)]);
    let betweenness = betweenness_centrality(&graph, Direction::Both, true);
    let closeness = closeness_centrality(&graph, Direction::Both);
    assert!(score(&betweenness, 1) > score(&betweenness, 0));
    assert!(score(&closeness, 1) > score(&closeness, 0));
    assert!((score(&betweenness, 0) - score(&betweenness, 2)).abs() < f64::EPSILON);
}

#[test]
fn spectral_centralities_converge_and_rank_a_star_center_first() {
    let graph = topology(
        5,
        &[
            (0, 1),
            (1, 0),
            (0, 2),
            (2, 0),
            (0, 3),
            (3, 0),
            (0, 4),
            (4, 0),
        ],
    );
    let eigenvector = eigenvector_centrality(&graph, 1_000, 1e-10).unwrap();
    let katz = katz_centrality(&graph, 0.1, 1.0, 1_000, 1e-10).unwrap();
    assert!(eigenvector.converged());
    assert!(katz.converged());
    assert!(score(eigenvector.scores(), 0) > score(eigenvector.scores(), 1));
    assert!(score(katz.scores(), 0) > score(katz.scores(), 1));
}

#[test]
fn core_cycle_and_community_analysis_are_deterministic() {
    let graph = topology(6, &[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)]);
    let cores = k_core_numbers(&graph);
    assert!(cores.iter().all(|(_, core)| *core == 2));
    assert_eq!(cycle_basis(&graph).len(), 2);

    let communities = label_propagation_communities(&graph, 100);
    assert!(communities.converged());
    assert_eq!(communities.groups().len(), 2);
}
