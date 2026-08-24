# Node.js and Bun benchmark snapshot

This file is generated. Every number below was produced by the
[weavatrix-benchmarks](https://github.com/Weavatrix/weavatrix-benchmarks)
harness and copied out of its recorded run; none of it is typed by hand.
That repository states the rules every suite obeys, including what each
row had to prove equal before it was allowed to be timed.

**Question.** How fast is a directed traversal over a code graph that carries provenance on every edge?

**Competitor.** `graphology`

| Property | Value |
| --- | --- |
| Measured | 2026-08-24 |
| Platform | win32 x64, 10.0.26200 |
| CPU | Intel(R) Core(TM) Ultra 7 255U (14 logical cores) |
| Memory | 47.5 GiB |
| Rounds | 7 measured, after 2 warm-ups, alternating order, median reported |
| Independent runs | 3 per suite, each in a fresh process; the table shows the median and the spread |
| Package | weavatrix-graph 0.6.4 |

## node 24.15.0

Corpus: `[{"nodes":50000,"edges":149991}]`

| Contract | Parity | Weavatrix | Competitor | Result |
| --- | --- | ---: | ---: | ---: |
| materialized directed BFS node ids | identical visit order | 7.464 ms | 22.072 ms | Weavatrix 2.98x faster (2.91x–3.14x) |

## bun 1.3.14

Corpus: `[{"nodes":50000,"edges":149991}]`

| Contract | Parity | Weavatrix | Competitor | Result |
| --- | --- | ---: | ---: | ---: |
| materialized directed BFS node ids | identical visit order | 4.907 ms | 14.602 ms | Weavatrix 2.98x faster (2.82x–3.20x) |

## Reading these rows

- **materialized directed BFS node ids** — graphology stores a plain topology; the Weavatrix graph also retains typed provenance per edge

## Reproduce

```console
git clone https://github.com/Weavatrix/weavatrix-benchmarks
cd weavatrix-benchmarks && npm ci
node run.mjs --suite=graph
bun run.mjs --suite=graph
node export.mjs
```

CPU, memory bandwidth, filesystem, antivirus, and JavaScript engine
version all move these timings. Treat them as a reproducible snapshot of
the environment above, not as a universal result.
