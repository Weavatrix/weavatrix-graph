use super::walk::{TraversalWorkspace, bfs_iter_filtered, dfs_iter_filtered};
use crate::IndexGraphView;
use crate::Vec;
use alloc::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    Outgoing,
    Incoming,
    Both,
}

#[must_use]
pub fn bfs<G>(graph: &G, start: G::Node) -> Vec<G::Node>
where
    G: IndexGraphView,
{
    if !graph.contains_node(start) {
        return Vec::new();
    }
    let mut seen = vec![false; graph.node_bound()];
    let mut visited = Vec::with_capacity(graph.node_count());
    seen[G::node_slot(start)] = true;
    visited.push(start);
    let mut cursor = 0;
    while cursor < visited.len() {
        let node = visited[cursor];
        cursor += 1;
        for edge in graph.outgoing_edges(node) {
            let Some(endpoints) = graph.edge_endpoints(edge) else {
                continue;
            };
            let neighbor = endpoints.target();
            let slot = G::node_slot(neighbor);
            if !seen[slot] {
                seen[slot] = true;
                visited.push(neighbor);
            }
        }
    }
    visited
}

#[must_use]
pub fn bfs_filtered<G, F>(
    graph: &G,
    start: G::Node,
    direction: Direction,
    keep_edge: F,
) -> Vec<G::Node>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    let mut workspace = TraversalWorkspace::new();
    bfs_iter_filtered(graph, start, direction, &mut workspace, keep_edge).collect()
}

#[must_use]
pub fn dfs<G>(graph: &G, start: G::Node) -> Vec<G::Node>
where
    G: IndexGraphView,
{
    dfs_filtered(graph, start, Direction::Outgoing, |_| true)
}

#[must_use]
pub fn dfs_filtered<G, F>(
    graph: &G,
    start: G::Node,
    direction: Direction,
    keep_edge: F,
) -> Vec<G::Node>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    let mut workspace = TraversalWorkspace::new();
    dfs_iter_filtered(graph, start, direction, &mut workspace, keep_edge).collect()
}

#[must_use]
pub fn reachable<G>(graph: &G, source: G::Node, target: G::Node) -> bool
where
    G: IndexGraphView,
{
    reachable_filtered(graph, source, target, Direction::Outgoing, |_| true)
}

pub fn reachable_filtered<G, F>(
    graph: &G,
    source: G::Node,
    target: G::Node,
    direction: Direction,
    keep_edge: F,
) -> bool
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    graph.contains_node(target)
        && bfs_filtered(graph, source, direction, keep_edge)
            .into_iter()
            .any(|node| node == target)
}

#[must_use]
pub fn shortest_path<G>(graph: &G, source: G::Node, target: G::Node) -> Option<Vec<G::Node>>
where
    G: IndexGraphView,
{
    shortest_path_filtered(graph, source, target, Direction::Outgoing, |_| true)
}

pub fn shortest_path_filtered<G, F>(
    graph: &G,
    source: G::Node,
    target: G::Node,
    direction: Direction,
    mut keep_edge: F,
) -> Option<Vec<G::Node>>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    if !graph.contains_node(source) || !graph.contains_node(target) {
        return None;
    }
    let mut predecessor = vec![None; graph.node_bound()];
    let mut seen = vec![false; graph.node_bound()];
    let mut queue = VecDeque::with_capacity(graph.node_count());
    seen[G::node_slot(source)] = true;
    queue.push_back(source);
    while let Some(node) = queue.pop_front() {
        if node == target {
            return Some(reconstruct_path::<G>(source, target, &predecessor));
        }
        for_each_neighbor(graph, node, direction, &mut keep_edge, |neighbor| {
            let slot = G::node_slot(neighbor);
            if !seen[slot] {
                seen[slot] = true;
                predecessor[slot] = Some(node);
                queue.push_back(neighbor);
            }
        });
    }
    None
}

fn reconstruct_path<G: IndexGraphView>(
    source: G::Node,
    target: G::Node,
    predecessor: &[Option<G::Node>],
) -> Vec<G::Node> {
    let mut path = vec![target];
    let mut cursor = target;
    while cursor != source {
        cursor = predecessor[G::node_slot(cursor)].expect("visited nodes have predecessors");
        path.push(cursor);
    }
    path.reverse();
    path
}

pub(super) fn for_each_neighbor<G, F, V>(
    graph: &G,
    node: G::Node,
    direction: Direction,
    keep_edge: &mut F,
    mut visit: V,
) where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
    V: FnMut(G::Node),
{
    for_each_adjacent(graph, node, direction, keep_edge, |_, neighbor| {
        visit(neighbor);
    });
}

pub(super) fn for_each_adjacent<G, F, V>(
    graph: &G,
    node: G::Node,
    direction: Direction,
    keep_edge: &mut F,
    mut visit: V,
) where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
    V: FnMut(G::Edge, G::Node),
{
    if matches!(direction, Direction::Outgoing | Direction::Both) {
        for edge in graph.outgoing_edges(node).filter(|edge| keep_edge(*edge)) {
            if let Some(endpoints) = graph.edge_endpoints(edge) {
                visit(edge, endpoints.target());
            }
        }
    }
    if matches!(direction, Direction::Incoming | Direction::Both) {
        for edge in graph.incoming_edges(node).filter(|edge| keep_edge(*edge)) {
            if let Some(endpoints) = graph.edge_endpoints(edge) {
                visit(edge, endpoints.source());
            }
        }
    }
}
