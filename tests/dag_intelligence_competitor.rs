use petgraph::algo::dominators::simple_fast;
use petgraph::graph::{DiGraph, NodeIndex as PetNodeIndex};
use std::collections::BTreeSet;
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, Topology, dag_weighted_longest_path, dominance_frontiers,
    topological_generations,
};

#[test]
fn dag_intelligence_matches_seeded_independent_references() {
    let mut seed = 0x8e3c_6a21_bf09_754d_u64;
    for node_count in 2..40 {
        for _ in 0..12 {
            let mut edges = Vec::new();
            for source in 0..node_count {
                for target in (source + 1)..node_count {
                    if next(&mut seed).is_multiple_of(7) {
                        edges.push((source, target, next(&mut seed) % 31 + 1));
                    }
                }
            }
            compare_dag(node_count, &edges);
        }
    }
}

#[test]
fn dominance_frontiers_match_petgraph_dominator_reference() {
    let mut seed = 0xd4c7_29e1_503b_86af_u64;
    for node_count in 2..36 {
        for _ in 0..10 {
            let mut edges = Vec::new();
            for source in 0..node_count {
                for target in 0..node_count {
                    if source != target && next(&mut seed).is_multiple_of(11) {
                        edges.push((source, target));
                    }
                }
            }
            compare_frontiers(node_count, &edges);
        }
    }
}

fn compare_dag(node_count: usize, edges: &[(usize, usize, u64)]) {
    let pairs = edges
        .iter()
        .map(|&(source, target, _)| (source, target))
        .collect::<Vec<_>>();
    let graph = topology(node_count, &pairs);
    let actual = dag_weighted_longest_path(&graph, |edge| Some(edges[edge.index()].2))
        .unwrap()
        .unwrap();
    assert_eq!(actual.total_cost(), reference_longest(node_count, edges));

    let generations = topological_generations(&graph).unwrap();
    let mut generation_at = vec![usize::MAX; node_count];
    for (generation, nodes) in generations.iter().enumerate() {
        for node in nodes {
            assert_eq!(generation_at[node.index()], usize::MAX);
            generation_at[node.index()] = generation;
        }
    }
    assert!(
        generation_at
            .iter()
            .all(|generation| *generation != usize::MAX)
    );
    for &(source, target, _) in edges {
        assert!(generation_at[source] < generation_at[target]);
    }
}

fn compare_frontiers(node_count: usize, edges: &[(usize, usize)]) {
    let ours = topology(node_count, edges);
    let actual = dominance_frontiers(&ours, NodeIndex::new(0)).unwrap();
    let (pet, nodes) = pet_graph(node_count, edges);
    let dominators = simple_fast(&pet, nodes[0]);
    let expected = reference_frontiers(node_count, edges, &nodes, &dominators);
    for (node, expected) in expected.iter().enumerate().take(node_count) {
        let actual = actual
            .frontier(NodeIndex::new(u32::try_from(node).unwrap()))
            .map(|frontier| frontier.iter().map(|node| node.index()).collect());
        assert_eq!(actual.as_ref(), expected.as_ref());
    }
}

fn reference_longest(node_count: usize, edges: &[(usize, usize, u64)]) -> u64 {
    let mut distances = vec![0_u64; node_count];
    for source in 0..node_count {
        for &(edge_source, target, weight) in edges {
            if edge_source == source {
                distances[target] = distances[target].max(distances[source] + weight);
            }
        }
    }
    distances.into_iter().max().unwrap_or(0)
}

fn reference_frontiers(
    node_count: usize,
    edges: &[(usize, usize)],
    nodes: &[PetNodeIndex],
    dominators: &petgraph::algo::dominators::Dominators<PetNodeIndex>,
) -> Vec<Option<BTreeSet<usize>>> {
    let mut immediate = vec![None; node_count];
    immediate[0] = Some(0);
    for node in 1..node_count {
        immediate[node] = dominators
            .immediate_dominator(nodes[node])
            .map(PetNodeIndex::index);
    }
    let mut predecessors = vec![BTreeSet::new(); node_count];
    for &(source, target) in edges {
        if immediate[source].is_some() && immediate[target].is_some() {
            predecessors[target].insert(source);
        }
    }
    let mut frontiers = vec![BTreeSet::new(); node_count];
    for join in 0..node_count {
        if predecessors[join].len() < 2 {
            continue;
        }
        let stop = immediate[join].unwrap();
        for &predecessor in &predecessors[join] {
            let mut runner = predecessor;
            while runner != stop {
                frontiers[runner].insert(join);
                runner = immediate[runner].unwrap();
            }
        }
    }
    immediate
        .into_iter()
        .enumerate()
        .map(|(node, parent)| parent.map(|_| core::mem::take(&mut frontiers[node])))
        .collect()
}

fn topology(node_count: usize, edges: &[(usize, usize)]) -> Topology {
    Topology::try_from_edges(
        node_count,
        edges.iter().map(|&(source, target)| {
            EdgeEndpoints::new(
                NodeIndex::new(u32::try_from(source).unwrap()),
                NodeIndex::new(u32::try_from(target).unwrap()),
            )
        }),
    )
    .unwrap()
}

fn pet_graph(node_count: usize, edges: &[(usize, usize)]) -> (DiGraph<(), ()>, Vec<PetNodeIndex>) {
    let mut graph = DiGraph::with_capacity(node_count, edges.len());
    let nodes = (0..node_count)
        .map(|_| graph.add_node(()))
        .collect::<Vec<_>>();
    for &(source, target) in edges {
        graph.add_edge(nodes[source], nodes[target], ());
    }
    (graph, nodes)
}

fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}
