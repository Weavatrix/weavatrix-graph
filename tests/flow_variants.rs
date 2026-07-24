use weavatrix_graph::{
    EdgeEndpoints, EdgeIndex, MaxFlow, NodeIndex, Topology, edmonds_karp, maximum_flow,
    min_cost_max_flow, push_relabel,
};

fn network() -> Topology {
    Topology::try_from_edges(
        6,
        [
            (0, 1),
            (0, 2),
            (1, 2),
            (2, 1),
            (1, 3),
            (2, 4),
            (3, 2),
            (4, 3),
            (3, 5),
            (4, 5),
        ]
        .map(|(source, target)| EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))),
    )
    .unwrap()
}

#[test]
fn maximum_flow_variants_agree_on_value_and_min_cut() {
    let graph = network();
    let capacities = [16_u64, 13, 10, 4, 12, 14, 9, 7, 20, 4];
    let source = NodeIndex::new(0);
    let sink = NodeIndex::new(5);
    let dinic = maximum_flow(&graph, source, sink, |edge| capacities[edge.index()])
        .unwrap()
        .unwrap();
    let edmonds = edmonds_karp(&graph, source, sink, |edge| capacities[edge.index()])
        .unwrap()
        .unwrap();
    let push = push_relabel(&graph, source, sink, |edge| capacities[edge.index()])
        .unwrap()
        .unwrap();
    assert_eq!(dinic.value(), 23);
    assert_eq!(edmonds.value(), dinic.value());
    assert_eq!(push.value(), dinic.value());
    assert_eq!(edmonds.source_side(), dinic.source_side());
    assert_eq!(push.source_side(), dinic.source_side());
}

#[test]
fn min_cost_flow_chooses_the_cheapest_maximum_flow() {
    let graph = Topology::try_from_edges(
        4,
        [(0, 1), (0, 2), (1, 3), (2, 3), (1, 2)].map(|(source, target)| {
            EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
        }),
    )
    .unwrap();
    let capacities = [2_u64, 2, 2, 2, 1];
    let costs = [1_i64, 5, 1, 1, -3];
    let flow = min_cost_max_flow(
        &graph,
        NodeIndex::new(0),
        NodeIndex::new(3),
        |edge| capacities[edge.index()],
        |edge| costs[edge.index()],
    )
    .unwrap()
    .unwrap();
    assert_eq!(flow.value(), 4);
    assert_eq!(flow.cost(), 16);
}

#[test]
fn maximum_flow_variants_match_petgraph_on_seeded_networks() {
    use petgraph::algo::ford_fulkerson;
    use petgraph::graph::DiGraph;

    for seed in 1..=48_u64 {
        let mut random = Lcg::new(seed);
        let node_count = 2 + random.usize(11);
        let edge_count = node_count + random.usize(node_count * 3);
        let mut pairs = Vec::with_capacity(edge_count);
        let mut capacities = Vec::with_capacity(edge_count);
        let mut pet = DiGraph::<(), u64>::new();
        let pet_nodes = (0..node_count)
            .map(|_| pet.add_node(()))
            .collect::<Vec<_>>();
        for _ in 0..edge_count {
            let source = random.usize(node_count);
            let mut target = random.usize(node_count);
            if target == source {
                target = (target + 1) % node_count;
            }
            let capacity = 1 + random.next() % 31;
            pairs.push((source, target));
            capacities.push(capacity);
            pet.add_edge(pet_nodes[source], pet_nodes[target], capacity);
        }
        let graph = topology(node_count, &pairs);
        let source = NodeIndex::new(0);
        let sink = NodeIndex::new(u32::try_from(node_count - 1).unwrap());
        let expected = ford_fulkerson(&pet, pet_nodes[0], pet_nodes[node_count - 1]).0;
        let variants = [
            maximum_flow(&graph, source, sink, |edge| capacities[edge.index()])
                .unwrap()
                .unwrap(),
            edmonds_karp(&graph, source, sink, |edge| capacities[edge.index()])
                .unwrap()
                .unwrap(),
            push_relabel(&graph, source, sink, |edge| capacities[edge.index()])
                .unwrap()
                .unwrap(),
        ];
        for flow in &variants {
            assert_eq!(flow.value(), expected, "seed {seed}");
            assert_feasible(&graph, &capacities, flow, source, sink);
        }
    }
}

