use crate::{IndexUndirectedGraphView, Vec};

/// Deterministic vertex-biconnected edge blocks and their cut vertices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BiconnectedComponents<Node, Edge> {
    components: Vec<Vec<Edge>>,
    articulation_points: Vec<Node>,
}

impl<Node, Edge> BiconnectedComponents<Node, Edge> {
    /// Returns canonical edge blocks, ordered by their smallest edge index.
    #[must_use]
    pub fn components(&self) -> &[Vec<Edge>] {
        &self.components
    }

    /// Returns the number of edge blocks.
    #[must_use]
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Returns cut vertices in canonical node-index order.
    #[must_use]
    pub fn articulation_points(&self) -> &[Node] {
        &self.articulation_points
    }

    /// Consumes the result and returns its canonical edge blocks.
    #[must_use]
    pub fn into_components(self) -> Vec<Vec<Edge>> {
        self.components
    }
}

/// Finds vertex-biconnected edge blocks and articulation points.
#[must_use]
pub fn biconnected_components<G>(graph: &G) -> BiconnectedComponents<G::Node, G::Edge>
where
    G: IndexUndirectedGraphView,
{
    biconnected_components_with_allowed::<G, false>(graph, Vec::new())
}

/// Finds vertex-biconnected edge blocks using accepted edges only.
///
/// The predicate is evaluated exactly once per edge. Isolated vertices do not
/// form edge blocks; a self-loop forms its own block.
#[must_use]
pub fn biconnected_components_filtered<G, F>(
    graph: &G,
    allows_edge: F,
) -> BiconnectedComponents<G::Node, G::Edge>
where
    G: IndexUndirectedGraphView,
    F: Fn(G::Edge) -> bool,
{
    let mut allowed = vec![false; graph.edge_bound()];
    for edge in graph.edge_indices() {
        allowed[G::edge_slot(edge)] = allows_edge(edge);
    }
    biconnected_components_with_allowed::<G, true>(graph, allowed)
}

fn biconnected_components_with_allowed<G, const FILTERED: bool>(
    graph: &G,
    allowed: Vec<bool>,
) -> BiconnectedComponents<G::Node, G::Edge>
where
    G: IndexUndirectedGraphView,
{
    let mut nodes = graph.node_indices().collect::<Vec<_>>();
    nodes.sort_unstable_by_key(|node| G::node_slot(*node));
    let mut state = State::<G, FILTERED>::new(graph, allowed);
    for root in nodes {
        if state.discovery[G::node_slot(root)] == usize::MAX {
            state.search_from(graph, root);
        }
    }
    state.finish(graph)
}

struct Frame<Node, Edge> {
    node: Node,
    parent_edge: Option<Edge>,
    next: usize,
    degree: usize,
    children: usize,
}

struct State<G, const FILTERED: bool>
where
    G: IndexUndirectedGraphView,
{
    time: usize,
    discovery: Vec<usize>,
    low: Vec<usize>,
    articulation: Vec<bool>,
    allowed: Vec<bool>,
    seen_self_loop: Vec<bool>,
    edge_stack: Vec<G::Edge>,
    edge_component: Vec<usize>,
    component_count: usize,
}

