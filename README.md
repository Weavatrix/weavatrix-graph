# Weavatrix Graph

[![CI](https://github.com/Weavatrix/weavatrix-graph/actions/workflows/ci.yml/badge.svg)](https://github.com/Weavatrix/weavatrix-graph/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/weavatrix-graph.svg)](https://crates.io/crates/weavatrix-graph)
[![docs.rs](https://docs.rs/weavatrix-graph/badge.svg)](https://docs.rs/weavatrix-graph)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/Weavatrix/weavatrix-graph/blob/main/LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue.svg)](https://github.com/Weavatrix/weavatrix-graph/blob/main/Cargo.toml)

Part of the [Weavatrix ecosystem](https://weavatrix.com/ecosystem): the evidence graph shared by repository-intelligence engines.

**The deterministic graph core behind Weavatrix repository intelligence.**

`weavatrix-graph` is the protocol-independent Rust library that gives
Weavatrix its typed, evidence-carrying **code / repository graph**. It is also
usable by repository analyzers, architecture tools, dependency explorers, and
other applications without depending on the Weavatrix engine or MCP product.

The crate owns graph integrity and serialization. It does **not** walk files,
parse programming languages, execute commands, access the network, or provide
an MCP/CLI transport.

### Complementary to Weavatrix Loom

`weavatrix-graph` and
**[Weavatrix Loom](https://github.com/Weavatrix/weavatrix-loom)** are
complementary layers of the same ecosystem, not competing graph products.

**Weavatrix Loom is a visual programming environment and compiler that turns
real code into reusable typed blocks, lets humans and AI compose programs from
those blocks, runs and debugs the graph, and compiles the result into ordinary
standalone software.**

Weavatrix supplies the grounded view of that real code: files, symbols, calls,
imports, source spans, revisions, and provenance. The integration contract is
for Loom to reference those facts when it identifies and traces
implementations, while its WVX model owns the typed capabilities, instances,
bindings, composition, execution, debugging, and compilation workflow.

```text
real code
   → Weavatrix code facts and provenance
   → Loom reusable typed blocks
   → human + AI visual composition
   → run and debug the graph
   → compile to ordinary standalone software
```

The two graph models remain distinct so evidence about existing code is not
confused with the program being composed. Loom references Weavatrix entities
through provenance rather than copying the repository graph into its project
IR. That separation is an integration contract, not a product rivalry. See
Loom [ADR-0012](https://github.com/Weavatrix/weavatrix-loom/blob/main/docs/adr/0012-ecosystem-boundaries.md).

The boundary is defined today; the direct Weavatrix-facts feed is still an
integration target, and Loom Forge currently retains a clearly marked bootstrap
inventory. `weavatrix-graph` therefore does not pretend to be an already-wired
Loom runtime dependency.

## Properties

- typed nodes and edges with custom extension kinds;
- strongly typed, non-empty node identifiers;
- source spans, optional language labels, evidence kind, confidence, and extractor
  provenance;
- structured node and edge attributes for parser-specific metadata;
- deterministic node and edge order independent of insertion order;
- compact numeric endpoints with incoming and outgoing CSR indexes;
- an optional direct-neighbor traversal cache with automatic, fast, bit-packed,
  and succinct layouts, reusable eager/lazy walks, and no new dependency;
- a mutable insertion-order graph with generation-stable node and edge keys;
- allocation-reusing lazy BFS, DFS, and generic Dijkstra iterators, early
  stopping, DFS events, and reusable workspaces;
- generic checked integer and finite floating-point path costs through the
  `Measure` trait;
- BFS, DFS, reachability, unweighted shortest paths, Dijkstra, A*, signed
  Bellman-Ford, filtered SCC, weak components, condensation DAGs, cycle
  discovery, topological sort, MST, and Dinic maximum flow;
- standard `PageRank` with dangling-mass redistribution, control-flow dominators,
  dominance frontiers, deterministic DAG longest paths and topological
  generations, and DAG transitive reduction plus closure;
- Floyd-Warshall and Johnson all-pairs shortest paths, bounded simple and
  K-shortest paths, bounded elementary circuit enumeration, undirected bridges
  and articulation points, iterative vertex-biconnected edge blocks, exact
  eccentricity/radius/diameter/center/periphery analytics, deterministic
  multigraph-safe chain decomposition, checked weighted Stoer-Wagner global
  min-cut, and label-aware graph/subgraph isomorphism;
- bidirectional Dijkstra and queue-based SPFA with checked arithmetic and
  reachable-negative-cycle reporting;
- bipartite partitioning, Hopcroft-Karp maximum bipartite matching, and bounded
  Bron-Kerbosch maximal-clique enumeration;
- exact general-graph maximum matching with Edmonds blossoms and deterministic
  DSATUR coloring;
- deterministic Eades feedback-arc approximation and a multi-start
  metric-closure Steiner-tree approximation that retains the standard
  two-approximation candidate;
- degree, closeness, node/edge betweenness, Katz, eigenvector, and HITS
  hub/authority centrality, plus k-core, cycle-basis, and deterministic
  label-propagation community analysis;
- Edmonds-Karp, push-relabel, min-cost maximum flow, and Prim spanning forests
  alongside Dinic and Kruskal;
- zero-copy reversed, edge-filtered, and induced-subgraph views, plus
  complement and union operators;
- edge-kind, evidence, extractor, confidence, and caller-defined traversal
  filters;
- automatic Floyd-Warshall/Johnson selection with the selected strategy exposed
  to callers;
- deterministic DOT, Graph6, and `GraphML` topology interchange;
- undirected incidence CSR, a generic dense matrix, a bit-packed adjacency
  matrix, and deterministic seeded graph generators;
- generic directed and undirected payload graphs alongside the
  evidence-carrying model;
- optional Rayon batch traversal, shortest-path queries, Johnson APSP,
  closeness, and node/edge betweenness centrality;
- a generic mutable payload graph with compact intrusive adjacency, generation
  keys, mapping, compaction, and an acyclic invariant wrapper;
- a GraphMap-style keyed payload index backed by generation-stable handles, plus
  a mutable undirected payload graph with O(1) intrusive incidence updates;
- `no_std + alloc` support with the default `std` feature kept for faster
  hash-backed rich graph construction;
- idempotent insertion of identical nodes and edges;
- rejection of conflicting nodes, dangling edges, and invalid source spans;
- validated deserialization that cannot bypass graph invariants;
- compatibility conversion from Weavatrix's legacy `{ nodes, links }` graph;
- no unsafe code in the default or featureless `no_std` build; the opt-in
  `unsafe-fast` feature is confined to audited bit-matrix and parallel-CSR
  primitives;
- the default build has one runtime dependency, `serde`, while the optional
  `rayon` feature is isolated from the core.

## Architecture

The crate is a layered graph core, not a monolithic graph type. Dependencies
flow from stable contracts toward specialized behavior:

| Layer | Modules | Responsibility |
| --- | --- | --- |
| Evidence model | `attribute`, `error`, `filter`, `kind`, `model` | Typed identities, kinds, provenance, confidence, spans, attributes, and validation errors |
| Graph views | `topology`, `undirected`, `matrix`, `view`, `operator` | Compact endpoints and the generic directed/undirected view contracts algorithms consume |
| Algorithms | `algo` | Traversal, paths, components, cuts, flow, ranking, matching, and structural analysis over views |
| Storage | `graph`, `working`, `payload`, `traversal_cache` | Canonical evidence snapshots, mutable builders, stable payload graphs, and derived traversal indexes |
| Interchange | `format`, `generator`, `legacy` | Deterministic wire formats, seeded graph construction, and compatibility conversion |
| Public facade | `lib.rs` | Stable crate-root re-exports without owning domain logic |

Module trees use one idiomatic source form: nested modules live under
`foo/mod.rs`, never beside a competing `foo.rs`. The strict
`.weavatrix/architecture.json` contract enforces a 300-line file budget,
100-line function budget, zero runtime dependency cycles, and no exception or
ratchet baseline. `tests/architecture.rs` independently guards file size,
single-form module layout, focused facades, dependency uniqueness, and wire-kind
uniqueness.

The default and `no_std` paths forbid unsafe code. The explicitly enabled
`unsafe-fast` feature remains isolated to audited matrix and CSR primitives;
it does not change the public layering or graph invariants.

## Layered graph contracts

The crate keeps storage contracts separate instead of making one graph type pay
for every feature:

| Type | Purpose | Ordering and validation |
| --- | --- | --- |
| `Topology` | Immutable directed numeric graph | Preserves edge order, validates compact endpoints, builds outgoing and incoming CSR |
| `TraversalCache` | Optional derived neighbor topology | Keeps both directions and exact adjacency order; selects direct, bit-packed, or succinct storage |
| `WorkingGraph` | Fast rich mutation and incremental extraction | Preserves insertion order, validates local invariants once, uses generation-stable keys |
| `Graph` | Immutable evidence snapshot and wire format | Sorts, deduplicates, validates, and emits canonical output |
| `UndirectedTopology` | General-purpose undirected algorithms | Compact incidence CSR with parallel-edge and self-loop support |
| `DenseMatrix<T>` | Small dense graphs | Fixed-size O(1) edge lookup without sparse-graph overhead |
| `BitMatrix` | Large dense boolean relations | One bit per possible directed edge with safe O(1) lookup |
| `PayloadGraph<N, E>` | Arbitrary application payloads | Validated topology plus separately owned node and edge values |
| `StablePayloadGraph<N, E>` | Generic mutable application graph | Generation-checked stable keys, compact intrusive adjacency, payload mapping, and immutable `freeze()` |
| `KeyedPayloadGraph<K, N, E>` | Domain-key lookup and mutation | Hash-backed under `std`, deterministic tree-backed under `no_std`, stable handles for algorithms |
| `StableUndirectedPayloadGraph<N, E>` | Mutable undirected multigraph | Generation-checked node/edge keys, intrusive double-ended incidence, parallel edges, self-loops, retargeting, and compact `freeze()` |
| `AcyclicPayloadGraph<N, E>` | Mutable DAG workflows | Rejects cycle-creating inserts and edge retargeting |

`WorkingGraph::freeze()` is the explicit boundary between extraction and
publication. It returns the canonical `Graph` plus a stable-to-compact index
map. `Graph::try_from_sorted_nodes` avoids rebuilding the node-id map when an
extractor already emits unique sorted nodes, while
`Graph::try_from_sorted_parts` is the fastest fully canonical input path.

`KeyedPayloadGraph` keeps application keys out of algorithm internals: lookups
resolve to `StableNodeKey`, while its inner `StablePayloadGraph` implements the
shared graph views. `StableUndirectedPayloadGraph` implements
`IndexUndirectedGraphView` directly, so BCC, MST, matching, coloring, and other
undirected algorithms run before or after mutation without an adapter.

## Example

```rust
use weavatrix_graph::{
    Confidence, Edge, EdgeKind, EvidenceKind, GraphBuilder, Node, NodeKind,
    Provenance,
};

let repository = Node::new("repo:demo", "demo", NodeKind::Repository)?;
let file = Node::new("file:src/lib.rs", "src/lib.rs", NodeKind::File)?;

let mut builder = GraphBuilder::new();
builder.add_node(repository.clone())?;
builder.add_node(file.clone())?;
builder.add_edge(Edge::new(
    repository.id,
    file.id,
    EdgeKind::Contains,
    Provenance::new("example", EvidenceKind::Parsed, Confidence::High)?,
))?;

let graph = builder.build()?;
assert_eq!(graph.node_count(), 2);
assert_eq!(graph.edge_count(), 1);
# Ok::<(), weavatrix_graph::GraphError>(())
```

Algorithms use `GraphView`/`IndexGraphView`, so the same call works with a
canonical `Graph`, a numeric `Topology`, or a mutable `WorkingGraph`.
Filtering stays outside the topology and can inspect the evidence payload:

```rust
use weavatrix_graph::{
    Confidence, Direction, EdgeFilter, EdgeKind, EvidenceKind, Graph,
    bfs_filtered,
};

# fn inspect(graph: &Graph) -> Result<(), weavatrix_graph::GraphError> {
let Some(start) = graph.node_index("repo:demo") else {
    return Ok(());
};
let filter = EdgeFilter::new()
    .with_kind(EdgeKind::Contains)
    .with_evidence(EvidenceKind::Parsed)
    .with_minimum_confidence(Confidence::High);

let reachable = bfs_filtered(graph, start, Direction::Outgoing, |index| {
    graph.edge_at(index).is_some_and(|edge| filter.matches(edge))
});
assert!(!reachable.is_empty());
# Ok(())
# }
```

Component algorithms accept the same pure edge predicate. Condensation records
the original component membership and produces a compact, deduplicated DAG:

```rust
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, Topology, condensation_filtered,
    strongly_connected_components_filtered,
};

let topology = Topology::try_from_edges(
    3,
    [(0, 1), (1, 0), (1, 2)].map(|(source, target)| {
        EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
    }),
)?;
let without_back_edge = |edge: weavatrix_graph::EdgeIndex| edge.index() != 1;

let components = strongly_connected_components_filtered(
    &topology,
    without_back_edge,
);
let condensed = condensation_filtered(&topology, without_back_edge)?;
assert_eq!(components.len(), 3);
assert_eq!(condensed.topology().node_count(), 3);
# Ok::<(), weavatrix_graph::GraphError>(())
```

Advanced algorithms retain the same index/view contract. A* accepts an
admissible heuristic, Bellman-Ford reports checked signed overflow and reachable
negative cycles, and `PageRank` returns values in deterministic graph-node order:

```rust
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, Topology, astar, dominators, page_rank,
};

let graph = Topology::try_from_edges(
    4,
    [(0, 1), (0, 2), (1, 3), (2, 3)].map(|(source, target)| {
        EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
    }),
)?;
let weights = [2_u64, 5, 2, 1];
let Some(path) = astar(
    &graph,
    NodeIndex::new(0),
    NodeIndex::new(3),
    |edge| weights[edge.index()],
    |_| 0,
) else {
    return Ok(());
};
assert_eq!(path.total_cost(), 4);
assert_eq!(page_rank(&graph, 0.85, 20)?.len(), 4);
assert!(dominators(&graph, NodeIndex::new(0)).is_some());
# Ok::<(), weavatrix_graph::GraphError>(())
```

All potentially exponential enumeration APIs require a caller-provided result
limit and report truncation. Isomorphism accepts node and edge predicates, so
architecture tools can match semantic kinds without coupling the graph crate to
one language model:

```rust
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, SubgraphMode, Topology, johnson_all_pairs,
    subgraph_isomorphisms,
};

let graph = Topology::try_from_edges(
    3,
    [(0, 1), (1, 2)].map(|(source, target)| {
        EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
    }),
)?;
let paths = johnson_all_pairs(&graph, |_| 1)?;
assert_eq!(paths.distance(NodeIndex::new(0), NodeIndex::new(2)), Some(2));

let matches = subgraph_isomorphisms(
    &graph,
    &graph,
    SubgraphMode::Induced,
    1,
    |left, right| left == right,
    |_, _| true,
);
assert_eq!(matches.mappings().len(), 1);
# Ok::<(), weavatrix_graph::GraphError>(())
```

Biconnected and chain analysis are iterative and preserve original edge IDs.
Both treat parallel edges and self-loops explicitly. Exact distance analytics
return `None` for an empty or disconnected accepted-edge graph rather than
inventing finite metrics. HITS ranks topological relationships rather than
evidence multiplicity: equal endpoint pairs are deduplicated after the semantic
edge predicate runs once per edge.

```rust
use weavatrix_graph::{
    EdgeEndpoints, NodeIndex, Topology, UndirectedTopology,
    biconnected_components, chain_decomposition, distance_analytics, hits,
    stoer_wagner_min_cut, undirected_edge_betweenness_centrality,
};

let edges = [(0, 1), (1, 2), (2, 0), (1, 3)].map(|(source, target)| {
    EdgeEndpoints::new(NodeIndex::new(source), NodeIndex::new(target))
});
let undirected = UndirectedTopology::try_from_edges(4, edges)?;
let blocks = biconnected_components(&undirected);
assert_eq!(blocks.component_count(), 2);
assert_eq!(blocks.articulation_points(), &[NodeIndex::new(1)]);
let Some(distances) = distance_analytics(&undirected) else {
    return Ok(());
};
assert_eq!(distances.diameter(), 2);
assert_eq!(distances.center(), &[NodeIndex::new(1)]);
let chains = chain_decomposition(&undirected);
assert_eq!(chains.chain_count(), 1);
let edge_scores = undirected_edge_betweenness_centrality(&undirected, true);
assert_eq!(edge_scores.len(), 4);
let Some(cut) = stoer_wagner_min_cut(&undirected, |_| 1_u64)? else {
    return Ok(());
};
assert_eq!(cut.weight(), 1);
assert_eq!(cut.partition(), &[NodeIndex::new(3)]);

let directed = Topology::try_from_edges(4, edges)?;
let scores = hits(&directed, 100, 1e-10)?;
assert!(scores.hub(NodeIndex::new(1)).is_some());
# Ok::<(), weavatrix_graph::GraphError>(())
```

## P2 storage, interchange, and portability

`all_pairs_auto` snapshots every accepted edge weight once, selects
Floyd-Warshall for small or dense graphs and Johnson for sparse graphs, and
returns the chosen `AllPairsStrategy` with the result. Explicit algorithms
remain available when an application already knows the best strategy.

Interchange intentionally targets deterministic topology rather than pretending
to support every dialect:

- DOT reads and writes a strict numeric directed/undirected subset and ignores
  attributes;
- Graph6 supports simple undirected graphs and rejects loops and parallel edges;
- `GraphML` preserves graph direction and node declaration order while rejecting
  ports, hyperedges, and per-edge direction overrides.

All decoders validate endpoint references and rebuild the crate's canonical
topology indexes. `PayloadGraph<N, E>` and `UndirectedPayloadGraph<N, E>` attach
arbitrary payloads without weakening those topology invariants.

The crate defaults to `std`. Embedded and WASM consumers can disable default
features for a real `no_std + alloc` build:

```toml
[dependencies]
weavatrix-graph = { version = "0.6", default-features = false }
```

Parallel batches are explicit and optional:

```toml
[dependencies]
weavatrix-graph = { version = "0.6", features = ["rayon"] }
```

Large topology construction can select a measured sequential/Rayon crossover
while preserving stable edge indexes and adjacency order:

```rust
# use weavatrix_graph::{EdgeEndpoints, NodeIndex, Topology};
# let edges = [EdgeEndpoints::new(NodeIndex::new(0), NodeIndex::new(1))];
# #[cfg(feature = "rayon")]
# {
let topology = Topology::try_from_edges_auto(2, edges)?;
# assert_eq!(topology.edge_count(), 1);
# }
# Ok::<(), weavatrix_graph::GraphError>(())
```

`try_from_edges_parallel` always uses the safe stable-order Rayon builder.
`try_from_edges_parallel_unordered` keeps edge identity but leaves node-local
adjacency order unspecified. The automatic builder uses sequential construction
below 1.5 million edges, where scheduling and atomic setup usually cost more
than they save.

Traversal-heavy callers can derive a separate cache without weakening stable
edge identity or evidence storage:

```rust
# use weavatrix_graph::{
#     Direction, EdgeEndpoints, NodeIndex, Topology, TraversalCacheWorkspace,
#     TraversalStorage,
# };
# let topology = Topology::try_from_edges(
#     2,
#     [EdgeEndpoints::new(NodeIndex::new(0), NodeIndex::new(1))],
# )?;
let cache = topology.traversal_cache(); // speed-aware Auto policy
let mut workspace = TraversalCacheWorkspace::new();
let visited = cache.bfs_with_workspace(
    NodeIndex::new(0),
    Direction::Outgoing,
    &mut workspace,
);
assert_eq!(visited.len(), 2);

let compact = topology.traversal_cache_with(TraversalStorage::Compact);
assert_eq!(compact.edge_count(), 1);
# Ok::<(), weavatrix_graph::GraphError>(())
```

`Fast` stores direct `u32` neighbors and offsets. `Balanced` bit-packs neighbor
ids while retaining direct offsets. `Compact` adds Elias-Fano monotone offsets
and automatically uses block-local frame-of-reference neighbor packing only
when its exact encoded size beats global packing. `Auto` chooses `Balanced`
only when it saves at least 12.5%; otherwise it keeps `Fast`. All modes preserve
parallel edges, self-loops, and node-local adjacency order. `Graph` exposes the
same two convenience methods, and `From<&Topology>` / `From<&Graph>` are
available for generic construction.

`bfs_batch_parallel` and `dijkstra_batch_parallel` preserve query order and the
same deterministic result contract as their sequential counterparts. They are
intended for batches large enough to amortize scheduling, not as replacements
for a single small query.

Bit-matrix lookup has an additional, separately auditable performance feature:

```toml
[dependencies]
weavatrix-graph = { version = "0.6", features = ["unsafe-fast"] }
```

The default `BitMatrix::contains` remains fully safe. With `unsafe-fast`,
`contains_fast` keeps a safe API and validates both endpoints before one
unchecked word access. `contains_unchecked` removes endpoint checks too and is
an `unsafe fn`: callers must guarantee that both indexes are inside the matrix.
The same feature exposes `try_from_edges_parallel_fast` and its unordered
variant. Their public API remains safe; an isolated scatter backend writes
validated edge slots directly and reuses atomic cursor storage as final offsets.
Enabling the feature never silently changes the behavior of `contains`.

## Extension Kinds

Known relation and node kinds are enum variants. Ecosystem-specific kinds remain
forward-compatible through `Custom` values. Language taxonomies intentionally
belong to analyzers, not the graph core; nodes carry language as a validated
string label.

```rust
use weavatrix_graph::NodeKind;

let kind = NodeKind::custom("terraform_resource")?;
assert_eq!(kind.as_str(), "terraform_resource");
# Ok::<(), weavatrix_graph::GraphError>(())
```

## Weavatrix compatibility

The core graph format intentionally keeps canonical `nodes` and `edges`, but the
crate can ingest the current JavaScript Weavatrix `{ nodes, links }` shape:

```rust
use weavatrix_graph::{Graph, LegacyGraph};

let legacy: LegacyGraph = serde_json::from_str(r#"{
  "nodes": [
    { "id": "src/lib.rs", "label": "lib.rs" },
    { "id": "src/lib.rs#entry@1", "label": "entry()" }
  ],
  "links": [
    {
      "source": "src/lib.rs",
      "target": "src/lib.rs#entry@1",
      "relation": "contains",
      "confidence": "EXTRACTED"
    }
  ],
  "edgeTypesV": 2,
  "edgeProvenanceV": 1
}"#)?;

let graph: Graph = legacy.into_graph("weavatrix-js")?;
assert_eq!(graph.edge_count(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Legacy metadata such as `line`, `compileOnly`, `typeOnly`, `specifier`,
`usage`, `source_range`, and unknown extension fields is preserved as structured
attributes.

## Benchmarks

The repository includes benchmark harnesses for graph construction, indexed
queries, JSON serialization, validated deserialization, and dev-only
comparisons with `petgraph 0.8.3` and `graaf 0.112.0`:

```sh
cargo bench --locked
```

Unless a section states otherwise, each workload runs two warmups and 11
measured iterations. The tables below use the median of five independent
harness medians on Windows 11 with Rust 1.97.1. They compare equal contracts
where possible and label preprocessing explicitly.

### Rich evidence construction

10,000 nodes and 30,000 evidence-carrying edges:

| Mode | Library | Median |
| --- | --- | ---: |
| Unsorted canonical snapshot | weavatrix-graph `Graph` | 40.725 ms |
| Sorted canonical snapshot | weavatrix-graph `Graph` | 23.427 ms |
| Validated mutable append | weavatrix-graph `WorkingGraph` | 35.362 ms |
| Payload append, no canonicalization | petgraph adapter | 18.372 ms |
| Mutable append plus canonical `freeze()` | weavatrix-graph | 47.199 ms |

The petgraph adapter resolves string ids and clones the same payload but does
not validate, sort, or deduplicate it. `WorkingGraph` remains slightly faster
than full canonical construction while validating local invariants.
`freeze()` is reported separately because it adds canonical sorting, evidence
deduplication, and immutable CSR construction. At repository scale the sorted
canonical implementation amortizes those guarantees and moves ahead of the
narrower adapter; see the 200,000 / 1,000,000 table below.

### Compact dual CSR

10,000 numeric nodes and 30,000 edges:

| Mode | Library | Median |
| --- | --- | ---: |
| Arbitrary input, endpoint validation, both CSR directions | weavatrix-graph | 0.365 ms |
| Two CSR builds from caller-provided pre-sorted directions | petgraph | 0.463 ms |
| Sorting/dedup plus both CSR builds | petgraph | 1.699 ms |

The pre-sorted petgraph row deliberately excludes preparing two differently
sorted edge arrays. It is retained because that narrower contract can be useful
when a caller already owns both orders.

### Algorithms

10,000 nodes and 30,000 edges, except maximum flow at 1,000/5,000:

| Algorithm | weavatrix-graph | petgraph |
| --- | ---: | ---: |
| BFS | 0.107 ms | 0.157 ms |
| Strongly connected components | 0.474 ms | 0.650 ms |
| Dijkstra to one target | 0.913 ms | 0.960 ms |
| Minimum spanning forest | 1.288 ms | 1.856 ms |
| Dinic maximum flow | 0.322 ms | 0.379 ms |

Deterministic randomized differential tests also compare reachability, shortest
path existence and cost, SCC partitions, cycle status, topological feasibility,
MST weight, and maximum-flow value against petgraph.

### Repository scale: 200,000 nodes and 1,000,000 edges

`scale_graph_competitors` models files as nodes and deterministic dependency
relations as directed edges. Each independent process performs one warmup and
five measured iterations; the table is the median of five process medians.
Every process asserts exact node and edge counts, equal reachable counts, equal
SCC partitions, and the same target distance before reporting timings.

| Contract | weavatrix-graph | Competitor | Result |
| --- | ---: | ---: | --- |
| Dual CSR from arbitrary unique endpoints | 14.391 ms | petgraph adapter 78.735 ms | 5.47x faster |
| Narrow baseline: both directions pre-sorted for petgraph | 14.391 ms | petgraph 12.895 ms | petgraph 11.6% faster |
| Mutable append, no reverse CSR or canonicalization | — | petgraph 9.934 ms / graaf 30.969 ms | narrower contract |
| BFS, materialized reachable nodes | 13.026 ms | petgraph 49.606 ms / graaf 34.896 ms | 3.81x / 2.68x faster |
| Strongly connected components | 92.304 ms | petgraph 316.340 ms | 3.43x faster |
| Dijkstra to one target | 69.190 ms | petgraph 102.050 ms | 1.47x faster |
| Rich evidence snapshot from sorted owned payloads | 623.691 ms | petgraph adapter 644.840 ms | 3.4% faster |

The generated workload contains no duplicate endpoint pair. Weavatrix validates
the arbitrary endpoints and builds both directions by linear counting placement
while retaining stable original edge indexes and support for parallel edges.
The equal-input petgraph adapter must produce, sort, and deduplicate two edge
orders for its simple CSR contract. Its pre-sorted row receives both orders
already prepared and therefore measures a deliberately narrower input
contract. Mutable append is shown only as a lower-bound reference because it
does not build an immutable dual-CSR snapshot.

The rich adapter resolves the same string ids and moves the same node/edge
payloads, but does not validate payloads, check canonical ordering, deduplicate
evidence, or build reverse CSR. The 6.2% construction premium is therefore the
remaining scale tradeoff for the stronger snapshot contract, not an
algorithmic correctness gap.

Median peak working set from three fresh processes, including input generation,
temporary construction storage, allocator high-water state, and the live
result:

| Construction | weavatrix-graph | Competitor |
| --- | ---: | ---: |
| Compact dual CSR | 37.9 MiB | petgraph dual CSR 68.5 MiB |
| Mutable adjacency | — | petgraph 44.9 MiB / graaf 47.5 MiB |
| Rich evidence graph, ownership transferred | 377.1 MiB | petgraph adapter 585.8 MiB |

Peak working set is a capacity-planning number, not the retained deep size of
the graph. Reproduce the scale run with:

```sh
cargo bench --locked --all-features --bench scale_graph_competitors
```

### Parallel construction at 1M, 10M, and 100M edges

This follow-up compares parallel with parallel. It ran locally on Windows 11,
Rust 1.97.1, and an Intel Core Ultra 7 255U with 14 Rayon workers. The
deterministic input has five outgoing edges per node. The 1M and 10M rows are
medians of nine and seven measured builds; 100M is a three-build median after
one warmup.

| Nodes / edges | weavatrix-graph | `graph_builder 0.4.2` Rayon | Result |
| --- | ---: | ---: | --- |
| 200k / 1M, auto stable | 14.165 ms | 15.223 ms | Weavatrix 7.5% faster |
| 2M / 10M, safe stable | 178.705 ms | 115.697 ms | narrower competitor 35.3% faster |
| 2M / 10M, `unsafe-fast` stable | 117.888 ms | 115.697 ms | within 1.9% |
| 20M / 100M, `unsafe-fast` stable | 1.705 s | 1.518 s | narrower competitor 12.3% faster |
| 20M / 100M, `unsafe-fast` unordered | 1.662 s | 1.518 s | narrower competitor 9.5% faster |

Both sides ingest arbitrary endpoint order, validate or infer bounds, and build
incoming plus outgoing CSR. The contracts are not identical:
`weavatrix-graph::Topology` also retains the original endpoint array, stable
`EdgeIndex` identity, parallel-edge identity, and, in stable modes,
deterministic node-local edge order. `graph_builder` stores direct neighbor
targets and uses internal unchecked scatter writes. Its row is a throughput
lower bound, not evidence that the stronger snapshot is incorrect. Every
Weavatrix stable build is asserted equal to the sequential topology before
timing.

The original evidence topology exposes the cost of resolving an `EdgeIndex`
through the endpoint array:

| Nodes / edges | Weavatrix evidence CSR | Direct-neighbor CSR | Result |
| --- | ---: | ---: | --- |
| 200k / 1M | 10.179 ms | 6.902 ms | direct neighbors 1.47x faster |
| 2M / 10M | 102.811 ms | 75.804 ms | direct neighbors 1.36x faster |

Both adapters assert the same reachable count. The derived `TraversalCache`
closes that gap without changing `Topology`. An interleaved, allocation-reusing
BFS comparison on the same five-outgoing-edge workload measured:

| Nodes / edges | Layout | Encoded dual-cache bytes | Weavatrix BFS | Paired `graph_builder` BFS | Result |
| --- | --- | ---: | ---: | ---: | --- |
| 200k / 1M | Fast | 9.60 MB | 17.131 ms | 17.636 ms | 2.9% faster |
| 200k / 1M | Balanced | 6.10 MB | 28.104 ms | 18.197 ms | 36.5% less memory, slower traversal |
| 200k / 1M | Compact | 4.55 MB | 76.027 ms | 18.592 ms | 52.6% less memory, smallest layout |
| 2M / 10M | Fast | 96.00 MB | 345.978 ms | 394.968 ms | 12.4% faster |
| 2M / 10M | Balanced | 68.50 MB | 464.830 ms | 322.858 ms | 28.6% less memory |
| 2M / 10M | Compact | 48.78 MB | 1,251.840 ms | 415.216 ms | 49.2% less memory |

The times are medians of nine runs at 1M edges and five runs at 10M, with the
measurement order alternated every run. Cache and competitor return the same
reachable count; both reuse traversal marks and queue storage. The compressed
layouts are explicit space/latency choices, not claims that bit decoding is
free. At 20M nodes / 100M edges the measured encoded sizes were 960.00 MB
(`Fast`), 785.00 MB (`Balanced`), and 536.76 MB (`Compact`). On adjacency with
local ids, Compact can reduce this further through block-local frames while
preserving the original order.

On the equal all-pairs output contract, parallel Johnson measured 41.638 ms
here versus petgraph's 64.769 ms, 1.56x faster.

The real-filesystem harness scanned `C:\Windows` with `weavatrix-scan 0.3.0`,
then built a parent-child containment graph. The scan returned 192,575 files,
240,734 nodes, 240,733 edges, and 51 warnings (`complete=false`) in 1.904 s.
Scanning is outside the graph-build interval:

| Real containment graph | Median |
| --- | ---: |
| Weavatrix auto stable | 3.469 ms |
| Weavatrix forced Rayon stable | 6.782 ms |
| `graph_builder` Rayon, narrower contract | 5.653 ms |

The real graph confirms why automatic selection matters: sequential construction
is 38.6% faster than the narrower parallel competitor at this size. Reproduce
the synthetic and filesystem runs with:

```powershell
cargo bench --locked --all-features --bench parallel_scale_competitors
cargo bench --locked --all-features --bench traversal_cache_competitors
$env:WEAVATRIX_GRAPH_NODES=20000000
$env:WEAVATRIX_GRAPH_EDGES=100000000
$env:WEAVATRIX_GRAPH_RUNS=3
$env:WEAVATRIX_GRAPH_MODE="fast"
cargo bench --locked --all-features --bench parallel_scale_competitors
$env:WEAVATRIX_REAL_ROOT="C:\Windows"
cargo bench --locked --all-features --bench filesystem_graph_workload
```

### Advanced algorithms

The A*, dominator, and DAG-intelligence workloads use 10,000 nodes / 30,000
edges. Bellman-Ford uses a 1,000-node / 5,000-edge signed DAG, `PageRank` uses
500 nodes / 2,000 unique edges and 20 iterations, and DAG reduction/closure
uses 512 nodes / 3,000 edges. Values are the median of five independent harness
medians:

| Algorithm | weavatrix-graph | petgraph | Result |
| --- | ---: | ---: | --- |
| A*, zero heuristic, cost and path | 1.130 ms | 1.495 ms | 1.32x faster |
| Bellman-Ford, distances and predecessors | 0.057 ms | 0.033 ms | 1.73x slower |
| `PageRank`, 20 iterations | 0.071 ms | 12.892 ms | 181.58x faster |
| Immediate dominators | 2.161 ms | 3.163 ms | 1.46x faster |
| Maximum-cost DAG path | 0.496 ms | 0.628 ms adapter | 1.27x faster |
| Topological generations | 0.655 ms | 0.804 ms adapter | 1.23x faster |
| Dominance frontiers | 4.081 ms | 6.151 ms adapter | 1.51x faster |
| DAG transitive reduction and closure | 0.684 ms | 1.051 ms | 1.54x faster |

The DAG row includes `petgraph`'s required conversion to a topologically ordered
adjacency list; `weavatrix-graph` accepts the original graph, validates
acyclicity, and returns deterministic node endpoints. The `PageRank` workload has
no parallel edges; our implementation is O(V + E) per iteration and follows
the standard teleport plus uniform dangling-mass contract. Its correctness is
checked against an independent reference because `petgraph 0.8.3` uses a
different transition formula.

The Bellman-Ford row is intentionally retained as a known tradeoff, not hidden:
our operation snapshots the filtered signed weights once, uses checked `i64`
addition, distinguishes unreachable nodes without an infinity sentinel, and
returns overflow or reachable-negative-cycle errors. `petgraph` uses `f64`
distances and does not provide integer overflow semantics. Randomized
differential tests cover A* costs, Bellman-Ford distances and negative-cycle
status, immediate dominators, longest-path costs, topological generation
invariants, dominance-frontier memberships, and exact transitive
reduction/closure edges. Petgraph does not expose the three DAG-intelligence
operations directly; those rows use equal-output adapters over its graph,
topological-sort, and immediate-dominator APIs.

### P0 all-pairs, cuts, and isomorphism

Local Windows release-mode sample from 2026-07-24, with two warmups and the
median of 11 measured runs over deterministic synthetic graphs:

| Contract | Workload | weavatrix-graph | petgraph |
| --- | --- | ---: | ---: |
| Floyd-Warshall APSP | 160 nodes / 1,200 weighted edges | 2.874 ms | 3.319 ms |
| Johnson APSP | 800 nodes / 4,000 weighted edges | 52.636 ms | 67.018 ms |
| Bridges plus articulation points | 5,000 nodes / 15,000 edges | 0.362 ms | 0.302 ms bridges only |
| Exact directed isomorphism | 64 nodes / 300 edges | 0.100 ms | 0.031 ms |

The cuts row is deliberately marked as a different contract rather than a speed
win: our traversal returns both cut-edge and cut-vertex evidence. Isomorphism is
a known optimization gap. Randomized differential tests compare APSP distances,
bridges, articulation points, and exact isomorphism against `petgraph 0.8.3`.
Integer Floyd-Warshall additionally preserves unreachable pairs as `None`
instead of allowing a negative edge to modify an infinity sentinel.

### Biconnected structure and HITS

Biconnected components use 2,000 nodes / 6,000 simple undirected edges. HITS
uses 10,000 nodes / 30,000 unique directed relationships, an iteration cap of
100, and tolerance `1e-10`. Values are the median of five independent harness
medians:

| Contract | weavatrix-graph | Reference | Result |
| --- | ---: | ---: | --- |
| Biconnected edge blocks plus articulation points | 0.225 ms | equal-output adapter 0.257 ms | 1.14x faster |
| HITS hubs and authorities, L2 normalized | 7.704 ms | petgraph equal-equation adapter 7.950 ms | 1.03x faster |

The direct BCC competitor uses recursive Hopcroft-Tarjan traversal and
overflowed the Windows stack on the 10,000 / 30,000 benchmark input. The
Weavatrix traversal is iterative; its regression suite includes a 200,000-node
chain. Its raw node-block-only lower bound is 0.157 ms, but adding original
edge blocks and articulation points raises it to 0.257 ms. The table compares
that equal output instead of presenting the narrower operation as equivalent.

Petgraph does not expose HITS directly, so that row uses an equal-equation
adapter over its graph storage, including endpoint deduplication, L2
normalization, convergence checks, and the same cap. Seeded dense-matrix
references check both score vectors.

### Keyed and stable undirected mutation

The keyed build uses 10,000 domain keys / 30,000 directed edges. Stable
undirected construction uses the same size; the churn row removes and reinserts
1,000 edges with cloning/setup outside the timed interval:

| Contract | weavatrix-graph | petgraph | Result |
| --- | ---: | ---: | --- |
| Keyed directed build | 0.650 ms | `GraphMap` 3.788 ms | 5.83x faster |
| Stable undirected build | 0.557 ms | `StableUnGraph` 0.230 ms | petgraph 2.42x faster |
| Stable undirected 1,000 remove + 1,000 insert | 0.169 ms | `StableUnGraph` 0.018 ms | petgraph 9.39x faster |

The keyed row is conservative for Weavatrix: it also stores independent node
payloads and returns generation-stable handles. The stable rows document a
deliberate safety cost rather than hiding it: Weavatrix detects stale node and
edge keys after slot reuse, while petgraph stable indexes can alias a reused
slot. Both Weavatrix operations use intrusive incidence and perform no
per-node heap allocation.

### Distance analytics and chain decomposition

Distance analytics use 1,500 nodes / 4,500 connected undirected edges. Chain
decomposition uses 20,000 nodes / 60,000 simple undirected edges. Values are the
median of five independent harness medians; every harness performs two warmups
and 11 measured runs:

| Equal output contract | weavatrix-graph | Reference | Result |
| --- | ---: | ---: | --- |
| Eccentricity, radius, diameter, center, and periphery | 30.890 ms | petgraph equal-output adapter 61.343 ms | 1.99x faster |
| Chain decomposition | 5.958 ms | `rustworkx-core 0.18.0` 10.797 ms | 1.81x faster |

The distance fast path builds compact neighbor CSR once, then reuses
epoch-stamped BFS storage for every source. The petgraph adapter computes the
same five outputs, reuses its queue and distance vector, and rejects
disconnected graphs under the same contract.

The chain row calls rustworkx-core directly and compares the exact non-bridge
endpoint set and chain count. Weavatrix additionally returns original edge IDs
and has defined behavior for parallel edges and self-loops; rustworkx documents
those inputs as unsupported. Seeded differential tests cover simple graphs, an
independent edge-removal reference checks exact non-bridge coverage, and a
200,000-node chain protects the iterative stack-safety contract.

### Edge betweenness and global min-cut

Edge betweenness uses 1,200 nodes / 4,800 simple undirected edges and returns a
normalized score for every original edge. Stoer-Wagner uses 350 nodes / 1,400
weighted edges. Values are the median of five independent harness medians;
every harness performs two warmups and 11 measured runs:

| Equal output contract | weavatrix-graph | `rustworkx-core 0.18.0` | Result |
| --- | ---: | ---: | --- |
| Edge betweenness, sequential | 71.367 ms | 262.602 ms | 3.68x faster |
| Edge betweenness, Rayon | 14.985 ms | 294.388 ms | 19.64x faster |
| Stoer-Wagner global min-cut | 22.644 ms | 24.012 ms | 1.06x faster |

Both edge-centrality rows call the rustworkx implementation directly and assert
every edge score before timing. The Weavatrix implementation keeps
source-local score vectors in the Rayon path and reduces them after traversal;
it also preserves parallel-edge identities, gives self-loops a defined zero
score, and evaluates semantic filters once per edge.

The min-cut row asserts the same minimum weight. Weavatrix additionally checks
negative/non-finite weights and arithmetic overflow, aggregates parallel edges,
and returns both sides in deterministic canonical order; rustworkx returns one
partition side and uses unchecked weight addition. Reproduce these rows with:

```sh
cargo bench --locked --all-features --bench edge_cut_analysis
```

### P1 matching, coloring, feedback, and Steiner

The workloads use 400 nodes / 1,400 edges for general matching, 180 / 500 for
maximal cliques, 5,000 / 15,000 for DSATUR, 10,000 / 30,000 directed edges for
feedback arc set, and 1,000 / 4,000 with 32 terminals for Steiner tree:

| Equal output contract | weavatrix-graph | petgraph | Result and quality |
| --- | ---: | ---: | --- |
| Exact maximum matching, materialized pairs | 0.097 ms | 0.124 ms | 1.28x faster; 200 pairs |
| All maximal cliques | 0.164 ms | 0.368 ms | 2.24x faster; 500 cliques |
| DSATUR coloring | 3.299 ms | 3.550 ms | 1.08x faster; 5 colors |
| Eades feedback arc set | 2.375 ms | 3.431 ms | 1.44x faster; 8,216 edges |
| Multi-start metric-closure Steiner tree | 5.596 ms | 965.055 ms | 172.45x faster; cost 340 vs median 344 |

The matching adapter consumes `petgraph::Matching::edges()` into endpoint pairs
instead of timing its lazy result. Feedback returns the same set cardinality on
the benchmark and no more edges across 24 seeded differential cases. DSATUR
uses the same color count across those cases. The deterministic multi-start
Steiner result is no more expensive across 20 seeded comparisons and preserves
the standard metric-closure two-approximation candidate; the brute-force tests
also verify the bound directly on small graphs. Petgraph's Steiner cost varied
from 340 to 344 across the five benchmark processes, while the
weavatrix-graph result remained 340.

### P2 bit matrix and optional parallel batches

The bit-matrix workload performs one million deterministic adjacency lookups
over 10,000 nodes / 30,000 edges. The batch workloads run 128 BFS traversals and
64 weighted Dijkstra queries over the same dual-CSR topology. Values are the
median of five independent harness medians:

| Equal output contract | Sequential / competitor | P2 implementation | Result |
| --- | ---: | ---: | --- |
| Safe bit-matrix lookup | petgraph `FixedBitSet` 5.284 ms | `BitMatrix::contains` 5.396 ms | within 2.1% |
| Checked opt-in fast lookup | petgraph `FixedBitSet` 5.284 ms | `contains_fast` 4.804 ms | 1.10x faster |
| Caller-validated lookup | petgraph `FixedBitSet` 5.284 ms | `contains_unchecked` 3.382 ms | 1.56x faster |
| 128 BFS traversals | sequential 33.258 ms | Rayon 6.609 ms | 5.03x faster |
| 64 Dijkstra queries | sequential 39.771 ms | Rayon 8.699 ms | 4.57x faster |
| Johnson APSP, 1,200 / 6,000 | sequential 217.512 ms | Rayon 64.038 ms | 3.40x faster |
| Closeness, 1,000 / 4,000 | sequential 21.041 ms | Rayon 4.818 ms | 4.37x faster |
| Betweenness, 1,000 / 4,000 | sequential 139.065 ms | Rayon 25.033 ms | 5.56x faster |

`BitMatrix` uses exactly 12,500,000 payload bytes for the 10,000-square matrix
in every mode. The default path is fully safe and essentially at parity.
The checked opt-in path beats petgraph while retaining a safe public contract;
the caller-validated path is fastest. Unsafe code is feature-gated, isolated,
and rejected everywhere else by an architecture test. Rayon is not enabled by
default and has no effect on single-query algorithms or `no_std` builds.

### Lazy traversal, generic costs, and stable mutation

The traversal graph has 50,000 nodes and 150,000 weighted edges. Lazy BFS stops
after 128 settlements, generic Dijkstra uses `f64`, and the mutation workload
builds the same graph before removing and replacing 100 nodes. Values are the
median of five independent harness medians:

| Equal output contract | weavatrix-graph | petgraph | Result |
| --- | ---: | ---: | --- |
| Lazy BFS, first 128 nodes | 0.002 ms | 0.003 ms | 1.50x faster |
| Generic `f64` Dijkstra to target | 8.342 ms | 11.936 ms | 1.43x faster |
| Stable build/remove/reinsert | 2.247 ms | 2.227 ms | within 0.9% |

The mutable row is conservative: `StablePayloadGraph` also detects stale keys
after slot reuse through a generation counter. Its adjacency is an intrusive
slot list, so construction performs no per-node adjacency allocations. Filtered
views are genuinely lazy: creating an adjacency iterator does not evaluate its
predicate, and early stopping evaluates only the consumed prefix.

### Filtered components and condensation

10,000 nodes and 30,000 edges. Each measured sample batches 64 operations; the
table is the median of five independent harness medians:

| Equal output contract | weavatrix-graph | petgraph |
| --- | ---: | ---: |
| Filtered SCC memberships | 0.899 ms | 1.353 ms |
| Filtered topological order | 0.080 ms | 0.427 ms |
| Weak component memberships | 0.206 ms | 0.208 ms |
| Condensation DAG and memberships | 0.821 ms | 2.385 ms |

The petgraph filtered rows use `EdgeFiltered` rather than rebuilding a graph.
Both weak-component rows return complete deterministic memberships, not only a
component count. Condensation consumes the petgraph input, so input clones are
prepared outside the timed interval. Randomized differential tests compare
exact SCC and weak-component partitions plus canonical condensation edges.

Incoming and outgoing indexes are rebuilt during graph construction and
deserialization. They are intentionally excluded from JSON, so the canonical
wire format remains only `nodes` and `edges`. Resolve a stable string id once
with `node_index`, then use `node_at`, `outgoing_at`, `incoming_at`,
`out_degree`, and `in_degree` in repeated graph algorithms.

Extractors that already emit sorted nodes can use
`Graph::try_from_sorted_nodes`; fully canonical input can use
`Graph::try_from_sorted_parts`. Both keep validation, endpoint checks,
deduplication, and both indexes. Unordered input safely falls back to the
canonicalizing constructor.

`petgraph`, `graaf`, `graph_builder`, `rustworkx-core`, and `weavatrix-scan` are
dev-dependencies only. The default runtime dependency budget remains `serde`;
Rayon and its transitive dependencies appear only when the `rayon` feature is
explicitly selected.

Timing varies by allocator, CPU, and build toolchain. Run the included harnesses
on the deployment target before using these figures for capacity planning.

## Quality Gates

Local checks:

```sh
cargo fmt --check
cargo test --all-features --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo check --no-default-features --target thumbv7em-none-eabihf --lib --locked
cargo check --no-default-features --features unsafe-fast --target thumbv7em-none-eabihf --lib --locked
cargo doc --locked --no-deps --all-features
cargo llvm-cov --workspace --all-features --fail-under-lines 85
cargo bench --all-features --locked
```

The release gates combine the test suite with strict architecture verification:
every Rust source stays at or below 300 lines, every function stays at or below
100 lines, module source forms cannot collide, domain facades remain focused,
runtime dependencies remain limited, and canonical kind strings cannot
collide. Production source contains no `.unwrap()` or `.expect()` calls:
internal graph invariants use checked conversions, fallible propagation, or
total control flow instead of process-terminating shortcuts.

CI also runs measured Rust coverage with `cargo-llvm-cov`, emits `lcov.info`
for analyzer import, and fails below 85% line coverage. It additionally runs
Linux `proptest` contracts, Miri over traversal/view/analytics/mutable storage,
and bounded libFuzzer smoke targets for topology, matrices, mutation, and
interchange formats. Weavatrix architecture verification is backed by the
strict `.weavatrix/architecture.json` contract. The current local
MSVC LLVM report measures 92.28% of lines and 88.81% of functions.

## License

MIT
