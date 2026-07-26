use std::{
    hint::black_box,
    time::{Duration, Instant},
};

pub fn topology_pairs(nodes: usize, edges: usize) -> Vec<(u32, u32)> {
    u32::try_from(nodes).expect("node count fits u32");
    (0..edges)
        .map(|index| {
            let source = index % nodes;
            let layer = index / nodes;
            let mut target = (source * 37 + layer * 7_919 + 17) % nodes;
            if target == source {
                target = (target + 1) % nodes;
            }
            (
                u32::try_from(source).expect("source fits u32"),
                u32::try_from(target).expect("target fits u32"),
            )
        })
        .collect()
}

pub fn measure<T>(runs: usize, mut operation: impl FnMut() -> T) -> Duration {
    if runs > 1 {
        black_box(operation());
    }
    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let started = Instant::now();
        black_box(operation());
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    samples[samples.len() / 2]
}

pub fn report(operation: &str, implementation: &str, duration: Duration) {
    println!(
        "operation={operation} implementation={implementation} median_ms={:.3}",
        duration.as_secs_f64() * 1_000.0
    );
}

pub fn setting(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