impl<G, const FILTERED: bool> State<G, FILTERED>
where
    G: IndexUndirectedGraphView,
{
    fn new(graph: &G, allowed: Vec<bool>) -> Self {
        Self {
            time: 0,
            discovery: vec![usize::MAX; graph.node_bound()],
            low: vec![0; graph.node_bound()],
            articulation: vec![false; graph.node_bound()],
            allowed,
            seen_self_loop: vec![false; graph.edge_bound()],
            edge_stack: Vec::new(),
            edge_component: vec![usize::MAX; graph.edge_bound()],
            component_count: 0,
        }
    }

    fn search_from(&mut self, graph: &G, root: G::Node) {
        self.discover(root);
        let mut frames = vec![Self::frame(graph, root, None)];
        while let Some(frame) = frames.last_mut() {
            let next = {
                if frame.next == frame.degree {
                    None
                } else {
                    let edge = graph.incident_edge_at(frame.node, frame.next);
                    frame.next += 1;
                    edge.map(|edge| (frame.node, frame.parent_edge, edge))
                }
            };
            if let Some((node, parent_edge, edge)) = next {
                self.explore_edge(graph, &mut frames, node, parent_edge, edge);
            } else {
                let Some(frame) = frames.pop() else {
                    break;
                };
                self.finish_frame(frames.last_mut(), &frame);
            }
        }
        self.edge_stack.clear();
    }

    fn explore_edge(
        &mut self,
        graph: &G,
        frames: &mut Vec<Frame<G::Node, G::Edge>>,
        node: G::Node,
        parent_edge: Option<G::Edge>,
        edge: G::Edge,
    ) {
        let edge_slot = G::edge_slot(edge);
        if FILTERED && !self.allowed[edge_slot] {
            return;
        }
        if Some(edge) == parent_edge {
            return;
        }
        let Some(neighbor) = graph.opposite(edge, node) else {
            return;
        };
        if neighbor == node {
            if !self.seen_self_loop[edge_slot] {
                self.seen_self_loop[edge_slot] = true;
                self.edge_component[edge_slot] = self.component_count;
                self.component_count += 1;
            }
            return;
        }
        let slot = G::node_slot(node);
        let neighbor_slot = G::node_slot(neighbor);
        if self.discovery[neighbor_slot] == usize::MAX {
            let Some(parent) = frames.last_mut() else {
                return;
            };
            parent.children += 1;
            self.edge_stack.push(edge);
            self.discover(neighbor);
            frames.push(Self::frame(graph, neighbor, Some(edge)));
        } else if self.discovery[neighbor_slot] < self.discovery[slot] {
            self.edge_stack.push(edge);
            self.low[slot] = self.low[slot].min(self.discovery[neighbor_slot]);
        }
    }

    fn finish_frame(
        &mut self,
        parent: Option<&mut Frame<G::Node, G::Edge>>,
        frame: &Frame<G::Node, G::Edge>,
    ) {
        let slot = G::node_slot(frame.node);
        let Some(tree_edge) = frame.parent_edge else {
            if frame.children > 1 {
                self.articulation[slot] = true;
            }
            return;
        };
        let Some(parent) = parent else {
            return;
        };
        let parent_slot = G::node_slot(parent.node);
        self.low[parent_slot] = self.low[parent_slot].min(self.low[slot]);
        let parent_discovery = self.discovery[parent_slot];
        if self.low[slot] >= parent_discovery {
            if parent.parent_edge.is_some() {
                self.articulation[parent_slot] = true;
            }
            self.pop_component(tree_edge);
        }
    }

    fn frame(graph: &G, node: G::Node, parent_edge: Option<G::Edge>) -> Frame<G::Node, G::Edge> {
        Frame {
            node,
            parent_edge,
            next: 0,
            degree: graph.incident_edges(node).len(),
            children: 0,
        }
    }

    fn discover(&mut self, node: G::Node) {
        let slot = G::node_slot(node);
        self.discovery[slot] = self.time;
        self.low[slot] = self.time;
        self.time += 1;
    }

    fn pop_component(&mut self, stop: G::Edge) {
        let component = self.component_count;
        self.component_count += 1;
        while let Some(edge) = self.edge_stack.pop() {
            self.edge_component[G::edge_slot(edge)] = component;
            if edge == stop {
                break;
            }
        }
    }

    fn finish(self, graph: &G) -> BiconnectedComponents<G::Node, G::Edge> {
        let mut counts = vec![0; self.component_count];
        for &component in &self.edge_component {
            if component != usize::MAX {
                counts[component] += 1;
            }
        }
        let mut components = counts
            .into_iter()
            .map(Vec::with_capacity)
            .collect::<Vec<_>>();
        let mut edges = graph.edge_indices().collect::<Vec<_>>();
        if !edges.is_sorted_by_key(|edge| G::edge_slot(*edge)) {
            edges.sort_unstable_by_key(|edge| G::edge_slot(*edge));
        }
        for edge in edges {
            let component = self.edge_component[G::edge_slot(edge)];
            if component != usize::MAX {
                components[component].push(edge);
            }
        }
        components.retain(|component| !component.is_empty());
        components.sort_unstable_by_key(|component| G::edge_slot(component[0]));
        let mut articulation_points = graph
            .node_indices()
            .filter(|node| self.articulation[G::node_slot(*node)])
            .collect::<Vec<_>>();
        articulation_points.sort_unstable_by_key(|node| G::node_slot(*node));
        BiconnectedComponents {
            components,
            articulation_points,
        }
    }
}
