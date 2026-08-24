'use strict'

const assert = require('node:assert/strict')
const test = require('node:test')
const { Graph } = require('../lib/index.js')

function fixture() {
  const node = (id) => ({ id, label: id, kind: 'function' })
  const edge = (source, target) => ({
    source,
    target,
    kind: 'calls',
    provenance: { extractor: 'test', evidence: 'parsed', confidence: 'exact' },
  })
  return {
    nodes: ['a', 'b', 'c', 'd'].map(node),
    edges: [edge('a', 'b'), edge('b', 'c'), edge('a', 'd')],
  }
}

test('canonical graph algorithms keep stable string identities', () => {
  const graph = new Graph(fixture())
  assert.equal(graph.nodeCount, 4)
  assert.equal(graph.edgeCount, 3)
  assert.deepEqual(graph.bfs('a'), ['a', 'b', 'd', 'c'])
  assert.deepEqual(graph.shortestPath('a', 'c'), ['a', 'b', 'c'])
  assert.deepEqual(graph.topologicalSort(), ['a', 'b', 'd', 'c'])
  assert.equal(graph.hasCycle(), false)
  assert.equal(graph.node('b').id, 'b')
  assert.equal(graph.outgoing('a').length, 2)
})

test('rejects dangling graph edges at the native boundary', () => {
  const input = fixture()
  input.edges.push({
    source: 'a',
    target: 'missing',
    kind: 'calls',
    provenance: { extractor: 'test', evidence: 'parsed', confidence: 'exact' },
  })
  assert.throws(() => new Graph(input), /missing/)
})
