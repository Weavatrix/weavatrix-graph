#![allow(clippy::cast_precision_loss)]

use std::cell::Cell;
use weavatrix_graph::{EdgeEndpoints, GraphError, NodeIndex, Topology, hits, hits_filtered};

fn graph(node_count: usize, edges: &[(usize, usize)]) -> Topology {
    Topology::try_from_edges(
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

#[test]
fn ranks_outgoing_star_center_as_hub_and_leaves_as_authorities() {
    let result = hits(&graph(4, &[(0, 1), (0, 2), (0, 3)]), 100, 1e-12).unwrap();
    assert!(result.converged());
    assert!(result.hub(node(0)).unwrap() > 0.999);
    assert!(result.authority(node(1)).unwrap() > 0.57);
    assert_eq!(result.authority(node(1)), result.authority(node(2)));
    assert_eq!(result.authority(node(2)), result.authority(node(3)));
}

#[test]
fn filtering_is_single_pass_and_duplicate_provenance_is_neutral() {
    let duplicate = graph(3, &[(0, 1), (0, 1), (0, 2), (2, 1)]);
    let canonical = graph(3, &[(0, 1), (0, 2), (2, 1)]);
    let left = hits(&duplicate, 1_000, 1e-12).unwrap();
    let right = hits(&canonical, 1_000, 1e-12).unwrap();
    close_scores(left.hubs(), right.hubs());
    close_scores(left.authorities(), right.authorities());

    let calls = Cell::new(0);
    let filtered = hits_filtered(&duplicate, 1_000, 1e-12, |edge| {
        calls.set(calls.get() + 1);
        edge.index() < 3
    })
    .unwrap();
    assert_eq!(calls.get(), 4);
    assert!(filtered.hub(node(0)).unwrap() > 0.999);
}

#[test]
fn empty_relationships_have_canonical_zero_scores() {
    let result = hits(&graph(3, &[]), 100, 1e-12).unwrap();
    assert!(result.converged());
    assert_eq!(result.iterations(), 0);
    assert!(result.hubs().iter().all(|(_, value)| *value == 0.0));
    assert!(result.authorities().iter().all(|(_, value)| *value == 0.0));
}

#[test]
fn rejects_invalid_iteration_parameters() {
    let topology = graph(2, &[(0, 1)]);
    for error in [
        hits(&topology, 0, 1e-6).unwrap_err(),
        hits(&topology, 10, 0.0).unwrap_err(),
        hits(&topology, 10, f64::NAN).unwrap_err(),
    ] {
        assert!(matches!(
            error,
            GraphError::InvalidAlgorithmParameter {
                algorithm: "hits",
                ..
            }
        ));
    }
}

#[test]
fn seeded_scores_match_dense_matrix_power_iteration() {
    for seed in 1..=32_u64 {
        let node_count = 3 + usize::try_from(seed % 7).unwrap();
        let mut state = seed;
        let mut edges = Vec::new();
        for source in 0..node_count {
            for target in 0..node_count {
                state = next(state);
                if source != target && state % 7 < 2 {
                    edges.push((source, target));
                }
            }
        }
        if edges.is_empty() {
            edges.push((0, 1));
        }
        let actual = hits(&graph(node_count, &edges), 2_000, 1e-13).unwrap();
        let (expected_hubs, expected_authorities) = dense_reference(node_count, &edges);
        for node_index in 0..node_count {
            assert!(
                (actual.hub(node(node_index)).unwrap() - expected_hubs[node_index]).abs() < 1e-9,
                "hub seed={seed} node={node_index}"
            );
            assert!(
                (actual.authority(node(node_index)).unwrap() - expected_authorities[node_index])
                    .abs()
                    < 1e-9,
                "authority seed={seed} node={node_index}"
            );
        }
    }
}

fn dense_reference(node_count: usize, edges: &[(usize, usize)]) -> (Vec<f64>, Vec<f64>) {
    let mut adjacency = vec![vec![false; node_count]; node_count];
    for &(source, target) in edges {
        if source != target {
            adjacency[source][target] = true;
        }
    }
    let initial = 1.0 / (node_count as f64).sqrt();
    let mut hubs = vec![initial; node_count];
    let mut authorities = vec![initial; node_count];
    for _ in 0..2_000 {
        let mut next_authorities = vec![0.0; node_count];
        for target in 0..node_count {
            next_authorities[target] = (0..node_count)
                .filter(|&source| adjacency[source][target])
                .map(|source| hubs[source])
                .sum();
        }
        normalize(&mut next_authorities);
        let mut next_hubs = vec![0.0; node_count];
        for source in 0..node_count {
            next_hubs[source] = (0..node_count)
                .filter(|&target| adjacency[source][target])
                .map(|target| next_authorities[target])
                .sum();
        }
        normalize(&mut next_hubs);
        let converged = hubs
            .iter()
            .zip(&next_hubs)
            .chain(authorities.iter().zip(&next_authorities))
            .all(|(left, right)| (*left - *right).abs() <= 1e-13);
        hubs = next_hubs;
        authorities = next_authorities;
        if converged {
            break;
        }
    }
    (hubs, authorities)
}

fn normalize(values: &mut [f64]) {
    let norm = values.iter().map(|value| value * value).sum::<f64>().sqrt();
    if norm > 0.0 {
        for value in values {
            *value /= norm;
        }
    }
}

fn close_scores(left: &[(NodeIndex, f64)], right: &[(NodeIndex, f64)]) {
    assert_eq!(left.len(), right.len());
    for ((left_node, left_score), (right_node, right_score)) in left.iter().zip(right) {
        assert_eq!(left_node, right_node);
        assert!((left_score - right_score).abs() < 1e-12);
    }
}

fn next(state: u64) -> u64 {
    state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1)
}
