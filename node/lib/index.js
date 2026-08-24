'use strict'

const { NativeGraph } = require('../index.js')

class Graph {
  constructor(input) {
    this._native = new NativeGraph(JSON.stringify(input))
  }

  get nodeCount() {
    return this._native.nodeCount
  }

  get edgeCount() {
    return this._native.edgeCount
  }

  canonical() {
    return JSON.parse(this._native.canonicalJson())
  }

  node(id) {
    const value = this._native.nodeJson(id)
    return value == null ? undefined : JSON.parse(value)
  }

  outgoing(id) {
    return JSON.parse(this._native.outgoingJson(id))
  }

  incoming(id) {
    return JSON.parse(this._native.incomingJson(id))
  }

  bfs(start) {
    return this._native.bfs(start)
  }

  shortestPath(source, target) {
    return this._native.shortestPath(source, target) ?? undefined
  }

  stronglyConnectedComponents() {
    return JSON.parse(this._native.stronglyConnectedComponentsJson())
  }

  topologicalSort() {
    return this._native.topologicalSort() ?? undefined
  }

  hasCycle() {
    return this._native.hasCycle()
  }

  pageRank(options = {}) {
    return JSON.parse(this._native.pageRankJson(options.damping, options.iterations))
  }

  toDot() {
    return this._native.toDot()
  }
}

module.exports = { Graph }
