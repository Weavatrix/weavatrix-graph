use super::TraversalCache;
use crate::{Direction, NodeIndex, Vec};
use alloc::collections::VecDeque;

/// Reusable allocation storage for cache-backed BFS and DFS.
#[derive(Debug, Clone)]
pub struct TraversalCacheWorkspace {
    pub(super) marks: Vec<u32>,
    predecessor: Vec<Option<NodeIndex>>,
    epoch: u32,
    queue: VecDeque<NodeIndex>,
    stack: Vec<NodeIndex>,
    scratch: Vec<NodeIndex>,
    pub(super) visited: Vec<NodeIndex>,
}

impl TraversalCacheWorkspace {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            marks: Vec::new(),
            predecessor: Vec::new(),
            epoch: 0,
            queue: VecDeque::new(),
            stack: Vec::new(),
            scratch: Vec::new(),
            visited: Vec::new(),
        }
    }

    pub(super) fn begin(&mut self, node_count: usize) {
        if self.marks.len() < node_count {
            self.marks.resize(node_count, 0);
        }
        self.predecessor.resize(node_count, None);
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.marks.fill(0);
            self.epoch = 1;
        }
        self.queue.clear();
        self.stack.clear();
        self.scratch.clear();
        self.visited.clear();
    }

    #[inline]
    pub(super) fn mark(&mut self, node: NodeIndex) -> bool {
        let mark = &mut self.marks[node.index()];
        if *mark == self.epoch {
            return false;
        }
        *mark = self.epoch;
        true
    }
}

impl Default for TraversalCacheWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

/// Lazy cache-backed breadth-first traversal.
pub struct CacheBfs<'cache, 'workspace> {
    cache: &'cache TraversalCache,
    workspace: &'workspace mut TraversalCacheWorkspace,
    direction: Direction,
}

impl<'cache, 'workspace> CacheBfs<'cache, 'workspace> {
    fn new(
        cache: &'cache TraversalCache,
        start: NodeIndex,
        direction: Direction,
        workspace: &'workspace mut TraversalCacheWorkspace,
    ) -> Self {
        workspace.begin(cache.node_count());
        if cache.contains(start) && workspace.mark(start) {
            workspace.queue.push_back(start);
        }
        Self {
            cache,
            workspace,
            direction,
        }
    }
}

impl Iterator for CacheBfs<'_, '_> {
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.workspace.queue.pop_front()?;
        for_each_neighbor(self.cache, node, self.direction, |neighbor| {
            if self.workspace.mark(neighbor) {
                self.workspace.queue.push_back(neighbor);
            }
        });
        Some(node)
    }
}

/// Lazy cache-backed depth-first traversal.
pub struct CacheDfs<'cache, 'workspace> {
    cache: &'cache TraversalCache,
    workspace: &'workspace mut TraversalCacheWorkspace,
    direction: Direction,
}

impl<'cache, 'workspace> CacheDfs<'cache, 'workspace> {
    fn new(
        cache: &'cache TraversalCache,
        start: NodeIndex,
        direction: Direction,
        workspace: &'workspace mut TraversalCacheWorkspace,
    ) -> Self {
        workspace.begin(cache.node_count());
        if cache.contains(start) && workspace.mark(start) {
            workspace.stack.push(start);
        }
        Self {
            cache,
            workspace,
            direction,
        }
    }
}

impl Iterator for CacheDfs<'_, '_> {
    type Item = NodeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.workspace.stack.pop()?;
        self.workspace.scratch.clear();
        for_each_neighbor(self.cache, node, self.direction, |neighbor| {
            if self.workspace.mark(neighbor) {
                self.workspace.scratch.push(neighbor);
            }
        });
        while let Some(neighbor) = self.workspace.scratch.pop() {
            self.workspace.stack.push(neighbor);
        }
        Some(node)
    }
}

impl TraversalCache {
    #[must_use]
    pub fn bfs_iter<'cache, 'workspace>(
        &'cache self,
        start: NodeIndex,
        direction: Direction,
        workspace: &'workspace mut TraversalCacheWorkspace,
    ) -> CacheBfs<'cache, 'workspace> {
        CacheBfs::new(self, start, direction, workspace)
    }

    #[must_use]
    pub fn dfs_iter<'cache, 'workspace>(
        &'cache self,
        start: NodeIndex,
        direction: Direction,
        workspace: &'workspace mut TraversalCacheWorkspace,
    ) -> CacheDfs<'cache, 'workspace> {
        CacheDfs::new(self, start, direction, workspace)
    }

    #[must_use]
    pub fn bfs(&self, start: NodeIndex, direction: Direction) -> Vec<NodeIndex> {
        self.bfs_with_workspace(start, direction, &mut TraversalCacheWorkspace::new())
            .to_vec()
    }

    #[must_use]
    pub fn dfs(&self, start: NodeIndex, direction: Direction) -> Vec<NodeIndex> {
        self.dfs_iter(start, direction, &mut TraversalCacheWorkspace::new())
            .collect()
    }

    #[must_use]
    pub fn reachable(&self, source: NodeIndex, target: NodeIndex, direction: Direction) -> bool {
        self.contains(target)
            && self
                .bfs_iter(source, direction, &mut TraversalCacheWorkspace::new())
                .any(|node| node == target)
    }

    #[must_use]
    pub fn shortest_path(
        &self,
        source: NodeIndex,
        target: NodeIndex,
        direction: Direction,
        workspace: &mut TraversalCacheWorkspace,
    ) -> Option<Vec<NodeIndex>> {
        if !self.contains(source) || !self.contains(target) {
            return None;
        }
        workspace.begin(self.node_count());
        workspace.mark(source);
        workspace.queue.push_back(source);
        while let Some(node) = workspace.queue.pop_front() {
            if node == target {
                return Some(reconstruct(source, target, &workspace.predecessor));
            }
            for_each_neighbor(self, node, direction, |neighbor| {
                if workspace.mark(neighbor) {
                    workspace.predecessor[neighbor.index()] = Some(node);
                    workspace.queue.push_back(neighbor);
                }
            });
        }
        None
    }
}

pub(super) fn for_each_neighbor(
    cache: &TraversalCache,
    node: NodeIndex,
    direction: Direction,
    mut visit: impl FnMut(NodeIndex),
) {
    if matches!(direction, Direction::Outgoing | Direction::Both) {
        cache.for_each_outgoing(node, &mut visit);
    }
    if matches!(direction, Direction::Incoming | Direction::Both) {
        cache.for_each_incoming(node, visit);
    }
}

fn reconstruct(
    source: NodeIndex,
    target: NodeIndex,
    predecessor: &[Option<NodeIndex>],
) -> Vec<NodeIndex> {
    let mut path = vec![target];
    let mut cursor = target;
    while cursor != source {
        cursor = predecessor[cursor.index()].expect("visited nodes have predecessors");
        path.push(cursor);
    }
    path.reverse();
    path
}
