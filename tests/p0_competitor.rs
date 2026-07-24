use petgraph::algo::articulation_points::articulation_points as pet_articulation;
use petgraph::algo::{
    bridges as pet_bridges, floyd_warshall as pet_floyd, is_isomorphic as pet_isomorphic,
};
use petgraph::graph::{DiGraph, UnGraph};
use petgraph::visit::EdgeRef;
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, Topology, UndirectedTopology, bridges_and_articulation_points,
    floyd_warshall, graph_isomorphic, johnson_all_pairs,
};

#[test]
fn all_pairs_matches_petgraph_on_seeded_weighted_dags() {
    for seed in 1..=24 {
        let node_count = 4 + usize::try_from(seed).unwrap() % 13;
        let mut state = seed;
        let mut edges = Vec::new();
        for source in 0..node_count {
            for target in source + 1..node_count {
                state = next(state);
                if state % 5 < 2 {
                    let weight = i64::try_from(state % 17).unwrap() - 6;
                    edges.push((source, target, weight));
                }
            }
        }
        compare_all_pairs(node_count, &edges);
    }
}

#[test]
fn undirected_cuts_match_petgraph_on_seeded_simple_graphs() {
    for seed in 1..=32 {
        let node_count = 3 + usize::try_from(seed).unwrap() % 17;
        let mut state = seed * 13;
        let mut edges = Vec::new();
        for source in 0..node_count {
            for target in source + 1..node_count {
                state = next(state);
                if state % 7 < 2 {
                    edges.push((source, target));
                }
            }
        }
        compare_cuts(node_count, &edges);
    }
}

#[test]
fn graph_isomorphism_matches_petgraph_on_seeded_pairs() {
    for seed in 1..=20 {
        let node_count = 4 + usize::try_from(seed).unwrap() % 7;
        let mut state = seed * 31;
        let mut left_edges = Vec::new();
        let mut right_edges = Vec::new();
        for source in 0..node_count {
            for target in 0..node_count {
                state = next(state);
                if source != target && state % 9 < 2 {
                    left_edges.push((source, target));
                }
                state = next(state);
                if source != target && state % 9 < 2 {
                    right_edges.push((source, target));
                }
            }
        }
        compare_isomorphism(node_count, &left_edges, &right_edges);
    }
}

fn compare_all_pairs(node_count: usize, edges: &[(usize, usize, i64)]) {
    let topology = Topology::try_from_edges(
        node_count,
        edges
            .iter()
            .map(|&(source, target, _)| EdgeEndpoints::new(compact(source), compact(target))),
    )
    .unwrap();
    let weights = edges.iter().map(|edge| edge.2).collect::<Vec<_>>();
    let floyd = floyd_warshall(&topology, |edge| weights[edge.index()]).unwrap();
    let johnson = johnson_all_pairs(&topology, |edge| weights[edge.index()]).unwrap();
    let mut pet = DiGraph::<(), i64>::new();
    let nodes = (0..node_count)
        .map(|_| pet.add_node(()))
        .collect::<Vec<_>>();
    for &(source, target, weight) in edges {
        pet.add_edge(nodes[source], nodes[target], weight);
    }
    let expected = pet_floyd(&pet, |edge| *edge.weight()).unwrap();
    for source in 0..node_count {
        for target in 0..node_count {
            let pet_distance = expected[&(nodes[source], nodes[target])];
            // petgraph's integer sentinel can absorb a negative edge and become
            // `i64::MAX - n`; the generated real distances stay near zero.
            let expected = (pet_distance < i64::MAX / 4).then_some(pet_distance);
            assert_eq!(floyd.distance(compact(source), compact(target)), expected);
            assert_eq!(johnson.distance(compact(source), compact(target)), expected);
        }
    }
}

fn compare_cuts(node_count: usize, edges: &[(usize, usize)]) {
    let graph = UndirectedTopology::try_from_edges(
        node_count,
        edges
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(compact(source), compact(target))),
    )
    .unwrap();
    let actual = bridges_and_articulation_points(&graph);
    let mut pet = UnGraph::<(), ()>::new_undirected();
    let nodes = (0..node_count)
        .map(|_| pet.add_node(()))
        .collect::<Vec<_>>();
    for &(source, target) in edges {
        pet.add_edge(nodes[source], nodes[target], ());
    }
    let mut expected_bridges = pet_bridges(&pet)
        .map(|edge| edge.id().index())
        .collect::<Vec<_>>();
    expected_bridges.sort_unstable();
    let mut expected_points = pet_articulation(&pet)
        .into_iter()
        .map(petgraph::graph::NodeIndex::index)
        .collect::<Vec<_>>();
    expected_points.sort_unstable();
    assert_eq!(
        actual
            .bridges()
            .iter()
            .map(|edge| edge.index())
            .collect::<Vec<_>>(),
        expected_bridges
    );
    assert_eq!(
        actual
            .articulation_points()
            .iter()
            .map(|node| node.index())
            .collect::<Vec<_>>(),
        expected_points
    );
}

fn compare_isomorphism(
    node_count: usize,
    left_edges: &[(usize, usize)],
    right_edges: &[(usize, usize)],
) {
    let left = directed(node_count, left_edges);
    let right = directed(node_count, right_edges);
    let mut pet_left = DiGraph::<(), ()>::new();
    let mut pet_right = DiGraph::<(), ()>::new();
    let left_nodes = (0..node_count)
        .map(|_| pet_left.add_node(()))
        .collect::<Vec<_>>();
    let right_nodes = (0..node_count)
        .map(|_| pet_right.add_node(()))
        .collect::<Vec<_>>();
    for &(source, target) in left_edges {
        pet_left.add_edge(left_nodes[source], left_nodes[target], ());
    }
    for &(source, target) in right_edges {
        pet_right.add_edge(right_nodes[source], right_nodes[target], ());
    }
    assert_eq!(
        graph_isomorphic(&left, &right, |_, _| true, |_, _| true),
        pet_isomorphic(&pet_left, &pet_right)
    );
}

fn directed(node_count: usize, edges: &[(usize, usize)]) -> Topology {
    Topology::try_from_edges(
        node_count,
        edges
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(compact(source), compact(target))),
    )
    .unwrap()
}

fn compact(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap())
}

fn next(value: u64) -> u64 {
    value
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}
