# Node.js and Bun benchmark snapshot

Measured on 2026-08-24 on Windows x64. Values are medians after two warm-up rounds; execution order alternates per round. The benchmark asserts equal materialized directed-BFS node IDs before reporting timings.

| Runtime | Nodes / edges | Weavatrix | Graphology 0.26.0 | Result |
| --- | ---: | ---: | ---: | ---: |
| Node 24.15.0 | 50,000 / 149,991 | 12.813 ms | 37.267 ms | Weavatrix 2.91x faster |
| Bun 1.4.0 | 50,000 / 149,991 | 9.105 ms | 24.001 ms | Weavatrix 2.64x faster |

A 10-node / 21-edge run completed in roughly 0.004-0.006 ms per side on Node, below a useful timer-noise boundary. Do not use that row to claim a small-graph winner: native-call overhead and timer resolution dominate it.

Reproduce from `node/`:

```console
npm ci
npm run build
npm run bench
bun run benchmark/graphology.mjs
```

These are scoped measurements of one equal-output traversal contract, not a universal claim about all graph workloads. Rust-crate algorithm, memory, and losing-row benchmarks remain in the repository's main benchmark section.
