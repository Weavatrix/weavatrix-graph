#![allow(unsafe_code)]

use super::parallel::count_cursors;
use super::{Csr, sort_adjacencies};
use crate::{EdgeEndpoints, EdgeIndex, GraphError, Result, Vec};
use rayon::prelude::*;
use std::{
    mem::ManuallyDrop,
    sync::atomic::{AtomicU32, Ordering},
};

pub(super) fn try_build_pair(
    node_count: usize,
    endpoints: &[EdgeEndpoints],
    stable_order: bool,
) -> Result<(Csr, Csr)> {
    let (outgoing_cursors, incoming_cursors) = count_cursors(node_count, endpoints)?;
    let mut outgoing_edges = Vec::<EdgeIndex>::with_capacity(endpoints.len());
    let mut incoming_edges = Vec::<EdgeIndex>::with_capacity(endpoints.len());
    let outgoing_slots = SharedSlots(outgoing_edges.as_mut_ptr());
    let incoming_slots = SharedSlots(incoming_edges.as_mut_ptr());
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
            let edge = EdgeIndex::new(edge);
            // SAFETY: each fetch_add returns a unique in-capacity slot.
            unsafe {
                outgoing_slots.write(outgoing, edge);
                incoming_slots.write(incoming, edge);
            }
            Ok(())
        })?;
    // SAFETY: the parallel loop initialized every allocated slot exactly once.
    unsafe {
        outgoing_edges.set_len(endpoints.len());
        incoming_edges.set_len(endpoints.len());
    }
    let (outgoing_offsets, incoming_offsets) = rayon::join(
        || into_offsets_fast(outgoing_cursors),
        || into_offsets_fast(incoming_cursors),
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

fn into_offsets_fast(cursors: Vec<AtomicU32>) -> Vec<u32> {
    if size_of::<AtomicU32>() != size_of::<u32>() || align_of::<AtomicU32>() != align_of::<u32>() {
        return super::parallel::into_offsets(cursors);
    }
    let mut cursors = ManuallyDrop::new(cursors);
    let (pointer, length, capacity) = (
        cursors.as_mut_ptr().cast::<u32>(),
        cursors.len(),
        cursors.capacity(),
    );
    // SAFETY: the checked layouts match and every atomic is exclusively owned.
    let mut offsets = unsafe { Vec::from_raw_parts(pointer, length, capacity) };
    offsets.push(0);
    offsets.rotate_right(1);
    offsets[0] = 0;
    offsets
}

#[derive(Clone, Copy)]
struct SharedSlots(*mut EdgeIndex);

unsafe impl Send for SharedSlots {}
unsafe impl Sync for SharedSlots {}

impl SharedSlots {
    unsafe fn write(self, index: usize, edge: EdgeIndex) {
        // SAFETY: the caller owns a unique, allocated slot.
        unsafe { self.0.add(index).write(edge) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_write_and_cursor_reuse_preserve_layout() {
        let mut edges = Vec::<EdgeIndex>::with_capacity(3);
        let slots = SharedSlots(edges.as_mut_ptr());
        for (index, edge) in [0, 1, 2].into_iter().enumerate() {
            // SAFETY: each in-capacity slot is written exactly once.
            unsafe {
                slots.write(index, EdgeIndex::new(edge));
            }
        }
        // SAFETY: all three allocated slots were initialized above.
        unsafe { edges.set_len(3) };
        assert_eq!(
            edges,
            [EdgeIndex::new(0), EdgeIndex::new(1), EdgeIndex::new(2)]
        );

        let mut cursors = Vec::with_capacity(4);
        cursors.extend([2, 4, 5].map(AtomicU32::new));
        assert_eq!(into_offsets_fast(cursors), [0, 2, 4, 5]);
    }
}
