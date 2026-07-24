use std::cell::Cell;
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, UndirectedTopology, biconnected_components,
    biconnected_components_filtered,
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

fn blocks(
    result: &weavatrix_graph::BiconnectedComponents<NodeIndex, weavatrix_graph::EdgeIndex>,
) -> Vec<Vec<usize>> {
    result
        .components()
        .iter()
        .map(|component| component.iter().map(|edge| edge.index()).collect())
        .collect()
}

#[test]
fn separates_cycles_bridge_and_cut_vertices_canonically() {
    let topology = graph(6, &[(0, 1), (1, 2), (2, 0), (1, 3), (3, 4), (4, 5), (5, 3)]);
    let result = biconnected_components(&topology);
    assert_eq!(blocks(&result), [vec![0, 1, 2], vec![3], vec![4, 5, 6]]);
    assert_eq!(result.articulation_points(), &[node(1), node(3)]);
    assert_eq!(result.component_count(), 3);
}

#[test]
fn keeps_parallel_edges_together_and_self_loops_separate() {
    let topology = graph(2, &[(0, 1), (0, 1), (1, 1)]);
    let result = biconnected_components(&topology);
    assert_eq!(blocks(&result), [vec![0, 1], vec![2]]);
    assert!(result.articulation_points().is_empty());
}

#[test]
fn filtering_is_single_pass_and_changes_the_induced_blocks() {
    let topology = graph(6, &[(0, 1), (1, 2), (2, 0), (1, 3), (3, 4), (4, 5), (5, 3)]);
    let calls = Cell::new(0);
    let result = biconnected_components_filtered(&topology, |edge| {
        calls.set(calls.get() + 1);
        edge.index() != 3
    });
    assert_eq!(calls.get(), 7);
    assert_eq!(blocks(&result), [vec![0, 1, 2], vec![4, 5, 6]]);
    assert!(result.articulation_points().is_empty());
}

#[test]
fn iterative_search_handles_a_deep_repository_chain() {
    const NODES: usize = 200_000;
    let edges = (0..NODES - 1)
        .map(|source| (source, source + 1))
        .collect::<Vec<_>>();
    let result = biconnected_components(&graph(NODES, &edges));
    assert_eq!(result.component_count(), NODES - 1);
    assert_eq!(result.articulation_points().len(), NODES - 2);
}

#[test]
fn seeded_blocks_match_an_exhaustive_definition_reference() {
    for seed in 1..=32_u64 {
        let node_count = 3 + usize::try_from(seed % 5).unwrap();
        let mut state = seed;
        let mut edges = Vec::new();
        for source in 0..node_count {
            for target in source + 1..node_count {
                state = next(state);
                if state % 5 < 2 && edges.len() < 12 {
                    edges.push((source, target));
                }
            }
        }
        let actual = biconnected_components(&graph(node_count, &edges))
            .into_components()
            .into_iter()
            .map(|component| {
                component
                    .into_iter()
                    .fold(0_u16, |mask, edge| mask | (1 << edge.index()))
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, reference_blocks(node_count, &edges), "seed={seed}");
    }
}

fn reference_blocks(node_count: usize, edges: &[(usize, usize)]) -> Vec<u16> {
    let limit = 1_u16 << edges.len();
    let valid = (1..limit)
        .filter(|&mask| is_block(node_count, edges, mask))
        .collect::<Vec<_>>();
    let mut maximal = valid
        .iter()
        .copied()
        .filter(|&candidate| {
            !valid
                .iter()
                .any(|&other| candidate != other && candidate & other == candidate)
        })
        .collect::<Vec<_>>();
    maximal.sort_unstable_by_key(|mask| mask.trailing_zeros());
    maximal
}

fn is_block(node_count: usize, edges: &[(usize, usize)], mask: u16) -> bool {
    let active = active_nodes(node_count, edges, mask);
    connected_without(edges, mask, &active, None)
        && (0..node_count)
            .filter(|&removed| active[removed])
            .all(|removed| connected_without(edges, mask, &active, Some(removed)))
}

fn active_nodes(node_count: usize, edges: &[(usize, usize)], mask: u16) -> Vec<bool> {
    let mut active = vec![false; node_count];
    for (index, &(source, target)) in edges.iter().enumerate() {
        if mask & (1 << index) != 0 {
            active[source] = true;
            active[target] = true;
        }
    }
    active
}

fn connected_without(
    edges: &[(usize, usize)],
    mask: u16,
    active: &[bool],
    removed: Option<usize>,
) -> bool {
    let Some(start) = (0..active.len()).find(|&node| active[node] && Some(node) != removed) else {
        return true;
    };
    let mut seen = vec![false; active.len()];
    let mut stack = vec![start];
    seen[start] = true;
    while let Some(node) = stack.pop() {
        for (index, &(source, target)) in edges.iter().enumerate() {
            if mask & (1 << index) == 0 || Some(source) == removed || Some(target) == removed {
                continue;
            }
            let neighbor = if source == node {
                Some(target)
            } else if target == node {
                Some(source)
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
    (0..active.len()).all(|node| !active[node] || Some(node) == removed || seen[node])
}

fn next(state: u64) -> u64 {
    state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1)
}
