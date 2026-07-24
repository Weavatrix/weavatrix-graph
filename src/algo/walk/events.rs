use crate::IndexGraphView;
use crate::Vec;
use crate::algo::traversal::{Direction, for_each_adjacent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalControl {
    Continue,
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DfsEvent<Node, Edge> {
    Discover(Node),
    TreeEdge {
        edge: Edge,
        source: Node,
        target: Node,
    },
    BackEdge {
        edge: Edge,
        source: Node,
        target: Node,
    },
    CrossForwardEdge {
        edge: Edge,
        source: Node,
        target: Node,
    },
    Finish(Node),
}

#[derive(Debug, Clone)]
struct Frame<Node, Edge> {
    node: Node,
    adjacent: Vec<(Edge, Node)>,
    next: usize,
}

/// Reusable color map and iterative stack for DFS event traversal.
#[derive(Debug, Clone)]
pub struct DfsEventWorkspace<Node, Edge> {
    colors: Vec<u8>,
    stack: Vec<Frame<Node, Edge>>,
}

impl<Node, Edge> DfsEventWorkspace<Node, Edge> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            colors: Vec::new(),
            stack: Vec::new(),
        }
    }

    fn begin(&mut self, node_bound: usize) {
        self.colors.resize(node_bound, 0);
        self.colors.fill(0);
        self.stack.clear();
    }
}

impl<Node, Edge> Default for DfsEventWorkspace<Node, Edge> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn depth_first_search<G, I, V>(
    graph: &G,
    starts: I,
    workspace: &mut DfsEventWorkspace<G::Node, G::Edge>,
    visitor: V,
) -> bool
where
    G: IndexGraphView,
    I: IntoIterator<Item = G::Node>,
    V: FnMut(DfsEvent<G::Node, G::Edge>) -> TraversalControl,
{
    depth_first_search_filtered(
        graph,
        starts,
        Direction::Outgoing,
        workspace,
        |_| true,
        visitor,
    )
}

pub fn depth_first_search_filtered<G, I, F, V>(
    graph: &G,
    starts: I,
    direction: Direction,
    workspace: &mut DfsEventWorkspace<G::Node, G::Edge>,
    mut keep_edge: F,
    mut visitor: V,
) -> bool
where
    G: IndexGraphView,
    I: IntoIterator<Item = G::Node>,
    F: FnMut(G::Edge) -> bool,
    V: FnMut(DfsEvent<G::Node, G::Edge>) -> TraversalControl,
{
    workspace.begin(graph.node_bound());
    for root in starts {
        if !graph.contains_node(root) || color::<G>(workspace, root) != 0 {
            continue;
        }
        if !discover(
            graph,
            root,
            direction,
            workspace,
            &mut keep_edge,
            &mut visitor,
        ) {
            return false;
        }
        while !workspace.stack.is_empty() {
            let frame_index = workspace.stack.len() - 1;
            let next = workspace.stack[frame_index].next;
            if let Some(&(edge, target)) = workspace.stack[frame_index].adjacent.get(next) {
                let source = workspace.stack[frame_index].node;
                workspace.stack[frame_index].next += 1;
                let event = match color::<G>(workspace, target) {
                    0 => DfsEvent::TreeEdge {
                        edge,
                        source,
                        target,
                    },
                    1 => DfsEvent::BackEdge {
                        edge,
                        source,
                        target,
                    },
                    _ => DfsEvent::CrossForwardEdge {
                        edge,
                        source,
                        target,
                    },
                };
                if visitor(event) == TraversalControl::Break {
                    return false;
                }
                if color::<G>(workspace, target) == 0
                    && !discover(
                        graph,
                        target,
                        direction,
                        workspace,
                        &mut keep_edge,
                        &mut visitor,
                    )
                {
                    return false;
                }
            } else {
                let Some(frame) = workspace.stack.pop() else {
                    break;
                };
                workspace.colors[G::node_slot(frame.node)] = 2;
                if visitor(DfsEvent::Finish(frame.node)) == TraversalControl::Break {
                    return false;
                }
            }
        }
    }
    true
}

fn discover<G, F, V>(
    graph: &G,
    node: G::Node,
    direction: Direction,
    workspace: &mut DfsEventWorkspace<G::Node, G::Edge>,
    keep_edge: &mut F,
    visitor: &mut V,
) -> bool
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
    V: FnMut(DfsEvent<G::Node, G::Edge>) -> TraversalControl,
{
    workspace.colors[G::node_slot(node)] = 1;
    if visitor(DfsEvent::Discover(node)) == TraversalControl::Break {
        return false;
    }
    let mut adjacent = Vec::new();
    for_each_adjacent(graph, node, direction, keep_edge, |edge, target| {
        adjacent.push((edge, target));
    });
    workspace.stack.push(Frame {
        node,
        adjacent,
        next: 0,
    });
    true
}

fn color<G>(workspace: &DfsEventWorkspace<G::Node, G::Edge>, node: G::Node) -> u8
where
    G: IndexGraphView,
{
    workspace
        .colors
        .get(G::node_slot(node))
        .copied()
        .unwrap_or(2)
}
