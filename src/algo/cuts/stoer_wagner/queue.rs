use crate::{Measure, Vec};
use core::cmp::Ordering;

#[derive(Clone, Copy)]
struct Entry<M> {
    node: usize,
    weight: M,
}

pub(super) struct MaxQueue<M> {
    heap: Vec<Entry<M>>,
}

impl<M: Measure> MaxQueue<M> {
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            heap: Vec::with_capacity(capacity),
        }
    }

    pub(super) fn clear(&mut self) {
        self.heap.clear();
    }

    pub(super) fn push(&mut self, node: usize, weight: M) {
        self.heap.push(Entry { node, weight });
        let mut child = self.heap.len() - 1;
        while child > 0 {
            let parent = (child - 1) / 2;
            if !higher(self.heap[child], self.heap[parent]) {
                break;
            }
            self.heap.swap(parent, child);
            child = parent;
        }
    }

    pub(super) fn pop(&mut self) -> Option<(usize, M)> {
        let root = *self.heap.first()?;
        let last = self.heap.pop().expect("nonempty heap");
        if !self.heap.is_empty() {
            self.heap[0] = last;
            let mut parent = 0;
            loop {
                let left = parent * 2 + 1;
                if left >= self.heap.len() {
                    break;
                }
                let right = left + 1;
                let child = if right < self.heap.len() && higher(self.heap[right], self.heap[left])
                {
                    right
                } else {
                    left
                };
                if !higher(self.heap[child], self.heap[parent]) {
                    break;
                }
                self.heap.swap(parent, child);
                parent = child;
            }
        }
        Some((root.node, root.weight))
    }
}

fn higher<M: Measure>(left: Entry<M>, right: Entry<M>) -> bool {
    match left.weight.compare(right.weight) {
        Some(Ordering::Greater) => true,
        Some(Ordering::Equal) => left.node < right.node,
        _ => false,
    }
}
