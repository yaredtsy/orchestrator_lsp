import { useCallback, useEffect, useMemo, useState } from "react";

import { request, vscode } from "./host.ts";
import { Canvas } from "./canvas/Canvas.tsx";
import { defaultCollapsed, layout } from "./canvas/layout.ts";
import type { StructureResult } from "./types.ts";

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
      {laid && <Canvas key={reloads} laid={laid} onToggle={toggle} />}
    </div>
  );
}
