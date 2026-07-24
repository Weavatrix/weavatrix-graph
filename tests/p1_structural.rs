use petgraph::algo::{maximal_cliques as pet_cliques, maximum_matching as pet_matching};
use petgraph::graph::UnGraph;
use std::collections::BTreeSet;
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, UndirectedTopology, bipartite_partition, dsatur_coloring,
    maximal_cliques, maximum_bipartite_matching, maximum_matching,
};

#[test]
fn bipartite_partition_and_hopcroft_karp_find_a_perfect_matching() {
    let graph = graph(6, &[(0, 3), (0, 4), (1, 3), (1, 5), (2, 4)]);
    let partition = bipartite_partition(&graph).unwrap();
    assert_eq!(partition.left().len(), 3);
    assert_eq!(partition.right().len(), 3);
    let matching = maximum_bipartite_matching(&graph).unwrap();
    assert_eq!(matching.len(), 3);
    let covered = matching
        .pairs()
        .iter()
        .flat_map(|&(left, right)| [left.index(), right.index()])
        .collect::<BTreeSet<_>>();
    assert_eq!(covered.len(), 6);
}

#[test]
fn odd_cycles_and_self_loops_are_not_bipartite() {
    let triangle = graph(3, &[(0, 1), (1, 2), (2, 0)]);
    assert!(bipartite_partition(&triangle).is_none());
    assert!(maximum_bipartite_matching(&triangle).is_none());
    assert!(bipartite_partition(&graph(1, &[(0, 0)])).is_none());
}

#[test]
fn bron_kerbosch_returns_canonical_maximal_cliques_and_limits_output() {
    let graph = graph(6, &[(0, 1), (1, 2), (2, 0), (2, 3), (3, 4), (4, 2)]);
    let result = maximal_cliques(&graph, 10);
    let actual = result
        .cliques()
        .iter()
        .map(|clique| clique.iter().map(|node| node.index()).collect::<Vec<_>>())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual,
        BTreeSet::from([vec![0, 1, 2], vec![2, 3, 4], vec![5]])
    );
    assert!(!result.truncated());

    let limited = maximal_cliques(&graph, 1);
    assert_eq!(limited.cliques().len(), 1);
    assert!(limited.truncated());
}

#[test]
fn matching_and_cliques_match_petgraph_on_seeded_graphs() {
    for seed in 1_u64..=20 {
        let left = 3 + usize::try_from(seed % 5).unwrap();
        let right = 3 + usize::try_from((seed * 3) % 5).unwrap();
        let mut state = seed;
        let mut edges = Vec::new();
        for source in 0..left {
            for target in left..left + right {
                state = next(state);
                if state % 3 != 0 {
                    edges.push((
                        u32::try_from(source).unwrap(),
                        u32::try_from(target).unwrap(),
                    ));
                }
            }
        }
        let ours = graph(left + right, &edges);
        let pet = pet_graph(left + right, &edges);
        assert_eq!(
            maximum_bipartite_matching(&ours).unwrap().len(),
            pet_matching(&pet).len()
        );

        let ours_cliques = maximal_cliques(&ours, usize::MAX)
            .cliques()
            .iter()
            .map(|clique| clique.iter().map(|node| node.index()).collect::<Vec<_>>())
            .collect::<BTreeSet<_>>();
        let pet_cliques = pet_cliques(&pet)
            .into_iter()
            .map(|clique| {
                let mut nodes = clique
                    .into_iter()
                    .map(petgraph::graph::NodeIndex::index)
                    .collect::<Vec<_>>();
                nodes.sort_unstable();
                nodes
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(ours_cliques, pet_cliques);
    }
}

#[test]
fn edmonds_matching_matches_petgraph_on_general_seeded_graphs() {
    for seed in 1_u64..=32 {
        let node_count = 5 + usize::try_from(seed % 9).unwrap();
        let mut state = seed * 47;
        let mut edges = Vec::new();
        for source in 0..node_count {
            for target in source + 1..node_count {
                state = next(state);
                if state % 5 < 2 {
                    edges.push((
                        u32::try_from(source).unwrap(),
                        u32::try_from(target).unwrap(),
                    ));
                }
            }
        }
        let ours = graph(node_count, &edges);
        let pet = pet_graph(node_count, &edges);
        assert_eq!(maximum_matching(&ours).len(), pet_matching(&pet).len());
    }
}

#[test]
fn edmonds_matching_ignores_self_loops_and_parallel_edges() {
    let graph = graph(4, &[(0, 0), (0, 1), (0, 1), (1, 2), (2, 3)]);
    let matching = maximum_matching(&graph);
    assert_eq!(matching.len(), 2);
    let covered = matching
        .pairs()
        .iter()
        .flat_map(|&(left, right)| [left.index(), right.index()])
        .collect::<BTreeSet<_>>();
    assert_eq!(covered, BTreeSet::from([0, 1, 2, 3]));
}

#[test]
fn dsatur_coloring_is_deterministic_valid_and_rejects_self_loops() {
    let colored_graph = graph(6, &[(0, 1), (1, 2), (2, 0), (2, 3), (3, 4), (4, 5), (5, 3)]);
    let coloring = dsatur_coloring(&colored_graph).unwrap();
    assert_eq!(coloring.color_count(), 3);
    let colors = coloring
        .assignments()
        .iter()
        .map(|&(node, color)| (node.index(), color))
        .collect::<std::collections::BTreeMap<_, _>>();
    for &(left, right) in &[(0, 1), (1, 2), (2, 0), (2, 3), (3, 4), (4, 5), (5, 3)] {
        assert_ne!(colors[&left], colors[&right]);
    }
    assert_eq!(dsatur_coloring(&colored_graph), Some(coloring));
    assert!(dsatur_coloring(&graph(1, &[(0, 0)])).is_none());
}

fn graph(node_count: usize, edges: &[(u32, u32)]) -> UndirectedTopology {
    UndirectedTopology::try_from_edges(
        node_count,
        edges.iter().map(|&(source, target)| {
            EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
        }),
    )
    .unwrap()
}

fn pet_graph(node_count: usize, edges: &[(u32, u32)]) -> UnGraph<(), ()> {
    let mut graph = UnGraph::new_undirected();
    let nodes = (0..node_count)
        .map(|_| graph.add_node(()))
        .collect::<Vec<_>>();
    for &(source, target) in edges {
        graph.add_edge(
            nodes[usize::try_from(source).unwrap()],
            nodes[usize::try_from(target).unwrap()],
            (),
        );
    }
    graph
}

fn next(value: u64) -> u64 {
    value
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}
