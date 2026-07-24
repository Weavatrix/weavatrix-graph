use super::TraversalWorkspace;
use crate::IndexGraphView;
use crate::algo::traversal::{Direction, for_each_neighbor};

fn accept_all<Edge>(_: Edge) -> bool {
    true
}

/// Lazy breadth-first traversal backed by reusable allocation storage.
pub struct Bfs<'graph, 'workspace, G, F>
where
    G: IndexGraphView,
{
    graph: &'graph G,
    workspace: &'workspace mut TraversalWorkspace<G::Node>,
    direction: Direction,
    keep_edge: F,
}

impl<'graph, 'workspace, G> Bfs<'graph, 'workspace, G, fn(G::Edge) -> bool>
where
    G: IndexGraphView,
{
    #[must_use]
    pub fn new(
        graph: &'graph G,
        start: G::Node,
        workspace: &'workspace mut TraversalWorkspace<G::Node>,
    ) -> Self {
        Self::filtered(graph, start, Direction::Outgoing, workspace, accept_all)
    }
}

impl<'graph, 'workspace, G, F> Bfs<'graph, 'workspace, G, F>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    #[must_use]
    pub fn filtered(
        graph: &'graph G,
        start: G::Node,
        direction: Direction,
        workspace: &'workspace mut TraversalWorkspace<G::Node>,
        keep_edge: F,
    ) -> Self {
        workspace.begin(graph.node_bound());
        if graph.contains_node(start) && workspace.mark(G::node_slot(start)) {
            workspace.queue.push_back(start);
        }
        Self {
            graph,
            workspace,
            direction,
            keep_edge,
        }
    }
}

impl<G, F> Iterator for Bfs<'_, '_, G, F>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    type Item = G::Node;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.workspace.queue.pop_front()?;
        let workspace = &mut *self.workspace;
        for_each_neighbor(
            self.graph,
            node,
            self.direction,
            &mut self.keep_edge,
            |neighbor| {
                if workspace.mark(G::node_slot(neighbor)) {
                    workspace.queue.push_back(neighbor);
                }
            },
        );
        Some(node)
    }
}

/// Lazy depth-first traversal backed by reusable allocation storage.
pub struct Dfs<'graph, 'workspace, G, F>
where
    G: IndexGraphView,
{
    graph: &'graph G,
    workspace: &'workspace mut TraversalWorkspace<G::Node>,
    direction: Direction,
    keep_edge: F,
}

impl<'graph, 'workspace, G> Dfs<'graph, 'workspace, G, fn(G::Edge) -> bool>
where
    G: IndexGraphView,
{
    #[must_use]
    pub fn new(
        graph: &'graph G,
        start: G::Node,
        workspace: &'workspace mut TraversalWorkspace<G::Node>,
    ) -> Self {
        Self::filtered(graph, start, Direction::Outgoing, workspace, accept_all)
    }
}

impl<'graph, 'workspace, G, F> Dfs<'graph, 'workspace, G, F>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    #[must_use]
    pub fn filtered(
        graph: &'graph G,
        start: G::Node,
        direction: Direction,
        workspace: &'workspace mut TraversalWorkspace<G::Node>,
        keep_edge: F,
    ) -> Self {
        workspace.begin(graph.node_bound());
        if graph.contains_node(start) && workspace.mark(G::node_slot(start)) {
            workspace.stack.push(start);
        }
        Self {
            graph,
            workspace,
            direction,
            keep_edge,
        }
    }
}

impl<G, F> Iterator for Dfs<'_, '_, G, F>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    type Item = G::Node;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.workspace.stack.pop()?;
        self.workspace.scratch.clear();
        let workspace = &mut *self.workspace;
        for_each_neighbor(
            self.graph,
            node,
            self.direction,
            &mut self.keep_edge,
            |neighbor| {
                if workspace.mark(G::node_slot(neighbor)) {
                    workspace.scratch.push(neighbor);
                }
            },
        );
        while let Some(neighbor) = self.workspace.scratch.pop() {
            self.workspace.stack.push(neighbor);
        }
        Some(node)
    }
}

#[must_use]
pub fn bfs_iter<'graph, 'workspace, G>(
    graph: &'graph G,
    start: G::Node,
    workspace: &'workspace mut TraversalWorkspace<G::Node>,
) -> Bfs<'graph, 'workspace, G, fn(G::Edge) -> bool>
where
    G: IndexGraphView,
{
    Bfs::new(graph, start, workspace)
}

#[must_use]
pub fn bfs_iter_filtered<'graph, 'workspace, G, F>(
    graph: &'graph G,
    start: G::Node,
    direction: Direction,
    workspace: &'workspace mut TraversalWorkspace<G::Node>,
    keep_edge: F,
) -> Bfs<'graph, 'workspace, G, F>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    Bfs::filtered(graph, start, direction, workspace, keep_edge)
}

#[must_use]
pub fn dfs_iter<'graph, 'workspace, G>(
    graph: &'graph G,
    start: G::Node,
    workspace: &'workspace mut TraversalWorkspace<G::Node>,
) -> Dfs<'graph, 'workspace, G, fn(G::Edge) -> bool>
where
    G: IndexGraphView,
{
    Dfs::new(graph, start, workspace)
}

#[must_use]
pub fn dfs_iter_filtered<'graph, 'workspace, G, F>(
    graph: &'graph G,
    start: G::Node,
    direction: Direction,
    workspace: &'workspace mut TraversalWorkspace<G::Node>,
    keep_edge: F,
) -> Dfs<'graph, 'workspace, G, F>
where
    G: IndexGraphView,
    F: FnMut(G::Edge) -> bool,
{
    Dfs::filtered(graph, start, direction, workspace, keep_edge)
}
