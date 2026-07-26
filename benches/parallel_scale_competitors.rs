use graph_builder::prelude::{
    CsrLayout, DirectedCsrGraph, DirectedNeighbors, Graph as BuilderGraph, GraphBuilder, Idx,
};
use rayon::prelude::*;
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicBool, Ordering},
};
use weavatrix_graph::{EdgeEndpoints, NodeIndex, Topology, bfs};

#[path = "support/parallel_johnson.rs"]
mod parallel_johnson;
#[path = "support/scale_harness.rs"]
mod scale_harness;
use scale_harness::{measure, report, setting, topology_pairs};

fn main() {
    let nodes = setting("WEAVATRIX_GRAPH_NODES", 200_000);
    let edges = setting("WEAVATRIX_GRAPH_EDGES", 1_000_000);
    let runs = setting(
        "WEAVATRIX_GRAPH_RUNS",
        if edges > 10_000_000 { 1 } else { 3 },
    );
    let mode = std::env::var("WEAVATRIX_GRAPH_MODE").unwrap_or_else(|_| "all".into());
    let threads = rayon::current_num_threads();
    println!("nodes={nodes} edges={edges} runs={runs} rayon_threads={threads} mode={mode}");
    let pairs = topology_pairs(nodes, edges);
    if matches!(mode.as_str(), "all" | "build" | "ours") {
        benchmark_ours(nodes, &pairs, runs);
    }
    if mode == "fast" {
        benchmark_ours_fast(nodes, &pairs, runs);
    }
    if matches!(mode.as_str(), "all" | "build" | "graph-builder") {
        benchmark_graph_builder(nodes, &pairs, runs, threads);
    }
    if matches!(mode.as_str(), "all" | "bfs") {
        benchmark_bfs(nodes, &pairs, runs, threads);
    }
    if matches!(mode.as_str(), "all" | "johnson") {
        parallel_johnson::benchmark();
    }
}

fn benchmark_ours(nodes: usize, pairs: &[(u32, u32)], runs: usize) {
    let graph = build_ours(nodes, pairs);
    let parallel = build_ours_parallel(nodes, pairs);
    assert_eq!(
        (graph.node_count(), graph.edge_count()),
        (nodes, pairs.len())
    );
    assert_eq!(parallel, graph);
    drop((graph, parallel));
    report(
        "dual-csr-build",
        "weavatrix-graph sequential",
        measure(runs, || build_ours(nodes, pairs)),
    );
    report(
        "dual-csr-build",
        "weavatrix-graph auto stable",
        measure(runs, || build_ours_auto(nodes, pairs)),
    );
    report(
        "dual-csr-build",
        "weavatrix-graph rayon stable",
        measure(runs, || build_ours_parallel(nodes, pairs)),
    );
    report(
        "dual-csr-build",
        "weavatrix-graph rayon unordered",
        measure(runs, || build_ours_parallel_unordered(nodes, pairs)),
    );
    report(
        "dual-csr-build",
        "weavatrix-graph unsafe-fast stable",
        measure(runs, || build_ours_parallel_fast(nodes, pairs)),
    );
    report(
        "dual-csr-build",
        "weavatrix-graph unsafe-fast unordered",
        measure(runs, || build_ours_parallel_unordered_fast(nodes, pairs)),
    );
}

fn benchmark_graph_builder(nodes: usize, pairs: &[(u32, u32)], runs: usize, threads: usize) {
    let graph = build_graph_builder(pairs);
    assert_eq!(graph.node_count().index(), nodes);
    assert_eq!(graph.edge_count().index(), pairs.len());
    drop(graph);
    let single = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    report(
        "dual-csr-build",
        "graph_builder threads=1",
        measure(runs, || single.install(|| build_graph_builder(pairs))),
    );
    let parallel = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    report(
        "dual-csr-build",
        "graph_builder rayon",
        measure(runs, || parallel.install(|| build_graph_builder(pairs))),
    );
}

fn benchmark_ours_fast(nodes: usize, pairs: &[(u32, u32)], runs: usize) {
    let stable = build_ours_parallel_fast(nodes, pairs);
    let sequential = build_ours(nodes, pairs);
    assert_eq!(stable, sequential);
    drop((stable, sequential));
    report(
        "dual-csr-build",
        "weavatrix-graph unsafe-fast stable",
        measure(runs, || build_ours_parallel_fast(nodes, pairs)),
    );
    report(
        "dual-csr-build",
        "weavatrix-graph unsafe-fast unordered",
        measure(runs, || build_ours_parallel_unordered_fast(nodes, pairs)),
    );
}

