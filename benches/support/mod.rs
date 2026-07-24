#![allow(dead_code)]

use std::collections::BTreeSet;
use std::time::{Duration, Instant};
use weavatrix_graph::{
    Confidence, Edge, EdgeKind, EvidenceKind, Graph, Node, NodeId, NodeKind, Provenance,
};

pub struct Measurement {
    pub median: Duration,
    pub minimum: Duration,
}

pub fn measure<T>(mut operation: impl FnMut() -> T) -> Measurement {
    const WARMUPS: usize = 2;
    const RUNS: usize = 11;

    for _ in 0..WARMUPS {
        std::hint::black_box(operation());
    }
    let mut samples = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let start = Instant::now();
        std::hint::black_box(operation());
        samples.push(start.elapsed());
    }
    samples.sort_unstable();
    Measurement {
        median: samples[RUNS / 2],
        minimum: samples[0],
    }
}

pub fn measure_batched<T>(iterations: u32, mut operation: impl FnMut() -> T) -> Measurement {
    measure_batched_with_setup(iterations, || (), |()| operation())
}

pub fn measure_batched_with_setup<Input, Output>(
    iterations: u32,
    mut setup: impl FnMut() -> Input,
    mut operation: impl FnMut(Input) -> Output,
) -> Measurement {
    const WARMUPS: usize = 2;
    const RUNS: usize = 11;
    assert!(iterations > 0);

    for _ in 0..WARMUPS {
        for _ in 0..iterations {
            std::hint::black_box(operation(setup()));
        }
    }
    let mut samples = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let inputs = (0..iterations).map(|_| setup()).collect::<Vec<_>>();
        let start = Instant::now();
        for input in inputs {
            std::hint::black_box(operation(input));
        }
        samples.push(start.elapsed() / iterations);
    }
    samples.sort_unstable();
    Measurement {
        median: samples[RUNS / 2],
        minimum: samples[0],
    }
}

pub fn print_measurement(mode: &str, details: &str, measurement: &Measurement) {
    println!(
        "mode={mode} {details} median_ms={:.3} min_ms={:.3}",
        measurement.median.as_secs_f64() * 1_000.0,
        measurement.minimum.as_secs_f64() * 1_000.0
    );
}

pub fn build_graph(node_count: usize, edge_count: usize) -> Graph {
    let (nodes, edges) = graph_parts(node_count, edge_count);
    Graph::try_from_parts(nodes, edges).unwrap()
}

pub fn graph_parts(node_count: usize, edge_count: usize) -> (Vec<Node>, Vec<Edge>) {
    let nodes = (0..node_count)
        .map(|index| {
            Node::new(
                format!("node:{index:05}"),
                format!("node_{index}"),
                NodeKind::Function,
            )
            .unwrap()
            .with_language("rust")
        })
        .collect();
    let edges = topology_pairs(node_count, edge_count)
        .into_iter()
        .enumerate()
        .map(|(index, (source_index, target_index))| {
            let source = NodeId::new(format!("node:{source_index:05}")).unwrap();
            let target = NodeId::new(format!("node:{target_index:05}")).unwrap();
            Edge::new(
                source,
                target,
                EdgeKind::Calls,
                Provenance::new("bench.graph", EvidenceKind::Resolved, Confidence::High)
                    .unwrap()
                    .with_detail(format!("edge:{index}")),
            )
        })
        .collect();
    (nodes, edges)
}

pub fn topology_pairs(node_count: usize, edge_count: usize) -> Vec<(usize, usize)> {
    (0..edge_count)
        .map(|index| {
            let source = index % node_count;
            let layer = index / node_count;
            let mut target = (source * 37 + layer * 7_919 + 17) % node_count;
            if target == source {
                target = (target + 1) % node_count;
            }
            (source, target)
        })
        .collect()
}

pub fn undirected_pairs(node_count: usize, edge_count: usize) -> Vec<(usize, usize)> {
    let mut pairs = BTreeSet::new();
    for source in 0..node_count - 1 {
        pairs.insert((source, source + 1));
    }
    for (source, target) in topology_pairs(node_count, edge_count * 4) {
        pairs.insert((source.min(target), source.max(target)));
        if pairs.len() == edge_count {
            break;
        }
    }
    pairs.into_iter().collect()
}
