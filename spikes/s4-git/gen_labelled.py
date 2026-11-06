#!/usr/bin/env python3
"""S4 / VS-O10: build a labelled set from REAL functions of an open-source repo (committed HEAD content only).

For each sampled function we write mutated versions at several edit levels: the function is renamed and a fraction of its
tokens are altered (identifiers, numbers, strings). The Rust side decides where each version lands (same scope / another
file / module level) and what the "new" candidates around it are. Truth is known: the mutated version IS the successor.

usage: gen_labelled.py <repo> <out.json> [n=120] [seed=1]
"""
import ast, collections, io, json, keyword, os, random, subprocess, sys, textwrap, tokenize, traceback

repo, out = sys.argv[1], sys.argv[2]
N = int(sys.argv[3]) if len(sys.argv) > 3 else 120
rnd = random.Random(int(sys.argv[4]) if len(sys.argv) > 4 else 1)
LEVELS = [0.0, 0.05, 0.10, 0.20, 0.35, 0.50, 0.70]
SKIP = ("tokenize.NL", "tokenize.COMMENT", "tokenize.NEWLINE", "tokenize.INDENT", "tokenize.DEDENT", "tokenize.ENDMARKER")


class Drv:
    """Talks to the S2 driver (ruff's parser, the one the ladder uses). `syntax_errors(text)` == 0 means it parses."""

    def __init__(self):
        here = os.path.dirname(os.path.abspath(__file__))
        bin_ = os.environ.get("VNOC_DRIVER") or os.path.join(here, "../s2-python-driver/spike-driver-python/target/release/spike-driver-python")
        self.p = subprocess.Popen([bin_], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        self.n = 0
        self.call("initialize", {"protocol": 1})
        self._send({"jsonrpc": "2.0", "method": "initialized", "params": {}})

    def _send(self, o):
        b = json.dumps(o).encode()
        self.p.stdin.write(b"Content-Length: %d\r\n\r\n" % len(b) + b)
        self.p.stdin.flush()

    def call(self, method, params):
        self.n += 1
        self._send({"jsonrpc": "2.0", "id": self.n, "method": method, "params": params})
        n = int(self.p.stdout.readline().split(b":")[1])
        self.p.stdout.readline()
        return json.loads(self.p.stdout.read(n)).get("result")

    def syntax_errors(self, text):
        return self.call("vnoc/defs", {"text": text})["syntax_errors"]

    def close(self):
        self.call("shutdown", None)
        self._send({"jsonrpc": "2.0", "method": "exit"})



def git(*a):
    return subprocess.run(["git", "-C", repo, *a], capture_output=True, text=True, check=True).stdout


def trivia(t):
    return t.type in (tokenize.NL, tokenize.COMMENT, tokenize.NEWLINE, tokenize.INDENT, tokenize.DEDENT, tokenize.ENDMARKER)


def functions(path, src):
    try:
        tree = ast.parse(src)
    except SyntaxError:
        return
    lines = src.split("\n")

    def walk(node, prefix, parent_kind):
        for ch in ast.iter_child_nodes(node):
            if isinstance(ch, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                q = f"{prefix}.{ch.name}" if prefix else ch.name
                if not isinstance(ch, ast.ClassDef):
                    seg = textwrap.dedent("\n".join(lines[ch.lineno - 1: ch.end_lineno]))
                    yield {"file": path, "qualname": q, "name": ch.name, "parent": prefix,
                           "parent_kind": parent_kind, "source": seg}
                yield from walk(ch, q, "class" if isinstance(ch, ast.ClassDef) else "function")
            else:
                yield from walk(ch, prefix, parent_kind)

    yield from walk(tree, "", None)


def ntokens(src):
    try:
        return sum(1 for t in tokenize.generate_tokens(io.StringIO(src).readline) if not trivia(t))
    except (tokenize.TokenError, IndentationError):
        return 0


def mutate(src, old_name, new_name, level):
    toks = [t for t in tokenize.generate_tokens(io.StringIO(src).readline)]
    starts = [0]
    for l in src.split("\n"):
        starts.append(starts[-1] + len(l) + 1)
    off = lambda rc: starts[rc[0] - 1] + rc[1]
    edits = []  # (start, end, replacement)
    seen_def = False
    eligible = []
    for i, t in enumerate(toks):
        if t is None:
            continue
        if t.type == tokenize.NAME and t.string == "def" and not seen_def:
            nxt = toks[i + 1]
            edits.append((off(nxt.start), off(nxt.end), new_name))
            seen_def = True
            toks[i + 1] = None  # never mutated again
            continue
        if (t.type == tokenize.NAME and not keyword.iskeyword(t.string)) or t.type in (tokenize.NUMBER, tokenize.STRING):
            eligible.append(i)
    body_tokens = sum(1 for t in toks if t is not None and not trivia(t))
    k = min(len(eligible), round(level * body_tokens))
    for i in rnd.sample(eligible, k):
        t = toks[i]
        a, b = off(t.start), off(t.end)
        s = t.string
        if t.type == tokenize.NAME:
            r = s + "_x"
        elif t.type == tokenize.NUMBER:
            r = "7" + s if s.isdigit() else s
        else:
            r = s[:-3] + "x" + s[-3:] if s[-3:] in ('"""', "'''") else s[:-1] + "x" + s[-1]
        edits.append((a, b, r))
    # apply back to front; overlapping edits (a string inside a def line) are dropped
    edits.sort(key=lambda e: -e[0])
    out, last = src, len(src) + 1
    for a, b, r in edits:
        if b <= last:
            out = out[:a] + r + out[b:]
            last = a
    return out


def rename(name):
    opts = [name + "_v2", "do_" + name.lstrip("_"), "new_" + name.lstrip("_"), name.lstrip("_") + "_impl"]
    return rnd.choice(opts)


files = [f for f in git("ls-tree", "-r", "--name-only", "HEAD").split("\n") if f.endswith(".py")]
pool = []
for f in files:
    if "test" in f.lower() or "/docs/" in f or "examples" in f.lower():
        continue
    src = git("show", f"HEAD:{f}")
    if len(src) > 300_000:
        continue
    for fn in functions(f, src):
        n = ntokens(fn["source"])
        if 25 <= n <= 400:
            fn["tokens"] = n
            pool.append(fn)
rnd.shuffle(pool)
sample = pool[:N]
cases = []
drv = Drv()
fails = collections.Counter()   # "ExceptionType at file:line in function" -> count
expected = len(sample) * len(LEVELS)
for fn in sample:
    new_name = rename(fn["name"])
    for level in LEVELS:
        try:
            new_src = mutate(fn["source"], fn["name"], new_name, level)
            ast.parse(new_src)
        except Exception as e:
            fr = traceback.extract_tb(e.__traceback__)[-1]
            fails[f"{type(e).__name__} at {os.path.basename(fr.filename)}:{fr.lineno} in {fr.name}"] += 1
            continue
        if drv.syntax_errors(new_src) != 0:
            fails["variant does not parse with the ladder's parser (ruff)"] += 1
            continue
        cases.append({"file": fn["file"], "qualname": fn["qualname"], "parent": fn["parent"], "parent_kind": fn["parent_kind"],
                      "new_name": new_name, "level": level, "new_source": new_src, "orig_tokens": fn["tokens"]})
drv.close()
failed = sum(fails.values())
assert len(cases) + failed == expected, f"accounting is off: {len(cases)} produced + {failed} failed != {expected} expected"
print(f"{repo}: {len(sample)} functions x {len(LEVELS)} levels = {expected} expected; {len(cases)} produced, {failed} discarded")
for k, v in fails.most_common():
    print(f"  {v:5d}  {k}")
json.dump({"repo": repo, "cases": cases, "functions": len(sample), "eligible": len(pool), "expected": expected, "discarded": failed}, open(out, "w"))
if not cases:
    sys.exit("FAIL: zero variants produced")
if failed / expected > 0.05:
    sys.exit(f"FAIL: {failed}/{expected} = {failed / expected:.1%} of attempts failed (limit 5%)")
