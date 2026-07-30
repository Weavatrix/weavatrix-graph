mod candidate;
mod queue;

use self::candidate::{Candidate, better, canonical_candidate, map_nodes, nodes, validate_weight};
use self::queue::MaxQueue;
use crate::{GraphError, IndexUndirectedGraphView, Measure, Result, Vec};
use alloc::collections::BTreeMap;
use core::cmp::Ordering;

/// A deterministic global minimum cut and both sides of its partition.
#[derive(Debug, Clone, PartialEq)]
pub struct StoerWagnerCut<Node, Weight> {
    weight: Weight,
    partition: Vec<Node>,
    complement: Vec<Node>,
}

impl<Node, Weight: Copy> StoerWagnerCut<Node, Weight> {
    #[must_use]
    pub const fn weight(&self) -> Weight {
        self.weight
    }

    #[must_use]
    pub fn partition(&self) -> &[Node] {
        &self.partition
    }

    #[must_use]
    pub fn complement(&self) -> &[Node] {
        &self.complement
    }

    #[must_use]
    pub fn into_parts(self) -> (Weight, Vec<Node>, Vec<Node>) {
        (self.weight, self.partition, self.complement)
    }
}

struct WorkingCut<M> {
    adjacency: Vec<BTreeMap<usize, M>>,
    active: Vec<bool>,
    groups: Vec<Vec<usize>>,
    active_count: usize,
}

/// Finds the global minimum cut of an undirected weighted multigraph.
///
/// Parallel edge weights are added with overflow checking; self-loops do not
/// affect the cut. Empty and singleton graphs return `Ok(None)`.
///
/// # Errors
///
/// Returns an error for a negative/non-finite weight or checked-add overflow.
pub fn stoer_wagner_min_cut<G, F, M>(
    graph: &G,
    edge_weight: F,
) -> Result<Option<StoerWagnerCut<G::Node, M>>>
where
    G: IndexUndirectedGraphView,
    F: FnMut(G::Edge) -> M,
    M: Measure,
{
    stoer_wagner_min_cut_filtered(graph, |_| true, edge_weight)
}

/// Filtered Stoer-Wagner min-cut. Each predicate and accepted weight callback
/// is evaluated exactly once per edge.
///
/// # Errors
///
/// Returns an error for invalid weights or arithmetic overflow.
pub fn stoer_wagner_min_cut_filtered<G, P, F, M>(
    graph: &G,
    mut allows_edge: P,
    mut edge_weight: F,
) -> Result<Option<StoerWagnerCut<G::Node, M>>>
where
    G: IndexUndirectedGraphView,
    P: FnMut(G::Edge) -> bool,
    F: FnMut(G::Edge) -> M,
    M: Measure,
{
    if graph.node_count() < 2 {
        return Ok(None);
    }
    let (nodes_by_slot, nodes) = nodes(graph);
    let mut working = WorkingCut::new(graph.node_bound(), &nodes);
    for edge in graph.edge_indices() {
        if !allows_edge(edge) {
            continue;
        }
        let weight = edge_weight(edge);
        validate_weight(weight)?;
        let Some(endpoints) = graph.edge_endpoints(edge) else {
            continue;
        };
        let source = G::node_slot(endpoints.source());
        let target = G::node_slot(endpoints.target());
        if source != target {
            working.add_edge(source, target, weight)?;
        }
    }
    let candidate = working.solve(&nodes)?;
    candidate
        .map(|cut| {
            Ok(StoerWagnerCut {
                weight: cut.weight,
                partition: map_nodes(&cut.partition, &nodes_by_slot)?,
                complement: map_nodes(&cut.complement, &nodes_by_slot)?,
            })
        })
        .transpose()
}

impl<M: Measure> WorkingCut<M> {
    fn new(node_bound: usize, nodes: &[usize]) -> Self {
        let mut active = vec![false; node_bound];
        for &node in nodes {
            active[node] = true;
        }
        let groups = (0..node_bound).map(|node| vec![node]).collect();
        Self {
            adjacency: vec![BTreeMap::new(); node_bound],
            active,
            groups,
            active_count: nodes.len(),
        }
    }

