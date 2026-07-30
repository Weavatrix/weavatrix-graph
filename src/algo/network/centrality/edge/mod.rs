#![allow(clippy::cast_precision_loss)]

#[cfg(feature = "rayon")]
mod parallel;

#[cfg(feature = "rayon")]
pub use parallel::{
    edge_betweenness_centrality_parallel, undirected_edge_betweenness_centrality_parallel,
};

use crate::algo::traversal::Direction;
use crate::{IndexGraphView, IndexUndirectedGraphView, Vec};
use alloc::collections::VecDeque;

#[derive(Clone, Copy)]
pub(super) struct Arc {
    node: usize,
    edge: usize,
}

pub(super) struct BrandesGraph<Edge> {
    pub(super) source_slots: Vec<usize>,
    pub(super) accepted_edges: Vec<(Edge, usize)>,
    pub(super) arcs_by_node: Vec<Vec<Arc>>,
}

pub(super) struct BrandesWorkspace {
    stack: Vec<usize>,
    predecessors: Vec<Vec<Arc>>,
    paths: Vec<f64>,
    distances: Vec<usize>,
    dependency: Vec<f64>,
    queue: VecDeque<usize>,
}

impl BrandesWorkspace {
    pub(super) fn new(node_bound: usize) -> Self {
        Self {
            stack: Vec::with_capacity(node_bound),
            predecessors: vec![Vec::new(); node_bound],
            paths: vec![0.0; node_bound],
            distances: vec![usize::MAX; node_bound],
            dependency: vec![0.0; node_bound],
            queue: VecDeque::with_capacity(node_bound),
        }
    }

    fn reset(&mut self, source: usize) {
        self.stack.clear();
        self.queue.clear();
        self.paths.fill(0.0);
        self.distances.fill(usize::MAX);
        self.dependency.fill(0.0);
        for predecessors in &mut self.predecessors {
            predecessors.clear();
        }
        self.paths[source] = 1.0;
        self.distances[source] = 0;
        self.queue.push_back(source);
    }
}

/// Computes unweighted Brandes edge betweenness for a directed graph view.
///
/// Parallel edges are distinct shortest-path alternatives. Self-loops have a
/// zero score. `Direction::Both` treats every accepted edge as undirected.
#[must_use]
pub fn edge_betweenness_centrality<G>(
    graph: &G,
    direction: Direction,
    normalized: bool,
) -> Vec<(G::Edge, f64)>
where
    G: IndexGraphView,
{
    edge_betweenness_centrality_filtered(graph, direction, normalized, |_| true)
}

/// Filtered edge betweenness. The predicate is evaluated once per edge.
#[must_use]
pub fn edge_betweenness_centrality_filtered<G, F>(
    graph: &G,
    direction: Direction,
    normalized: bool,
    allows_edge: F,
) -> Vec<(G::Edge, f64)>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    let adjacent = directed_adjacency(graph, direction, allows_edge);
    calculate(&adjacent, direction == Direction::Both, normalized)
}

/// Computes unweighted Brandes edge betweenness for an undirected graph view.
#[must_use]
pub fn undirected_edge_betweenness_centrality<G>(graph: &G, normalized: bool) -> Vec<(G::Edge, f64)>
where
    G: IndexUndirectedGraphView,
{
    undirected_edge_betweenness_centrality_filtered(graph, normalized, |_| true)
}

/// Filtered undirected edge betweenness. The predicate runs once per edge.
#[must_use]
pub fn undirected_edge_betweenness_centrality_filtered<G, F>(
    graph: &G,
    normalized: bool,
    allows_edge: F,
) -> Vec<(G::Edge, f64)>
where
    G: IndexUndirectedGraphView,
    F: FnMut(G::Edge) -> bool,
{
    let adjacent = undirected_adjacency(graph, allows_edge);
    calculate(&adjacent, true, normalized)
}

fn calculate<Edge: Copy>(
    adjacent: &BrandesGraph<Edge>,
    undirected: bool,
    normalized: bool,
) -> Vec<(Edge, f64)> {
    let mut scores = vec![0.0; edge_score_capacity(adjacent)];
    let mut workspace = BrandesWorkspace::new(adjacent.arcs_by_node.len());
    for &source in &adjacent.source_slots {
        brandes_source(adjacent, source, &mut workspace, &mut scores);
    }
    scale(
        &mut scores,
        adjacent.source_slots.len(),
        undirected,
        normalized,
    );
    adjacent
        .accepted_edges
        .iter()
        .copied()
        .map(|(edge, slot)| (edge, scores[slot]))
        .collect()
}

