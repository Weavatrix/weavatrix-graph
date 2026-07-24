use super::undirected_snapshot::UndirectedSnapshot;
use crate::{IndexUndirectedGraphView, Vec};

/// One oriented edge in a DFS chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainStep<Node, Edge> {
    edge: Edge,
    source: Node,
    target: Node,
}

impl<Node: Copy, Edge: Copy> ChainStep<Node, Edge> {
    /// Returns the original graph edge.
    #[must_use]
    pub const fn edge(&self) -> Edge {
        self.edge
    }

    /// Returns the step source in chain orientation.
    #[must_use]
    pub const fn source(&self) -> Node {
        self.source
    }

    /// Returns the step target in chain orientation.
    #[must_use]
    pub const fn target(&self) -> Node {
        self.target
    }
}

/// Deterministic edge-disjoint chains of an undirected DFS forest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainDecomposition<Node, Edge> {
    chains: Vec<Vec<ChainStep<Node, Edge>>>,
}

impl<Node, Edge> ChainDecomposition<Node, Edge> {
    /// Returns chains in deterministic DFS discovery order.
    #[must_use]
    pub fn chains(&self) -> &[Vec<ChainStep<Node, Edge>>] {
        &self.chains
    }

    /// Returns the number of chains.
    #[must_use]
    pub fn chain_count(&self) -> usize {
        self.chains.len()
    }

    /// Consumes the result and returns its chains.
    #[must_use]
    pub fn into_chains(self) -> Vec<Vec<ChainStep<Node, Edge>>> {
        self.chains
    }
}

/// Computes a chain decomposition of every connected component in `O(V + E)`.
///
/// Unlike simple-graph-only variants, this preserves parallel edge identities
/// and represents a self-loop as a one-edge chain.
#[must_use]
pub fn chain_decomposition<G>(graph: &G) -> ChainDecomposition<G::Node, G::Edge>
where
    G: IndexUndirectedGraphView,
{
    chain_decomposition_filtered(graph, |_| true)
}

/// Computes chains using accepted edges only.
///
/// The predicate is evaluated exactly once per edge.
#[must_use]
pub fn chain_decomposition_filtered<G, F>(
    graph: &G,
    allows_edge: F,
) -> ChainDecomposition<G::Node, G::Edge>
where
    G: IndexUndirectedGraphView,
    F: Fn(G::Edge) -> bool,
{
    let snapshot = UndirectedSnapshot::new(graph, allows_edge);
    let roots = snapshot.nodes().to_vec();
    decompose(graph, &snapshot, &roots)
}

/// Computes chains only in the connected component containing `source`.
///
/// Returns `None` when `source` is absent.
#[must_use]
pub fn chain_decomposition_from<G>(
    graph: &G,
    source: G::Node,
) -> Option<ChainDecomposition<G::Node, G::Edge>>
where
    G: IndexUndirectedGraphView,
{
    chain_decomposition_from_filtered(graph, source, |_| true)
}

/// Computes accepted-edge chains in the component containing `source`.
///
/// The predicate is evaluated exactly once per edge. Returns `None` when
/// `source` is absent.
#[must_use]
pub fn chain_decomposition_from_filtered<G, F>(
    graph: &G,
    source: G::Node,
    allows_edge: F,
) -> Option<ChainDecomposition<G::Node, G::Edge>>
where
    G: IndexUndirectedGraphView,
    F: Fn(G::Edge) -> bool,
{
    if !graph.contains_node(source) {
        return None;
    }
    let snapshot = UndirectedSnapshot::new(graph, allows_edge);
    Some(decompose(graph, &snapshot, &[source]))
}

fn decompose<G>(
    graph: &G,
    snapshot: &UndirectedSnapshot<G>,
    roots: &[G::Node],
) -> ChainDecomposition<G::Node, G::Edge>
where
    G: IndexUndirectedGraphView,
{
    let mut forest = DfsForest::<G>::new(graph);
    for &root in roots {
        if forest.discovery[G::node_slot(root)].is_none() {
            forest.search_from(graph, snapshot, root);
        }
    }
    forest.build_chains()
}

