use crate::IndexGraphView;
use crate::Vec;

type Adjacency<Node> = Vec<Vec<Node>>;

#[derive(Debug, Clone)]
pub struct Dominators<Node> {
    root: Node,
    reachable: Vec<Node>,
    immediate: Vec<(Node, Node)>,
}

impl<Node> Dominators<Node>
where
    Node: Copy + Eq,
{
    #[must_use]
    pub const fn root(&self) -> Node {
        self.root
    }

    #[must_use]
    pub fn reachable_nodes(&self) -> &[Node] {
        &self.reachable
    }

    #[must_use]
    pub fn immediate_dominator(&self, node: Node) -> Option<Node> {
        (node != self.root).then(|| {
            self.immediate
                .iter()
                .find_map(|&(candidate, parent)| (candidate == node).then_some(parent))
        })?
    }

    pub fn immediate_dominators(&self) -> impl Iterator<Item = (Node, Node)> + '_ {
        self.immediate.iter().copied()
    }

    pub fn dominators(&self, node: Node) -> Option<DominatorsIter<'_, Node>> {
        self.is_reachable(node).then_some(DominatorsIter {
            result: self,
            next: Some(node),
        })
    }

    pub fn strict_dominators(&self, node: Node) -> Option<DominatorsIter<'_, Node>> {
        let mut result = self.dominators(node)?;
        result.next();
        Some(result)
    }

    #[must_use]
    pub fn immediately_dominated_by(&self, node: Node) -> Vec<Node> {
        self.reachable
            .iter()
            .copied()
            .filter(|candidate| self.immediate_dominator(*candidate) == Some(node))
            .collect()
    }

    #[must_use]
    pub fn dominates(&self, dominator: Node, node: Node) -> bool {
        self.dominators(node)
            .is_some_and(|mut chain| chain.any(|candidate| candidate == dominator))
    }

    #[must_use]
    pub fn is_reachable(&self, node: Node) -> bool {
        node == self.root
            || self
                .immediate
                .iter()
                .any(|&(candidate, _)| candidate == node)
    }
}

pub struct DominatorsIter<'result, Node> {
    result: &'result Dominators<Node>,
    next: Option<Node>,
}

impl<Node> Iterator for DominatorsIter<'_, Node>
where
    Node: Copy + Eq,
{
    type Item = Node;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.next?;
        self.next = self.result.immediate_dominator(node);
        Some(node)
    }
}

#[must_use]
pub fn dominators<G>(graph: &G, root: G::Node) -> Option<Dominators<G::Node>>
where
    G: IndexGraphView,
{
    dominators_filtered(graph, root, |_| true)
}

#[must_use]
pub fn dominators_filtered<G, F>(
    graph: &G,
    root: G::Node,
    allows_edge: F,
) -> Option<Dominators<G::Node>>
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    if !graph.contains_node(root) {
        return None;
    }
    let (adjacency, predecessors) = adjacency_pair(graph, &allows_edge);
    let postorder = reachable_postorder::<G>(root, &adjacency);
    let reachable = postorder.iter().rev().copied().collect::<Vec<_>>();
    let mut position = vec![None; graph.node_bound()];
    for (index, &node) in reachable.iter().enumerate() {
        position[G::node_slot(node)] = Some(index);
    }
    let mut immediate = vec![None; reachable.len()];
    immediate[0] = Some(0);
    let mut changed = true;
    while changed {
        changed = false;
        for index in 1..reachable.len() {
            let node = reachable[index];
            let candidate = predecessors[G::node_slot(node)]
                .iter()
                .filter_map(|predecessor| position[G::node_slot(*predecessor)])
                .find(|predecessor| immediate[*predecessor].is_some());
            let Some(mut new_idom) = candidate else {
                continue;
            };
            for predecessor in predecessors[G::node_slot(node)]
                .iter()
                .filter_map(|predecessor| position[G::node_slot(*predecessor)])
                .filter(|predecessor| immediate[*predecessor].is_some())
            {
                new_idom = intersect(predecessor, new_idom, &immediate)?;
            }
            if immediate[index] != Some(new_idom) {
                immediate[index] = Some(new_idom);
                changed = true;
            }
        }
    }
    let immediate = reachable
        .iter()
        .copied()
        .enumerate()
        .skip(1)
        .filter_map(|(index, node)| immediate[index].map(|parent| (node, reachable[parent])))
        .collect();
    Some(Dominators {
        root,
        reachable,
        immediate,
    })
}

fn adjacency_pair<G, F>(graph: &G, allows_edge: &F) -> (Adjacency<G::Node>, Adjacency<G::Node>)
where
    G: IndexGraphView,
    F: Fn(G::Edge) -> bool,
{
    let mut adjacency = vec![Vec::new(); graph.node_bound()];
    let mut predecessors = vec![Vec::new(); graph.node_bound()];
    for (edge, endpoints) in graph.edge_references() {
        if allows_edge(edge) {
            adjacency[G::node_slot(endpoints.source())].push(endpoints.target());
            predecessors[G::node_slot(endpoints.target())].push(endpoints.source());
        }
    }
    (adjacency, predecessors)
}

fn reachable_postorder<G>(root: G::Node, adjacency: &[Vec<G::Node>]) -> Vec<G::Node>
where
    G: IndexGraphView,
{
    let mut seen = vec![false; adjacency.len()];
    let mut order = Vec::new();
    let mut stack = vec![(root, 0_usize)];
    seen[G::node_slot(root)] = true;
    while let Some((node, next)) = stack.last_mut() {
        let Some(&target) = adjacency[G::node_slot(*node)].get(*next) else {
            order.push(*node);
            stack.pop();
            continue;
        };
        *next += 1;
        let slot = G::node_slot(target);
        if !seen[slot] {
            seen[slot] = true;
            stack.push((target, 0));
        }
    }
    order
}

fn intersect(mut left: usize, mut right: usize, immediate: &[Option<usize>]) -> Option<usize> {
    while left != right {
        while left > right {
            left = immediate.get(left).copied().flatten()?;
        }
        while right > left {
            right = immediate.get(right).copied().flatten()?;
        }
    }
    Some(left)
}
