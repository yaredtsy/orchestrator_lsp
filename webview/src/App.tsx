import { Background, Controls, Handle, MiniMap, type Node, type NodeProps, Position, ReactFlow } from "@xyflow/react";
import { useCallback, useEffect, useMemo, useState } from "react";

import { request, vscode } from "./host.ts";
import { type Placed, defaultCollapsed, layout } from "./layout.ts";
import type { StructureResult } from "./types.ts";

const ICON: Record<string, string> = { folder: "▸", file: "▤", class: "◆", function: "ƒ" };

function Card({ data }: NodeProps<Node<Placed>>) {
  const label = data.kind === "folder" ? `${data.name}/` : data.kind === "function" ? `${data.name}()` : data.name;
  return (
    <div className={`card ${data.kind}${data.syntaxError ? " broken" : ""}`} title={data.path}>
      <Handle type="target" position={Position.Left} isConnectable={false} />
      <span className="icon">{ICON[data.kind]}</span>
      <span className="label">{label}</span>
      {data.childCount > 0 && <span className="badge">{data.collapsed ? `+${data.childCount}` : "−"}</span>}
      <Handle type="source" position={Position.Right} isConnectable={false} />
    </div>
  );
}

const nodeTypes = { card: Card };

type Load = { status: "loading" } | { status: "error"; message: string } | { status: "ready"; data: StructureResult };

export function App() {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [reloads, setReloads] = useState(0);
  const saved = vscode.getState<{ collapsed: string[] }>();
  const [collapsed, setCollapsed] = useState<Set<string> | null>(saved ? new Set(saved.collapsed) : null);

  const refresh = useCallback(() => {
    setLoad({ status: "loading" });
    request<StructureResult>("structure").then(
      (data) => {
        setCollapsed((c) => c ?? new Set(defaultCollapsed(data.root)));
        setLoad({ status: "ready", data });
        setReloads((n) => n + 1);
      },
      (e: Error) => setLoad({ status: "error", message: e.message }),
    );
  }, []);
  useEffect(refresh, [refresh]);

  const toggle = useCallback((id: string) => {
    setCollapsed((c) => {
      const next = new Set(c ?? []);
      if (!next.delete(id)) next.add(id);
      vscode.setState({ collapsed: [...next] }); // survives closing and reopening the panel
      return next;
    });
  }, []);

  const laid = useMemo(() => (load.status === "ready" && collapsed ? layout(load.data.root, collapsed) : null), [load, collapsed]);
  const nodes = useMemo(() => (laid?.nodes ?? []).map((p) => ({ id: p.id, type: "card", position: { x: p.x, y: p.y }, data: p, draggable: false })), [laid]);
  const edges = useMemo(() => laid?.edges ?? [], [laid]);

  // Tell the host what was drawn: the extension's test suite uses it to prove the whole chain ran (bundle, request, core, driver, layout).
  useEffect(() => {
    if (laid) vscode.postMessage({ type: "rendered", visible: laid.nodes.length, hidden: laid.hidden });
  }, [laid]);

  return (
    <div className="app">
      <header className="bar">
        <strong>V-NOC</strong>
        {load.status === "loading" && <span>reading the code…</span>}
        {load.status === "error" && <span className="error">{load.message}</span>}
        {load.status === "ready" && (
          <span>
            {load.data.root.name}: {load.data.stats.folders} folders · {load.data.stats.files} files · {load.data.stats.classes} classes · {load.data.stats.functions} functions · {load.data.stats.ms} ms
            {laid && laid.hidden > 0 ? ` · ${laid.hidden} hidden (click a node to expand)` : ""}
            {load.data.truncated ? " · TRUNCATED: too many files" : ""}
            {load.data.failed.length > 0 ? ` · ${load.data.failed.length} files could not be read` : ""}
          </span>
        )}
        <button onClick={refresh}>Refresh</button>
      </header>
      <ReactFlow
        key={reloads}
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodeClick={(_, node) => (node.data as unknown as Placed).childCount > 0 && toggle(node.id)}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable={false}
        onlyRenderVisibleElements
        fitView
        minZoom={0.02}
        colorMode={document.body.classList.contains("vscode-light") ? "light" : "dark"}
        proOptions={{ hideAttribution: true }}
      >
        <Background />
        <Controls showInteractive={false} />
        <MiniMap pannable zoomable />
      </ReactFlow>
    </div>
  );
}
