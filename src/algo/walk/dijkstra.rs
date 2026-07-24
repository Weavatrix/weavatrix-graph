use super::super::measure::Measure;
use super::super::traversal::{Direction, for_each_adjacent};
use crate::{GraphError, IndexGraphView, Result, String, Vec};
use alloc::collections::BinaryHeap;
use core::cmp::Ordering;

#[derive(Debug, Clone, Copy)]
struct Scored<Cost> {
    cost: Cost,
    slot: usize,
}

impl<Cost: Measure> PartialEq for Scored<Cost> {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot && self.cost.compare(other.cost) == Some(Ordering::Equal)
    }
}

impl<Cost: Measure> Eq for Scored<Cost> {}

impl<Cost: Measure> PartialOrd for Scored<Cost> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Cost: Measure> Ord for Scored<Cost> {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .compare(self.cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| other.slot.cmp(&self.slot))
    }
}

/// Reusable distances, predecessors, node slots, and priority queue.
#[derive(Debug, Clone)]
pub struct DijkstraWorkspace<Node, Cost> {
    distances: Vec<Option<Cost>>,
    predecessors: Vec<Option<Node>>,
    nodes: Vec<Option<Node>>,
    queue: BinaryHeap<Scored<Cost>>,
}

impl<Node, Cost: Measure> DijkstraWorkspace<Node, Cost> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            distances: Vec::new(),
            predecessors: Vec::new(),
            nodes: Vec::new(),
            queue: BinaryHeap::new(),
        }
    }

    #[must_use]
    pub fn distance_at(&self, slot: usize) -> Option<Cost>
    where
        Cost: Copy,
    {
        self.distances.get(slot).copied().flatten()
    }

    #[must_use]
    pub fn predecessor_at(&self, slot: usize) -> Option<Node>
    where
        Node: Copy,
    {
        self.predecessors.get(slot).copied().flatten()
    }

    #[must_use]
    pub fn path_to<G>(&self, source: Node, target: Node) -> Option<Vec<Node>>
    where
        G: IndexGraphView<Node = Node>,
        Node: Copy + Eq,
        Cost: Copy,
    {
        self.distance_at(G::node_slot(target))?;
        let mut path = vec![target];
        let mut cursor = target;
        while cursor != source {
            cursor = self.predecessor_at(G::node_slot(cursor))?;
            path.push(cursor);
        }
        path.reverse();
        Some(path)
    }

    fn begin<G>(&mut self, graph: &G, source: Node)
    where
        G: IndexGraphView<Node = Node>,
        Node: Copy,
        Cost: Measure,
    {
        let bound = graph.node_bound();
        self.distances.resize(bound, None);
        self.predecessors.resize(bound, None);
        self.nodes.resize(bound, None);
        self.distances.fill(None);
        self.predecessors.fill(None);
        self.nodes.fill(None);
        self.queue.clear();
        for node in graph.node_indices() {
            self.nodes[G::node_slot(node)] = Some(node);
        }
        if graph.contains_node(source) {
            let slot = G::node_slot(source);
            self.distances[slot] = Some(Cost::zero());
            self.queue.push(Scored {
                cost: Cost::zero(),
                slot,
            });
        }
    }
}

impl<Node, Cost: Measure> Default for DijkstraWorkspace<Node, Cost> {
    fn default() -> Self {
        Self::new()
    }
}

/// Lazy Dijkstra settlement order with reusable allocation storage.
pub struct Dijkstra<'graph, 'workspace, G, Cost, F>
where
    G: IndexGraphView,
{
    graph: &'graph G,
    workspace: &'workspace mut DijkstraWorkspace<G::Node, Cost>,
    direction: Direction,
    edge_cost: F,
    failed: bool,
}