fn benchmark_bfs(nodes: usize, pairs: &[(u32, u32)], runs: usize, threads: usize) {
    let ours = build_ours(nodes, pairs);
    let builder = build_graph_builder(pairs);
    let expected = bfs(&ours, NodeIndex::new(0)).len();
    assert_eq!(builder_bfs(&builder, 0), expected);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    assert_eq!(pool.install(|| parallel_bfs_ours(&ours, 0)), expected);
    assert_eq!(pool.install(|| parallel_bfs_builder(&builder, 0)), expected);
    report(
        "single-source-bfs",
        "weavatrix-graph sequential",
        measure(runs, || bfs(&ours, NodeIndex::new(0)).len()),
    );
    report(
        "single-source-bfs",
        "weavatrix-graph frontier-rayon prototype",
        measure(runs, || pool.install(|| parallel_bfs_ours(&ours, 0))),
    );
    report(
        "single-source-bfs",
        "graph_builder frontier-rayon adapter",
        measure(runs, || pool.install(|| parallel_bfs_builder(&builder, 0))),
    );
}

fn build_ours(nodes: usize, pairs: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges(
        nodes,
        pairs
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node_u32(source), node_u32(target))),
    )
    .unwrap()
}

fn build_ours_auto(nodes: usize, pairs: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges_auto(
        nodes,
        pairs
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node_u32(source), node_u32(target))),
    )
    .unwrap()
}

fn build_ours_parallel(nodes: usize, pairs: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges_parallel(
        nodes,
        pairs
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node_u32(source), node_u32(target))),
    )
    .unwrap()
}

fn build_ours_parallel_unordered(nodes: usize, pairs: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges_parallel_unordered(
        nodes,
        pairs
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node_u32(source), node_u32(target))),
    )
    .unwrap()
}

fn build_ours_parallel_fast(nodes: usize, pairs: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges_parallel_fast(
        nodes,
        pairs
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node_u32(source), node_u32(target))),
    )
    .unwrap()
}

fn build_ours_parallel_unordered_fast(nodes: usize, pairs: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges_parallel_unordered_fast(
        nodes,
        pairs
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node_u32(source), node_u32(target))),
    )
    .unwrap()
}

fn build_graph_builder(pairs: &[(u32, u32)]) -> DirectedCsrGraph<u32> {
    GraphBuilder::new()
        .csr_layout(CsrLayout::Unsorted)
        .edges(pairs.iter().copied())
        .build()
}

fn builder_bfs(graph: &DirectedCsrGraph<u32>, start: u32) -> usize {
    let mut seen = vec![false; graph.node_count().index()];
    let mut queue = VecDeque::from([start]);
    seen[start as usize] = true;
    let mut count = 0;
    while let Some(node) = queue.pop_front() {
        count += 1;
        for &neighbor in graph.out_neighbors(node) {
            if !seen[neighbor as usize] {
                seen[neighbor as usize] = true;
                queue.push_back(neighbor);
            }
        }
    }
    count
}

fn parallel_bfs_ours(graph: &Topology, start: u32) -> usize {
    parallel_frontier(graph.node_count(), start, |node, next, seen| {
        for neighbor in graph.outgoing_neighbors(node_u32(node)) {
            visit(
                u32::try_from(neighbor.index()).expect("topology index fits u32"),
                next,
                seen,
            );
        }
    })
}

fn parallel_bfs_builder(graph: &DirectedCsrGraph<u32>, start: u32) -> usize {
    parallel_frontier(graph.node_count().index(), start, |node, next, seen| {
        for &neighbor in graph.out_neighbors(node) {
            visit(neighbor, next, seen);
        }
    })
}

fn parallel_frontier(
    nodes: usize,
    start: u32,
    visit_neighbors: impl Fn(u32, &mut Vec<u32>, &[AtomicBool]) + Sync,
) -> usize {
    let seen = (0..nodes)
        .map(|_| AtomicBool::new(false))
        .collect::<Vec<_>>();
    seen[start as usize].store(true, Ordering::Relaxed);
    let mut frontier = vec![start];
    let mut count = 0;
    while !frontier.is_empty() {
        count += frontier.len();
        frontier = frontier
            .par_iter()
            .fold(Vec::new, |mut next, &node| {
                visit_neighbors(node, &mut next, &seen);
                next
            })
            .reduce(Vec::new, |mut left, mut right| {
                left.append(&mut right);
                left
            });
    }
    count
}

fn visit(node: u32, next: &mut Vec<u32>, seen: &[AtomicBool]) {
    if !seen[node as usize].swap(true, Ordering::Relaxed) {
        next.push(node);
    }
}

const fn node_u32(index: u32) -> NodeIndex {
    NodeIndex::new(index)
}
