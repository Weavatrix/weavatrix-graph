use crate::IndexGraphView;
use crate::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackArcSet<Node, Edge> {
    order: Vec<Node>,
    edges: Vec<Edge>,
}

impl<Node, Edge> FeedbackArcSet<Node, Edge> {
    #[must_use]
    pub fn order(&self) -> &[Node] {
        &self.order
    }

    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
}

/// Approximates a directed feedback arc set with the Eades ordering heuristic.
///
/// Minimum feedback arc set is NP-hard. The returned edges are always a valid
/// set whose removal makes the selected ordering acyclic, but not necessarily
/// a minimum-cardinality set.
pub fn feedback_arc_set_heuristic<G>(graph: &G) -> FeedbackArcSet<G::Node, G::Edge>
where
    G: IndexGraphView,
{
    let bound = graph.node_bound();
    let mut nodes = vec![None; bound];
    let mut outgoing = vec![Vec::new(); bound];
    let mut incoming = vec![Vec::new(); bound];
    for node in graph.node_indices() {
        nodes[G::node_slot(node)] = Some(node);
    }
    for (_, endpoints) in graph.edge_references() {
        let source = G::node_slot(endpoints.source());
        let target = G::node_slot(endpoints.target());
        outgoing[source].push(target);
        incoming[target].push(source);
    }
    let active = nodes.iter().map(Option::is_some).collect::<Vec<_>>();
    let mut state = OrderingState::new(active, outgoing, incoming);
    let mut left = Vec::new();
    let mut right = Vec::new();
    while state.remaining > 0 {
        let previous = state.remaining;
        while let Some(node) = state.pop_sink() {
            right.push(node);
            state.remove(node);
        }
        while let Some(node) = state.pop_source() {
            left.push(node);
            state.remove(node);
        }
        if let Some(node) = state.pop_delta() {
            left.push(node);
            state.remove(node);
        }
        if state.remaining == previous {
            break;
        }
    }
    right.reverse();
    left.extend(right);
    let mut position = vec![usize::MAX; bound];
    for (index, &node) in left.iter().enumerate() {
        position[node] = index;
    }
    let mut edges = graph
        .edge_references()
        .filter_map(|(edge, endpoints)| {
            (position[G::node_slot(endpoints.source())]
                >= position[G::node_slot(endpoints.target())])
            .then_some(edge)
        })
        .collect::<Vec<_>>();
    edges.sort_unstable_by_key(|edge| G::edge_slot(*edge));
    let order = left.into_iter().filter_map(|slot| nodes[slot]).collect();
    FeedbackArcSet { order, edges }
}

struct OrderingState {
    active: Vec<bool>,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
    out_degree: Vec<usize>,
    in_degree: Vec<usize>,
    stamps: Vec<u64>,
    sinks: Vec<(u64, usize)>,
    sources: Vec<(u64, usize)>,
    positive: Vec<Vec<(u64, usize)>>,
    negative: Vec<Vec<(u64, usize)>>,
    clock: u64,
    remaining: usize,
}

impl OrderingState {
    fn new(active: Vec<bool>, outgoing: Vec<Vec<usize>>, incoming: Vec<Vec<usize>>) -> Self {
        let out_degree = outgoing.iter().map(Vec::len).collect();
        let in_degree = incoming.iter().map(Vec::len).collect();
        let remaining = active.iter().filter(|active| **active).count();
        let bound = active.len();
        let mut state = Self {
            active,
            outgoing,
            incoming,
            out_degree,
            in_degree,
            stamps: vec![0; bound],
            sinks: Vec::new(),
            sources: Vec::new(),
            positive: Vec::new(),
            negative: Vec::new(),
            clock: 0,
            remaining,
        };
        for node in 0..state.active.len() {
            state.refresh(node);
        }
        state
    }

    fn pop_sink(&mut self) -> Option<usize> {
        while let Some((stamp, node)) = self.sinks.pop() {
            if self.active[node] && self.out_degree[node] == 0 && self.stamps[node] == stamp {
                return Some(node);
            }
        }
        None
    }

    fn pop_source(&mut self) -> Option<usize> {
        while let Some((stamp, node)) = self.sources.pop() {
            if self.active[node]
                && self.in_degree[node] == 0
                && self.out_degree[node] > 0
                && self.stamps[node] == stamp
            {
                return Some(node);
            }
        }
        None
    }

    fn pop_delta(&mut self) -> Option<usize> {
        while !self.positive.is_empty() {
            let index = self.positive.len() - 1;
            let delta = isize::try_from(index).unwrap_or(isize::MAX);
            let bucket = &mut self.positive[index];
            while let Some((stamp, node)) = bucket.pop() {
                if self.active[node]
                    && self.in_degree[node] > 0
                    && self.out_degree[node] > 0
                    && signed(self.out_degree[node]) - signed(self.in_degree[node]) == delta
                    && self.stamps[node] == stamp
                {
                    return Some(node);
                }
            }
            self.positive.pop();
        }
        for index in 0..self.negative.len() {
            while let Some((stamp, node)) = self.negative[index].pop() {
                let delta = -isize::try_from(index + 1).unwrap_or(isize::MAX);
                if self.active[node]
                    && self.in_degree[node] > 0
                    && self.out_degree[node] > 0
                    && self.delta(node) == delta
                    && self.stamps[node] == stamp
                {
                    return Some(node);
                }
            }
        }
        None
    }

    fn remove(&mut self, node: usize) {
        self.active[node] = false;
        self.remaining = self.remaining.saturating_sub(1);
        for index in 0..self.outgoing[node].len() {
            let target = self.outgoing[node][index];
            if self.active[target] {
                self.in_degree[target] = self.in_degree[target].saturating_sub(1);
                self.refresh(target);
            }
        }
        for index in 0..self.incoming[node].len() {
            let source = self.incoming[node][index];
            if self.active[source] {
                self.out_degree[source] = self.out_degree[source].saturating_sub(1);
                self.refresh(source);
            }
        }
    }

    fn refresh(&mut self, node: usize) {
        if !self.active[node] {
            return;
        }
        self.clock = self.clock.saturating_add(1);
        self.stamps[node] = self.clock;
        if self.out_degree[node] == 0 {
            self.sinks.push((self.clock, node));
        } else if self.in_degree[node] == 0 {
            self.sources.push((self.clock, node));
        } else {
            let delta = self.delta(node);
            if delta >= 0 {
                let index = delta.unsigned_abs();
                if self.positive.len() <= index {
                    self.positive.resize_with(index + 1, Vec::new);
                }
                self.positive[index].push((self.clock, node));
            } else {
                let index = delta.unsigned_abs() - 1;
                if self.negative.len() <= index {
                    self.negative.resize_with(index + 1, Vec::new);
                }
                self.negative[index].push((self.clock, node));
            }
        }
    }

    fn delta(&self, node: usize) -> isize {
        signed(self.out_degree[node]) - signed(self.in_degree[node])
    }
}

fn signed(value: usize) -> isize {
    isize::try_from(value).unwrap_or(isize::MAX)
}
