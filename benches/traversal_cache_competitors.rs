use graph_builder::prelude::{
    CsrLayout, DirectedCsrGraph, DirectedNeighbors, Graph as BuilderGraph, GraphBuilder, Idx,
};
use std::{
    collections::VecDeque,
    hint::black_box,
    time::{Duration, Instant},
};
use weavatrix_graph::{
    Direction, EdgeEndpoints, NodeIndex, Topology, TraversalCache, TraversalCacheWorkspace,
    TraversalStorage,
};

#[path = "support/scale_harness.rs"]
mod scale_harness;
use scale_harness::{measure, report, setting, topology_pairs};

fn main() {
    let nodes = setting("WEAVATRIX_GRAPH_NODES", 200_000);
    let edges = setting("WEAVATRIX_GRAPH_EDGES", 1_000_000);
    let runs = setting(
        "WEAVATRIX_GRAPH_RUNS",
        if edges > 10_000_000 { 1 } else { 5 },
    );
    let mode = std::env::var("WEAVATRIX_CACHE_MODE").unwrap_or_else(|_| "all".into());
    println!("nodes={nodes} edges={edges} runs={runs} mode={mode}");
    let pairs = topology_pairs(nodes, edges);
    let topology = build_topology(nodes, &pairs);
    benchmark_caches(&topology, runs);
    if mode == "all" {
        benchmark_traversal(&topology, &pairs, runs);
    }
}

fn benchmark_caches(topology: &Topology, runs: usize) {
    for (name, storage) in [
        ("weavatrix cache fast", TraversalStorage::Fast),
        ("weavatrix cache balanced", TraversalStorage::Balanced),
        ("weavatrix cache compact", TraversalStorage::Compact),
        ("weavatrix cache auto", TraversalStorage::Auto),
    ] {
        let cache = topology.traversal_cache_with(storage);
        verify_cache(topology, &cache);
        println!(
            "storage implementation={name} layout={:?} bytes={} fast_equivalent={} ratio={:.3}",
            cache.layout(),
            cache.storage_bytes(),
            cache.fast_equivalent_bytes(),
            cache.storage_ratio()
        );
        drop(cache);
        report(
            "derived-cache-build",
            name,
            measure(runs, || topology.traversal_cache_with(storage)),
        );
    }
}

fn benchmark_traversal(topology: &Topology, pairs: &[(u32, u32)], runs: usize) {
    let fast = topology.traversal_cache_with(TraversalStorage::Fast);
    let balanced = topology.traversal_cache_with(TraversalStorage::Balanced);
    let compact = topology.traversal_cache_with(TraversalStorage::Compact);
    let builder = build_graph_builder(pairs);
    let expected = cache_bfs(&fast);
    assert_eq!(cache_bfs(&balanced), expected);
    assert_eq!(cache_bfs(&compact), expected);
    assert_eq!(builder_bfs(&builder), expected);
    for (name, cache) in [
        ("weavatrix cache fast", &fast),
        ("weavatrix cache balanced", &balanced),
        ("weavatrix cache compact", &compact),
    ] {
        let mut cache_workspace = TraversalCacheWorkspace::new();
        let mut builder_workspace = BuilderWorkspace::new(builder.node_count().index());
        let (cache_time, builder_time) = compare(
            runs,
            || {
                cache
                    .bfs_with_workspace(
                        NodeIndex::new(0),
                        Direction::Outgoing,
                        &mut cache_workspace,
                    )
                    .len()
            },
            || builder_workspace.bfs(&builder, 0),
        );
        report("single-source-bfs-reused", name, cache_time);
        report(
            "single-source-bfs-reused",
            &format!("graph_builder direct csr vs {name}"),
            builder_time,
        );
    }
}

fn build_topology(nodes: usize, pairs: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges_parallel_fast(
        nodes,
        pairs.iter().map(|&(source, target)| {
            EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
        }),
    )
    .unwrap()
}

fn build_graph_builder(pairs: &[(u32, u32)]) -> DirectedCsrGraph<u32> {
    GraphBuilder::new()
        .csr_layout(CsrLayout::Unsorted)
        .edges(pairs.iter().copied())
        .build()
}

fn cache_bfs(cache: &TraversalCache) -> usize {
    cache
        .bfs_iter(
            NodeIndex::new(0),
            Direction::Outgoing,
            &mut TraversalCacheWorkspace::new(),
        )
        .count()
}

fn builder_bfs(graph: &DirectedCsrGraph<u32>) -> usize {
    BuilderWorkspace::new(graph.node_count().index()).bfs(graph, 0)
}

fn verify_cache(topology: &Topology, cache: &TraversalCache) {
    assert_eq!(cache.node_count(), topology.node_count());
    assert_eq!(cache.edge_count(), topology.edge_count());
    for raw in (0..topology.node_count()).step_by((topology.node_count() / 997).max(1)) {
        let node = NodeIndex::new(u32::try_from(raw).expect("topology node fits u32"));
        assert_eq!(
            cache.outgoing_neighbors(node).collect::<Vec<_>>(),
            topology.outgoing_neighbors(node).collect::<Vec<_>>()
        );
        assert_eq!(
            cache.incoming_neighbors(node).collect::<Vec<_>>(),
            topology.incoming_neighbors(node).collect::<Vec<_>>()
        );
    }
}

fn compare(
    runs: usize,
    mut left: impl FnMut() -> usize,
    mut right: impl FnMut() -> usize,
) -> (Duration, Duration) {
    assert_eq!(left(), right());
    let mut left_samples = Vec::with_capacity(runs);
    let mut right_samples = Vec::with_capacity(runs);
    for run in 0..runs {
        if run % 2 == 0 {
            left_samples.push(timed(&mut left));
            right_samples.push(timed(&mut right));
        } else {
            right_samples.push(timed(&mut right));
            left_samples.push(timed(&mut left));
        }
    }
    left_samples.sort_unstable();
    right_samples.sort_unstable();
    (left_samples[runs / 2], right_samples[runs / 2])
}

fn timed(operation: &mut impl FnMut() -> usize) -> Duration {
    let started = Instant::now();
    black_box(operation());
    started.elapsed()
}

struct BuilderWorkspace {
    marks: Vec<u32>,
    epoch: u32,
    queue: VecDeque<u32>,
}

impl BuilderWorkspace {
    fn new(nodes: usize) -> Self {
        Self {
            marks: vec![0; nodes],
            epoch: 0,
            queue: VecDeque::new(),
        }
    }

    fn bfs(&mut self, graph: &DirectedCsrGraph<u32>, start: u32) -> usize {
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.marks.fill(0);
            self.epoch = 1;
        }
        self.queue.clear();
        self.marks[start as usize] = self.epoch;
        self.queue.push_back(start);
        let mut count = 0;
        while let Some(node) = self.queue.pop_front() {
            count += 1;
            for &neighbor in graph.out_neighbors(node) {
                let mark = &mut self.marks[neighbor as usize];
                if *mark != self.epoch {
                    *mark = self.epoch;
                    self.queue.push_back(neighbor);
                }
            }
        }
        count
    }
}
