use super::{EdgeEndpoints, EdgeIndex};
use crate::Vec;
use crate::{GraphError, Result};
#[cfg(feature = "rayon")]
mod parallel;

#[cfg(all(feature = "rayon", feature = "unsafe-fast"))]
mod unsafe_fast;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Csr {
    offsets: Vec<u32>,
    edges: Vec<EdgeIndex>,
}

impl Csr {
    pub(super) fn try_build_pair(
        node_count: usize,
        endpoints: &[EdgeEndpoints],
    ) -> Result<(Self, Self)> {
        let mut outgoing_counts = vec![0_u32; node_count];
        let mut incoming_counts = vec![0_u32; node_count];
        for (edge, &endpoints) in endpoints.iter().enumerate() {
            increment(
                &mut outgoing_counts,
                endpoints.source().index(),
                edge,
                node_count,
                "csr entries",
            )?;
            increment(
                &mut incoming_counts,
                endpoints.target().index(),
                edge,
                node_count,
                "csr entries",
            )?;
        }

        let outgoing_offsets = offsets(&outgoing_counts, endpoints.len())?;
        let incoming_offsets = offsets(&incoming_counts, endpoints.len())?;
        outgoing_counts.copy_from_slice(&outgoing_offsets[..node_count]);
        incoming_counts.copy_from_slice(&incoming_offsets[..node_count]);
        let mut outgoing_edges = vec![EdgeIndex::new(0); endpoints.len()];
        let mut incoming_edges = vec![EdgeIndex::new(0); endpoints.len()];
        for (edge, &endpoints) in endpoints.iter().enumerate() {
            let edge = compact_edge(edge)?;
            place(
                &mut outgoing_edges,
                &mut outgoing_counts,
                endpoints.source().index(),
                edge,
            );
            place(
                &mut incoming_edges,
                &mut incoming_counts,
                endpoints.target().index(),
                edge,
            );
        }
        Ok((
            Self {
                offsets: outgoing_offsets,
                edges: outgoing_edges,
            },
            Self {
                offsets: incoming_offsets,
                edges: incoming_edges,
            },
        ))
    }

    #[cfg(feature = "rayon")]
    pub(super) fn try_build_pair_parallel(
        node_count: usize,
        endpoints: &[EdgeEndpoints],
        stable_order: bool,
    ) -> Result<(Self, Self)> {
        parallel::try_build_pair(node_count, endpoints, stable_order)
    }

    #[cfg(all(feature = "rayon", feature = "unsafe-fast"))]
    pub(super) fn try_build_pair_parallel_fast(
        node_count: usize,
        endpoints: &[EdgeEndpoints],
        stable_order: bool,
    ) -> Result<(Self, Self)> {
        unsafe_fast::try_build_pair(node_count, endpoints, stable_order)
    }

    pub(crate) fn get(&self, node: usize) -> &[EdgeIndex] {
        let Some((&start, &end)) = self.offsets.get(node).zip(self.offsets.get(node + 1)) else {
            return &[];
        };
        &self.edges[start as usize..end as usize]
    }

    pub(crate) fn offsets(&self) -> &[u32] {
        &self.offsets
    }

    pub(crate) fn edges(&self) -> &[EdgeIndex] {
        &self.edges
    }

    pub(crate) fn try_build_undirected(
        node_count: usize,
        endpoints: &[EdgeEndpoints],
    ) -> Result<Self> {
        let mut counts = vec![0_u32; node_count];
        for (edge, endpoints) in endpoints.iter().copied().enumerate() {
            increment(
                &mut counts,
                endpoints.source().index(),
                edge,
                node_count,
                "undirected incidences",
            )?;
            if endpoints.source() != endpoints.target() {
                increment(
                    &mut counts,
                    endpoints.target().index(),
                    edge,
                    node_count,
                    "undirected incidences",
                )?;
            }
        }
        let offsets = offsets(&counts, endpoints.len().saturating_mul(2))?;
        let total = offsets.last().copied().unwrap_or(0) as usize;
        counts.copy_from_slice(&offsets[..node_count]);
        let mut edges = vec![EdgeIndex::new(0); total];
        for (edge, endpoints) in endpoints.iter().copied().enumerate() {
            let edge = compact_edge(edge)?;
            place(&mut edges, &mut counts, endpoints.source().index(), edge);
            if endpoints.source() != endpoints.target() {
                place(&mut edges, &mut counts, endpoints.target().index(), edge);
            }
        }
        Ok(Self { offsets, edges })
    }
}

#[cfg(feature = "rayon")]
fn sort_adjacencies(offsets: &[u32], edges: &mut [EdgeIndex], base: u32) {
    const NODE_GRAIN: usize = 8_192;
    let node_count = offsets.len().saturating_sub(1);
    if node_count <= NODE_GRAIN {
        for pair in offsets.windows(2) {
            let start = (pair[0] - base) as usize;
            let end = (pair[1] - base) as usize;
            edges[start..end].sort_unstable();
        }
        return;
    }
    let middle = node_count / 2;
    let edge_middle = (offsets[middle] - base) as usize;
    let (left_edges, right_edges) = edges.split_at_mut(edge_middle);
    let left_offsets = &offsets[..=middle];
    let right_offsets = &offsets[middle..];
    let right_base = right_offsets[0];
    rayon::join(
        || sort_adjacencies(left_offsets, left_edges, base),
        || sort_adjacencies(right_offsets, right_edges, right_base),
    );
}

fn offsets(counts: &[u32], capacity_hint: usize) -> Result<Vec<u32>> {
    let mut offsets = Vec::with_capacity(counts.len() + 1);
    offsets.push(0_u32);
    for &count in counts {
        let total = offsets
            .last()
            .copied()
            .unwrap_or(0)
            .checked_add(count)
            .ok_or(GraphError::IndexCapacityExceeded {
                category: "csr entries",
                count: capacity_hint,
            })?;
        offsets.push(total);
    }
    Ok(offsets)
}

fn increment(
    counts: &mut [u32],
    node: usize,
    edge: usize,
    node_count: usize,
    category: &'static str,
) -> Result<()> {
    let Some(count) = counts.get_mut(node) else {
        return Err(GraphError::InvalidTopologyEndpoint {
            edge,
            node,
            node_count,
        });
    };
    *count = count
        .checked_add(1)
        .ok_or(GraphError::IndexCapacityExceeded {
            category,
            count: usize::MAX,
        })?;
    Ok(())
}

fn compact_edge(index: usize) -> Result<EdgeIndex> {
    u32::try_from(index)
        .map(EdgeIndex::new)
        .map_err(|_| GraphError::IndexCapacityExceeded {
            category: "edge index",
            count: index,
        })
}

fn place(edges: &mut [EdgeIndex], cursors: &mut [u32], node: usize, edge: EdgeIndex) {
    let slot = cursors[node] as usize;
    edges[slot] = edge;
    cursors[node] += 1;
}
