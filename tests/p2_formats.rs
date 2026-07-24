use petgraph::{Graph as PetGraph, Undirected, graph6::ToGraph6};
use weavatrix_graph::{
    EdgeEndpoints, GraphMlTopology, NodeIndex, Topology, UndirectedTopology, graph6_decode,
    graph6_encode, graphml_decode, topology_from_dot, topology_to_dot, topology_to_graphml,
    undirected_from_dot, undirected_to_dot, undirected_to_graphml,
};

#[test]
fn dot_round_trips_directed_and_undirected_numeric_topologies() {
    let directed = directed(4, &[(0, 1), (1, 2), (2, 2)]);
    let dot = topology_to_dot(&directed);
    assert_eq!(topology_from_dot(&dot).unwrap(), directed);

    let undirected = undirected(4, &[(0, 1), (1, 2), (2, 3)]);
    let dot = undirected_to_dot(&undirected);
    assert_eq!(undirected_from_dot(&dot).unwrap(), undirected);
}

#[test]
fn dot_import_accepts_attributes_quotes_and_edge_chains() {
    let dot = r#"
        strict digraph Example {
          "n0" [label="root"];
          n1;
          2;
          n0 -> n1 -> 2 [kind="calls"];
        }
    "#;
    assert_eq!(
        topology_from_dot(dot).unwrap(),
        directed(3, &[(0, 1), (1, 2)])
    );
    assert!(topology_from_dot("graph G { 0 -- 1; }").is_err());
}

#[test]
fn graph6_matches_known_records_and_round_trips() {
    assert_eq!(graph6_encode(&undirected(0, &[])).unwrap(), "?");
    assert_eq!(graph6_encode(&undirected(1, &[])).unwrap(), "@");
    let triangle = undirected(3, &[(0, 1), (0, 2), (1, 2)]);
    assert_eq!(graph6_encode(&triangle).unwrap(), "Bw");
    assert_eq!(graph6_decode(">>graph6<<Bw\n").unwrap(), triangle);

    let graph = undirected(70, &[(0, 69), (13, 27), (31, 44)]);
    let encoded = graph6_encode(&graph).unwrap();
    assert_eq!(
        graph6_encode(&graph6_decode(&encoded).unwrap()).unwrap(),
        encoded
    );
}

#[test]
fn graph6_matches_petgraph_across_header_sizes() {
    for node_count in [2_usize, 5, 16, 62, 63, 70] {
        let mut pet = PetGraph::<(), (), Undirected>::default();
        let nodes = (0..node_count)
            .map(|_| pet.add_node(()))
            .collect::<Vec<_>>();
        let mut pairs = Vec::new();
        for right in 1..node_count {
            for left in 0..right {
                if (left * 37 + right * 17) % 11 == 0 {
                    pet.add_edge(nodes[left], nodes[right], ());
                    pairs.push((left.try_into().unwrap(), right.try_into().unwrap()));
                }
            }
        }
        assert_eq!(
            graph6_encode(&undirected(node_count, &pairs)).unwrap(),
            pet.graph6_string()
        );
    }
}

#[test]
fn graph6_rejects_features_and_noncanonical_padding() {
    assert!(graph6_encode(&undirected(2, &[(0, 0)])).is_err());
    assert!(graph6_encode(&undirected(2, &[(0, 1), (0, 1)])).is_err());
    assert!(graph6_decode("Bz").is_err());
    assert!(graph6_decode("A").is_err());
}

#[test]
fn graphml_round_trips_both_graph_kinds() {
    let directed = directed(4, &[(0, 1), (1, 3)]);
    assert_eq!(
        graphml_decode(&topology_to_graphml(&directed)).unwrap(),
        GraphMlTopology::Directed(directed)
    );

    let undirected = undirected(4, &[(0, 1), (1, 3)]);
    assert_eq!(
        graphml_decode(&undirected_to_graphml(&undirected)).unwrap(),
        GraphMlTopology::Undirected(undirected)
    );
}

#[test]
fn graphml_maps_arbitrary_ids_and_rejects_unknown_references() {
    let input = r#"
      <graphml>
        <graph id="g" edgedefault="directed">
          <node id="repo"/>
          <node id="file"/>
          <edge source="repo" target="file"/>
        </graph>
      </graphml>
    "#;
    assert_eq!(
        graphml_decode(input).unwrap(),
        GraphMlTopology::Directed(directed(2, &[(0, 1)]))
    );
    assert!(
        graphml_decode(
            r#"<graphml><graph edgedefault="directed">
               <node id="a"/><edge source="a" target="missing"/>
               </graph></graphml>"#
        )
        .is_err()
    );
}

fn directed(node_count: usize, pairs: &[(u32, u32)]) -> Topology {
    Topology::try_from_edges(
        node_count,
        pairs
            .iter()
            .map(|&(source, target)| endpoints(source, target)),
    )
    .unwrap()
}

fn undirected(node_count: usize, pairs: &[(u32, u32)]) -> UndirectedTopology {
    UndirectedTopology::try_from_edges(
        node_count,
        pairs
            .iter()
            .map(|&(source, target)| endpoints(source, target)),
    )
    .unwrap()
}

fn endpoints(source: u32, target: u32) -> EdgeEndpoints {
    EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
}
