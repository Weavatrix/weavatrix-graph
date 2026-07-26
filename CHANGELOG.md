# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

- Add safe and opt-in `unsafe-fast` parallel dual-CSR builders with stable and
  unordered adjacency policies, automatic scale selection, Miri coverage, and
  reproducible 1M/10M/100M plus real-filesystem benchmarks.
- Add an optional dual-direction `TraversalCache` with direct, bit-packed, and
  succinct Elias-Fano layouts, adaptive local-id compression, exact adjacency
  preservation, lazy/eager BFS and DFS, reusable workspaces, and competitor
  benchmarks.

## 0.5.0 - 2026-07-24

- Add `KeyedPayloadGraph` for GraphMap-style domain lookup backed by
  generation-stable handles under both `std` and `no_std`.
- Add `StableUndirectedPayloadGraph` with intrusive double-ended incidence,
  parallel-edge/self-loop semantics, stable retargeting and removal, direct
  algorithm views, and immutable compaction.
- Specialize eager BFS storage, accelerate sorted rich-edge indexing, and add
  equal-contract keyed/stable competitor benchmarks plus the 200,000-node /
  1,000,000-edge regression.
- Remove BCC per-block sorting and optional discovery states, add an
  equal-output competitor adapter, and specialize dense HITS convergence; the
  measured equal-output implementations are now 1.14x and 1.03x faster.
- Add directed and undirected Brandes edge betweenness with normalized/raw,
  filtered, multigraph-safe, self-loop-defined, and Rayon variants using
  allocation-reusing per-source workspaces.
- Add generic checked Stoer-Wagner global min-cut with deterministic two-sided
  partitions, parallel-edge aggregation, semantic filtering, and finite
  non-negative `Measure` weights.
- Add exact examples, seeded rustworkx differential tests, exhaustive
  brute-force min-cut coverage, and equal-output competitor benchmarks; the
  measured sequential/parallel edge-centrality paths are 3.68x/19.64x faster
  and min-cut is 1.06x faster.
- Add exact connected-graph eccentricity, radius, diameter, center, and
  periphery analytics with canonical output, single-pass semantic filtering,
  reusable BFS storage, and a compact neighbor-CSR fast path.
- Add iterative deterministic chain decomposition with original edge IDs,
  component-rooted and filtered variants, plus defined parallel-edge and
  self-loop behavior.
- Add exhaustive and seeded references, rustworkx differential coverage, a
  200,000-node stack-safety regression, and equal-output distance/chain
  benchmarks; the measured implementations are 1.99x and 1.81x faster,
  respectively.
- Add iterative vertex-biconnected edge components with canonical ordering,
  articulation points, parallel-edge/self-loop handling, semantic filtering,
  exhaustive references, and a 200,000-node stack-safety regression.
- Add deterministic L2-normalized HITS hub and authority centrality with
  single-pass edge filtering and provenance-neutral endpoint deduplication.
- Add equal-result BCC and HITS benchmark adapters, documenting both the
  recursive competitor's stack overflow and the remaining small-graph
  throughput gaps.
- Add deterministic weighted and unweighted DAG longest paths, topological
  generations, and dominance frontiers with semantic edge filtering.
- Add seeded differential references and equal-output competitor benchmarks for
  DAG intelligence; replace an initial O(E * V) frontier reachability check
  exposed by the benchmark with slot-indexed O(V + E) preparation.
- Add lazy BFS, DFS, and generic Dijkstra settlement iterators with reusable
  workspaces, early stopping, and callback-controlled DFS events.
- Generalize shortest-path costs through checked integer and finite
  floating-point `Measure` implementations.
- Add lazy reversed, edge-filtered, and induced-subgraph views without
  predicate pre-scans, plus complement and union operators.
- Add degree, closeness, betweenness, Katz, and eigenvector centrality, k-core,
  cycle basis, and deterministic label-propagation communities.
- Add Edmonds-Karp, push-relabel, min-cost max-flow, Prim, parallel Johnson
  APSP, and parallel centrality.
- Add `StablePayloadGraph` with compact intrusive adjacency, generation-checked
  keys, payload mapping, immutable compaction, and `AcyclicPayloadGraph`.
- Add seeded max-flow differential tests, exhaustive min-cost-flow reference
  checks, Linux property tests, Miri coverage, and libFuzzer mutation targets.
- Add equal-contract lazy traversal, generic Dijkstra, and stable-mutation
  competitor benchmarks; remove per-node adjacency allocations after the first
  benchmark exposed a stable-build bottleneck.
- Add a 200,000-node / 1,000,000-edge scale harness with exact-count and
  differential correctness checks, fair dual-CSR preprocessing comparisons,
  rich evidence construction, and isolated peak-working-set measurements.
- Add P0/P1 shortest-path, cut, isomorphism, matching, clique, coloring,
  feedback-arc, Steiner-tree, and structural graph algorithms with seeded
  differential tests and equal-contract competitor benchmarks.
- Add automatic Floyd-Warshall/Johnson selection with one-time weight
  snapshotting and an explicit strategy result.
