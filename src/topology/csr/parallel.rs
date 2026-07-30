use super::{Csr, sort_adjacencies};
use crate::{EdgeEndpoints, EdgeIndex, GraphError, Result, Vec};
use rayon::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

pub(super) fn try_build_pair(
    node_count: usize,
    endpoints: &[EdgeEndpoints],
    stable_order: bool,
) -> Result<(Csr, Csr)> {
    let (outgoing_cursors, incoming_cursors) = count_cursors(node_count, endpoints)?;
    let (outgoing_slots, incoming_slots) = rayon::join(
        || atomic_zeros(endpoints.len()),
        || atomic_zeros(endpoints.len()),
    );
    endpoints
        .par_iter()
        .copied()
        .enumerate()
        .try_for_each(|(edge, endpoints)| -> Result<()> {
            let edge = u32::try_from(edge).map_err(|_| GraphError::IndexCapacityExceeded {
                category: "edge index",
                count: edge,
            })?;
            let outgoing = outgoing_cursors[endpoints.source().index()]
                .fetch_add(1, Ordering::Relaxed) as usize;
            let incoming = incoming_cursors[endpoints.target().index()]
                .fetch_add(1, Ordering::Relaxed) as usize;
            outgoing_slots[outgoing].store(edge, Ordering::Relaxed);
            incoming_slots[incoming].store(edge, Ordering::Relaxed);
            Ok(())
        })?;
    let (mut outgoing_edges, mut incoming_edges) =
        rayon::join(|| into_edges(outgoing_slots), || into_edges(incoming_slots));
    let (outgoing_offsets, incoming_offsets) = rayon::join(
        || into_offsets(outgoing_cursors),
        || into_offsets(incoming_cursors),
    );
    if stable_order {
        rayon::join(
            || sort_adjacencies(&outgoing_offsets, &mut outgoing_edges, 0),
            || sort_adjacencies(&incoming_offsets, &mut incoming_edges, 0),
        );
    }
    Ok((
        Csr {
            offsets: outgoing_offsets,
            edges: outgoing_edges,
        },
        Csr {
            offsets: incoming_offsets,
            edges: incoming_edges,
        },
    ))
}

pub(super) fn count_cursors(
    node_count: usize,
    endpoints: &[EdgeEndpoints],
) -> Result<(Vec<AtomicU32>, Vec<AtomicU32>)> {
    let (mut outgoing, mut incoming) =
        rayon::join(|| atomic_zeros(node_count), || atomic_zeros(node_count));
    endpoints
        .par_iter()
        .copied()
        .enumerate()
        .try_for_each(|(edge, endpoints)| -> Result<()> {
            increment(&outgoing, endpoints.source().index(), edge, node_count)?;
            increment(&incoming, endpoints.target().index(), edge, node_count)?;
            Ok(())
        })?;
    prefix_cursors(&mut outgoing, endpoints.len())?;
    prefix_cursors(&mut incoming, endpoints.len())?;
    Ok((outgoing, incoming))
}

pub(super) fn into_offsets(cursors: Vec<AtomicU32>) -> Vec<u32> {
    let ends = cursors
        .into_par_iter()
        .map(AtomicU32::into_inner)
        .collect::<Vec<_>>();
    let mut offsets = Vec::with_capacity(ends.len() + 1);
    offsets.push(0);
    offsets.extend(ends);
    offsets
}

fn atomic_zeros(len: usize) -> Vec<AtomicU32> {
    let mut values = Vec::with_capacity(len.saturating_add(1));
    values.par_extend((0..len).into_par_iter().map(|_| AtomicU32::new(0)));
    values
}

fn into_edges(values: Vec<AtomicU32>) -> Vec<EdgeIndex> {
    values
        .into_par_iter()
        .map(|slot| EdgeIndex::new(slot.into_inner()))
        .collect()
}

fn prefix_cursors(cursors: &mut [AtomicU32], capacity_hint: usize) -> Result<()> {
    let mut total = 0_u32;
    for cursor in cursors {
        let degree = cursor.load(Ordering::Relaxed);
        cursor.store(total, Ordering::Relaxed);
        total = total
            .checked_add(degree)
            .ok_or(GraphError::IndexCapacityExceeded {
                category: "csr entries",
                count: capacity_hint,
            })?;
    }
    Ok(())
}

fn increment(counts: &[AtomicU32], node: usize, edge: usize, node_count: usize) -> Result<()> {
    let Some(count) = counts.get(node) else {
        return Err(GraphError::InvalidTopologyEndpoint {
            edge,
            node,
            node_count,
        });
    };
    count.fetch_add(1, Ordering::Relaxed);
    Ok(())
}
