use petgraph::{algo::parallel_johnson, graph::DiGraph};
use std::{hint::black_box, time::Instant};
use weavatrix_graph::{EdgeEndpoints, NodeIndex, Topology, johnson_all_pairs_parallel};

pub fn benchmark() {
    const NODES: usize = 1_200;
    const EDGES: usize = 6_000;
    let pairs = pairs(NODES, EDGES);
    let ours = Topology::try_from_edges(
        NODES,
        pairs
            .iter()
            .map(|&(source, target)| EdgeEndpoints::new(node(source), node(target))),
    )
    .unwrap();
    let mut pet = DiGraph::<(), i64>::with_capacity(NODES, EDGES);
    let pet_nodes = (0..NODES).map(|_| pet.add_node(())).collect::<Vec<_>>();
    for (edge, &(source, target)) in pairs.iter().enumerate() {
        pet.add_edge(pet_nodes[source], pet_nodes[target], weight(edge));
    }
    let ours_result = johnson_all_pairs_parallel(&ours, |edge| weight(edge.index())).unwrap();
    let pet_result = parallel_johnson(&pet, |edge| *edge.weight()).unwrap();
    for source in 0..NODES {
        for target in 0..NODES {
            assert_eq!(
                ours_result.distance(node(source), node(target)),
                pet_result
                    .get(&(pet_nodes[source], pet_nodes[target]))
                    .copied()
            );
        }
    }
    report("weavatrix-graph", || {
        johnson_all_pairs_parallel(&ours, |edge| weight(edge.index())).unwrap()
    });
    report("petgraph", || {
        parallel_johnson(&pet, |edge| *edge.weight()).unwrap()
    });
}

fn report<T>(implementation: &str, mut operation: impl FnMut() -> T) {
    black_box(operation());
    let mut samples = Vec::with_capacity(5);
    for _ in 0..5 {
        let started = Instant::now();
        black_box(operation());
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    println!(
        "operation=parallel-johnson implementation={implementation} median_ms={:.3}",
        samples[2].as_secs_f64() * 1_000.0
    );
}

fn pairs(nodes: usize, edges: usize) -> Vec<(usize, usize)> {
    (0..edges)
        .map(|edge| {
            let source = edge % nodes;
            let target = (source * 37 + edge / nodes * 7_919 + 17) % nodes;
            (
                source,
                if target == source {
                    (target + 1) % nodes
                } else {
                    target
                },
            )
        })
        .collect()
}

fn node(index: usize) -> NodeIndex {
    NodeIndex::new(u32::try_from(index).expect("node index fits u32"))
}

fn weight(edge: usize) -> i64 {
    1 + i64::try_from(edge % 19).expect("weight remainder fits i64")
}