pub(super) fn brandes_source<Edge>(
    adjacent: &BrandesGraph<Edge>,
    source: usize,
    workspace: &mut BrandesWorkspace,
    scores: &mut [f64],
) {
    workspace.reset(source);
    while let Some(node) = workspace.queue.pop_front() {
        workspace.stack.push(node);
        for &arc in &adjacent.arcs_by_node[node] {
            if workspace.distances[arc.node] == usize::MAX {
                workspace.distances[arc.node] = workspace.distances[node] + 1;
                workspace.queue.push_back(arc.node);
            }
            if workspace.distances[arc.node] == workspace.distances[node] + 1 {
                workspace.paths[arc.node] += workspace.paths[node];
                workspace.predecessors[arc.node].push(Arc {
                    node,
                    edge: arc.edge,
                });
            }
        }
    }
    while let Some(node) = workspace.stack.pop() {
        for &arc in &workspace.predecessors[node] {
            let contribution = workspace.paths[arc.node] / workspace.paths[node]
                * (1.0 + workspace.dependency[node]);
            workspace.dependency[arc.node] += contribution;
            scores[arc.edge] += contribution;
        }
    }
}

pub(super) fn scale(scores: &mut [f64], node_count: usize, undirected: bool, normalized: bool) {
    let factor = if normalized && node_count > 1 {
        1.0 / (node_count * (node_count - 1)) as f64
    } else if undirected {
        0.5
    } else {
        1.0
    };
    for score in scores {
        *score *= factor;
    }
}

pub(super) fn directed_adjacency<G, F>(
    graph: &G,
    direction: Direction,
    mut allows_edge: F,
) -> BrandesGraph<G::Edge>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    let mut nodes = graph.node_indices().map(G::node_slot).collect::<Vec<_>>();
    nodes.sort_unstable();
    let mut edges = Vec::new();
    let mut neighbors = vec![Vec::new(); graph.node_bound()];
    for edge in graph.edge_indices() {
        if !allows_edge(edge) {
            continue;
        }
        let slot = G::edge_slot(edge);
        edges.push((edge, slot));
        let Some(endpoints) = graph.edge_endpoints(edge) else {
            continue;
        };
        let source = G::node_slot(endpoints.source());
        let target = G::node_slot(endpoints.target());
        add_directed_arcs(&mut neighbors, source, target, slot, direction);
    }
    edges.sort_unstable_by_key(|&(_, slot)| slot);
    sort_arcs(&mut neighbors);
    BrandesGraph {
        source_slots: nodes,
        accepted_edges: edges,
        arcs_by_node: neighbors,
    }
}

fn add_directed_arcs(
    neighbors: &mut [Vec<Arc>],
    source: usize,
    target: usize,
    edge: usize,
    direction: Direction,
) {
    if source == target {
        return;
    }
    if matches!(direction, Direction::Outgoing | Direction::Both) {
        neighbors[source].push(Arc { node: target, edge });
    }
    if matches!(direction, Direction::Incoming | Direction::Both) {
        neighbors[target].push(Arc { node: source, edge });
    }
}

pub(super) fn undirected_adjacency<G, F>(graph: &G, mut allows_edge: F) -> BrandesGraph<G::Edge>
where
    G: IndexUndirectedGraphView,
    F: FnMut(G::Edge) -> bool,
{
    let mut nodes = graph.node_indices().map(G::node_slot).collect::<Vec<_>>();
    nodes.sort_unstable();
    let mut edges = Vec::new();
    let mut neighbors = vec![Vec::new(); graph.node_bound()];
    for edge in graph.edge_indices() {
        if !allows_edge(edge) {
            continue;
        }
        let slot = G::edge_slot(edge);
        edges.push((edge, slot));
        let Some(endpoints) = graph.edge_endpoints(edge) else {
            continue;
        };
        let source = G::node_slot(endpoints.source());
        let target = G::node_slot(endpoints.target());
        if source != target {
            neighbors[source].push(Arc {
                node: target,
                edge: slot,
            });
            neighbors[target].push(Arc {
                node: source,
                edge: slot,
            });
        }
    }
    edges.sort_unstable_by_key(|&(_, slot)| slot);
    sort_arcs(&mut neighbors);
    BrandesGraph {
        source_slots: nodes,
        accepted_edges: edges,
        arcs_by_node: neighbors,
    }
}

fn sort_arcs(neighbors: &mut [Vec<Arc>]) {
    for row in neighbors {
        row.sort_unstable_by_key(|arc| (arc.node, arc.edge));
    }
}

pub(super) fn edge_score_capacity<Edge>(graph: &BrandesGraph<Edge>) -> usize {
    graph.accepted_edges.last().map_or(0, |(_, slot)| slot + 1)
}
