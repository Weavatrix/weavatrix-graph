use weavatrix_graph::{
    GraphError, KeyedPayloadGraph, StableUndirectedPayloadGraph, UndirectedGraphView,
    biconnected_components, minimum_spanning_forest,
};

#[test]
fn keyed_graph_upserts_without_changing_the_stable_handle() {
    let mut graph = KeyedPayloadGraph::<&str, &str, u32>::new();
    let (alpha, previous) = graph.insert_node("alpha", "first").unwrap();
    assert_eq!(previous, None);

    let (same, previous) = graph.insert_node("alpha", "updated").unwrap();
    assert_eq!(same, alpha);
    assert_eq!(previous, Some("first"));
    assert_eq!(graph.node(&"alpha"), Some(&"updated"));

    let (beta, _) = graph.insert_node("beta", "second").unwrap();
    let edge = graph.add_edge(&"alpha", &"beta", 7).unwrap();
    assert_eq!(graph.graph().edge(edge), Some(&7));
    assert_eq!(graph.graph().edge_count(), 1);
    assert_ne!(alpha, beta);
}

#[test]
fn keyed_graph_reports_missing_endpoints_and_invalidates_removed_keys() {
    let mut graph = KeyedPayloadGraph::<String, (), ()>::new();
    graph.insert_node("live".into(), ()).unwrap();

    assert_eq!(
        graph.add_edge(&"missing".into(), &"live".into(), ()),
        Err(GraphError::MissingKeyedNode { endpoint: "source" })
    );
    assert_eq!(
        graph.add_edge(&"live".into(), &"missing".into(), ()),
        Err(GraphError::MissingKeyedNode { endpoint: "target" })
    );
    assert_eq!(graph.remove_node(&"live".into()), Some(()));
    assert!(graph.node_key(&"live".into()).is_none());
    assert_eq!(graph.node_count(), 0);
}

#[test]
fn stable_undirected_graph_handles_parallel_edges_loops_and_reuse() {
    let mut graph = StableUndirectedPayloadGraph::new();
    let alpha = graph.add_node("alpha").unwrap();
    let beta = graph.add_node("beta").unwrap();
    let first = graph.add_edge(alpha, beta, 3_u64).unwrap();
    let second = graph.add_edge(alpha, beta, 5).unwrap();
    let loop_edge = graph.add_edge(alpha, alpha, 7).unwrap();

    assert_eq!(graph.incident_edges(alpha).count(), 3);
    assert_eq!(graph.incident_edges(beta).count(), 2);
    assert_eq!(graph.edge_endpoints(loop_edge).unwrap().source(), alpha);

    assert_eq!(graph.remove_edge(first), Some(3));
    let replacement = graph.add_edge(alpha, beta, 11).unwrap();
    assert_eq!(replacement.slot(), first.slot());
    assert_ne!(replacement.generation(), first.generation());
    assert!(graph.edge(first).is_none());

    assert_eq!(graph.remove_node(beta), Some("beta"));
    assert!(graph.edge(second).is_none());
    assert!(graph.edge(replacement).is_none());
    assert_eq!(graph.edge_count(), 1);
}

#[test]
fn stable_undirected_graph_supports_algorithms_and_retargeting() {
    let mut graph = StableUndirectedPayloadGraph::new();
    let a = graph.add_node('a').unwrap();
    let b = graph.add_node('b').unwrap();
    let c = graph.add_node('c').unwrap();
    let d = graph.add_node('d').unwrap();
    let ab = graph.add_edge(a, b, 1_u64).unwrap();
    let bc = graph.add_edge(b, c, 2).unwrap();
    let ca = graph.add_edge(c, a, 3).unwrap();
    let bridge = graph.add_edge(c, d, 4).unwrap();

    let blocks = biconnected_components(&graph);
    assert_eq!(blocks.component_count(), 2);
    assert_eq!(blocks.articulation_points(), &[c]);

    let forest = minimum_spanning_forest(&graph, |edge| *graph.edge(edge).unwrap());
    assert_eq!(forest.edges(), &[ab, bc, bridge]);
    assert_eq!(forest.total_weight(), 7);

    graph.set_edge_endpoints(bridge, b, d).unwrap();
    assert_eq!(graph.edge_endpoints(bridge).unwrap().source(), b);
    assert_eq!(graph.incident_edges(c).count(), 2);
    assert_eq!(graph.incident_edges(b).count(), 3);
    assert!(graph.contains_edge(ca));
}

#[test]
fn stable_undirected_freeze_compacts_only_live_slots() {
    let mut graph = StableUndirectedPayloadGraph::new();
    let removed = graph.add_node("removed").unwrap();
    let alpha = graph.add_node("alpha").unwrap();
    let beta = graph.add_node("beta").unwrap();
    let dead_edge = graph.add_edge(removed, alpha, "dead").unwrap();
    graph.remove_node(removed);
    let live_edge = graph.add_edge(alpha, beta, "live").unwrap();

    let frozen = graph.freeze().unwrap();
    assert_eq!(frozen.graph().nodes(), &["alpha", "beta"]);
    assert_eq!(frozen.graph().edges(), &["live"]);
    assert!(frozen.indices().node(removed).is_none());
    assert!(frozen.indices().edge(dead_edge).is_none());
    assert_eq!(frozen.indices().node(alpha).unwrap().index(), 0);
    assert_eq!(frozen.indices().node(beta).unwrap().index(), 1);
    assert_eq!(frozen.indices().edge(live_edge).unwrap().index(), 0);
}
