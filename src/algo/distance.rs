use super::undirected_neighbors::UndirectedNeighbors;
use crate::{IndexUndirectedGraphView, Vec};
use alloc::collections::VecDeque;

/// Exact unweighted distance metrics for a connected undirected graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistanceAnalytics<Node> {
    eccentricities: Vec<(Node, usize)>,
    radius: usize,
    diameter: usize,
    center: Vec<Node>,
    periphery: Vec<Node>,
}

impl<Node: Copy + Eq> DistanceAnalytics<Node> {
    /// Returns `(node, eccentricity)` pairs in canonical node-index order.
    #[must_use]
    pub fn eccentricities(&self) -> &[(Node, usize)] {
        &self.eccentricities
    }

    /// Returns the eccentricity of `node`, if it belongs to the graph.
    #[must_use]
    pub fn eccentricity(&self, node: Node) -> Option<usize> {
        self.eccentricities
            .iter()
            .find_map(|&(candidate, value)| (candidate == node).then_some(value))
    }

    /// Returns the minimum eccentricity.
    #[must_use]
    pub const fn radius(&self) -> usize {
        self.radius
    }

    /// Returns the maximum eccentricity.
    #[must_use]
    pub const fn diameter(&self) -> usize {
        self.diameter
    }

    /// Returns all minimum-eccentricity nodes in canonical order.
    #[must_use]
    pub fn center(&self) -> &[Node] {
        &self.center
    }

    /// Returns all maximum-eccentricity nodes in canonical order.
    #[must_use]
    pub fn periphery(&self) -> &[Node] {
        &self.periphery
    }

    /// Consumes the result and returns its canonical eccentricity pairs.
    #[must_use]
    pub fn into_eccentricities(self) -> Vec<(Node, usize)> {
        self.eccentricities
    }
}

/// Computes exact unweighted distance metrics.
///
/// Returns `None` for an empty or disconnected graph. A singleton has radius,
/// diameter, and eccentricity zero.
#[must_use]
pub fn distance_analytics<G>(graph: &G) -> Option<DistanceAnalytics<G::Node>>
where
    G: IndexUndirectedGraphView,
{
    let neighbors = UndirectedNeighbors::new(graph, |_| true);
    neighbors.nodes().first()?;
    let mut workspace = BfsWorkspace::<G::Node>::new(graph.node_bound());
    let mut eccentricities = Vec::with_capacity(neighbors.nodes().len());
    for &node in neighbors.nodes() {
        let (value, reached) = workspace.run(&neighbors, node);
        if reached != neighbors.nodes().len() {
            return None;
        }
        eccentricities.push((node, value));
    }
    finish(eccentricities)
}

/// Computes exact metrics using accepted edges only.
///
/// The predicate is evaluated exactly once per edge. Returns `None` when the
/// accepted-edge graph is empty or disconnected.
#[must_use]
pub fn distance_analytics_filtered<G, F>(
    graph: &G,
    allows_edge: F,
) -> Option<DistanceAnalytics<G::Node>>
where
    G: IndexUndirectedGraphView,
    F: Fn(G::Edge) -> bool,
{
    let neighbors = UndirectedNeighbors::new(graph, allows_edge);
    neighbors.nodes().first()?;
    let mut workspace = BfsWorkspace::<G::Node>::new(graph.node_bound());
    let mut eccentricities = Vec::with_capacity(neighbors.nodes().len());
    for &node in neighbors.nodes() {
        let (eccentricity, reached) = workspace.run(&neighbors, node);
        if reached != neighbors.nodes().len() {
            return None;
        }
        eccentricities.push((node, eccentricity));
    }
    finish(eccentricities)
}

fn finish<Node: Copy + Eq>(eccentricities: Vec<(Node, usize)>) -> Option<DistanceAnalytics<Node>> {
    let radius = eccentricities.iter().map(|pair| pair.1).min()?;
    let diameter = eccentricities.iter().map(|pair| pair.1).max()?;
    let center = select_nodes(&eccentricities, radius);
    let periphery = select_nodes(&eccentricities, diameter);
    Some(DistanceAnalytics {
        eccentricities,
        radius,
        diameter,
        center,
        periphery,
    })
}

/// Returns the exact eccentricity of `node` in a connected graph.
#[must_use]
pub fn eccentricity<G>(graph: &G, node: G::Node) -> Option<usize>
where
    G: IndexUndirectedGraphView,
{
    if !graph.contains_node(node) || graph.node_count() == 0 {
        return None;
    }
    let neighbors = UndirectedNeighbors::new(graph, |_| true);
    let (value, reached) = BfsWorkspace::<G::Node>::new(graph.node_bound()).run(&neighbors, node);
    (reached == neighbors.nodes().len()).then_some(value)
}

/// Returns the exact diameter of a connected graph.
#[must_use]
pub fn diameter<G>(graph: &G) -> Option<usize>
where
    G: IndexUndirectedGraphView,
{
    distance_analytics(graph).map(|result| result.diameter())
}

/// Returns the exact radius of a connected graph.
#[must_use]
pub fn radius<G>(graph: &G) -> Option<usize>
where
    G: IndexUndirectedGraphView,
{
    distance_analytics(graph).map(|result| result.radius())
}

/// Returns all center nodes of a connected graph.
#[must_use]
pub fn center<G>(graph: &G) -> Option<Vec<G::Node>>
where
    G: IndexUndirectedGraphView,
{
    distance_analytics(graph).map(|result| result.center)
}

/// Returns all peripheral nodes of a connected graph.
#[must_use]
pub fn periphery<G>(graph: &G) -> Option<Vec<G::Node>>
where
    G: IndexUndirectedGraphView,
{
    distance_analytics(graph).map(|result| result.periphery)
}

fn select_nodes<Node: Copy>(values: &[(Node, usize)], target: usize) -> Vec<Node> {
    values
        .iter()
        .filter_map(|&(node, value)| (value == target).then_some(node))
        .collect()
}

struct BfsWorkspace<Node> {
    seen: Vec<usize>,
    distances: Vec<usize>,
    epoch: usize,
    queue: VecDeque<Node>,
}

impl<Node: Copy> BfsWorkspace<Node> {
    fn new(node_bound: usize) -> Self {
        Self {
            seen: vec![0; node_bound],
            distances: vec![0; node_bound],
            epoch: 0,
            queue: VecDeque::new(),
        }
    }

    fn run<G>(&mut self, graph: &UndirectedNeighbors<G>, source: G::Node) -> (usize, usize)
    where
        G: IndexUndirectedGraphView<Node = Node>,
    {
        self.next_epoch();
        let source_slot = G::node_slot(source);
        self.seen[source_slot] = self.epoch;
        self.distances[source_slot] = 0;
        self.queue.push_back(source);
        let mut reached = 0;
        let mut maximum = 0;
        while let Some(node) = self.queue.pop_front() {
            let slot = G::node_slot(node);
            reached += 1;
            maximum = maximum.max(self.distances[slot]);
            for &neighbor in graph.neighbors(node) {
                let neighbor_slot = G::node_slot(neighbor);
                if self.seen[neighbor_slot] != self.epoch {
                    self.seen[neighbor_slot] = self.epoch;
                    self.distances[neighbor_slot] = self.distances[slot] + 1;
                    self.queue.push_back(neighbor);
                }
            }
        }
        (maximum, reached)
    }

    fn next_epoch(&mut self) {
        self.queue.clear();
        if self.epoch == usize::MAX {
            self.seen.fill(0);
            self.epoch = 1;
        } else {
            self.epoch += 1;
        }
    }
}