#[derive(Clone, Copy)]
struct BackEdge<Node, Edge> {
    descendant: Node,
    edge: Edge,
}

struct Frame<Node, Edge> {
    node: Node,
    parent_edge: Option<Edge>,
    next: usize,
}

struct DfsForest<G>
where
    G: IndexUndirectedGraphView,
{
    discovery: Vec<Option<usize>>,
    parent_node: Vec<Option<G::Node>>,
    parent_edge: Vec<Option<G::Edge>>,
    order: Vec<G::Node>,
    back_edges: Vec<Vec<BackEdge<G::Node, G::Edge>>>,
    seen_self_loop: Vec<bool>,
}

impl<G> DfsForest<G>
where
    G: IndexUndirectedGraphView,
{
    fn new(graph: &G) -> Self {
        Self {
            discovery: vec![None; graph.node_bound()],
            parent_node: vec![None; graph.node_bound()],
            parent_edge: vec![None; graph.node_bound()],
            order: Vec::with_capacity(graph.node_count()),
            back_edges: (0..graph.node_bound()).map(|_| Vec::new()).collect(),
            seen_self_loop: vec![false; graph.edge_bound()],
        }
    }

    fn search_from(&mut self, graph: &G, snapshot: &UndirectedSnapshot<G>, root: G::Node) {
        self.discover(root, None, None);
        let mut frames = vec![Frame {
            node: root,
            parent_edge: None,
            next: 0,
        }];
        while let Some(frame) = frames.last_mut() {
            let incident = snapshot.incident(frame.node);
            if frame.next == incident.len() {
                frames.pop();
                continue;
            }
            let edge = incident[frame.next];
            frame.next += 1;
            if Some(edge) == frame.parent_edge {
                continue;
            }
            let node = frame.node;
            let neighbor = graph.opposite(edge, node).expect("incident edge has node");
            if neighbor == node {
                let edge_slot = G::edge_slot(edge);
                if !self.seen_self_loop[edge_slot] {
                    self.seen_self_loop[edge_slot] = true;
                    self.back_edges[G::node_slot(node)].push(BackEdge {
                        descendant: node,
                        edge,
                    });
                }
                continue;
            }
            let slot = G::node_slot(node);
            let neighbor_slot = G::node_slot(neighbor);
            if self.discovery[neighbor_slot].is_none() {
                self.discover(neighbor, Some(node), Some(edge));
                frames.push(Frame {
                    node: neighbor,
                    parent_edge: Some(edge),
                    next: 0,
                });
            } else if self.discovery[neighbor_slot] < self.discovery[slot] {
                self.back_edges[neighbor_slot].push(BackEdge {
                    descendant: node,
                    edge,
                });
            }
        }
    }

    fn discover(&mut self, node: G::Node, parent: Option<G::Node>, edge: Option<G::Edge>) {
        let slot = G::node_slot(node);
        self.discovery[slot] = Some(self.order.len());
        self.parent_node[slot] = parent;
        self.parent_edge[slot] = edge;
        self.order.push(node);
    }

    fn build_chains(self) -> ChainDecomposition<G::Node, G::Edge> {
        let mut visited = vec![false; self.discovery.len()];
        let mut chains = Vec::new();
        for ancestor in self.order {
            visited[G::node_slot(ancestor)] = true;
            for back in &self.back_edges[G::node_slot(ancestor)] {
                let mut chain = vec![ChainStep {
                    edge: back.edge,
                    source: ancestor,
                    target: back.descendant,
                }];
                let mut cursor = back.descendant;
                while !visited[G::node_slot(cursor)] {
                    visited[G::node_slot(cursor)] = true;
                    let slot = G::node_slot(cursor);
                    let parent = self.parent_node[slot].expect("unvisited descendant has parent");
                    chain.push(ChainStep {
                        edge: self.parent_edge[slot].expect("non-root has parent edge"),
                        source: cursor,
                        target: parent,
                    });
                    cursor = parent;
                }
                chains.push(chain);
            }
        }
        ChainDecomposition { chains }
    }
}
