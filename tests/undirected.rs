use petgraph::algo::min_spanning_tree;
use petgraph::data::Element;
use petgraph::graph::UnGraph;
use weavatrix_graph::{
    EdgeEndpoints, EdgeIndex, IndexUndirectedGraphView, NodeIndex, UndirectedTopology,
    minimum_spanning_forest, prim_spanning_forest,
};

fn endpoints(source: u32, target: u32) -> EdgeEndpoints {
    EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
}

#[test]
fn compact_incidence_supports_neighbors_parallel_edges_and_self_loops() {
    let graph = UndirectedTopology::try_from_edges(
        4,
        [
            endpoints(0, 1),
            endpoints(1, 0),
            endpoints(1, 1),
            endpoints(1, 2),
        ],
    )
    .unwrap();

    assert_eq!(graph.node_count(), 4);
    assert_eq!(graph.edge_count(), 4);
    assert_eq!(graph.degree(NodeIndex::new(0)), Some(2));
    assert_eq!(graph.degree(NodeIndex::new(1)), Some(5));
    assert_eq!(
        graph.neighbors(NodeIndex::new(1)).collect::<Vec<_>>(),
        [0, 0, 1, 2].map(NodeIndex::new)
    );
    assert_eq!(graph.degree(NodeIndex::new(3)), Some(0));
}

#[test]
fn undirected_wire_format_rebuilds_derived_csr() {
    let graph = UndirectedTopology::try_from_edges(3, [endpoints(0, 1), endpoints(1, 2)]).unwrap();
    let json = serde_json::to_string(&graph).unwrap();
    assert!(!json.contains("incidence"));
    assert_eq!(
        serde_json::from_str::<UndirectedTopology>(&json).unwrap(),
        graph
    );
}

#[test]
fn kruskal_returns_a_forest_for_disconnected_graphs() {
    let graph = UndirectedTopology::try_from_edges(
        6,
        [
            endpoints(0, 1),
            endpoints(0, 2),
            endpoints(1, 2),
            endpoints(3, 4),
        ],
    )
    .unwrap();
    let weights = [4_u64, 1, 2, 7];
    let forest = minimum_spanning_forest(&graph, |edge| weights[edge.index()]);
    assert_eq!(forest.total_weight(), 10);
    assert_eq!(forest.edges().len(), 3);
    assert_eq!(forest.component_count(), 3);

    let wide = UndirectedTopology::try_from_edges(3, [endpoints(0, 1), endpoints(1, 2)]).unwrap();
    let wide = minimum_spanning_forest(&wide, |_| u64::MAX);
    assert_eq!(wide.total_weight(), u128::from(u64::MAX) * 2);
}

#[test]
fn mst_weight_matches_petgraph_on_random_weighted_graphs() {
    let mut seed = 0x243f_6a88_u64;
    for node_count in [1_usize, 2, 9, 31] {
        let mut pairs = Vec::new();
        let mut weights = Vec::new();
        for source in 0..node_count {
            for target in source..node_count {
                seed = seed
                    .wrapping_mul(2_862_933_555_777_941_757)
                    .wrapping_add(3_037_000_493);
                if seed % 11 < 3 {
                    pairs.push((source, target));
                    weights.push(seed % 97 + 1);
                }
            }
        }
        compare_mst(node_count, &pairs, &weights);
    }
}

#[test]
fn undirected_index_view_exposes_every_compact_operation() {
    fn inspect<G>(graph: &G, node: G::Node, edge: G::Edge)
    where
        G: IndexUndirectedGraphView,
    {
        assert_eq!(graph.node_count(), graph.node_indices().count());
        assert_eq!(graph.edge_count(), graph.edge_indices().count());
        assert!(graph.contains_node(node));
        assert!(graph.contains_edge(edge));
        assert!(graph.edge_endpoints(edge).is_some());
        assert_eq!(graph.incident_edges(node).count(), 1);
        assert!(graph.opposite(edge, node).is_some());
        assert!(G::node_slot(node) < graph.node_bound());
        assert!(G::edge_slot(edge) < graph.edge_bound());
    }

    let graph = UndirectedTopology::try_from_edges(2, [endpoints(0, 1)]).unwrap();
    inspect(&graph, NodeIndex::new(0), EdgeIndex::new(0));
    assert!(!graph.contains_node(NodeIndex::new(2)));
    assert!(!graph.contains_edge(EdgeIndex::new(1)));
}

fn compare_mst(node_count: usize, pairs: &[(usize, usize)], weights: &[u64]) {
    let ours = UndirectedTopology::try_from_edges(
        node_count,
        pairs.iter().map(|&(source, target)| {
            endpoints(
                u32::try_from(source).unwrap(),
                u32::try_from(target).unwrap(),
            )
        }),
    )
    .unwrap();
    let ours = minimum_spanning_forest(&ours, |edge| weights[edge.index()]);
    let prim = prim_spanning_forest(
        &UndirectedTopology::try_from_edges(
            node_count,
            pairs.iter().map(|&(source, target)| {
                endpoints(
                    u32::try_from(source).unwrap(),
                    u32::try_from(target).unwrap(),
                )
            }),
        )
        .unwrap(),
        |edge| weights[edge.index()],
    );
    let ours_weight = ours.total_weight();
    let ours_edges = ours.clone().into_edges();
    assert_eq!(
        ours_edges.len(),
        node_count.saturating_sub(ours.component_count())
    );

    let mut pet = UnGraph::<(), u64>::with_capacity(node_count, pairs.len());
    let nodes = (0..node_count)
        .map(|_| pet.add_node(()))
        .collect::<Vec<_>>();
    for (&(source, target), &weight) in pairs.iter().zip(weights) {
        pet.add_edge(nodes[source], nodes[target], weight);
    }
    let expected = min_spanning_tree(&pet)
        .filter_map(|element| match element {
            Element::Edge { weight, .. } => Some(weight),
            Element::Node { .. } => None,
        })
        .sum::<u64>();
    assert_eq!(ours_weight, u128::from(expected));
    assert_eq!(prim.total_weight(), ours_weight);
    assert_eq!(prim.component_count(), ours.component_count());
}
