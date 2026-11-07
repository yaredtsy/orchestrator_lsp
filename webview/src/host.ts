// The bridge to the extension host: the webview cannot reach the core, so it posts a request and waits for the answer.

declare function acquireVsCodeApi(): {
  postMessage(message: unknown): void;
  getState<T>(): T | undefined;
  setState<T>(state: T): void;
};

/** `acquireVsCodeApi` may be called once per webview. */
export const vscode = acquireVsCodeApi();

let nextId = 0;
const pending = new Map<number, { resolve: (v: any) => void; reject: (e: Error) => void }>();

window.addEventListener("message", (e: MessageEvent) => {
  if (e.data?.type !== "res") return;
  const p = pending.get(e.data.id);
  if (!p) return;
  pending.delete(e.data.id);
  if (e.data.error) p.reject(new Error(e.data.error));
  else p.resolve(e.data.result);
});

export function request<T>(method: "structure" | "ping"): Promise<T> {
  const id = ++nextId;
  return new Promise<T>((resolve, reject) => {
    pending.set(id, { resolve, reject });
    vscode.postMessage({ type: "req", id, method });
  });
}
