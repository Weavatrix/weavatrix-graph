# weavatrix-graph

The evidence graph behind Weavatrix, available as an independent native product for Node.js and Bun.

It combines deterministic topology with stable identities, typed relations, provenance, confidence, source spans, canonical serialization, and production graph algorithms. This is the Rust `weavatrix-graph` core through Node-API—not a JavaScript rewrite and not an MCP server.

## Install

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
    provenance: {
      extractor: 'architecture',
      evidence: 'parsed',
      confidence: 'exact',
    },
  }],
})

console.log(graph.shortestPath('api', 'db'))
console.log(graph.stronglyConnectedComponents())
console.log(graph.pageRank())
```

The first native surface includes validated canonical graphs, incoming and outgoing evidence, BFS, shortest paths, strongly connected components, cycle detection, topological sort, PageRank, and deterministic DOT. The Rust product also owns the broader algorithm suite used by repository intelligence and architecture tooling.

## Measured Node and Bun performance

Equal-output contract: materialize the same directed-BFS node IDs on 50,000 nodes and 149,991 edges. Windows x64 medians after two warmups, with execution order alternated per round:

| Runtime | Weavatrix | Graphology 0.26.0 | Result |
| --- | ---: | ---: | ---: |
| Node 24.15.0 | 12.813 ms | 37.267 ms | 2.91x faster |
| Bun 1.4.0 | 9.105 ms | 24.001 ms | 2.64x faster |

At ten nodes both sides fall into roughly 0.004-0.006 ms timer noise, so the project does not claim a small-graph winner. See the [full report, parity rule, caveats, and reproduction commands](https://github.com/Weavatrix/weavatrix-graph/blob/main/node/benchmark/RESULTS.md).

## Runtime and ownership boundary

One self-contained npm package supports Node.js 18+ and Bun 1.4+ and includes the Windows, macOS, and glibc Linux binaries for x64 and arm64. No public platform-package names are created. The Node-API 8 ABI keeps the addon independent of a single Node major version.

Graph owns its repository, package, release evidence, and MIT license. It can be used independently of every other Weavatrix product.

Repository: [Weavatrix/weavatrix-graph](https://github.com/Weavatrix/weavatrix-graph) · Rust crate: [crates.io/crates/weavatrix-graph](https://crates.io/crates/weavatrix-graph) · License: [MIT](https://github.com/Weavatrix/weavatrix-graph/blob/main/LICENSE)
