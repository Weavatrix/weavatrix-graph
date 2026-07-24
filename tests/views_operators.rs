use std::cell::Cell;
use weavatrix_graph::{
    EdgeEndpoints, GraphView, NodeIndex, Topology, bfs, complement, edge_filtered,
    induced_subgraph_view, reversed, strongly_connected_components, union,
};

fn topology(node_count: usize, edges: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges(
        node_count,
        edges.iter().map(|&(source, target)| {
            EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
        }),
    )
    .unwrap()
}

#[test]
fn lazy_views_drive_existing_algorithms_without_rebuilding() {
    let graph = topology(5, &[(0, 1), (1, 2), (2, 0), (2, 3), (3, 4)]);
    let reverse = reversed(&graph);
    assert_eq!(
        bfs(&reverse, NodeIndex::new(4)),
        [4, 3, 2, 1, 0].map(NodeIndex::new)
    );

    let filtered = edge_filtered(&graph, |edge| edge.index() != 2);
    assert_eq!(filtered.edge_count(), 4);
    assert_eq!(
        strongly_connected_components(&filtered).len(),
        graph.node_count()
    );

    let induced = induced_subgraph_view(&graph, |node| node.index() <= 2);
    assert_eq!(induced.node_count(), 3);
    assert_eq!(induced.edge_count(), 3);
    assert_eq!(induced.outgoing_edges(NodeIndex::new(2)).count(), 1);
}

#[test]
fn complement_and_union_preserve_node_identity_maps() {
    let left = topology(3, &[(0, 1), (1, 2)]);
    let right = topology(3, &[(0, 1), (2, 0)]);
    let merged = union(&left, &right).unwrap();
    assert_eq!(merged.topology().edge_count(), 3);
    assert_eq!(
        merged.original_node(NodeIndex::new(2)),
        Some(&NodeIndex::new(2))
    );

    let inverse = complement(&left, false).unwrap();
    let edges = inverse
        .topology()
        .edge_references()
        .map(|(_, endpoints)| (endpoints.source().index(), endpoints.target().index()))
        .collect::<Vec<_>>();
    assert_eq!(edges, [(0, 2), (1, 0), (2, 0), (2, 1)]);
}

#[test]
fn filtered_adjacency_does_no_work_until_the_iterator_advances() {
    let graph = topology(4, &[(0, 1), (0, 2), (0, 3)]);
    let calls = Cell::new(0);
    let filtered = edge_filtered(&graph, |_| {
        calls.set(calls.get() + 1);
        true
    });
    let mut outgoing = filtered.outgoing_edges(NodeIndex::new(0));
    assert_eq!(calls.get(), 0);
    assert!(outgoing.next().is_some());
    assert_eq!(calls.get(), 1);
}