    fn add_edge(&mut self, source: usize, target: usize, weight: M) -> Result<()> {
        let sum = self.adjacency[source]
            .get(&target)
            .copied()
            .unwrap_or_else(M::zero)
            .checked_add(weight)
            .ok_or(GraphError::ArithmeticOverflow {
                operation: "Stoer-Wagner edge aggregation",
            })?;
        self.adjacency[source].insert(target, sum);
        self.adjacency[target].insert(source, sum);
        Ok(())
    }

    fn solve(&mut self, nodes: &[usize]) -> Result<Option<Candidate<M>>> {
        let mut best = None;
        let mut queue = MaxQueue::with_capacity(nodes.len() * 2);
        let mut weights = vec![M::zero(); self.active.len()];
        let mut added = vec![false; self.active.len()];
        while self.active_count > 1 {
            let (source, target, weight) =
                self.phase(nodes, &mut queue, &mut weights, &mut added)?;
            let candidate = canonical_candidate(weight, &self.groups[target], nodes);
            if better(&candidate, best.as_ref()) {
                best = Some(candidate);
            }
            self.merge(source, target)?;
        }
        Ok(best)
    }

    fn phase(
        &self,
        nodes: &[usize],
        queue: &mut MaxQueue<M>,
        weights: &mut [M],
        added: &mut [bool],
    ) -> Result<(usize, usize, M)> {
        weights.fill(M::zero());
        added.fill(false);
        queue.clear();
        for &node in nodes {
            if self.active[node] {
                queue.push(node, M::zero());
            }
        }
        let mut previous = None;
        for index in 0..self.active_count {
            let (node, weight) = pop_current(queue, &self.active, added, weights)
                .ok_or_else(|| invalid_phase_state("no queued active node remains"))?;
            if index + 1 == self.active_count {
                let source = previous
                    .ok_or_else(|| invalid_phase_state("phase has fewer than two nodes"))?;
                return Ok((source, node, weight));
            }
            added[node] = true;
            previous = Some(node);
            for (&neighbor, &edge_weight) in &self.adjacency[node] {
                if self.active[neighbor] && !added[neighbor] {
                    weights[neighbor] = weights[neighbor].checked_add(edge_weight).ok_or(
                        GraphError::ArithmeticOverflow {
                            operation: "Stoer-Wagner phase",
                        },
                    )?;
                    queue.push(neighbor, weights[neighbor]);
                }
            }
        }
        Err(invalid_phase_state("phase selected no target"))
    }

    fn merge(&mut self, source: usize, target: usize) -> Result<()> {
        let removed = core::mem::take(&mut self.adjacency[target]);
        self.adjacency[source].remove(&target);
        for (neighbor, weight) in removed {
            self.adjacency[neighbor].remove(&target);
            if neighbor == source || !self.active[neighbor] {
                continue;
            }
            let sum = self.adjacency[source]
                .get(&neighbor)
                .copied()
                .unwrap_or_else(M::zero)
                .checked_add(weight)
                .ok_or(GraphError::ArithmeticOverflow {
                    operation: "Stoer-Wagner contraction",
                })?;
            self.adjacency[source].insert(neighbor, sum);
            self.adjacency[neighbor].insert(source, sum);
        }
        self.active[target] = false;
        self.active_count -= 1;
        let mut target_group = core::mem::take(&mut self.groups[target]);
        self.groups[source].append(&mut target_group);
        self.groups[source].sort_unstable();
        Ok(())
    }
}

fn pop_current<M: Measure>(
    queue: &mut MaxQueue<M>,
    active: &[bool],
    added: &[bool],
    weights: &[M],
) -> Option<(usize, M)> {
    while let Some((node, weight)) = queue.pop() {
        let Some((&is_active, &is_added, &current_weight)) = active
            .get(node)
            .zip(added.get(node))
            .zip(weights.get(node))
            .map(|((active, added), weight)| (active, added, weight))
        else {
            continue;
        };
        if is_active && !is_added && weight.compare(current_weight) == Some(Ordering::Equal) {
            return Some((node, weight));
        }
    }
    None
}

fn invalid_phase_state(value: &'static str) -> GraphError {
    GraphError::InvalidAlgorithmParameter {
        algorithm: "Stoer-Wagner",
        parameter: "active cut state",
        value: value.into(),
    }
}
