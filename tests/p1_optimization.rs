use std::collections::BTreeSet;
use weavatrix_graph::{
    EdgeEndpoints, EdgeIndex, NodeIndex, Topology, UndirectedGraphView, UndirectedTopology,
    feedback_arc_set_heuristic, steiner_tree_approximation, topological_sort_filtered,
};

#[test]
fn feedback_arc_set_is_empty_for_a_dag_and_breaks_cycles() {
    let dag = directed(4, &[(0, 1), (0, 2), (1, 3), (2, 3)]);
    assert!(feedback_arc_set_heuristic(&dag).edges().is_empty());

    let cyclic = directed(5, &[(0, 1), (1, 2), (2, 0), (2, 3), (3, 4), (4, 3)]);
    let result = feedback_arc_set_heuristic(&cyclic);
    assert_eq!(result.order().len(), cyclic.node_count());
    assert_acyclic_without(&cyclic, result.edges());
}

#[test]
fn feedback_arc_set_handles_self_loops_and_seeded_graphs() {
    let looped = directed(2, &[(0, 0), (0, 1)]);
    let result = feedback_arc_set_heuristic(&looped);
    assert_eq!(result.edges(), &[EdgeIndex::new(0)]);
    assert_acyclic_without(&looped, result.edges());

    for seed in 1_u64..=32 {
        let mut state = seed;
        let mut edges = Vec::new();
        for source in 0..7 {
            for target in 0..7 {
                state = next(state);
                if source != target && state % 7 == 0 {
                    edges.push((source, target));
                }
            }
        }
        let graph = directed(7, &edges);
        let result = feedback_arc_set_heuristic(&graph);
        assert_acyclic_without(&graph, result.edges());
    }
}

#[test]
fn steiner_tree_uses_shared_paths_and_prunes_non_terminal_leaves() {
    let graph = undirected(6, &[(0, 1), (1, 2), (2, 3), (0, 4), (4, 3), (2, 5)]);
    let weights = [1, 1, 1, 5, 5, 1];
    let tree = steiner_tree_approximation(
        &graph,
        &[NodeIndex::new(0), NodeIndex::new(3), NodeIndex::new(5)],
        |edge| weights[edge.index()],
    )
    .unwrap()
    .unwrap();
    assert_eq!(tree.total_cost(), 4);
    assert_eq!(edge_slots(tree.edges()), BTreeSet::from([0_usize, 1, 2, 5]));
    assert_terminals_connected(&graph, tree.edges(), tree.terminals());
}

#[test]
fn steiner_tree_handles_degenerate_invalid_and_disconnected_inputs() {
    let graph = undirected(3, &[(0, 1)]);
    let singleton =
        steiner_tree_approximation(&graph, &[NodeIndex::new(1), NodeIndex::new(1)], |_| 1)
            .unwrap()
            .unwrap();
    assert!(singleton.edges().is_empty());
    assert_eq!(singleton.terminals(), &[NodeIndex::new(1)]);

    assert!(
        steiner_tree_approximation(&graph, &[NodeIndex::new(0), NodeIndex::new(2)], |_| 1)
            .unwrap()
            .is_none()
    );
    assert!(
        steiner_tree_approximation(&graph, &[NodeIndex::new(99)], |_| 1)
            .unwrap()
            .is_none()
    );
}

#[test]
fn steiner_approximation_respects_the_two_approximation_bound_on_seeded_graphs() {
    for seed in 1_u64..=24 {
        let (graph, weights) = seeded_connected_graph(seed);
        let terminals = [NodeIndex::new(0), NodeIndex::new(2), NodeIndex::new(5)];
        let tree = steiner_tree_approximation(&graph, &terminals, |edge| weights[edge.index()])
            .unwrap()
            .unwrap();
        assert_terminals_connected(&graph, tree.edges(), &terminals);
        let optimum = exact_steiner_cost(&graph, &weights, &terminals);
        assert!(tree.total_cost() <= optimum.saturating_mul(2));
    }
}

fn assert_acyclic_without(graph: &Topology, removed: &[EdgeIndex]) {
    let removed = edge_slots(removed);
    assert!(topological_sort_filtered(graph, |edge| !removed.contains(&edge.index())).is_some());
}

fn assert_terminals_connected(
    graph: &UndirectedTopology,
    selected: &[EdgeIndex],
    terminals: &[NodeIndex],
) {
    let selected = edge_slots(selected);
    assert!(subset_connects(graph, &selected, terminals));
}

fn exact_steiner_cost(graph: &UndirectedTopology, weights: &[u64], terminals: &[NodeIndex]) -> u64 {
    (0_u64..(1_u64 << graph.edge_count()))
        .filter_map(|mask| {
            let selected = (0..graph.edge_count())
                .filter(|edge| mask & (1 << edge) != 0)
                .collect::<BTreeSet<_>>();
            subset_connects(graph, &selected, terminals)
                .then(|| selected.iter().map(|edge| weights[*edge]).sum::<u64>())
        })
        .min()
        .unwrap()
}

fn subset_connects(
    graph: &UndirectedTopology,
    selected: &BTreeSet<usize>,
    terminals: &[NodeIndex],
) -> bool {
    let mut reached = vec![false; graph.node_count()];
    let mut stack = vec![terminals[0].index()];
    reached[terminals[0].index()] = true;
    while let Some(node) = stack.pop() {
        for edge in graph.incident_edges(NodeIndex::new(u32::try_from(node).unwrap())) {
            if !selected.contains(&edge.index()) {
                continue;
            }
            let neighbor = graph
                .opposite(edge, NodeIndex::new(u32::try_from(node).unwrap()))
                .unwrap()
                .index();
            if !reached[neighbor] {
                reached[neighbor] = true;
                stack.push(neighbor);
            }
        }
    }
    terminals.iter().all(|node| reached[node.index()])
}

fn seeded_connected_graph(seed: u64) -> (UndirectedTopology, Vec<u64>) {
    let mut state = seed;
    let mut edges = vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)];
    for edge in [(0, 2), (0, 3), (1, 4), (2, 5), (0, 5)] {
        state = next(state);
        if state.is_multiple_of(2) {
            edges.push(edge);
        }
    }
    let graph = undirected(6, &edges);
    let weights = (0..edges.len())
        .map(|_| {
            state = next(state);
            1 + state % 9
        })
        .collect();
    (graph, weights)
}

fn directed(node_count: usize, edges: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges(
        node_count,
        edges
            .iter()
            .map(|&(source, target)| endpoints(source, target)),
    )
    .unwrap()
}

fn undirected(node_count: usize, edges: &[(u32, u32)]) -> UndirectedTopology {
    UndirectedTopology::try_from_edges(
        node_count,
        edges
            .iter()
            .map(|&(source, target)| endpoints(source, target)),
    )
    .unwrap()
}

fn endpoints(source: u32, target: u32) -> EdgeEndpoints {
    EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
}

fn edge_slots(edges: &[EdgeIndex]) -> BTreeSet<usize> {
    edges.iter().map(|edge| edge.index()).collect()
}

fn next(value: u64) -> u64 {
    value
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}
