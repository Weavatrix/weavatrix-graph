use weavatrix_graph::{
    EdgeEndpoints, EdgeIndex, GraphView, IndexGraphView, IndexUndirectedGraphView, NodeIndex,
    PayloadGraph, UndirectedGraphView, UndirectedPayloadGraph, dijkstra, minimum_spanning_forest,
};

#[test]
fn directed_payload_graph_uses_the_shared_algorithm_contract() {
    let mut graph = PayloadGraph::try_from_edges(
        vec!["repo", "file", "symbol"],
        [
            (endpoints(0, 1), 2_u64),
            (endpoints(1, 2), 3),
            (endpoints(0, 2), 10),
        ],
    )
    .unwrap();
    let path = dijkstra(&graph, node(0), node(2), |edge| *graph.edge(edge).unwrap()).unwrap();
    assert_eq!(path.total_cost(), 5);
    assert_eq!(graph.node(node(1)), Some(&"file"));
    *graph.node_mut(node(1)).unwrap() = "renamed";
    *graph.edge_mut(EdgeIndex::new(2)).unwrap() = 1;
    assert_eq!(graph.node(node(1)), Some(&"renamed"));
    assert_eq!(graph.edge_count(), 3);
}

#[test]
fn undirected_payload_graph_drives_mst_and_round_trips() {
    let mut graph = UndirectedPayloadGraph::try_from_edges(
        vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
        [
            (endpoints(0, 1), 4_u64),
            (endpoints(1, 2), 2),
            (endpoints(0, 2), 9),
        ],
    )
    .unwrap();
    let forest = minimum_spanning_forest(&graph, |edge| *graph.edge(edge).unwrap());
    assert_eq!(forest.total_weight(), 6);
    *graph.node_mut(node(1)).unwrap() = "renamed".to_owned();
    *graph.edge_mut(EdgeIndex::new(2)).unwrap() = 1;
    assert_eq!(graph.node(node(1)).map(String::as_str), Some("renamed"));
    assert_eq!(*graph.edge(EdgeIndex::new(2)).unwrap(), 1);
    let json = serde_json::to_string(&graph).unwrap();
    assert_eq!(
        serde_json::from_str::<UndirectedPayloadGraph<String, u64>>(&json).unwrap(),
        graph
    );
}

#[test]
fn payload_deserialization_cannot_bypass_count_validation() {
    let graph = PayloadGraph::try_from_edges(vec!["a", "b"], [(endpoints(0, 1), "edge")]).unwrap();
    let mut value = serde_json::to_value(graph).unwrap();
    value["nodes"] = serde_json::json!(["a"]);
    assert!(serde_json::from_value::<PayloadGraph<String, String>>(value).is_err());
}

#[test]
fn payload_parts_are_lossless_and_reject_mismatched_counts() {
    let graph = PayloadGraph::try_from_edges(vec![10, 20], [(endpoints(0, 1), 30)]).unwrap();
    let (topology, nodes, edges) = graph.into_parts();
    assert_eq!(nodes, vec![10, 20]);
    assert_eq!(edges, vec![30]);
    assert!(PayloadGraph::try_from_parts(topology.clone(), vec![10], vec![30]).is_err());
    assert!(PayloadGraph::try_from_parts(topology, vec![10, 20], Vec::<i32>::new()).is_err());
}

#[test]
fn directed_payload_exposes_the_complete_index_view() {
    let graph = PayloadGraph::try_from_edges(vec!["a", "b"], [(endpoints(0, 1), "calls")]).unwrap();
    assert_eq!(graph.topology().node_count(), 2);
    assert_eq!(graph.nodes(), ["a", "b"]);
    assert_eq!(graph.edges(), ["calls"]);
    assert!(graph.contains_node(node(1)));
    assert!(graph.contains_edge(EdgeIndex::new(0)));
    assert_eq!(graph.node_indices().collect::<Vec<_>>(), [node(0), node(1)]);
    assert_eq!(
        graph.edge_indices().collect::<Vec<_>>(),
        [EdgeIndex::new(0)]
    );
    assert_eq!(
        graph.edge_endpoints(EdgeIndex::new(0)),
        Some(endpoints(0, 1))
    );
    assert_eq!(graph.outgoing_edges(node(0)).count(), 1);
    assert_eq!(graph.incoming_edges(node(1)).count(), 1);
    assert_eq!(graph.node_bound(), 2);
    assert_eq!(graph.edge_bound(), 1);
    assert_eq!(PayloadGraph::<&str, &str>::node_slot(node(1)), 1);
    assert_eq!(PayloadGraph::<&str, &str>::edge_slot(EdgeIndex::new(0)), 0);
    assert!(graph.node(node(9)).is_none());
    assert!(graph.edge(EdgeIndex::new(9)).is_none());
}

#[test]
fn undirected_payload_exposes_parts_and_the_complete_index_view() {
    let graph = UndirectedPayloadGraph::try_from_edges(vec!["a", "b"], [(endpoints(0, 1), "link")])
        .unwrap();
    assert_eq!(graph.topology().node_count(), 2);
    assert_eq!(graph.nodes(), ["a", "b"]);
    assert_eq!(graph.edges(), ["link"]);
    assert!(graph.contains_node(node(1)));
    assert!(graph.contains_edge(EdgeIndex::new(0)));
    assert_eq!(graph.node_indices().count(), 2);
    assert_eq!(graph.edge_indices().count(), 1);
    assert_eq!(
        graph.edge_endpoints(EdgeIndex::new(0)),
        Some(endpoints(0, 1))
    );
    assert_eq!(graph.incident_edges(node(0)).len(), 1);
    assert_eq!(graph.node_bound(), 2);
    assert_eq!(graph.edge_bound(), 1);
    assert_eq!(UndirectedPayloadGraph::<&str, &str>::node_slot(node(1)), 1);
    assert_eq!(
        UndirectedPayloadGraph::<&str, &str>::edge_slot(EdgeIndex::new(0)),
        0
    );
    let (topology, nodes, edges) = graph.into_parts();
    assert_eq!(nodes, ["a", "b"]);
    assert_eq!(edges, ["link"]);
    assert!(
        UndirectedPayloadGraph::try_from_parts(topology.clone(), vec!["a"], vec!["link"]).is_err()
    );
    assert!(
        UndirectedPayloadGraph::try_from_parts(topology, vec!["a", "b"], Vec::<&str>::new())
            .is_err()
    );
}

fn endpoints(source: u32, target: u32) -> EdgeEndpoints {
    EdgeEndpoints::new(node(source), node(target))
}

fn node(index: u32) -> NodeIndex {
    NodeIndex::new(index)
}
