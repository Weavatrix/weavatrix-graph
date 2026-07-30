use super::TraversalCache;
use super::core::{NeighborCsr, NeighborStorage, OffsetStorage};
use super::elias_fano::EliasFano;
use super::packed::PackedU32;
use super::walk::TraversalCacheWorkspace;
use crate::{Direction, NodeIndex};

impl TraversalCache {
    /// Runs a materialized BFS without allocating after the workspace has grown.
    #[must_use]
    pub fn bfs_with_workspace<'workspace>(
        &self,
        start: NodeIndex,
        direction: Direction,
        workspace: &'workspace mut TraversalCacheWorkspace,
    ) -> &'workspace [NodeIndex] {
        workspace.begin(self.node_count());
        if !self.contains(start) {
            return &workspace.visited;
        }
        workspace.mark(start);
        workspace.visited.push(start);
        match direction {
            Direction::Outgoing => bfs_adjacency(&self.outgoing, workspace),
            Direction::Incoming => bfs_adjacency(&self.incoming, workspace),
            Direction::Both => bfs_both(self, workspace),
        }
        &workspace.visited
    }
}

fn bfs_adjacency(adjacency: &NeighborCsr, workspace: &mut TraversalCacheWorkspace) {
    match (&adjacency.offsets, &adjacency.neighbors) {
        (OffsetStorage::Direct(offsets), NeighborStorage::Direct(neighbors)) => {
            bfs_direct(offsets, neighbors, workspace);
        }
        (OffsetStorage::Direct(offsets), NeighborStorage::Packed(neighbors)) => {
            bfs_packed(offsets, neighbors, workspace);
        }
        (OffsetStorage::EliasFano(offsets), NeighborStorage::Packed(neighbors)) => {
            bfs_succinct(offsets, neighbors, workspace);
        }
        (OffsetStorage::EliasFano(offsets), NeighborStorage::Adaptive(neighbors)) => {
            bfs_adaptive(offsets, neighbors, workspace);
        }
        (OffsetStorage::Direct(_), NeighborStorage::Adaptive(_))
        | (OffsetStorage::EliasFano(_), NeighborStorage::Direct(_)) => {
            bfs_flexible(adjacency, workspace);
        }
    }
}

fn bfs_flexible(adjacency: &NeighborCsr, workspace: &mut TraversalCacheWorkspace) {
    let mut cursor = 0;
    while cursor < workspace.visited.len() {
        let node = workspace.visited[cursor];
        cursor += 1;
        append_adjacency(adjacency, node, workspace);
    }
}

fn bfs_adaptive(
    offsets: &EliasFano,
    neighbors: &super::adaptive::AdaptivePackedU32,
    workspace: &mut TraversalCacheWorkspace,
) {
    let mut cursor = 0;
    while cursor < workspace.visited.len() {
        let node = workspace.visited[cursor].index();
        cursor += 1;
        let start = offsets.get(node) as usize;
        let end = offsets.get(node + 1) as usize;
        neighbors.for_each(start, end, |raw| {
            push_unseen(NodeIndex::new(raw), workspace);
        });
    }
}

fn bfs_direct(offsets: &[u32], neighbors: &[u32], workspace: &mut TraversalCacheWorkspace) {
    let mut cursor = 0;
    while cursor < workspace.visited.len() {
        let node = workspace.visited[cursor].index();
        cursor += 1;
        let start = offsets[node] as usize;
        let end = offsets[node + 1] as usize;
        for &raw in &neighbors[start..end] {
            let neighbor = NodeIndex::new(raw);
            if workspace.mark(neighbor) {
                workspace.visited.push(neighbor);
            }
        }
    }
}

fn bfs_packed(offsets: &[u32], neighbors: &PackedU32, workspace: &mut TraversalCacheWorkspace) {
    let mut cursor = 0;
    while cursor < workspace.visited.len() {
        let node = workspace.visited[cursor].index();
        cursor += 1;
        let start = offsets[node] as usize;
        let end = offsets[node + 1] as usize;
        append_packed(neighbors, start, end, workspace);
    }
}

fn bfs_succinct(
    offsets: &EliasFano,
    neighbors: &PackedU32,
    workspace: &mut TraversalCacheWorkspace,
) {
    let mut cursor = 0;
    while cursor < workspace.visited.len() {
        let node = workspace.visited[cursor].index();
        cursor += 1;
        let start = offsets.get(node) as usize;
        let end = offsets.get(node + 1) as usize;
        append_packed(neighbors, start, end, workspace);
    }
}

fn bfs_both(cache: &TraversalCache, workspace: &mut TraversalCacheWorkspace) {
    let mut cursor = 0;
    while cursor < workspace.visited.len() {
        let node = workspace.visited[cursor];
        cursor += 1;
        append_adjacency(&cache.outgoing, node, workspace);
        append_adjacency(&cache.incoming, node, workspace);
    }
}

fn append_adjacency(
    adjacency: &NeighborCsr,
    node: NodeIndex,
    workspace: &mut TraversalCacheWorkspace,
) {
    let start = adjacency.offsets.get(node.index()) as usize;
    let end = adjacency.offsets.get(node.index() + 1) as usize;
    match &adjacency.neighbors {
        NeighborStorage::Direct(values) => {
            for &raw in &values[start..end] {
                push_unseen(NodeIndex::new(raw), workspace);
            }
        }
        NeighborStorage::Packed(values) => append_packed(values, start, end, workspace),
        NeighborStorage::Adaptive(values) => values.for_each(start, end, |raw| {
            push_unseen(NodeIndex::new(raw), workspace);
        }),
    }
}

fn append_packed(
    neighbors: &PackedU32,
    start: usize,
    end: usize,
    workspace: &mut TraversalCacheWorkspace,
) {
    neighbors.for_each(start, end, |raw| {
        push_unseen(NodeIndex::new(raw), workspace);
    });
}

#[inline]
fn push_unseen(neighbor: NodeIndex, workspace: &mut TraversalCacheWorkspace) {
    if workspace.mark(neighbor) {
        workspace.visited.push(neighbor);
    }
}
