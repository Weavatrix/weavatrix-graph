# weavatrix-graph for Node.js and Bun

Native Node-API bindings to the current Rust `weavatrix-graph` core. This is
not a JavaScript rewrite and not an MCP server: the npm API runs the same
deterministic graph validation, evidence model, and algorithms as the crate.

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

console.log(graph.shortestPath('api', 'db'))
```

The same npm package is designed for Node.js 18+ and Bun 1.4+. Platform
binaries are published for Windows, macOS, and glibc Linux on x64 and arm64.
The benchmark compares equal materialized BFS output against Graphology and
reports Node and Bun separately; results are published only with the runtime,
input size, parity contract, and losing rows visible.
