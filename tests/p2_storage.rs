#![cfg_attr(feature = "unsafe-fast", allow(unsafe_code))]

use weavatrix_graph::{
    BitMatrix, GraphView, NodeIndex, RandomGraphGenerator, complete_bipartite_topology,
    grid_topology, star_topology,
};

#[test]
fn bit_matrix_crosses_word_boundaries_and_tracks_edges() {
    let mut matrix = BitMatrix::try_new(9).unwrap();
    for &(source, target) in &[(0, 0), (7, 0), (7, 1), (8, 8)] {
        assert!(matrix.insert(node(source), node(target)).unwrap());
        assert!(!matrix.insert(node(source), node(target)).unwrap());
    }
    assert_eq!(matrix.edge_count(), 4);
    assert_eq!(matrix.storage_bytes(), 16);
    assert_eq!(
        matrix
            .outgoing(node(7))
            .map(NodeIndex::index)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        matrix
            .incoming(node(8))
            .map(NodeIndex::index)
            .collect::<Vec<_>>(),
        vec![8]
    );
    assert!(matrix.remove(node(7), node(0)));
    assert!(!matrix.remove(node(7), node(0)));
    assert_eq!(matrix.edge_count(), 3);
    assert!(!matrix.contains(node(99), node(0)));
}

#[test]
fn bit_matrix_materializes_topology_and_validates_wire_data() {
    let mut matrix = BitMatrix::try_new(3).unwrap();
    matrix.insert(node(0), node(1)).unwrap();
    matrix.insert_undirected(node(1), node(2)).unwrap();
    let topology = matrix.to_topology().unwrap();
    assert_eq!(
        topology
            .edge_references()
            .map(|(_, edge)| (edge.source().index(), edge.target().index()))
            .collect::<Vec<_>>(),
        vec![(0, 1), (1, 2), (2, 1)]
    );

    let mut value = serde_json::to_value(&matrix).unwrap();
    value["edge_count"] = serde_json::json!(99);
    assert_eq!(
        serde_json::from_value::<BitMatrix>(value)
            .unwrap()
            .edge_count(),
        3
    );
    assert!(
        serde_json::from_value::<BitMatrix>(serde_json::json!({
            "node_count": 1,
            "edge_count": 0,
            "words": [2]
        }))
        .is_err()
    );
}

#[test]
fn deterministic_generators_cover_standard_p2_shapes() {
    let star = star_topology(5).unwrap();
    assert_eq!(pairs(&star), vec![(0, 1), (0, 2), (0, 3), (0, 4)]);

    let grid = grid_topology(2, 3).unwrap();
    assert_eq!(
        pairs(&grid),
        vec![(0, 1), (0, 3), (1, 2), (1, 4), (2, 5), (3, 4), (4, 5)]
    );

    let bipartite = complete_bipartite_topology(2, 3).unwrap();
    assert_eq!(
        pairs(&bipartite),
        vec![(0, 2), (0, 3), (0, 4), (1, 2), (1, 3), (1, 4)]
    );
}

#[test]
fn seeded_dag_generator_is_reproducible_and_acyclic_by_construction() {
    let mut first = RandomGraphGenerator::new(42);
    let mut second = RandomGraphGenerator::new(42);
    let first = first.dag(32, 1, 4).unwrap();
    let second = second.dag(32, 1, 4).unwrap();
    assert_eq!(first, second);
    assert!(
        first
            .edge_references()
            .all(|(_, edge)| edge.source().index() < edge.target().index())
    );
}

#[cfg(feature = "unsafe-fast")]
#[test]
fn fast_bit_lookups_match_safe_mode_and_keep_a_safe_checked_entrypoint() {
    let mut matrix = BitMatrix::try_new(65).unwrap();
    matrix.insert(node(1), node(64)).unwrap();
    for source in 0..65 {
        for target in 0..65 {
            let source = node(source);
            let target = node(target);
            assert_eq!(
                matrix.contains_fast(source, target),
                matrix.contains(source, target)
            );
            // SAFETY: Both loop indexes are inside the 65-node matrix.
            assert_eq!(
                unsafe { matrix.contains_unchecked(source, target) },
                matrix.contains(source, target)
            );
        }
    }
    assert!(!matrix.contains_fast(node(65), node(0)));
}

fn pairs(graph: &weavatrix_graph::Topology) -> Vec<(usize, usize)> {
    graph
        .edge_references()
        .map(|(_, edge)| (edge.source().index(), edge.target().index()))
        .collect()
}

fn node(index: u32) -> NodeIndex {
    NodeIndex::new(index)
}
