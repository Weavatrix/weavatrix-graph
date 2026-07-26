use super::core::NeighborStorage;
use crate::NodeIndex;

/// Exact-size lazy iterator over direct neighbor node indexes.
#[derive(Debug, Clone)]
pub struct NeighborIter<'cache> {
    storage: &'cache NeighborStorage,
    front: usize,
    back: usize,
}

impl<'cache> NeighborIter<'cache> {
    pub(super) const fn new(storage: &'cache NeighborStorage, front: usize, back: usize) -> Self {
        Self {
            storage,
            front,
            back,
        }
    }

    pub(super) const fn empty(storage: &'cache NeighborStorage) -> Self {
        Self::new(storage, 0, 0)
    }
}

impl Iterator for NeighborIter<'_> {
    type Item = NodeIndex;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }
        let value = self.storage.get(self.front);
        self.front += 1;
        Some(NodeIndex::new(value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.back - self.front;
        (remaining, Some(remaining))
    }
}

impl DoubleEndedIterator for NeighborIter<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            return None;
        }
        self.back -= 1;
        Some(NodeIndex::new(self.storage.get(self.back)))
    }
}

impl ExactSizeIterator for NeighborIter<'_> {}
