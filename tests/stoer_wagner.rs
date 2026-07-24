use core::cell::Cell;
use petgraph::{Undirected, graph::Graph};
use rustworkx_core::connectivity::stoer_wagner_min_cut as rustworkx_min_cut;
use weavatrix_graph::{
    EdgeEndpoints, EdgeIndex, GraphError, NodeIndex, UndirectedTopology, stoer_wagner_min_cut,
    stoer_wagner_min_cut_filtered,
};

fn graph(node_count: usize, pairs: &[(u32, u32)]) -> UndirectedTopology {
    UndirectedTopology::try_from_edges(
        node_count,
        pairs.iter().map(|&(source, target)| {
            EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
        }),
    )
    .unwrap()
}

fn slots(nodes: &[NodeIndex]) -> Vec<usize> {
    nodes.iter().map(|node| node.index()).collect()
}

#[test]
fn weighted_triangle_returns_exact_canonical_cut() {
    let graph = graph(3, &[(0, 1), (1, 2), (0, 2)]);
    let weights = [2_u64, 4, 3];
    let cut = stoer_wagner_min_cut(&graph, |edge| weights[edge.index()])
        .unwrap()
        .unwrap();
    assert_eq!(cut.weight(), 5);
    assert_eq!(slots(cut.partition()), vec![0]);
    assert_eq!(slots(cut.complement()), vec![1, 2]);
}

#[test]
fn disconnected_graph_has_zero_cut_and_small_graphs_have_none() {
    let disconnected = graph(4, &[(0, 1), (2, 3)]);
    assert_eq!(
        stoer_wagner_min_cut(&disconnected, |_| 1_u64)
            .unwrap()
            .unwrap()
            .weight(),
        0
    );
    assert!(
        stoer_wagner_min_cut(&graph(0, &[]), |_| 1_u64)
            .unwrap()
            .is_none()
    );
    assert!(
        stoer_wagner_min_cut(&graph(1, &[]), |_| 1_u64)
            .unwrap()
            .is_none()
    );
}

#[test]
fn parallel_edges_aggregate_and_self_loops_do_not_change_cut() {
    let graph = graph(2, &[(0, 1), (0, 1), (0, 0)]);
    let weights = [3_u64, 5, 100];
    let cut = stoer_wagner_min_cut(&graph, |edge| weights[edge.index()])
        .unwrap()
        .unwrap();
    assert_eq!(cut.weight(), 8);
    assert_eq!(slots(cut.partition()), vec![0]);
}

#[test]
fn filtered_callbacks_run_exactly_once() {
    let graph = graph(3, &[(0, 1), (1, 2), (0, 2)]);
    let predicates = Cell::new(0);
    let weights = Cell::new(0);
    let cut = stoer_wagner_min_cut_filtered(
        &graph,
        |edge| {
            predicates.set(predicates.get() + 1);
            edge != EdgeIndex::new(2)
        },
        |_| {
            weights.set(weights.get() + 1);
            1_u64
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(predicates.get(), 3);
    assert_eq!(weights.get(), 2);
    assert_eq!(cut.weight(), 1);
}

#[test]
fn invalid_weights_and_overflow_are_reported() {
    let negative = stoer_wagner_min_cut(&graph(2, &[(0, 1)]), |_| -1_i64);
    assert!(matches!(
        negative,
        Err(GraphError::InvalidAlgorithmParameter { .. })
    ));
    let non_finite = stoer_wagner_min_cut(&graph(2, &[(0, 1)]), |_| f64::NAN);
    assert!(matches!(
        non_finite,
        Err(GraphError::InvalidAlgorithmParameter { .. })
    ));
    let overflow = stoer_wagner_min_cut(&graph(2, &[(0, 1), (0, 1)]), |_| u8::MAX);
    assert!(matches!(
        overflow,
        Err(GraphError::ArithmeticOverflow { .. })
    ));
}

#[test]
fn exhaustive_small_graphs_match_brute_force() {
    let node_count = 5;
    let possible = (0..node_count)
        .flat_map(|source| (source + 1..node_count).map(move |target| (source, target)))
        .collect::<Vec<_>>();
    for mask in 0..(1_usize << possible.len()) {
        let pairs = possible
            .iter()
            .enumerate()
            .filter_map(|(index, &(source, target))| {
                ((mask >> index) & 1 == 1).then_some((
                    u32::try_from(source).unwrap(),
                    u32::try_from(target).unwrap(),
                ))
            })
            .collect::<Vec<_>>();
        let graph = graph(node_count, &pairs);
        let cut = stoer_wagner_min_cut(&graph, |_| 1_u64).unwrap().unwrap();
        assert_eq!(cut.weight(), brute_force(node_count, &pairs));
        assert_eq!(cut_capacity(cut.partition(), &pairs), cut.weight());
    }
}

#[test]
fn weighted_results_match_rustworkx() {
    let pairs = [
        (0, 1),
        (0, 2),
        (1, 2),
        (1, 3),
        (2, 4),
        (3, 4),
        (3, 5),
        (4, 5),
    ];
    let weights = [3_u64, 2, 7, 1, 4, 3, 5, 2];
    let ours = graph(6, &pairs);
    let ours_cut = stoer_wagner_min_cut(&ours, |edge| weights[edge.index()])
        .unwrap()
        .unwrap();
    let mut reference = Graph::<(), u64, Undirected>::new_undirected();
    let nodes = (0..6).map(|_| reference.add_node(())).collect::<Vec<_>>();
    for (index, &(source, target)) in pairs.iter().enumerate() {
        reference.add_edge(
            nodes[source as usize],
            nodes[target as usize],
            weights[index],
        );
    }
    let reference_cut = rustworkx_min_cut(&reference, |edge| Ok::<u64, ()>(*edge.weight()))
        .unwrap()
        .unwrap();
    assert_eq!(ours_cut.weight(), reference_cut.0);
}

fn brute_force(node_count: usize, pairs: &[(u32, u32)]) -> u64 {
    (1..(1_usize << (node_count - 1)))
        .map(|mask| {
            pairs
                .iter()
                .filter(|&&(source, target)| {
                    let source_side = (mask >> source) & 1;
                    let target_side = (mask >> target) & 1;
                    source_side != target_side
                })
                .count() as u64
        })
        .min()
        .unwrap()
}

fn cut_capacity(partition: &[NodeIndex], pairs: &[(u32, u32)]) -> u64 {
    pairs
        .iter()
        .filter(|&&(source, target)| {
            let source_inside = partition.contains(&NodeIndex::new(source));
            let target_inside = partition.contains(&NodeIndex::new(target));
            source_inside != target_inside
        })
        .count() as u64
}
