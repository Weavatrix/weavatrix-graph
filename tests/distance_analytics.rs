use std::cell::Cell;
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, UndirectedTopology, center, diameter, distance_analytics,
    distance_analytics_filtered, eccentricity, periphery, radius,
};

fn graph(node_count: usize, edges: &[(usize, usize)]) -> UndirectedTopology {
    UndirectedTopology::try_from_edges(
        node_count,
        edges
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node(source), node(target))),
    )
    .unwrap()
}

fn node(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).unwrap())
}

fn values(result: &weavatrix_graph::DistanceAnalytics<NodeIndex>) -> Vec<usize> {
    result.eccentricities().iter().map(|pair| pair.1).collect()
}

#[test]
fn path_metrics_are_exact_and_canonical() {
    let topology = graph(5, &[(0, 1), (1, 2), (2, 3), (3, 4)]);
    let result = distance_analytics(&topology).unwrap();
    assert_eq!(values(&result), [4, 3, 2, 3, 4]);
    assert_eq!(result.radius(), 2);
    assert_eq!(result.diameter(), 4);
    assert_eq!(result.center(), &[node(2)]);
    assert_eq!(result.periphery(), &[node(0), node(4)]);
    assert_eq!(result.eccentricity(node(3)), Some(3));
    assert_eq!(eccentricity(&topology, node(3)), Some(3));
    assert_eq!(radius(&topology), Some(2));
    assert_eq!(diameter(&topology), Some(4));
    assert_eq!(center(&topology), Some(vec![node(2)]));
    assert_eq!(periphery(&topology), Some(vec![node(0), node(4)]));
}

#[test]
fn cycle_and_singleton_cover_metric_ties_and_zero() {
    let cycle = graph(4, &[(0, 1), (1, 2), (2, 3), (3, 0)]);
    let result = distance_analytics(&cycle).unwrap();
    assert_eq!(values(&result), [2, 2, 2, 2]);
    assert_eq!(result.center(), &[node(0), node(1), node(2), node(3)]);
    assert_eq!(result.periphery(), result.center());

    let singleton = graph(1, &[]);
    let result = distance_analytics(&singleton).unwrap();
    assert_eq!(values(&result), [0]);
    assert_eq!(result.radius(), 0);
    assert_eq!(result.diameter(), 0);
}

#[test]
fn empty_disconnected_and_unknown_nodes_return_none() {
    assert!(distance_analytics(&graph(0, &[])).is_none());
    let disconnected = graph(3, &[(0, 1)]);
    assert!(distance_analytics(&disconnected).is_none());
    assert_eq!(eccentricity(&disconnected, node(0)), None);
    assert_eq!(eccentricity(&disconnected, node(9)), None);
}

#[test]
fn filtering_is_single_pass_and_can_disconnect_the_graph() {
    let topology = graph(4, &[(0, 1), (1, 2), (2, 3), (0, 3)]);
    let calls = Cell::new(0);
    let result = distance_analytics_filtered(&topology, |edge| {
        calls.set(calls.get() + 1);
        edge.index() != 1
    })
    .unwrap();
    assert_eq!(calls.get(), 4);
    assert_eq!(result.diameter(), 3);

    let calls = Cell::new(0);
    assert!(
        distance_analytics_filtered(&topology, |edge| {
            calls.set(calls.get() + 1);
            edge.index() < 2
        })
        .is_none()
    );
    assert_eq!(calls.get(), 4);
}

#[test]
fn seeded_connected_graphs_match_floyd_reference() {
    for seed in 1..=64_u64 {
        let node_count = 2 + usize::try_from(seed % 8).unwrap();
        let mut state = seed;
        let mut edges = (0..node_count - 1)
            .map(|index| (index, index + 1))
            .collect::<Vec<_>>();
        for source in 0..node_count {
            for target in source + 1..node_count {
                state = next(state);
                if state % 5 == 0 && !edges.contains(&(source, target)) {
                    edges.push((source, target));
                }
            }
        }
        let expected = reference(node_count, &edges);
        let actual = distance_analytics(&graph(node_count, &edges)).unwrap();
        assert_eq!(values(&actual), expected, "seed={seed}");
        assert_eq!(actual.radius(), *expected.iter().min().unwrap());
        assert_eq!(actual.diameter(), *expected.iter().max().unwrap());
    }
}

fn reference(node_count: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let infinity = usize::MAX / 2;
    let mut distances = vec![vec![infinity; node_count]; node_count];
    for (node, row) in distances.iter_mut().enumerate() {
        row[node] = 0;
    }
    for &(source, target) in edges {
        distances[source][target] = 1;
        distances[target][source] = 1;
    }
    for intermediate in 0..node_count {
        for source in 0..node_count {
            for target in 0..node_count {
                distances[source][target] = distances[source][target]
                    .min(distances[source][intermediate] + distances[intermediate][target]);
            }
        }
    }
    distances
        .into_iter()
        .map(|row| row.into_iter().max().unwrap())
        .collect()
}

fn next(state: u64) -> u64 {
    state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1)
}