impl<'graph, 'workspace, G, Cost, F> Dijkstra<'graph, 'workspace, G, Cost, F>
where
    G: IndexGraphView,
    Cost: Measure,
    F: FnMut(G::Edge) -> Option<Cost>,
{
    #[must_use]
    pub fn filtered(
        graph: &'graph G,
        source: G::Node,
        direction: Direction,
        workspace: &'workspace mut DijkstraWorkspace<G::Node, Cost>,
        edge_cost: F,
    ) -> Self {
        workspace.begin(graph, source);
        Self {
            graph,
            workspace,
            direction,
            edge_cost,
            failed: false,
        }
    }

    fn fail(&mut self, error: GraphError) -> Result<(G::Node, Cost)> {
        self.failed = true;
        self.workspace.queue.clear();
        Err(error)
    }
}

impl<G, Cost, F> Iterator for Dijkstra<'_, '_, G, Cost, F>
where
    G: IndexGraphView,
    Cost: Measure,
    F: FnMut(G::Edge) -> Option<Cost>,
{
    type Item = Result<(G::Node, Cost)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        while let Some(Scored { cost, slot }) = self.workspace.queue.pop() {
            let Some(known) = self.workspace.distances.get(slot).copied().flatten() else {
                continue;
            };
            if known.compare(cost) != Some(Ordering::Equal) {
                continue;
            }
            let Some(node) = self.workspace.nodes.get(slot).copied().flatten() else {
                continue;
            };
            let mut error = None;
            let workspace = &mut *self.workspace;
            for_each_adjacent(
                self.graph,
                node,
                self.direction,
                &mut |_| true,
                |edge, neighbor| {
                    if error.is_some() {
                        return;
                    }
                    let Some(weight) = (self.edge_cost)(edge) else {
                        return;
                    };
                    if !weight.is_valid() || weight.is_negative() {
                        error = Some(invalid_weight());
                        return;
                    }
                    let Some(candidate) = cost.checked_add(weight) else {
                        error = Some(GraphError::ArithmeticOverflow {
                            operation: "generic Dijkstra path cost",
                        });
                        return;
                    };
                    let neighbor_slot = G::node_slot(neighbor);
                    let improves = workspace.distances[neighbor_slot]
                        .is_none_or(|known| candidate.compare(known) == Some(Ordering::Less));
                    if improves {
                        workspace.distances[neighbor_slot] = Some(candidate);
                        workspace.predecessors[neighbor_slot] = Some(node);
                        workspace.queue.push(Scored {
                            cost: candidate,
                            slot: neighbor_slot,
                        });
                    }
                },
            );
            if let Some(error) = error {
                return Some(self.fail(error));
            }
            return Some(Ok((node, cost)));
        }
        None
    }
}

pub fn dijkstra_iter<'graph, 'workspace, G, Cost, F>(
    graph: &'graph G,
    source: G::Node,
    workspace: &'workspace mut DijkstraWorkspace<G::Node, Cost>,
    mut edge_cost: F,
) -> impl Iterator<Item = Result<(G::Node, Cost)>> + 'workspace
where
    'graph: 'workspace,
    G: IndexGraphView + 'graph,
    Cost: Measure + 'workspace,
    F: FnMut(G::Edge) -> Cost + 'workspace,
{
    Dijkstra::filtered(graph, source, Direction::Outgoing, workspace, move |edge| {
        Some(edge_cost(edge))
    })
}

#[must_use]
pub fn dijkstra_iter_filtered<'graph, 'workspace, G, Cost, F>(
    graph: &'graph G,
    source: G::Node,
    direction: Direction,
    workspace: &'workspace mut DijkstraWorkspace<G::Node, Cost>,
    edge_cost: F,
) -> Dijkstra<'graph, 'workspace, G, Cost, F>
where
    G: IndexGraphView,
    Cost: Measure,
    F: FnMut(G::Edge) -> Option<Cost>,
{
    Dijkstra::filtered(graph, source, direction, workspace, edge_cost)
}

fn invalid_weight() -> GraphError {
    GraphError::InvalidAlgorithmParameter {
        algorithm: "Dijkstra",
        parameter: "edge_cost",
        value: String::from("must be finite and non-negative"),
    }
}
