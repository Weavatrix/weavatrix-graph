use weavatrix_graph::{
    DfsEvent, DfsEventWorkspace, Direction, EdgeEndpoints, NodeIndex, Topology, TraversalControl,
    TraversalWorkspace, bfs_iter, bfs_iter_filtered, depth_first_search, dfs_iter,
};

fn topology() -> Topology {
    Topology::try_from_edges(
        6,
        [(0, 1), (0, 2), (1, 3), (2, 4), (3, 5), (4, 5)].map(|(source, target)| {
            EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
        }),
    )
    .unwrap()
}

#[test]
fn lazy_traversals_preserve_order_and_support_early_stop() {
    let graph = topology();
    let mut workspace = TraversalWorkspace::new();
    let first_three = bfs_iter(&graph, NodeIndex::new(0), &mut workspace)
        .take(3)
        .collect::<Vec<_>>();
    assert_eq!(first_three, [0, 1, 2].map(NodeIndex::new));

    let depth = dfs_iter(&graph, NodeIndex::new(0), &mut workspace).collect::<Vec<_>>();
    assert_eq!(depth, [0, 1, 3, 5, 2, 4].map(NodeIndex::new));
}

#[test]
fn workspaces_are_reusable_and_filters_remain_lazy() {
    let graph = topology();
    let mut workspace = TraversalWorkspace::new();
    let even_edges = bfs_iter_filtered(
        &graph,
        NodeIndex::new(0),
        Direction::Outgoing,
        &mut workspace,
        |edge| edge.index() % 2 == 0,
    )
    .collect::<Vec<_>>();
    assert_eq!(even_edges, [0, 1, 3, 5].map(NodeIndex::new));

    let reverse = bfs_iter_filtered(
        &graph,
        NodeIndex::new(5),
        Direction::Incoming,
        &mut workspace,
        |_| true,
    )
    .collect::<Vec<_>>();
    assert_eq!(reverse, [5, 3, 4, 1, 2, 0].map(NodeIndex::new));
}

#[test]
fn dfs_events_classify_edges_and_can_stop_without_finishing() {
    let graph = Topology::try_from_edges(
        4,
        [(0, 1), (1, 2), (2, 0), (1, 3)].map(|(source, target)| {
            EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
        }),
    )
    .unwrap();
    let mut workspace = DfsEventWorkspace::new();
    let mut events = Vec::new();
    assert!(depth_first_search(
        &graph,
        [NodeIndex::new(0)],
        &mut workspace,
        |event| {
            events.push(event);
            TraversalControl::Continue
        }
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        DfsEvent::BackEdge {
            source,
            target,
            ..
        } if *source == NodeIndex::new(2) && *target == NodeIndex::new(0)
    )));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DfsEvent::Discover(_)))
            .count(),
        4
    );

    let completed =
        depth_first_search(
            &graph,
            [NodeIndex::new(0)],
            &mut workspace,
            |event| match event {
                DfsEvent::Discover(node) if node == NodeIndex::new(2) => TraversalControl::Break,
                _ => TraversalControl::Continue,
            },
        );
    assert!(!completed);
}
