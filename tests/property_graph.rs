#![cfg(not(windows))]

use proptest::prelude::*;
use weavatrix_graph::{
    Direction, EdgeEndpoints, GraphView, NodeIndex, StablePayloadGraph, Topology,
    TraversalWorkspace, betweenness_centrality, bfs, bfs_iter, closeness_centrality, dijkstra,
    dijkstra_measure, edge_filtered, k_core_numbers, reversed,
};

fn node_index(value: u8, node_count: usize) -> NodeIndex {
    let node_count = u32::try_from(node_count).expect("property range fits u32");
    NodeIndex::new(u32::from(value) % node_count)
}

fn graph(node_count: usize, raw_edges: &[(u8, u8)]) -> Topology {
    let edges = raw_edges.iter().map(|&(source, target)| {
        EdgeEndpoints::new(
            node_index(source, node_count),
            node_index(target, node_count),
        )
    });
    Topology::try_from_edges(node_count, edges).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn lazy_and_materialized_bfs_are_equivalent(
        node_count in 1_usize..32,
        edges in proptest::collection::vec((any::<u8>(), any::<u8>()), 0..128),
        start in any::<u8>(),
    ) {
        let graph = graph(node_count, &edges);
        let start = node_index(start, node_count);
        let mut workspace = TraversalWorkspace::new();
        prop_assert_eq!(
            bfs_iter(&graph, start, &mut workspace).collect::<Vec<_>>(),
            bfs(&graph, start),
        );
    }

    #[test]
    fn generic_and_specialized_dijkstra_agree(
        node_count in 1_usize..24,
        edges in proptest::collection::vec((any::<u8>(), any::<u8>(), 0_u16..1000), 0..96),
        source in any::<u8>(),
        target in any::<u8>(),
    ) {
        let raw = edges.iter().map(|&(source, target, _)| (source, target)).collect::<Vec<_>>();
        let graph = graph(node_count, &raw);
        let weights = edges.iter().map(|edge| edge.2).collect::<Vec<_>>();
        let source = node_index(source, node_count);
        let target = node_index(target, node_count);
        let specialized = dijkstra(&graph, source, target, |edge| u64::from(weights[edge.index()]));
        let generic = dijkstra_measure(&graph, source, target, |edge| weights[edge.index()])
            .unwrap();
        prop_assert_eq!(
            specialized
                .as_ref()
                .map(weavatrix_graph::WeightedPath::total_cost),
            generic.as_ref().map(|path| u64::from(path.total_cost())),
        );
    }

    #[test]
    fn views_and_analytics_preserve_structural_invariants(
        node_count in 1_usize..24,
        edges in proptest::collection::vec((any::<u8>(), any::<u8>()), 0..96),
    ) {
        let graph = graph(node_count, &edges);
        let reverse = reversed(&graph);
        let twice = reversed(&reverse);
        prop_assert_eq!(
            graph.edge_references().collect::<Vec<_>>(),
            twice.edge_references().collect::<Vec<_>>(),
        );

        let filtered = edge_filtered(&graph, |edge| edge.index() % 2 == 0);
        prop_assert!(filtered.edge_count() <= graph.edge_count());
        prop_assert!(k_core_numbers(&graph).iter().all(|(_, core)| *core < node_count));
        prop_assert!(closeness_centrality(&graph, Direction::Both)
            .iter()
            .all(|(_, score)| score.is_finite() && *score >= 0.0));
        prop_assert!(betweenness_centrality(&graph, Direction::Both, true)
            .iter()
            .all(|(_, score)| score.is_finite() && *score >= 0.0));
    }

    #[test]
    fn stable_payload_mutations_preserve_bidirectional_adjacency(
        node_count in 1_usize..24,
        edges in proptest::collection::vec((any::<u8>(), any::<u8>(), any::<bool>()), 0..128),
        removed_node in any::<u8>(),
    ) {
        let mut graph = StablePayloadGraph::new();
        let nodes = (0..node_count)
            .map(|node| graph.add_node(node).unwrap())
            .collect::<Vec<_>>();
        for &(source, target, remove) in &edges {
            let source = nodes[usize::from(source) % node_count];
            let target = nodes[usize::from(target) % node_count];
            let edge = graph.add_edge(source, target, ()).unwrap();
            if remove {
                graph.remove_edge(edge);
            }
        }
        graph.remove_node(nodes[usize::from(removed_node) % node_count]);
        prop_assert_eq!(graph.node_count(), graph.node_indices().count());
        prop_assert_eq!(graph.edge_count(), graph.edge_indices().count());
        for node in graph.node_indices() {
            for edge in graph.outgoing_edges(node) {
                prop_assert_eq!(graph.edge_endpoints(edge).unwrap().source(), node);
            }
            for edge in graph.incoming_edges(node) {
                prop_assert_eq!(graph.edge_endpoints(edge).unwrap().target(), node);
            }
        }
    }
}
