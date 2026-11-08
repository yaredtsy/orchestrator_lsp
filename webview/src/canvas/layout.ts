// A simple, deterministic tree layout: one row per visible node, depth to the right. Pure, so it is tested without a browser.

import type { Kind, TreeNode } from "../types.ts";

export const COL_W = 300;
/** One card (80 px, see node-card.css) plus a gap. Fixed until the code toggle makes cards grow and the layout has to measure them. */
export const ROW_H = 96;

// a `type`, not an interface: React Flow wants node data to be assignable to Record<string, unknown>
export type Placed = {
  id: string;
  kind: Kind;
  name: string;
  path: string;
  /** 1-based line where the definition starts; null for folders and files */
  line: number | null;
  depth: number;
  x: number;
  y: number;
  /** how many nodes are directly under it (shown on a collapsed node) */
  childCount: number;
  collapsed: boolean;
  syntaxError: boolean;
};

export interface Edge {
  id: string;
  source: string;
  target: string;
}

export interface Layout {
  nodes: Placed[];
  edges: Edge[];
  /** nodes in the tree that are not drawn because an ancestor is collapsed */
  hidden: number;
}

/** What is collapsed when the canvas first opens: every file (so classes and functions are one click away) and every folder from depth 2 down. */
export function defaultCollapsed(root: TreeNode): string[] {
  const out: string[] = [];
  const walk = (n: TreeNode, depth: number) => {
    if (n.children.length > 0 && (n.kind === "file" || (n.kind === "folder" && depth >= 2))) out.push(n.id);
    n.children.forEach((c) => walk(c, depth + 1));
  };
  walk(root, 0);
  return out;
}

function size(n: TreeNode): number {
  return 1 + n.children.reduce((sum, c) => sum + size(c), 0);
}

export function layout(root: TreeNode, collapsed: ReadonlySet<string>): Layout {
  const nodes: Placed[] = [];
  const edges: Edge[] = [];
  const walk = (n: TreeNode, depth: number) => {
    const isCollapsed = n.children.length > 0 && collapsed.has(n.id);
    nodes.push({ id: n.id, kind: n.kind, name: n.name, path: n.path, line: n.range ? n.range.start.line + 1 : null, depth, x: depth * COL_W, y: nodes.length * ROW_H, childCount: n.children.length, collapsed: isCollapsed, syntaxError: n.syntax_error });
    if (isCollapsed) return;
    for (const c of n.children) {
      edges.push({ id: `${n.id}->${c.id}`, source: n.id, target: c.id });
      walk(c, depth + 1);
    }
  };
  walk(root, 0);
  return { nodes, edges, hidden: size(root) - nodes.length };
}
