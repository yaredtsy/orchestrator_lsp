#!/usr/bin/env python3
"""S2 test harness. Stands in for the core: spawns the driver, speaks the wire protocol,
and enforces the stdout guard (C2): every byte on the driver's stdout must be inside a
`Content-Length` frame, otherwise the run FAILS.

usage: drive.py <driver-binary> <project-root> <file.py> <entry-function> [noisy]
"""
import json, subprocess, sys, threading, time

binary, root, file, entry = sys.argv[1:5]
noisy = len(sys.argv) > 5

p = subprocess.Popen([binary] + (["noisy"] if noisy else []),
                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
inbox, bad = {}, []
cv = threading.Condition()


def reader():
    f = p.stdout
    while True:
        head = f.readline()
        if not head:
            return  # EOF
        if not head.startswith(b"Content-Length:"):
            bad.append(head[:80])  # THE GUARD: anything that is not a frame header
            continue
        n = int(head.split(b":")[1])
        blank = f.readline()
        if blank != b"\r\n":
            bad.append(b"missing blank line after header")
        msg = json.loads(f.read(n))
        with cv:
            inbox[msg.get("id")] = msg
            cv.notify_all()


threading.Thread(target=reader, daemon=True).start()
_next = [0]


def send(obj):
    body = json.dumps(obj).encode()
    p.stdin.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
    p.stdin.flush()


def request(method, params=None):
    _next[0] += 1
    send({"jsonrpc": "2.0", "id": _next[0], "method": method, "params": params})
    return _next[0]


def wait(i, timeout=60):
    t = time.time()
    with cv:
        while i not in inbox:
            if not cv.wait(timeout=max(0.01, timeout - (time.time() - t))) and time.time() - t > timeout:
                raise TimeoutError(i)
    return inbox[i]


def check(name, ok, detail=""):
    print(("PASS " if ok else "FAIL ") + name + (f"  [{detail}]" if detail else ""))
    return ok


results = []
try:
    t0 = time.time()
    r = wait(request("initialize", {"protocol": 1}))
    results.append(check("handshake", r.get("result", {}).get("name") == "python", json.dumps(r.get("result"))))
    send({"jsonrpc": "2.0", "method": "initialized", "params": {}})
    send({"jsonrpc": "2.0", "method": "$/madeUpNotification", "params": {}})  # must not kill the driver

    t = time.time()
    r = wait(request("vnoc/outline", {"root": root, "file": file}))
    n = len(r["result"]["nodes"]) if "result" in r else -1
    results.append(check("outline", n > 0, f"{n} top-level nodes, {time.time()-t:.2f}s (includes opening the project)"))

    for depth in (1, 3):
        t = time.time()
        r = wait(request("vnoc/callTree", {"root": root, "file": file, "entry": entry, "depth": depth}))
        results.append(check(f"callTree depth={depth}", "result" in r, f"{time.time()-t:.2f}s, {len(json.dumps(r))} bytes"))

    # cancel: ask for an unbounded tree, cancel at once. Either answer is legal; we record which.
    i = request("vnoc/callTree", {"root": root, "file": file, "entry": entry})
    send({"jsonrpc": "2.0", "method": "$/cancelRequest", "params": {"id": i}})
    t = time.time()
    r = wait(i)
    code = r.get("error", {}).get("code")
    results.append(check("cancel answered", True, f"code={code} after {time.time()-t:.2f}s (-32800 = cancelled)"))

    r = wait(request("vnoc/nope", {"root": root, "file": file}))
    results.append(check("unknown method -> error, driver alive", "error" in r))

    wait(request("shutdown"))
    send({"jsonrpc": "2.0", "method": "exit"})
    try:
        p.wait(timeout=5)
        results.append(check("clean exit after shutdown+exit", p.returncode == 0, f"rc={p.returncode}"))
    except subprocess.TimeoutExpired:
        p.kill()
        results.append(check("clean exit after shutdown+exit", False, "still running after 5s"))
except Exception as e:  # noqa: BLE001
    results.append(check("run", False, repr(e)))
    p.kill()

results.append(check("STDOUT GUARD (C2): only framed messages on stdout", not bad, f"stray: {bad[:3]}" if bad else "clean"))
sys.exit(0 if all(results) else 1)
