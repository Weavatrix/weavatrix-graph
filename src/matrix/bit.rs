use crate::Vec;
use crate::{EdgeEndpoints, GraphError, NodeIndex, Result, Topology};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

#[cfg(feature = "unsafe-fast")]
mod unsafe_fast;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BitMatrix {
    node_count: u32,
    edge_count: usize,
    words: Vec<u64>,
}

impl BitMatrix {
    /// Creates an empty directed bit-packed adjacency matrix.
    ///
    /// # Errors
    ///
    /// Returns an error when its dimensions exceed index or address capacity.
    pub fn try_new(node_count: usize) -> Result<Self> {
        let compact = u32::try_from(node_count).map_err(|_| GraphError::IndexCapacityExceeded {
            category: "bit matrix nodes",
            count: node_count,
        })?;
        let cells = node_count
            .checked_mul(node_count)
            .ok_or(GraphError::ArithmeticOverflow {
                operation: "bit matrix dimensions",
            })?;
        let word_count = cells
            .checked_add(63)
            .ok_or(GraphError::ArithmeticOverflow {
                operation: "bit matrix word count",
            })?
            / 64;
        Ok(Self {
            node_count: compact,
            edge_count: 0,
            words: vec![0; word_count],
        })
    }

    #[must_use]
    pub const fn node_count(&self) -> usize {
        self.node_count as usize
    }

    #[must_use]
    pub const fn edge_count(&self) -> usize {
        self.edge_count
    }

    #[must_use]
    pub fn storage_bytes(&self) -> usize {
        self.words.len() * size_of::<u64>()
    }

    #[must_use]
    #[inline]
    pub fn contains(&self, source: NodeIndex, target: NodeIndex) -> bool {
        let count = self.node_count();
        let source = source.index();
        let target = target.index();
        if source >= count || target >= count {
            return false;
        }
        let slot = source * count + target;
        self.words[slot / 64] & (1_u64 << (slot % 64)) != 0
    }

    /// Inserts one directed edge and reports whether it was new.
    ///
    /// # Errors
    ///
    /// Returns an error when either endpoint is outside the matrix.
    pub fn insert(&mut self, source: NodeIndex, target: NodeIndex) -> Result<bool> {
        let slot = self.slot(source, target)?;
        let mask = 1_u64 << (slot % 64);
        let word = &mut self.words[slot / 64];
        let inserted = *word & mask == 0;
        *word |= mask;
        self.edge_count += usize::from(inserted);
        Ok(inserted)
    }

    /// Inserts both directions of one undirected relation.
    ///
    /// # Errors
    ///
    /// Returns an error when either endpoint is outside the matrix.
    pub fn insert_undirected(&mut self, left: NodeIndex, right: NodeIndex) -> Result<usize> {
        let first = usize::from(self.insert(left, right)?);
        let second = usize::from(left != right && self.insert(right, left)?);
        Ok(first + second)
    }

    pub fn remove(&mut self, source: NodeIndex, target: NodeIndex) -> bool {
        let Ok(slot) = self.slot(source, target) else {
            return false;
        };
        let mask = 1_u64 << (slot % 64);
        let word = &mut self.words[slot / 64];
        let removed = *word & mask != 0;
        *word &= !mask;
        self.edge_count -= usize::from(removed);
        removed
    }

    pub fn outgoing(&self, source: NodeIndex) -> impl Iterator<Item = NodeIndex> + '_ {
        let valid = source.index() < self.node_count();
        (0..self.node_count()).filter_map(move |target| {
            let target = NodeIndex::new(u32::try_from(target).ok()?);
            (valid && self.contains(source, target)).then_some(target)
        })
    }

    pub fn incoming(&self, target: NodeIndex) -> impl Iterator<Item = NodeIndex> + '_ {
        let valid = target.index() < self.node_count();
        (0..self.node_count()).filter_map(move |source| {
            let source = NodeIndex::new(u32::try_from(source).ok()?);
            (valid && self.contains(source, target)).then_some(source)
        })
    }

    pub fn edges(&self) -> impl Iterator<Item = EdgeEndpoints> + '_ {
        self.words
            .iter()
            .enumerate()
            .flat_map(|(word, bits)| SetBits::new(word * 64, *bits))
            .filter_map(|slot| {
                let source = u32::try_from(slot / self.node_count()).ok()?;
                let target = u32::try_from(slot % self.node_count()).ok()?;
                Some(EdgeEndpoints::new(
                    NodeIndex::new(source),
                    NodeIndex::new(target),
                ))
            })
    }

    /// Materializes a compact dual-CSR topology.
    ///
    /// # Errors
    ///
    /// Returns an error only if the matrix violates topology capacity.
    pub fn to_topology(&self) -> Result<Topology> {
        Topology::try_from_edges(self.node_count(), self.edges())
    }

    fn slot(&self, source: NodeIndex, target: NodeIndex) -> Result<usize> {
        let count = self.node_count();
        for node in [source, target] {
            if node.index() >= count {
                return Err(GraphError::InvalidNodeIndex {
                    node: node.index(),
                    node_count: count,
                });
            }
        }
        Ok(source.index() * count + target.index())
    }
}

struct SetBits {
    base: usize,
    bits: u64,
}

impl SetBits {
    const fn new(base: usize, bits: u64) -> Self {
        Self { base, bits }
    }
}

impl Iterator for SetBits {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bits == 0 {
            return None;
        }
        let offset = usize::try_from(self.bits.trailing_zeros()).unwrap_or(usize::MAX);
        self.bits &= self.bits - 1;
        Some(self.base + offset)
    }
}

#[derive(Deserialize)]
struct BitWire {
    node_count: u32,
    #[serde(default, rename = "edge_count")]
    _edge_count: usize,
    words: Vec<u64>,
}

impl<'de> Deserialize<'de> for BitMatrix {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BitWire::deserialize(deserializer)?;
        let mut matrix = Self::try_new(wire.node_count as usize).map_err(D::Error::custom)?;
        if matrix.words.len() != wire.words.len() {
            return Err(D::Error::custom("invalid bit matrix word count"));
        }
        matrix.words = wire.words;
        let cells = matrix.node_count() * matrix.node_count();
        if let Some(last) = matrix.words.last() {
            let used = cells % 64;
            if used != 0 && *last & !((1_u64 << used) - 1) != 0 {
                return Err(D::Error::custom("bit matrix contains out-of-range bits"));
            }
        }
        matrix.edge_count = matrix
            .words
            .iter()
            .map(|word| usize::try_from(word.count_ones()).unwrap_or(usize::MAX))
            .sum();
        Ok(matrix)
    }
}
