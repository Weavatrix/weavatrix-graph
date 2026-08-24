export type AttributeValue = null | boolean | number | string | AttributeValue[] | { [key: string]: AttributeValue }

export interface SourcePosition { line: number; column: number }
export interface SourceSpan { file: string; start: SourcePosition; end: SourcePosition }

export interface GraphNode {
  id: string
  label: string
  kind: string
  language?: string
  span?: SourceSpan
  attributes?: Record<string, AttributeValue>
}

export interface Provenance {
  extractor: string
  evidence: string
  confidence: 'exact' | 'high' | 'medium' | 'low'
  span?: SourceSpan
  detail?: string
}

export interface GraphEdge {
  source: string
  target: string
  kind: string
  provenance: Provenance
  attributes?: Record<string, AttributeValue>
}

export interface GraphInput { nodes: GraphNode[]; edges: GraphEdge[] }
export interface RankEntry { id: string; score: number }

export declare class Graph {
  constructor(input: GraphInput)
  readonly nodeCount: number
  readonly edgeCount: number
  canonical(): GraphInput
  node(id: string): GraphNode | undefined
  outgoing(id: string): GraphEdge[]
  incoming(id: string): GraphEdge[]
  bfs(start: string): string[]
  shortestPath(source: string, target: string): string[] | undefined
  stronglyConnectedComponents(): string[][]
  topologicalSort(): string[] | undefined
  hasCycle(): boolean
  pageRank(options?: { damping?: number; iterations?: number }): RankEntry[]
  toDot(): string
}
