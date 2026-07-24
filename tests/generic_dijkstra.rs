use weavatrix_graph::{
    DijkstraWorkspace, EdgeEndpoints, NodeIndex, Topology, bellman_ford_measure, dijkstra_iter,
    dijkstra_measure, dijkstra_measure_filtered,
};

fn topology() -> Topology {
    Topology::try_from_edges(
        4,
        [(0, 1), (0, 2), (2, 1), (1, 3), (2, 3)].map(|(source, target)| {
            EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
        }),
    )
    .unwrap()
}

#[test]
fn generic_bellman_ford_supports_signed_integer_and_float_measures() {
    let graph = Topology::try_from_edges(
        4,
        [(0, 1), (0, 2), (2, 1), (1, 3)].map(|(source, target)| {
            EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
        }),
    )
    .unwrap();
    let signed = bellman_ford_measure(&graph, NodeIndex::new(0), |edge| {
        [4_i32, 5, -3, 2][edge.index()]
    })
    .unwrap()
    .unwrap();
    assert_eq!(signed.distance_to(NodeIndex::new(3)), Some(4));

    let float = bellman_ford_measure(&graph, NodeIndex::new(0), |edge| {
        [4.0_f64, 5.0, -3.5, 2.0][edge.index()]
    })
    .unwrap()
    .unwrap();
    assert_eq!(float.distance_to(NodeIndex::new(3)), Some(3.5));
}

#[test]
fn generic_dijkstra_supports_integer_and_float_measures() {
    let graph = topology();
    let integer = dijkstra_measure(&graph, NodeIndex::new(0), NodeIndex::new(3), |edge| {
        [10_u32, 2, 1, 2, 20][edge.index()]
    })
    .unwrap()
    .unwrap();
    assert_eq!(integer.total_cost(), 5_u32);

    let float = dijkstra_measure(&graph, NodeIndex::new(0), NodeIndex::new(3), |edge| {
        [10.0_f64, 2.5, 1.0, 2.0, 20.0][edge.index()]
    })
    .unwrap()
    .unwrap();
    assert!((float.total_cost() - 5.5).abs() < f64::EPSILON);
}

#[test]
fn lazy_dijkstra_reuses_storage_and_exposes_predecessors() {
    let graph = topology();
    let mut workspace = DijkstraWorkspace::new();
    let settled = dijkstra_iter(&graph, NodeIndex::new(0), &mut workspace, |edge| {
        [10_u16, 2, 1, 2, 20][edge.index()]
    })
    .take(3)
    .collect::<Result<Vec<_>, _>>()
    .unwrap();
    assert_eq!(
        settled,
        [
            (NodeIndex::new(0), 0),
            (NodeIndex::new(2), 2),
            (NodeIndex::new(1), 3)
        ]
    );
    assert_eq!(
        workspace.predecessor_at(NodeIndex::new(1).index()),
        Some(NodeIndex::new(2))
    );
}

#[test]
fn generic_dijkstra_rejects_negative_non_finite_and_overflowing_weights() {
    let graph = topology();
    for result in [
        dijkstra_measure(&graph, NodeIndex::new(0), NodeIndex::new(3), |edge| {
            if edge.index() == 0 { -1_i32 } else { 1 }
        })
        .map(|_| ()),
        dijkstra_measure(&graph, NodeIndex::new(0), NodeIndex::new(3), |edge| {
            if edge.index() == 0 { f64::NAN } else { 1.0 }
        })
        .map(|_| ()),
    ] {
        assert!(result.is_err());
    }

    let overflow = Topology::try_from_edges(
        3,
        [(0, 1), (1, 2)].map(|(source, target)| {
            EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
        }),
    )
    .unwrap();
    assert!(
        dijkstra_measure(&overflow, NodeIndex::new(0), NodeIndex::new(2), |edge| [
            u8::MAX,
            1
        ][edge.index()])
        .is_err()
    );

    let filtered = dijkstra_measure_filtered(
        &graph,
        NodeIndex::new(0),
        NodeIndex::new(3),
        weavatrix_graph::Direction::Outgoing,
        |edge| (edge.index() != 2).then_some(1_u8),
    )
    .unwrap()
    .unwrap();
    assert_eq!(filtered.total_cost(), 2);
}
