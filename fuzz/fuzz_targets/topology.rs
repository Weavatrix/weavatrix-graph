#![no_main]

use libfuzzer_sys::fuzz_target;
use weavatrix_graph::{
    BitMatrix, EdgeEndpoints, GraphView, NodeIndex, StablePayloadGraph, Topology, bfs,
    strongly_connected_components,
};

fuzz_target!(|data: &[u8]| {
    let Some((&head, bytes)) = data.split_first() else {
        return;
    };
    let node_count = usize::from(head % 64) + 1;
    let edges = bytes.chunks_exact(2).map(|pair| {
        EdgeEndpoints::new(
            NodeIndex::new(u32::from(pair[0]) % node_count as u32),
            NodeIndex::new(u32::from(pair[1]) % node_count as u32),
        )
    });
    let Ok(graph) = Topology::try_from_edges(node_count, edges) else {
        return;
    };
    let _ = bfs(&graph, NodeIndex::new(0));
    let _ = strongly_connected_components(&graph);
    let Ok(mut matrix) = BitMatrix::try_new(node_count) else {
        return;
    };
    for (_, endpoints) in graph.edge_references() {
        let _ = matrix.insert(endpoints.source(), endpoints.target());
    }
    for node in 0..node_count {
        let index = NodeIndex::new(node as u32);
        let _ = matrix.contains(index, index);
        let _ = matrix.contains_fast(index, index);
    }

    let mut stable = StablePayloadGraph::new();
    let nodes = (0..node_count)
        .filter_map(|node| stable.add_node(node).ok())
        .collect::<Vec<_>>();
    let mut stable_edges = Vec::new();
    for pair in bytes.chunks_exact(2) {
        let source = usize::from(pair[0]) % nodes.len();
        let target = usize::from(pair[1]) % nodes.len();
        if let Ok(edge) = stable.add_edge(nodes[source], nodes[target], ()) {
            stable_edges.push(edge);
        }
    }
    for (index, edge) in stable_edges.into_iter().enumerate() {
        if bytes.get(index).is_some_and(|byte| byte & 1 == 1) {
            stable.remove_edge(edge);
        }
    }
    for node in stable.node_indices() {
        for edge in stable.outgoing_edges(node) {
            assert_eq!(stable.edge_endpoints(edge).unwrap().source(), node);
        }
    }
});
