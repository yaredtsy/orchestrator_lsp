import { useEffect, useState } from "react";

import type { Placed } from "../canvas/layout.ts";
import { request } from "../host.ts";

type State = { text: string } | { error: string } | null;

/** Lines after the first still carry the definition's own indentation (the range starts at `def`, not at the line start): take it off. */
function dedent(text: string, indent: number): string[] {
  const [first, ...rest] = text.split("\n");
  return [first, ...rest.map((l) => l.slice(Math.min(indent, l.length - l.trimStart().length)))];
}

/** The node's own source, read by the host from VS Code's document (so unsaved edits show). Read-only for now. */
export function NodeCode({ node }: { node: Placed }) {
  const [state, setState] = useState<State>(null);
  useEffect(() => {
    let live = true;
    request<string>("code", { path: node.path, range: node.range }).then(
      (text) => live && setState({ text }),
      (e: Error) => live && setState({ error: e.message }),
    );
    return () => {
      live = false;
    };
  }, [node.id]);

  if (!state) return <div className="node-code muted">loading…</div>;
  if ("error" in state) return <div className="node-code error">{state.error}</div>;
  const first = node.line ?? 1;
  // `nodrag nopan nowheel`: selecting text must not drag the card, and the wheel scrolls the code, not the canvas
  return (
    <pre className="node-code nodrag nopan nowheel">
      {dedent(state.text, node.range?.start.character ?? 0).map((line, i) => (
        <div key={i}>
          <span className="ln">{first + i}</span>
          {line}
        </div>
      ))}
    </pre>
  );
}
