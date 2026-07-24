use petgraph::algo::{
    dsatur_coloring as pet_coloring, greedy_feedback_arc_set as pet_feedback,
    steiner_tree as pet_steiner,
};
use petgraph::graph::{DiGraph, UnGraph};
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, Topology, UndirectedTopology, dsatur_coloring,
    feedback_arc_set_heuristic, steiner_tree_approximation,
};

#[test]
fn dsatur_and_feedback_match_petgraph_on_seeded_graphs() {
    for seed in 1_u64..=24 {
        let pairs = seeded_pairs(12, seed, false);
        let (ours, pet, _) = undirected_graphs(12, &pairs, seed);
        assert_eq!(
            dsatur_coloring(&ours).unwrap().color_count(),
            pet_coloring(&pet).1
        );

        let directed_pairs = seeded_pairs(12, seed * 31, true);
        let ours = directed_graph(12, &directed_pairs);
        let pet = pet_directed_graph(12, &directed_pairs);
        let ours_edges = feedback_arc_set_heuristic(&ours).edges().len();
        let pet_edges = pet_feedback(&pet).count();
        assert!(
            ours_edges <= pet_edges,
            "seed {seed}: {ours_edges} > {pet_edges}"
        );
    }
}

#[test]
fn multi_start_steiner_is_no_worse_than_petgraph_on_seeded_graphs() {
    for seed in 1_u64..=20 {
        let mut pairs = (0..7).map(|node| (node, node + 1)).collect::<Vec<_>>();
        pairs.extend(seeded_pairs(8, seed, false));
        pairs.sort_unstable();
        pairs.dedup();
        let (ours, pet, weights) = undirected_graphs(8, &pairs, seed);
        let ours_terminals = [node(0), node(3), node(7)];
        let pet_terminals = [
            petgraph::graph::NodeIndex::new(0),
            petgraph::graph::NodeIndex::new(3),
            petgraph::graph::NodeIndex::new(7),
        ];
        let ours_cost =
            steiner_tree_approximation(&ours, &ours_terminals, |edge| weights[edge.index()])
                .unwrap()
                .unwrap()
                .total_cost();
        let pet_cost = pet_steiner(&pet, &pet_terminals)
            .edge_weights()
            .map(|weight| u64::from(*weight))
            .sum::<u64>();
        assert!(
            ours_cost <= pet_cost,
            "seed {seed}: {ours_cost} > {pet_cost}"
        );
    }
}

fn seeded_pairs(node_count: usize, seed: u64, directed: bool) -> Vec<(usize, usize)> {
    let mut state = seed;
    let mut pairs = Vec::new();
    for source in 0..node_count {
        for target in 0..node_count {
            if source == target || (!directed && source > target) {
                continue;
            }
            state = next(state);
            if state % 5 < 2 {
                pairs.push((source, target));
            }
        }
    }
    pairs
}

fn undirected_graphs(
    node_count: usize,
    pairs: &[(usize, usize)],
    seed: u64,
) -> (UndirectedTopology, UnGraph<(), u32>, Vec<u64>) {
    let ours = UndirectedTopology::try_from_edges(
        node_count,
        pairs
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node(source), node(target))),
    )
    .unwrap();
    let mut pet = UnGraph::new_undirected();
    let nodes = (0..node_count)
        .map(|_| pet.add_node(()))
        .collect::<Vec<_>>();
    let mut state = seed;
    let mut weights = Vec::new();
    for &(source, target) in pairs {
        state = next(state);
        let weight = u32::try_from(1 + state % 17).unwrap();
        pet.add_edge(nodes[source], nodes[target], weight);
        weights.push(u64::from(weight));
    }
    (ours, pet, weights)
}

fn directed_graph(node_count: usize, pairs: &[(usize, usize)]) -> Topology {
    Topology::try_from_edges(
        node_count,
        pairs
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node(source), node(target))),
    )
    .unwrap()
}

fn pet_directed_graph(node_count: usize, pairs: &[(usize, usize)]) -> DiGraph<(), ()> {
    let mut graph = DiGraph::new();
    let nodes = (0..node_count)
        .map(|_| graph.add_node(()))
        .collect::<Vec<_>>();
    for &(source, target) in pairs {
        graph.add_edge(nodes[source], nodes[target], ());
    }
    graph
}

fn node(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap())
}

fn next(value: u64) -> u64 {
    value
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}
