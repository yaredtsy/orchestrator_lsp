import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { ReactFlow, type Node } from "@xyflow/react";
import "@xyflow/react/dist/style.css";

// acquireVsCodeApi() exists only inside a VS Code webview, and may be called once.
declare function acquireVsCodeApi(): {
  postMessage(m: unknown): void;
  getState<T>(): T | undefined;
  setState<T>(s: T): void;
};
const vscode = acquireVsCodeApi();

function App() {
  // Restore the last answer if VS Code re-created this panel (Step 5).
  const saved = vscode.getState<{ label: string }>();
  const [label, setLabel] = useState(saved?.label ?? "waiting for core…");

  useEffect(() => {
    const on = (e: MessageEvent) => {
      if (e.data.type !== "res") return;
      const l = `pong from pid ${e.data.result.pid} (${e.data.ms} ms since activation)`;
      setLabel(l);
      vscode.setState({ label: l });
    };
    window.addEventListener("message", on);
    vscode.postMessage({ type: "req", id: 1, method: "vnoc/ping" });
    return () => window.removeEventListener("message", on);
  }, []);

  const nodes: Node[] = [{ id: "1", position: { x: 80, y: 80 }, data: { label } }];
  return (
    <div style={{ width: "100vw", height: "100vh" }}>
      <ReactFlow nodes={nodes} edges={[]} colorMode="dark" fitView />
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
