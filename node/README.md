# weavatrix-graph

A deterministic, evidence-carrying directed graph with production algorithms —
written in Rust, exposed to Node.js and Bun through Node-API.

Every edge carries **provenance**: which extractor produced it, what evidence
backs it, how confident that evidence is, and optionally where in the source it
came from. That is the difference between a graph you can query and a graph you
can defend.

```console
npm install weavatrix-graph
# or
bun add weavatrix-graph
```

```js
const { Graph } = require('weavatrix-graph')

const graph = new Graph({
  nodes: [
    { id: 'api', label: 'API', kind: 'service' },
    { id: 'db', label: 'Database', kind: 'table' },
  ],
  edges: [{
    source: 'api',
    target: 'db',
    kind: 'reads',
    provenance: { extractor: 'architecture', evidence: 'parsed', confidence: 'exact' },
  }],
})

graph.shortestPath('api', 'db')          // ['api', 'db']
graph.hasCycle()                          // false
graph.pageRank({ damping: 0.85 })         // [{ id, score }, …]
graph.outgoing('api')[0].provenance        // why that edge exists
```

---

## Input

### `GraphNode`

| Field | Type | Notes |
| --- | --- | --- |
| `id` | `string` | Stable identity. Must be unique. |
| `label` | `string` | Human-readable name. |
| `kind` | `string` | Free-form node category. |
| `language` | `string?` | |
| `span` | `SourceSpan?` | `{ file, start: { line, column }, end: { line, column } }` |
| `attributes` | `Record<string, AttributeValue>?` | Nested JSON values are allowed. |

### `GraphEdge`

| Field | Type | Notes |
| --- | --- | --- |
| `source`, `target` | `string` | Must reference declared node ids. |
| `kind` | `string` | Relation type, such as `calls`, `reads`, `implements`. |
| `provenance` | `Provenance` | **Required.** `{ extractor, evidence, confidence, span?, detail? }` where `confidence` is `'exact' \| 'high' \| 'medium' \| 'low'`. |
| `attributes` | `Record<string, AttributeValue>?` | |

Requiring provenance on construction is deliberate: an edge nobody can justify
never enters the graph.

---

## API

### `new Graph(input)`

Validates the whole input and throws on a duplicate node id, a dangling edge
endpoint, or a malformed provenance record.

| Member | Returns | Notes |
| --- | --- | --- |
| `nodeCount` | `number` | |
| `edgeCount` | `number` | |
| `canonical()` | `GraphInput` | Deterministic serialization. Two graphs with equal content produce byte-identical output, which is what makes a graph diffable and cacheable. |
| `node(id)` | `GraphNode \| undefined` | |
| `outgoing(id)` | `GraphEdge[]` | Edges leaving `id`, with provenance intact. |
| `incoming(id)` | `GraphEdge[]` | Edges arriving at `id`. |
| `bfs(start)` | `string[]` | Materialized breadth-first visit order. |
| `shortestPath(source, target)` | `string[] \| undefined` | Node ids inclusive of both ends; `undefined` when unreachable. |
| `stronglyConnectedComponents()` | `string[][]` | Each component in deterministic order. |
| `topologicalSort()` | `string[] \| undefined` | `undefined` when the graph has a cycle. |
| `hasCycle()` | `boolean` | |
| `pageRank(options?)` | `{ id, score }[]` | `{ damping = 0.85, iterations = 20 }`. Deterministic for a given graph and settings. |
| `toDot()` | `string` | Deterministic Graphviz DOT. |

Traversal methods throw `InvalidArg` for an unknown node id, rather than
silently returning nothing.

---

## Determinism

Component order, visit order, DOT output, and canonical serialization are all
derived from content, never from insertion order or thread scheduling. The same
input produces the same bytes on every platform and every run.

---

## Errors

| `code` | Cause |
| --- | --- |
| `InvalidArg` | Malformed input JSON, duplicate node id, edge endpoint that references no node, unknown node id passed to a traversal, invalid PageRank settings. |

---

## What ships

| | |
| --- | --- |
| Runtimes | Node.js 18+ (Node-API 8), Bun 1.4+ |
| Platforms | Windows x64/arm64, macOS x64/arm64, glibc Linux x64/arm64 |
| Install script | none |
| Network at install | none |
| Runtime dependencies | none |
| Platform packages | none — all six bindings are in this one tarball |

The Node-API 8 ABI keeps the addon independent of any single Node major
version.

---

## Measured

[`benchmark/RESULTS.md`](benchmark/RESULTS.md) is generated from the
[weavatrix-benchmarks](https://github.com/Weavatrix/weavatrix-benchmarks)
harness, which forces both sides to materialize the identical BFS visit order
before either is timed. The competitor is `graphology`, the standard JavaScript
graph library.

Medians of three independent runs over 50,000 nodes and 149,991 edges:

| Contract | Node 24 | Bun 1.3 |
| --- | ---: | ---: |
| Materialized directed BFS node ids | **2.98x** (2.91–3.14) | **2.98x** (2.82–3.20) |

`graphology` stores a plain topology; the Weavatrix graph also retains typed
provenance on every edge, so it is carrying more and still traversing faster.
On a ten-node graph both sides sit in timer noise, and this project makes no
small-graph claim.

---

Graph owns its repository, package, release evidence, and MIT license, and can
be used entirely on its own.

Repository: [Weavatrix/weavatrix-graph](https://github.com/Weavatrix/weavatrix-graph) ·
Rust crate: [crates.io/crates/weavatrix-graph](https://crates.io/crates/weavatrix-graph) ·
License: [MIT](https://github.com/Weavatrix/weavatrix-graph/blob/main/LICENSE)