#[test]
fn min_cost_flow_matches_exhaustive_small_dags() {
    for seed in 1..=24_u64 {
        let mut random = Lcg::new(seed);
        let mut candidates = (0..4)
            .flat_map(|source| (source + 1..5).map(move |target| (source, target)))
            .collect::<Vec<_>>();
        for index in (1..candidates.len()).rev() {
            let selected = random.usize(index + 1);
            candidates.swap(index, selected);
        }
        let pairs = candidates.into_iter().take(7).collect::<Vec<_>>();
        let capacities = (0..pairs.len())
            .map(|_| 1 + random.next() % 2)
            .collect::<Vec<_>>();
        let costs = (0..pairs.len())
            .map(|_| i64::try_from(random.next() % 11).unwrap() - 5)
            .collect::<Vec<_>>();
        let graph = topology(5, &pairs);
        let actual = min_cost_max_flow(
            &graph,
            NodeIndex::new(0),
            NodeIndex::new(4),
            |edge| capacities[edge.index()],
            |edge| costs[edge.index()],
        )
        .unwrap()
        .unwrap();
        let expected = exhaustive_min_cost(&pairs, &capacities, &costs);
        assert_eq!((actual.value(), actual.cost()), expected, "seed {seed}");
    }
}

fn topology(node_count: usize, pairs: &[(usize, usize)]) -> Topology {
    Topology::try_from_edges(
        node_count,
        pairs.iter().map(|&(source, target)| {
            EdgeEndpoints::new(
                NodeIndex::new(u32::try_from(source).unwrap()),
                NodeIndex::new(u32::try_from(target).unwrap()),
            )
        }),
    )
    .unwrap()
}

fn assert_feasible(
    graph: &Topology,
    capacities: &[u64],
    flow: &MaxFlow<NodeIndex, EdgeIndex>,
    source: NodeIndex,
    sink: NodeIndex,
) {
    let mut balance = vec![0_i128; graph.node_count()];
    for &(edge, amount) in flow.edge_flows() {
        assert!(amount <= capacities[edge.index()]);
        let endpoints = graph.edge_endpoints(edge).unwrap();
        balance[endpoints.source().index()] -= i128::from(amount);
        balance[endpoints.target().index()] += i128::from(amount);
    }
    for (slot, value) in balance.iter().enumerate() {
        if slot != source.index() && slot != sink.index() {
            assert_eq!(*value, 0);
        }
    }
    assert_eq!(-balance[source.index()], i128::from(flow.value()));
    assert_eq!(balance[sink.index()], i128::from(flow.value()));
}

fn exhaustive_min_cost(pairs: &[(usize, usize)], capacities: &[u64], costs: &[i64]) -> (u64, i128) {
    let mut best = (0_u64, i128::MAX);
    enumerate_flows(
        0,
        &mut vec![0; pairs.len()],
        pairs,
        capacities,
        costs,
        &mut best,
    );
    best
}

fn enumerate_flows(
    edge: usize,
    flows: &mut [u64],
    pairs: &[(usize, usize)],
    capacities: &[u64],
    costs: &[i64],
    best: &mut (u64, i128),
) {
    if edge < flows.len() {
        for amount in 0..=capacities[edge] {
            flows[edge] = amount;
            enumerate_flows(edge + 1, flows, pairs, capacities, costs, best);
        }
        return;
    }
    let mut balance = [0_i64; 5];
    let mut cost = 0_i128;
    for (index, &(source, target)) in pairs.iter().enumerate() {
        let amount = i64::try_from(flows[index]).unwrap();
        balance[source] -= amount;
        balance[target] += amount;
        cost += i128::from(amount) * i128::from(costs[index]);
    }
    if balance[1..4].iter().any(|value| *value != 0) || balance[0] != -balance[4] {
        return;
    }
    let value = u64::try_from(balance[4]).unwrap();
    if value > best.0 || (value == best.0 && cost < best.1) {
        *best = (value, cost);
    }
}

struct Lcg(u64);

impl Lcg {
    const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn usize(&mut self, upper: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(upper).unwrap()).unwrap()
    }
}
