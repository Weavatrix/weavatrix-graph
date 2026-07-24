#![allow(unsafe_code)]
#![allow(
    clippy::inline_always,
    reason = "these feature-gated leaf lookups are benchmarked hot-path APIs"
)]

use super::BitMatrix;
use crate::NodeIndex;

impl BitMatrix {
    /// Checks adjacency using an audited unchecked word access internally.
    ///
    /// Unlike [`Self::contains_unchecked`], this method remains safe for
    /// arbitrary indexes. It is available only with the `unsafe-fast` feature.
    #[must_use]
    #[inline(always)]
    pub fn contains_fast(&self, source: NodeIndex, target: NodeIndex) -> bool {
        let count = self.node_count();
        if source.index() >= count || target.index() >= count {
            return false;
        }
        // SAFETY: Both compact endpoint indexes were checked against the
        // immutable matrix dimension immediately above.
        unsafe { self.contains_unchecked(source, target) }
    }

    /// Checks adjacency without validating either endpoint.
    ///
    /// # Safety
    ///
    /// Both `source.index()` and `target.index()` must be strictly less than
    /// [`Self::node_count`]. Violating this contract may access memory outside
    /// the bit storage.
    #[must_use]
    #[inline(always)]
    pub unsafe fn contains_unchecked(&self, source: NodeIndex, target: NodeIndex) -> bool {
        debug_assert!(source.index() < self.node_count());
        debug_assert!(target.index() < self.node_count());
        let slot = source.index() * self.node_count() + target.index();
        // SAFETY: The caller guarantees valid endpoints, and construction
        // allocates one bit for every valid flattened endpoint pair.
        let word = unsafe { self.words.get_unchecked(slot / 64) };
        word & (1_u64 << (slot % 64)) != 0
    }
}
