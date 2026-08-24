import { performance } from 'node:perf_hooks'
import { createRequire } from 'node:module'
import Graphology from 'graphology'

const require = createRequire(import.meta.url)
const { Graph } = require('../lib/index.js')

const size = Number(process.env.WEAVATRIX_BENCH_NODES ?? 50_000)
const rounds = Number(process.env.WEAVATRIX_BENCH_ROUNDS ?? 9)

const nodes = Array.from({ length: size }, (_, index) => ({
  id: `n${index}`,
  label: `n${index}`,
  kind: 'function',
}))
const edges = []
for (let index = 0; index < size - 3; index += 1) {
  for (const offset of [1, 2, 3]) {
    edges.push({
      source: `n${index}`,
      target: `n${index + offset}`,
      kind: 'calls',
      provenance: { extractor: 'bench', evidence: 'parsed', confidence: 'exact' },
    })
  }
}

const ours = new Graph({ nodes, edges })
const competitor = new Graphology({ type: 'directed', multi: false, allowSelfLoops: true })
for (const node of nodes) competitor.addNode(node.id)
for (const edge of edges) competitor.mergeEdge(edge.source, edge.target)

function graphologyBfs(start) {
  const queue = [start]
  const visited = new Set(queue)
  for (let cursor = 0; cursor < queue.length; cursor += 1) {
    for (const neighbor of competitor.outNeighbors(queue[cursor])) {
      if (!visited.has(neighbor)) {
        visited.add(neighbor)
        queue.push(neighbor)
      }
    }
  }
  return queue
}

function median(samples) {
  const sorted = [...samples].sort((left, right) => left - right)
  return sorted[Math.floor(sorted.length / 2)]
}

function runOnce(run) {
  const started = performance.now()
  const result = run()
  const elapsed = performance.now() - started
  if (result.length !== size) throw new Error(`parity failure: ${result.length} != ${size}`)
  return elapsed
}

function measurePair(left, right) {
  const leftSamples = []
  const rightSamples = []
  for (let round = 0; round < rounds + 2; round += 1) {
    let leftElapsed
    let rightElapsed
    if (round % 2 === 0) {
      leftElapsed = runOnce(left)
      rightElapsed = runOnce(right)
    } else {
      rightElapsed = runOnce(right)
      leftElapsed = runOnce(left)
    }
    if (round >= 2) {
      leftSamples.push(leftElapsed)
      rightSamples.push(rightElapsed)
    }
  }
  return [median(leftSamples), median(rightSamples)]
}

const [nativeMs, graphologyMs] = measurePair(
  () => ours.bfs('n0'),
  () => graphologyBfs('n0'),
)

console.log(JSON.stringify({
  contract: 'materialized directed BFS node ids',
  nodes: size,
  edges: edges.length,
  rounds,
  runtime: process.versions.bun ? `bun ${process.versions.bun}` : `node ${process.version}`,
  weavatrixMs: nativeMs,
  graphologyMs,
  ratio: graphologyMs / nativeMs,
}, null, 2))
