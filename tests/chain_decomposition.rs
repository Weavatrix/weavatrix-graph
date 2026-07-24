use rustworkx_core::petgraph::graph::UnGraph;
use std::cell::Cell;
use std::collections::BTreeSet;
use weavatrix_graph::{
    EdgeEndpoints, EdgeIndex, NodeIndex, UndirectedTopology, chain_decomposition,
    chain_decomposition_filtered, chain_decomposition_from,
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

fn edge_ids(result: &weavatrix_graph::ChainDecomposition<NodeIndex, EdgeIndex>) -> Vec<Vec<usize>> {
    result
        .chains()
        .iter()
        .map(|chain| chain.iter().map(|step| step.edge().index()).collect())
        .collect()
}

#[test]
fn decomposes_two_cycles_and_excludes_the_bridge() {
    let topology = graph(6, &[(0, 1), (1, 2), (2, 0), (1, 3), (3, 4), (4, 5), (5, 3)]);
    let result = chain_decomposition(&topology);
    assert_eq!(edge_ids(&result), [vec![2, 1, 0], vec![6, 5, 4]]);
    assert_eq!(result.chain_count(), 2);
    assert_contiguous(&topology, &result);
}

#[test]
fn preserves_parallel_edges_and_self_loops() {
    let topology = graph(2, &[(0, 1), (0, 1), (1, 1)]);
    let result = chain_decomposition(&topology);
    assert_eq!(edge_ids(&result), [vec![1, 0], vec![2]]);
    assert_contiguous(&topology, &result);
}

#[test]
fn source_limits_search_to_its_component() {
    let topology = graph(7, &[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)]);
    assert_eq!(
        edge_ids(&chain_decomposition_from(&topology, node(3)).unwrap()),
        [vec![5, 4, 3]]
    );
    assert!(chain_decomposition_from(&topology, node(9)).is_none());
}

#[test]
fn filtering_is_single_pass_and_can_break_a_cycle() {
    let topology = graph(4, &[(0, 1), (1, 2), (2, 0), (2, 3)]);
    let calls = Cell::new(0);
    let result = chain_decomposition_filtered(&topology, |edge| {
        calls.set(calls.get() + 1);
        edge.index() != 1
    });
    assert_eq!(calls.get(), 4);
    assert!(result.chains().is_empty());
}

#[test]
fn iterative_dfs_handles_a_deep_repository_chain() {
    const NODES: usize = 200_000;
    let edges = (0..NODES - 1)
        .map(|source| (source, source + 1))
        .collect::<Vec<_>>();
    assert!(
        chain_decomposition(&graph(NODES, &edges))
            .chains()
            .is_empty()
    );
}

#[test]
fn seeded_simple_graphs_cover_exactly_the_non_bridge_edges() {
    for seed in 1..=64_u64 {
        let node_count = 2 + usize::try_from(seed % 8).unwrap();
        let mut state = seed;
        let mut edges = Vec::new();
        for source in 0..node_count {
            for target in source + 1..node_count {
                state = next(state);
                if state % 4 == 0 {
                    edges.push((source, target));
                }
            }
        }
        let topology = graph(node_count, &edges);
        let result = chain_decomposition(&topology);
        let mut actual = result
            .chains()
            .iter()
            .flat_map(|chain| chain.iter().map(|step| step.edge().index()))
            .collect::<Vec<_>>();
        actual.sort_unstable();
        assert_eq!(actual, non_bridge_edges(node_count, &edges), "seed={seed}");
        actual.dedup();
        assert_eq!(
            actual.len(),
            result.chains().iter().map(Vec::len).sum::<usize>(),
            "edge-disjoint seed={seed}"
        );
        assert_eq!(
            endpoint_set(&result),
            rustworkx_endpoint_set(node_count, &edges),
            "rustworkx differential seed={seed}"
        );
        assert_contiguous(&topology, &result);
    }
}

fn endpoint_set(
    result: &weavatrix_graph::ChainDecomposition<NodeIndex, EdgeIndex>,
) -> BTreeSet<(usize, usize)> {
    result
        .chains()
        .iter()
        .flatten()
        .map(|step| {
            let source = step.source().index();
            let target = step.target().index();
            (source.min(target), source.max(target))
        })
        .collect()
}

fn rustworkx_endpoint_set(node_count: usize, edges: &[(usize, usize)]) -> BTreeSet<(usize, usize)> {
    let mut graph = UnGraph::<(), ()>::with_capacity(node_count, edges.len());
    let nodes = (0..node_count)
        .map(|_| graph.add_node(()))
        .collect::<Vec<_>>();
    for &(source, target) in edges {
        graph.add_edge(nodes[source], nodes[target], ());
    }
    rustworkx_core::connectivity::chain_decomposition(&graph, None)
        .into_iter()
        .flatten()
        .map(|(source, target)| {
            let source = source.index();
            let target = target.index();
            (source.min(target), source.max(target))
        })
        .collect()
}

fn assert_contiguous(
    graph: &UndirectedTopology,
    result: &weavatrix_graph::ChainDecomposition<NodeIndex, EdgeIndex>,
) {
    for chain in result.chains() {
        for (index, step) in chain.iter().enumerate() {
            let endpoints = graph.edge_endpoints(step.edge()).unwrap();
            assert!(
                (endpoints.source() == step.source() && endpoints.target() == step.target())
                    || (endpoints.source() == step.target() && endpoints.target() == step.source())
            );
            if let Some(next) = chain.get(index + 1) {
                assert_eq!(step.target(), next.source());
            }
        }
    }
}

fn non_bridge_edges(node_count: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    (0..edges.len())
        .filter(|&removed| connected_endpoints(node_count, edges, removed))
        .collect()
}

fn connected_endpoints(node_count: usize, edges: &[(usize, usize)], removed: usize) -> bool {
    let (source, target) = edges[removed];
    let mut seen = vec![false; node_count];
    let mut stack = vec![source];
    seen[source] = true;
    while let Some(node) = stack.pop() {
        for (index, &(left, right)) in edges.iter().enumerate() {
            if index == removed {
                continue;
            }
            let neighbor = if left == node {
                Some(right)
            } else if right == node {
                Some(left)
            } else {
                None
            };
            if let Some(neighbor) = neighbor
                && !seen[neighbor]
            {
                seen[neighbor] = true;
                stack.push(neighbor);
            }
        }
    }
    seen[target]
}

fn next(state: u64) -> u64 {
    state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1)
}
