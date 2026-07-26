use graph_builder::prelude::{DirectedCsrGraph, Graph as BuilderGraph, GraphBuilder, Idx};
use std::{
    collections::BTreeMap,
    hint::black_box,
    path::PathBuf,
    time::{Duration, Instant},
};
use weavatrix_graph::{EdgeEndpoints, NodeIndex, Topology, bfs};
use weavatrix_scan::{ScanOptions, Scanner, StandardSkips};

fn main() {
    let root = std::env::var_os("WEAVATRIX_REAL_ROOT")
        .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
    let scan_started = Instant::now();
    let report = Scanner::new(&root)
        .options(scan_options())
        .scan_compact()
        .unwrap();
    let scan_time = scan_started.elapsed();
    let (nodes, pairs) = containment_graph(&report.files);
    println!(
        "root={} files={} nodes={} edges={} scan_ms={:.3} complete={} warnings={} skipped={}",
        root.display(),
        report.files.len(),
        nodes,
        pairs.len(),
        scan_time.as_secs_f64() * 1_000.0,
        report.complete,
        report.warnings.len(),
        report.skipped.len()
    );
    let ours = build_ours(nodes, &pairs);
    let competitor = build_graph_builder(&pairs);
    assert_eq!(ours.node_count(), competitor.node_count().index());
    assert_eq!(ours.edge_count(), competitor.edge_count().index());
    let reachable = bfs(&ours, NodeIndex::new(0)).len();
    assert_eq!(reachable, nodes);
    report_timing(
        "filesystem-dual-csr",
        "weavatrix-graph sequential stable",
        measure(|| build_ours(nodes, &pairs)),
    );
    report_timing(
        "filesystem-dual-csr",
        "weavatrix-graph auto stable",
        measure(|| build_ours_auto(nodes, &pairs)),
    );
    report_timing(
        "filesystem-dual-csr",
        "weavatrix-graph rayon stable",
        measure(|| build_ours_parallel(nodes, &pairs)),
    );
    report_timing(
        "filesystem-dual-csr",
        "weavatrix-graph unsafe-fast stable",
        measure(|| build_ours_parallel_fast(nodes, &pairs)),
    );
    report_timing(
        "filesystem-dual-csr",
        "graph_builder rayon narrower",
        measure(|| build_graph_builder(&pairs)),
    );
    report_timing(
        "filesystem-bfs",
        "weavatrix-graph",
        measure(|| bfs(&ours, NodeIndex::new(0)).len()),
    );
}

fn scan_options() -> ScanOptions {
    let mut options = ScanOptions::default()
        .metadata_only()
        .selected_files_only()
        .with_skip_hidden(false);
    options.max_file_bytes = u64::MAX;
    options.ignore_files.clear();
    options.standard_skips = StandardSkips::Disabled;
    options
}

fn containment_graph(files: &[weavatrix_scan::CompactScannedFile]) -> (usize, Vec<(u32, u32)>) {
    let mut nodes = BTreeMap::<String, u32>::from([(String::new(), 0)]);
    let mut edges = Vec::new();
    for file in files {
        let components = normalized_components(&file.relative);
        let mut parent = 0;
        let mut path = String::new();
        for component in components {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(component);
            let next = if let Some(&existing) = nodes.get(&path) {
                existing
            } else {
                let next = u32::try_from(nodes.len()).expect("filesystem graph fits u32");
                nodes.insert(path.clone(), next);
                edges.push((parent, next));
                next
            };
            parent = next;
        }
    }
    (nodes.len(), edges)
}

fn normalized_components(path: &str) -> impl Iterator<Item = &str> {
    path.split(['/', '\\'])
        .filter(|component| !component.is_empty())
}

fn build_ours(nodes: usize, pairs: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges(
        nodes,
        pairs.iter().map(|&(source, target)| {
            EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
        }),
    )
    .unwrap()
}

fn build_ours_parallel(nodes: usize, pairs: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges_parallel(nodes, endpoint_iter(pairs)).unwrap()
}

fn build_ours_auto(nodes: usize, pairs: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges_auto(nodes, endpoint_iter(pairs)).unwrap()
}

fn build_ours_parallel_fast(nodes: usize, pairs: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges_parallel_fast(nodes, endpoint_iter(pairs)).unwrap()
}

fn endpoint_iter(pairs: &[(u32, u32)]) -> impl Iterator<Item = EdgeEndpoints> + '_ {
    pairs
        .iter()
        .map(|&(source, target)| EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target)))
}

fn build_graph_builder(pairs: &[(u32, u32)]) -> DirectedCsrGraph<u32> {
    GraphBuilder::new().edges(pairs.iter().copied()).build()
}

fn measure<T>(mut operation: impl FnMut() -> T) -> Duration {
    black_box(operation());
    let mut samples = Vec::with_capacity(5);
    for _ in 0..5 {
        let started = Instant::now();
        black_box(operation());
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    samples[2]
}

fn report_timing(operation: &str, implementation: &str, duration: Duration) {
    println!(
        "operation={operation} implementation={implementation} median_ms={:.3}",
        duration.as_secs_f64() * 1_000.0
    );
}
