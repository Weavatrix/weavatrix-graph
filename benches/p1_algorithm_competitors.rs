mod support;

use petgraph::algo::{
    dsatur_coloring as pet_coloring, greedy_feedback_arc_set as pet_feedback,
    maximal_cliques as pet_cliques, maximum_matching as pet_matching, steiner_tree as pet_steiner,
};
use petgraph::graph::{DiGraph, UnGraph};
use std::collections::BTreeSet;
use std::hint::black_box;
use support::{measure, print_measurement, topology_pairs};
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, Topology, UndirectedTopology, dsatur_coloring,
    feedback_arc_set_heuristic, maximal_cliques, maximum_matching, steiner_tree_approximation,
};

fn main() {
    println!("statistic=median runs=11 warmups=2");
    compare_matching();
    compare_cliques();
    compare_coloring();
    compare_feedback();
    compare_steiner();
}

fn compare_matching() {
    let pairs = undirected_pairs(400, 1_400);
    let (ours, pet, _) = undirected_graphs(400, &pairs);
    let ours_result = maximum_matching(&ours);
    let pet_result = pet_matching(&pet);
    assert_eq!(ours_result.len(), pet_result.len());
    println!("quality=maximum-matching pairs={}", ours_result.len());
    report(
        "maximum-matching",
        &measure(|| black_box(maximum_matching(&ours))),
        &measure(|| {
            black_box(
                pet_matching(&pet)
                    .edges()
                    .collect::<Vec<(petgraph::graph::NodeIndex, petgraph::graph::NodeIndex)>>(),
            )
        }),
    );
}

fn compare_cliques() {
    let pairs = undirected_pairs(180, 500);
    let (ours, pet, _) = undirected_graphs(180, &pairs);
    let ours_result = maximal_cliques(&ours, usize::MAX);
    let pet_result = pet_cliques(&pet);
    assert_eq!(ours_result.cliques().len(), pet_result.len());
    println!("quality=maximal-cliques count={}", pet_result.len());
    report(
        "maximal-cliques",
        &measure(|| black_box(maximal_cliques(&ours, usize::MAX))),
        &measure(|| black_box(pet_cliques(&pet))),
    );
}

fn compare_coloring() {
    let pairs = undirected_pairs(5_000, 15_000);
    let (ours, pet, _) = undirected_graphs(5_000, &pairs);
    let ours_colors = dsatur_coloring(&ours).unwrap().color_count();
    let pet_colors = pet_coloring(&pet).1;
    println!("quality=dsatur ours_colors={ours_colors} petgraph_colors={pet_colors}");
    report(
        "dsatur-coloring",
        &measure(|| black_box(dsatur_coloring(&ours).unwrap())),
        &measure(|| black_box(pet_coloring(&pet))),
    );
}

fn compare_feedback() {
    let pairs = topology_pairs(10_000, 30_000);
    let ours = directed_graph(10_000, &pairs);
    let pet = pet_directed_graph(10_000, &pairs);
    let ours_count = feedback_arc_set_heuristic(&ours).edges().len();
    let pet_count = pet_feedback(&pet).count();
    println!("quality=feedback-arc-set ours_edges={ours_count} petgraph_edges={pet_count}");
    report(
        "feedback-arc-set",
        &measure(|| black_box(feedback_arc_set_heuristic(&ours))),
        &measure(|| black_box(pet_feedback(&pet).count())),
    );
}

fn compare_steiner() {
    let pairs = undirected_pairs(1_000, 4_000);
    let (ours, pet, weights) = undirected_graphs(1_000, &pairs);
    let ours_terminals = terminal_slots(1_000, 32).map(node).collect::<Vec<_>>();
    let pet_terminals = terminal_slots(1_000, 32)
        .map(petgraph::graph::NodeIndex::new)
        .collect::<Vec<_>>();
    let ours_tree =
        steiner_tree_approximation(&ours, &ours_terminals, |edge| weights[edge.index()])
            .unwrap()
            .unwrap();
    let pet_tree = pet_steiner(&pet, &pet_terminals);
    let pet_cost = pet_tree
        .edge_weights()
        .map(|weight| u64::from(*weight))
        .sum::<u64>();
    println!(
        "quality=steiner ours_cost={} petgraph_cost={pet_cost}",
        ours_tree.total_cost()
    );
    report(
        "steiner-tree",
        &measure(|| {
            black_box(
                steiner_tree_approximation(&ours, &ours_terminals, |edge| weights[edge.index()])
                    .unwrap()
                    .unwrap(),
            )
        }),
        &measure(|| black_box(pet_steiner(&pet, &pet_terminals))),
    );
}

fn report(mode: &str, ours: &support::Measurement, petgraph: &support::Measurement) {
    print_measurement(mode, "library=weavatrix-graph", ours);
    print_measurement(mode, "library=petgraph", petgraph);
}

fn undirected_graphs(
    node_count: usize,
    pairs: &[(usize, usize)],
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
    let mut weights = Vec::with_capacity(pairs.len());
    for (index, &(source, target)) in pairs.iter().enumerate() {
        let weight = u32::try_from((source * 17 + target * 31 + index) % 19 + 1).unwrap();
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

fn undirected_pairs(node_count: usize, edge_count: usize) -> Vec<(usize, usize)> {
    let mut seen = BTreeSet::new();
    let mut pairs = Vec::with_capacity(edge_count);
    for (mut source, mut target) in topology_pairs(node_count, edge_count * 8) {
        if source > target {
            std::mem::swap(&mut source, &mut target);
        }
        if seen.insert((source, target)) {
            pairs.push((source, target));
            if pairs.len() == edge_count {
                break;
            }
        }
    }
    assert_eq!(pairs.len(), edge_count);
    pairs
}

fn terminal_slots(node_count: usize, count: usize) -> impl Iterator<Item = usize> {
    (0..count).map(move |index| index * node_count / count)
}

fn node(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap())
}
