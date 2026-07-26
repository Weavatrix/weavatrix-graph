use weavatrix_graph::{
    Direction, EdgeEndpoints, NodeIndex, Topology, TraversalCacheWorkspace, TraversalLayout,
    TraversalStorage, bfs_filtered, dfs_filtered, shortest_path_filtered,
};

fn edge(source: u32, target: u32) -> EdgeEndpoints {
    EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
}

fn fixture() -> Topology {
    Topology::try_from_edges(
        9,
        [
            edge(0, 1),
            edge(0, 1),
            edge(0, 0),
            edge(1, 2),
            edge(2, 3),
            edge(4, 3),
            edge(3, 5),
            edge(5, 6),
            edge(6, 4),
            edge(7, 7),
        ],
    )
    .unwrap()
}

#[test]
fn every_layout_preserves_exact_direct_adjacencies() {
    let topology = fixture();
    for storage in [
        TraversalStorage::Fast,
        TraversalStorage::Balanced,
        TraversalStorage::Compact,
    ] {
        let cache = topology.traversal_cache_with(storage);
        assert_eq!(cache.node_count(), topology.node_count());
        assert_eq!(cache.edge_count(), topology.edge_count());
        for raw in 0..topology.node_count() {
            let node = NodeIndex::new(u32::try_from(raw).unwrap());
            assert_eq!(
                cache.outgoing_neighbors(node).collect::<Vec<_>>(),
                topology.outgoing_neighbors(node).collect::<Vec<_>>()
            );
            assert_eq!(
                cache.incoming_neighbors(node).collect::<Vec<_>>(),
                topology.incoming_neighbors(node).collect::<Vec<_>>()
            );
            assert_eq!(cache.out_degree(node), topology.out_degree(node));
            assert_eq!(cache.in_degree(node), topology.in_degree(node));
        }
    }
}

#[test]
fn lazy_walks_match_generic_walks_in_every_direction() {
    let topology = fixture();
    for storage in [
        TraversalStorage::Fast,
        TraversalStorage::Balanced,
        TraversalStorage::Compact,
    ] {
        let cache = topology.traversal_cache_with(storage);
        for direction in [Direction::Outgoing, Direction::Incoming, Direction::Both] {
            let start = NodeIndex::new(3);
            let breadth = bfs_filtered(&topology, start, direction, |_| true);
            let depth = dfs_filtered(&topology, start, direction, |_| true);
            let mut workspace = TraversalCacheWorkspace::new();
            assert_eq!(
                cache
                    .bfs_iter(start, direction, &mut workspace)
                    .collect::<Vec<_>>(),
                breadth
            );
            assert_eq!(
                cache
                    .dfs_iter(start, direction, &mut workspace)
                    .collect::<Vec<_>>(),
                depth
            );
        }
    }
}

#[test]
fn reachability_paths_and_workspace_reuse_match_topology() {
    let topology = fixture();
    let cache = topology.traversal_cache_with(TraversalStorage::Compact);
    let mut workspace = TraversalCacheWorkspace::new();
    for (source, target) in [(0, 6), (6, 2), (4, 0), (7, 7)] {
        let source = NodeIndex::new(source);
        let target = NodeIndex::new(target);
        let expected =
            shortest_path_filtered(&topology, source, target, Direction::Outgoing, |_| true);
        assert_eq!(
            cache.shortest_path(source, target, Direction::Outgoing, &mut workspace),
            expected
        );
        assert_eq!(
            cache.reachable(source, target, Direction::Outgoing),
            expected.is_some()
        );
    }
}

#[test]
fn invalid_and_empty_graph_queries_are_total() {
    let empty = Topology::try_from_edges(0, []).unwrap().traversal_cache();
    let invalid = NodeIndex::new(99);
    assert!(empty.outgoing_neighbors(invalid).next().is_none());
    assert_eq!(empty.out_degree(invalid), None);
    assert!(empty.bfs(invalid, Direction::Outgoing).is_empty());
    assert!(!empty.reachable(invalid, invalid, Direction::Both));
    assert_eq!(
        empty.shortest_path(
            invalid,
            invalid,
            Direction::Outgoing,
            &mut TraversalCacheWorkspace::new()
        ),
        None
    );
}

#[test]
fn packed_values_cross_word_and_block_boundaries_losslessly() {
    let node_count = 100_003;
    let edges = (0..257_u32)
        .flat_map(|source| {
            [
                edge(source, (source * 997 + 65_537) % node_count),
                edge(source, (source * 313 + 99_991) % node_count),
            ]
        })
        .collect::<Vec<_>>();
    let topology = Topology::try_from_edges(node_count as usize, edges).unwrap();
    for storage in [TraversalStorage::Balanced, TraversalStorage::Compact] {
        let cache = topology.traversal_cache_with(storage);
        for raw in 0..node_count {
            let node = NodeIndex::new(raw);
            assert_eq!(
                cache.outgoing_neighbors(node).collect::<Vec<_>>(),
                topology.outgoing_neighbors(node).collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn auto_selects_a_smaller_speed_oriented_layout_when_worthwhile() {
    let node_count = 1_024_u32;
    let edges = (0..8_192_u32)
        .map(|index| edge(index % node_count, (index * 17 + 3) % node_count))
        .collect::<Vec<_>>();
    let cache = Topology::try_from_edges(node_count as usize, edges)
        .unwrap()
        .traversal_cache();
    assert!(matches!(cache.layout(), TraversalLayout::Balanced { .. }));
    assert!(cache.storage_bytes() < cache.fast_equivalent_bytes());
}

#[test]
fn compact_layout_exploits_local_ids_without_reordering() {
    let edges = (0..128_u32)
        .flat_map(|source| {
            (0..64_u32).map(move |offset| edge(source, 500_000 + source * 64 + offset))
        })
        .collect::<Vec<_>>();
    let topology = Topology::try_from_edges(1_000_000, edges).unwrap();
    let balanced = topology.traversal_cache_with(TraversalStorage::Balanced);
    let compact = topology.traversal_cache_with(TraversalStorage::Compact);
    assert!(compact.storage_bytes() * 2 < balanced.storage_bytes());
    for source in 0..128 {
        let node = NodeIndex::new(source);
        assert_eq!(
            compact.outgoing_neighbors(node).collect::<Vec<_>>(),
            topology.outgoing_neighbors(node).collect::<Vec<_>>()
        );
    }
}
