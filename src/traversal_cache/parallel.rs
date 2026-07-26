use crate::{EdgeEndpoints, EdgeIndex, Vec};
use rayon::prelude::*;

pub(super) fn build_neighbor_pair(
    endpoints: &[EdgeEndpoints],
    outgoing: &[EdgeIndex],
    incoming: &[EdgeIndex],
) -> (Vec<u32>, Vec<u32>) {
    rayon::join(
        || {
            outgoing
                .par_iter()
                .map(|edge| endpoints[edge.index()].target().get())
                .collect()
        },
        || {
            incoming
                .par_iter()
                .map(|edge| endpoints[edge.index()].source().get())
                .collect()
        },
    )
}
