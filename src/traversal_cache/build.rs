use super::TraversalStorage;
use crate::{EdgeEndpoints, EdgeIndex, Vec};

pub(super) fn build_neighbor_pair(
    endpoints: &[EdgeEndpoints],
    outgoing: &[EdgeIndex],
    incoming: &[EdgeIndex],
) -> (Vec<u32>, Vec<u32>) {
    #[cfg(feature = "rayon")]
    if endpoints.len() >= 262_144 {
        return super::parallel::build_neighbor_pair(endpoints, outgoing, incoming);
    }
    (
        outgoing
            .iter()
            .map(|edge| endpoints[edge.index()].target().get())
            .collect(),
        incoming
            .iter()
            .map(|edge| endpoints[edge.index()].source().get())
            .collect(),
    )
}

pub(super) fn neighbor_bits(node_count: usize) -> u8 {
    let maximum = u32::try_from(node_count.saturating_sub(1)).unwrap_or(u32::MAX);
    u8::try_from((u32::BITS - maximum.leading_zeros()).max(1)).unwrap_or(32)
}

pub(super) fn select_storage(
    requested: TraversalStorage,
    nodes: usize,
    edges: usize,
    bits: u8,
) -> TraversalStorage {
    if requested != TraversalStorage::Auto {
        return requested;
    }
    let fast = (edges * 2 + (nodes + 1) * 2) * size_of::<u32>();
    let packed_neighbors = edges.div_ceil(64) * usize::from(bits) * size_of::<u64>() * 2;
    let balanced = packed_neighbors + (nodes + 1) * size_of::<u32>() * 2;
    if balanced <= fast.saturating_mul(7) / 8 {
        TraversalStorage::Balanced
    } else {
        TraversalStorage::Fast
    }
}
