use weavatrix_graph::{
    AcyclicPayloadGraph, GraphError, GraphView, StablePayloadGraph, bfs, topological_sort,
};

#[test]
fn stable_payload_keys_survive_unrelated_mutations_and_detect_reuse() {
    let mut graph = StablePayloadGraph::new();
    let alpha = graph.add_node("alpha").unwrap();
    let beta = graph.add_node("beta").unwrap();
    let edge = graph.add_edge(alpha, beta, 7_u64).unwrap();

    assert_eq!(graph.remove_node(alpha), Some("alpha"));
    assert!(graph.edge(edge).is_none());
    let replacement = graph.add_node("replacement").unwrap();
    assert_eq!(replacement.slot(), alpha.slot());
    assert_ne!(replacement.generation(), alpha.generation());
    assert!(graph.node(alpha).is_none());
    assert_eq!(graph.node(beta), Some(&"beta"));
}

#[test]
fn stable_payload_graph_supports_algorithms_retargeting_and_compaction() {
    let mut graph = StablePayloadGraph::new();
    let alpha = graph.add_node("alpha").unwrap();
    let beta = graph.add_node("beta").unwrap();
    let gamma = graph.add_node("gamma").unwrap();
    let edge = graph.add_edge(alpha, beta, "calls").unwrap();
    assert_eq!(bfs(&graph, alpha), [alpha, beta]);

    graph.set_edge_endpoints(edge, alpha, gamma).unwrap();
    assert_eq!(bfs(&graph, alpha), [alpha, gamma]);
    let frozen = graph.freeze().unwrap();
    assert_eq!(frozen.graph().node_count(), 3);
    assert_eq!(frozen.graph().edge_count(), 1);
    assert_eq!(frozen.indices().node(alpha).unwrap().index(), 0);
    assert_eq!(frozen.indices().edge(edge).unwrap().index(), 0);
}

#[test]
fn payload_mapping_preserves_keys_and_holes() {
    let mut graph = StablePayloadGraph::new();
    let removed = graph.add_node(1_u32).unwrap();
    let live = graph.add_node(2).unwrap();
    graph.remove_node(removed);
    let mapped = graph.map_payloads(|node| node.to_string(), |edge: u64| edge + 1);
    assert_eq!(mapped.node(live).map(String::as_str), Some("2"));
    assert!(mapped.node(removed).is_none());
}

#[test]
fn acyclic_graph_rejects_cycles_and_ignores_the_retargeted_edge() {
    let mut graph = AcyclicPayloadGraph::new();
    let alpha = graph.add_node("alpha").unwrap();
    let beta = graph.add_node("beta").unwrap();
    let gamma = graph.add_node("gamma").unwrap();
    let edge = graph.add_edge(alpha, beta, ()).unwrap();
    graph.add_edge(beta, gamma, ()).unwrap();

    assert_eq!(
        graph.add_edge(gamma, alpha, ()),
        Err(GraphError::CycleWouldBeCreated)
    );
    assert!(graph.set_edge_endpoints(edge, alpha, gamma).unwrap());
    assert_eq!(topological_sort(&graph).unwrap().len(), 3);
}

#[test]
fn stale_endpoints_are_rejected_without_mutation() {
    let mut graph = StablePayloadGraph::<_, ()>::new();
    let removed = graph.add_node("removed").unwrap();
    let live = graph.add_node("live").unwrap();
    graph.remove_node(removed);
    assert_eq!(
        graph.add_edge(removed, live, ()),
        Err(GraphError::InvalidStableKey {
            category: "node",
            slot: removed.slot(),
            generation: removed.generation(),
        })
    );
    assert_eq!(graph.edge_count(), 0);
}
