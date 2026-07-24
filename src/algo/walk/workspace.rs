use crate::Vec;
use alloc::collections::VecDeque;

/// Reusable allocation storage for breadth-first and depth-first traversals.
#[derive(Debug, Clone)]
pub struct TraversalWorkspace<Node> {
    marks: Vec<u32>,
    epoch: u32,
    pub(super) queue: VecDeque<Node>,
    pub(super) stack: Vec<Node>,
    pub(super) scratch: Vec<Node>,
}

impl<Node> TraversalWorkspace<Node> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            marks: Vec::new(),
            epoch: 0,
            queue: VecDeque::new(),
            stack: Vec::new(),
            scratch: Vec::new(),
        }
    }

    pub(super) fn begin(&mut self, node_bound: usize) {
        if self.marks.len() < node_bound {
            self.marks.resize(node_bound, 0);
        }
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.marks.fill(0);
            self.epoch = 1;
        }
        self.queue.clear();
        self.stack.clear();
        self.scratch.clear();
    }

    pub(super) fn mark(&mut self, slot: usize) -> bool {
        let Some(mark) = self.marks.get_mut(slot) else {
            return false;
        };
        if *mark == self.epoch {
            return false;
        }
        *mark = self.epoch;
        true
    }
}

impl<Node> Default for TraversalWorkspace<Node> {
    fn default() -> Self {
        Self::new()
    }
}
