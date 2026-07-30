use super::adaptive::AdaptivePackedU32;
use super::build::{build_neighbor_pair, neighbor_bits, select_storage};
use super::elias_fano::EliasFano;
use super::iter::NeighborIter;
use super::packed::PackedU32;
use crate::{NodeIndex, Topology, Vec};

/// Chooses the speed/space trade-off of a derived traversal cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraversalStorage {
    /// Selects `Balanced` when packing saves at least 12.5%, otherwise `Fast`.
    #[default]
    Auto,
    /// Direct `u32` neighbors and offsets for minimum traversal overhead.
    Fast,
    /// Bit-packed neighbors with direct `u32` offsets.
    Balanced,
    /// Bit-packed neighbors and Elias-Fano monotone offsets.
    Compact,
}

/// Actual physical layout selected for a traversal cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalLayout {
    Fast,
    Balanced {
        neighbor_bits: u8,
    },
    Compact {
        neighbor_bits: u8,
        offset_low_bits: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalCache {
    node_count: usize,
    edge_count: usize,
    pub(super) outgoing: NeighborCsr,
    pub(super) incoming: NeighborCsr,
    layout: TraversalLayout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NeighborCsr {
    pub(super) offsets: OffsetStorage,
    pub(super) neighbors: NeighborStorage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum OffsetStorage {
    Direct(Vec<u32>),
    EliasFano(EliasFano),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NeighborStorage {
    Direct(Vec<u32>),
    Packed(PackedU32),
    Adaptive(AdaptivePackedU32),
}

impl TraversalCache {
    #[must_use]
    pub fn from_topology(topology: &Topology) -> Self {
        Self::with_storage(topology, TraversalStorage::Auto)
    }

    #[must_use]
    pub fn with_storage(topology: &Topology, requested: TraversalStorage) -> Self {
        let (endpoints, outgoing, incoming) = topology.traversal_parts();
        let (out_neighbors, in_neighbors) =
            build_neighbor_pair(endpoints, outgoing.edges(), incoming.edges());
        let bits = neighbor_bits(topology.node_count());
        let selected = select_storage(
            requested,
            topology.node_count(),
            topology.edge_count(),
            bits,
        );
        let (outgoing, layout) =
            NeighborCsr::build(outgoing.offsets(), out_neighbors, selected, bits);
        let (incoming, _) = NeighborCsr::build(incoming.offsets(), in_neighbors, selected, bits);
        Self {
            node_count: topology.node_count(),
            edge_count: topology.edge_count(),
            outgoing,
            incoming,
            layout,
        }
    }

    #[must_use]
    pub const fn node_count(&self) -> usize {
        self.node_count
    }

    #[must_use]
    pub const fn edge_count(&self) -> usize {
        self.edge_count
    }

    #[must_use]
    pub const fn layout(&self) -> TraversalLayout {
        self.layout
    }

    #[must_use]
    pub fn storage_bytes(&self) -> usize {
        self.outgoing.storage_bytes() + self.incoming.storage_bytes()
    }

    #[must_use]
    pub fn fast_equivalent_bytes(&self) -> usize {
        (self.edge_count() * 2 + (self.node_count() + 1) * 2) * size_of::<u32>()
    }

    #[must_use]
    pub fn outgoing_neighbors(&self, node: NodeIndex) -> NeighborIter<'_> {
        self.outgoing.neighbors(node.index(), self.node_count())
    }

    #[must_use]
    pub fn incoming_neighbors(&self, node: NodeIndex) -> NeighborIter<'_> {
        self.incoming.neighbors(node.index(), self.node_count())
    }

    #[must_use]
    pub fn out_degree(&self, node: NodeIndex) -> Option<usize> {
        self.outgoing.degree(node.index(), self.node_count())
    }

    #[must_use]
    pub fn in_degree(&self, node: NodeIndex) -> Option<usize> {
        self.incoming.degree(node.index(), self.node_count())
    }

    pub(super) fn contains(&self, node: NodeIndex) -> bool {
        node.index() < self.node_count()
    }

    pub(super) fn for_each_outgoing(&self, node: NodeIndex, visit: impl FnMut(NodeIndex)) {
        self.outgoing
            .for_each(node.index(), self.node_count(), visit);
    }

    pub(super) fn for_each_incoming(&self, node: NodeIndex, visit: impl FnMut(NodeIndex)) {
        self.incoming
            .for_each(node.index(), self.node_count(), visit);
    }
}

impl NeighborCsr {
    fn build(
        offsets: &[u32],
        neighbors: Vec<u32>,
        storage: TraversalStorage,
        bits: u8,
    ) -> (Self, TraversalLayout) {
        match storage {
            TraversalStorage::Fast | TraversalStorage::Auto => (
                Self {
                    offsets: OffsetStorage::Direct(offsets.to_vec()),
                    neighbors: NeighborStorage::Direct(neighbors),
                },
                TraversalLayout::Fast,
            ),
            TraversalStorage::Balanced => (
                Self {
                    offsets: OffsetStorage::Direct(offsets.to_vec()),
                    neighbors: NeighborStorage::Packed(PackedU32::from_values(&neighbors, bits)),
                },
                TraversalLayout::Balanced {
                    neighbor_bits: bits,
                },
            ),
            TraversalStorage::Compact => {
                let offsets = EliasFano::from_monotone(offsets);
                let offset_low_bits = offsets.low_bits();
                let global_bytes =
                    neighbors.len().div_ceil(64) * usize::from(bits) * size_of::<u64>();
                let neighbor_storage = if AdaptivePackedU32::estimated_storage_bytes(&neighbors)
                    < global_bytes
                {
                    match AdaptivePackedU32::try_from_values(&neighbors) {
                        Some(values) => NeighborStorage::Adaptive(values),
                        None => NeighborStorage::Packed(PackedU32::from_values(&neighbors, bits)),
                    }
                } else {
                    NeighborStorage::Packed(PackedU32::from_values(&neighbors, bits))
                };
                (
                    Self {
                        offsets: OffsetStorage::EliasFano(offsets),
                        neighbors: neighbor_storage,
                    },
                    TraversalLayout::Compact {
                        neighbor_bits: bits,
                        offset_low_bits,
                    },
                )
            }
        }
    }

    fn neighbors(&self, node: usize, node_count: usize) -> NeighborIter<'_> {
        let Some((start, end)) = self.bounds(node, node_count) else {
            return NeighborIter::empty(&self.neighbors);
        };
        NeighborIter::new(&self.neighbors, start, end)
    }

    fn degree(&self, node: usize, node_count: usize) -> Option<usize> {
        self.bounds(node, node_count)
            .map(|(start, end)| end - start)
    }

    fn bounds(&self, node: usize, node_count: usize) -> Option<(usize, usize)> {
        (node < node_count).then(|| {
            (
                self.offsets.get(node) as usize,
                self.offsets.get(node + 1) as usize,
            )
        })
    }

    fn storage_bytes(&self) -> usize {
        self.offsets.storage_bytes() + self.neighbors.storage_bytes()
    }

    fn for_each(&self, node: usize, node_count: usize, visit: impl FnMut(NodeIndex)) {
        let Some((start, end)) = self.bounds(node, node_count) else {
            return;
        };
        self.neighbors.for_each(start, end, visit);
    }
}

impl OffsetStorage {
    #[inline]
    pub(super) fn get(&self, index: usize) -> u32 {
        match self {
            Self::Direct(values) => values[index],
            Self::EliasFano(values) => values.get(index),
        }
    }

    fn storage_bytes(&self) -> usize {
        match self {
            Self::Direct(values) => values.len() * size_of::<u32>(),
            Self::EliasFano(values) => values.storage_bytes(),
        }
    }
}

impl NeighborStorage {
    #[inline]
    pub(super) fn get(&self, index: usize) -> u32 {
        match self {
            Self::Direct(values) => values[index],
            Self::Packed(values) => values.get(index),
            Self::Adaptive(values) => values.get(index),
        }
    }

    fn storage_bytes(&self) -> usize {
        match self {
            Self::Direct(values) => values.len() * size_of::<u32>(),
            Self::Packed(values) => values.storage_bytes(),
            Self::Adaptive(values) => values.storage_bytes(),
        }
    }

    fn for_each(&self, start: usize, end: usize, mut visit: impl FnMut(NodeIndex)) {
        match self {
            Self::Direct(values) => {
                for &neighbor in &values[start..end] {
                    visit(NodeIndex::new(neighbor));
                }
            }
            Self::Packed(values) => {
                values.for_each(start, end, |raw| visit(NodeIndex::new(raw)));
            }
            Self::Adaptive(values) => {
                values.for_each(start, end, |raw| visit(NodeIndex::new(raw)));
            }
        }
    }
}