- Add validated deterministic DOT, Graph6, and GraphML topology interchange.
- Add `BitMatrix`, seeded DAG plus deterministic star/grid/bipartite
  generators, and generic directed/undirected payload graphs.
- Add a real `no_std + alloc` build while retaining hash-backed maps under the
  default `std` feature.
- Add optional Rayon BFS and Dijkstra batch APIs that preserve input order.
- Add an opt-in `unsafe-fast` bit-matrix module with checked and
  caller-validated lookup APIs; keep the default lookup fully safe.
- Add P2 correctness tests, release benchmarks, documentation, and a dedicated
  CI gate for `cargo check --no-default-features --lib`.
- Keep the MIT license, isolate unsafe code to one feature-gated module, retain
  the 300-line source budget, and keep `serde` as the only default dependency.

## 0.4.0 - 2026-07-23

- Add A* with admissible heuristics, direction selection, and edge filtering.
- Add checked signed Bellman-Ford distances, predecessors, path reconstruction,
  overflow reporting, and reachable-negative-cycle detection.
- Add standard O(V + E)-per-iteration PageRank with uniform dangling-mass
  redistribution and semantic-edge filtering.
- Add deterministic control-flow dominators with immediate, strict, dominance,
  and child queries.
- Add bitset-backed DAG transitive reduction and closure with cycle rejection,
  canonical endpoints, duplicate-edge elimination, and filtered variants.
- Add deterministic reference cases and randomized differential tests against
  `petgraph` for shortest paths, negative cycles, dominators, and DAG results.
- Add reproducible competitor benchmarks and document the one known
  Bellman-Ford numeric-contract tradeoff instead of conflating unlike results.
- Keep the MIT license, `unsafe` ban, 300-line source budget, Rust 1.88 MSRV,
  and unchanged one-dependency runtime surface.

## 0.3.0 - 2026-07-23

- Add filter-aware SCC, cycle discovery, cycle checks, and topological sorting
  for semantic subgraphs selected by edge kind or provenance.
- Add deterministic weakly connected components, including a filtered variant,
  using a safe compact union-find implementation.
- Add `Condensation` snapshots with component membership lookup, deduplicated
  cross-component edges, and a validated acyclic `Topology`.
- Add `GraphView::edge_references` with optimized immutable graph and topology
  implementations for one-pass generic algorithms.
- Add randomized differential correctness tests against `petgraph` for
  filtered components, weak components, and condensation structure.
- Add batched equal-contract component benchmarks with setup excluded from
  consuming condensation measurements.
- Keep the MIT license, `unsafe` ban, 300-line source budget, and unchanged
  one-dependency runtime surface.

## 0.2.0 - 2026-07-23

- Add compact directed `Topology` with validated numeric endpoints and outgoing
  plus incoming CSR.
- Add the shared `GraphView`/`IndexGraphView` contracts and algorithms for BFS,
  DFS, reachability, shortest paths, SCC, cycles, topological sorting, MST, and
  Dinic maximum flow.
- Add evidence-aware traversal filtering by edge kind, provenance, extractor,
  and confidence.
- Add mutable `WorkingGraph` with local validation, generation-stable keys,
  removals/replacements, and explicit canonical `freeze()` remapping.
- Add `UndirectedTopology`, `DenseMatrix<T>`, and dependency-free deterministic
  graph generators.
- Add `Graph::try_from_sorted_nodes` and bulk construction paths that avoid the
  previous heavy builder when node order is already canonical.
- Add randomized differential correctness tests and equal-contract performance
  harnesses against `petgraph`, plus `graaf` topology comparisons.
- Keep the MIT license, `unsafe` ban, 300-line file budget, and `serde` as the
  only runtime dependency.

## 0.1.2 - 2026-07-23

- Add dev-only differential benchmarks against `petgraph` and `graaf`.
- Add compact node indexes and O(1) in/out degree queries for repeated graph
  algorithms.
- Replace per-node map indexes with source ranges and a compact incoming index.
- Canonicalize edges in source buckets while preserving deterministic output,
  validation, and evidence deduplication.
- Add a checked sorted-input fast path that avoids redundant canonical sorting
  and safely falls back for unordered input.
- Update performance documentation with comparable and non-comparable modes.

## 0.1.1 - 2026-07-23

- Add deterministic incoming and outgoing adjacency indexes.
- Reduce a 10,000-node/30,000-edge full adjacency workload from seconds to
  milliseconds without changing the serialized graph contract.
- Add repeatable build, query, JSON serialization, and validated
  deserialization benchmarks.
- Document benchmark methodology and sample results.

## 0.1.0 - 2026-07-22

- Initial typed graph model and builder.
- Deterministic ordering and idempotent insertion.
- Evidence provenance and source spans.
- Extensible node, edge, and evidence kinds.
- Language remains a validated node label instead of a graph-core taxonomy.
- Canonical Weavatrix relation/provenance values, including `method`,
  `implements`, `re_exports`, `EXACT_LSP`, `EXTRACTED`, `RESOLVED`,
  `INFERRED`, and `CONFLICT`.
- Structured node and edge attributes without adding a runtime JSON dependency.
- Compatibility conversion from Weavatrix's legacy `{ nodes, links }` graph.
- Integrity-checked serialization boundary.
