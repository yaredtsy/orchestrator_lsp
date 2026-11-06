//! S4 step 5: six refactor fixtures (+ one deletion) on a small Python repo, one branch each, off a common base.
//! For every documented node it prints which ladder step found it, or whether it was asked / Missing,
//! and compares with the known truth.

use std::path::{Path, PathBuf};
use std::process::Command;

use spike_match::driver::Driver;
use spike_match::git::{CliGit, GitBackend};
use spike_match::ladder::{Config, Main, Outcome, locate, matching};
use spike_match::model::Node;
use spike_match::snapshot::snapshot;

const PRICING: &str = r#""""Pricing rules."""

TAX_RATE = 0.2


def apply_discount(price, rate, minimum=0.0):
    """Apply a percentage discount, never going below minimum."""
    if rate < 0 or rate > 1:
        raise ValueError("rate must be between 0 and 1")
    discounted = price * (1 - rate)
    if discounted < minimum:
        discounted = minimum
    return round(discounted, 2)


def tax_eu(amount, country, rates):
    """VAT for an EU country."""
    if country not in rates:
        raise KeyError("unknown country: " + country)
    rate = rates[country]
    vat = amount * rate
    return round(vat, 2)


def tax_us(amount, state, rates):
    """Sales tax for a US state."""
    if state not in rates:
        raise KeyError("unknown state: " + state)
    rate = rates[state]
    tax = amount * rate
    return round(tax, 2)
"#;

const ORDERS: &str = r#"class OrderService:
    def __init__(self, repo):
        self.repo = repo

    def parse_order(self, raw):
        """Turn a raw dict into an order."""
        items = []
        for line in raw.get("lines", []):
            sku = line["sku"]
            qty = int(line.get("qty", 1))
            if qty <= 0:
                continue
            items.append({"sku": sku, "qty": qty})
        return {"id": raw["id"], "items": items, "note": raw.get("note", "")}

    def total(self, order, prices):
        total = 0.0
        for item in order["items"]:
            total += prices[item["sku"]] * item["qty"]
        return round(total, 2)
"#;

const TEXT: &str = r#"import re


def slugify(text, sep="-"):
    """Lower-case the text and join its words with sep."""
    text = text.strip().lower()
    text = re.sub(r"[^a-z0-9]+", sep, text)
    text = text.strip(sep)
    return text or "untitled"


def truncate(text, width, suffix="..."):
    """Cut text to width characters, adding suffix when cut."""
    if len(text) <= width:
        return text
    return text[: width - len(suffix)] + suffix
"#;

const PAIRS: &str = r#"class Totals:
    def gross(self, items):
        total = 0.0
        for item in items:
            total += item["price"] * item["qty"]
        return round(total, 2)

    def net(self, items):
        total = 0.0
        for item in items:
            total += item["price"] * item["qty"]
        return round(total, 2)
"#;

const MATH: &str = r#"def clamp(value, low, high):
    """Limit value to the range low..high."""
    if low > high:
        raise ValueError("low must not exceed high")
    if value < low:
        return low
    if value > high:
        return high
    return value


def average(values):
    """Mean of a non-empty list."""
    if not values:
        raise ValueError("no values")
    return sum(values) / len(values)
"#;

fn git(dir: &Path, args: &[&str]) {
    let o = Command::new("git").arg("-C").arg(dir).args(args)
        .env("GIT_AUTHOR_NAME", "t").env("GIT_AUTHOR_EMAIL", "t@t").env("GIT_COMMITTER_NAME", "t").env("GIT_COMMITTER_EMAIL", "t@t")
        .output().unwrap();
    assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
}

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn base(dir: &Path) {
    git(dir, &["init", "-q", "-b", "base"]);
    for (p, t) in [("shop/__init__.py", ""), ("shop/pricing.py", PRICING), ("shop/orders.py", ORDERS), ("shop/util/__init__.py", ""), ("shop/util/text.py", TEXT), ("shop/util/math_ops.py", MATH), ("shop/pairs.py", PAIRS)] {
        write(dir, p, t);
    }
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "base"]);
}

fn branch(dir: &Path, name: &str, f: impl FnOnce(&Path)) {
    git(dir, &["checkout", "-q", "-b", name, "base"]);
    f(dir);
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", name]);
}

fn func(path: &str, q: &str) -> Node { Node::Function { path: path.into(), qualname: q.into() } }
fn file(path: &str) -> Node { Node::File { path: path.into() } }
fn folder(path: &str) -> Node { Node::Folder { path: path.into() } }

fn main() -> anyhow::Result<()> {
    let tmp: PathBuf = std::env::temp_dir().join(format!("vnoc-s4-fixtures-{}", std::process::id()));
    std::fs::create_dir_all(&tmp)?;
    base(&tmp);

    // the documented nodes, at the base commit
    let nodes = vec![
        func("shop/pricing.py", "apply_discount"),
        func("shop/pricing.py", "tax_eu"),
        func("shop/orders.py", "OrderService.parse_order"),
        func("shop/util/text.py", "slugify"),
        func("shop/util/math_ops.py", "clamp"),
        file("shop/util/math_ops.py"),
        folder("shop/util"),
        func("shop/pairs.py", "Totals.gross"),
        func("shop/pairs.py", "Totals.net"),
    ];

    // (branch, description, edit, truth per node: None = Missing)
    type Edit = Box<dyn Fn(&Path)>;
    let mut cases: Vec<(&str, &str, Edit, Vec<Option<Node>>)> = vec![
        ("r1", "rename a function", Box::new(|d| {
            let t = std::fs::read_to_string(d.join("shop/pricing.py")).unwrap().replace("def apply_discount", "def apply_rate_discount");
            write(d, "shop/pricing.py", &t);
        }), vec![Some(func("shop/pricing.py", "apply_rate_discount")), Some(nodes[1].clone()), Some(nodes[2].clone()), Some(nodes[3].clone()), Some(nodes[4].clone()), Some(nodes[5].clone()), Some(nodes[6].clone())]),
        ("r2", "move a file", Box::new(|d| git(d, &["mv", "shop/pricing.py", "shop/billing.py"])),
            vec![Some(func("shop/billing.py", "apply_discount")), Some(func("shop/billing.py", "tax_eu")), Some(nodes[2].clone()), Some(nodes[3].clone()), Some(nodes[4].clone()), Some(nodes[5].clone()), Some(nodes[6].clone())]),
        ("r3", "rename a folder", Box::new(|d| git(d, &["mv", "shop/util", "shop/helpers"])),
            vec![Some(nodes[0].clone()), Some(nodes[1].clone()), Some(nodes[2].clone()), Some(func("shop/helpers/text.py", "slugify")), Some(func("shop/helpers/math_ops.py", "clamp")), Some(file("shop/helpers/math_ops.py")), Some(folder("shop/helpers"))]),
        ("r4", "move AND edit a file", Box::new(|d| {
            git(d, &["mv", "shop/orders.py", "shop/sales.py"]);
            let t = std::fs::read_to_string(d.join("shop/sales.py")).unwrap()
                .replace("line.get(\"qty\", 1)", "line.get(\"quantity\", 1)")
                .replace("round(total, 2)", "round(total, 4)")
                .replace("raw.get(\"note\", \"\")", "raw.get(\"note\", \"\").strip()");
            write(d, "shop/sales.py", &t);
        }), vec![Some(nodes[0].clone()), Some(nodes[1].clone()), Some(func("shop/sales.py", "OrderService.parse_order")), Some(nodes[3].clone()), Some(nodes[4].clone()), Some(nodes[5].clone()), Some(nodes[6].clone())]),
        ("r5", "move a function to another file", Box::new(|d| {
            let t = std::fs::read_to_string(d.join("shop/util/text.py")).unwrap();
            let (slug, rest) = t.split_once("def truncate").unwrap();
            let slug = slug.trim_start_matches("import re\n\n\n").trim_end().to_string() + "\n";
            write(d, "shop/util/text.py", &format!("def truncate{rest}"));
            write(d, "shop/util/strings.py", &format!("import re\n\n\n{slug}"));
        }), vec![Some(nodes[0].clone()), Some(nodes[1].clone()), Some(nodes[2].clone()), Some(func("shop/util/strings.py", "slugify")), Some(nodes[4].clone()), Some(nodes[5].clone()), Some(nodes[6].clone())]),
        ("r6", "rename a function AND change its body", Box::new(|d| {
            let t = std::fs::read_to_string(d.join("shop/util/math_ops.py")).unwrap();
            let (_, rest) = t.split_once("def average").unwrap();
            write(d, "shop/util/math_ops.py", &format!("def limit(value, lower, upper, strict=False):\n    \"\"\"Restrict value to lower..upper.\"\"\"\n    if lower > upper:\n        raise ValueError(\"lower must not exceed upper\")\n    if strict and (value < lower or value > upper):\n        raise ValueError(\"value out of range\")\n    return max(lower, min(value, upper))\n\n\ndef average{rest}"));
        }), vec![Some(nodes[0].clone()), Some(nodes[1].clone()), Some(nodes[2].clone()), Some(nodes[3].clone()), Some(func("shop/util/math_ops.py", "limit")), Some(nodes[5].clone()), Some(nodes[6].clone())]),
        ("r7", "delete a function whose sibling looks the same (extra)", Box::new(|d| {
            let t = std::fs::read_to_string(d.join("shop/pricing.py")).unwrap();
            let (head, rest) = t.split_once("def tax_eu").unwrap();
            let (_, us) = rest.split_once("def tax_us").unwrap();
            write(d, "shop/pricing.py", &format!("{head}def tax_us{us}"));
        }), vec![Some(nodes[0].clone()), None, Some(nodes[2].clone()), Some(nodes[3].clone()), Some(nodes[4].clone()), Some(nodes[5].clone()), Some(nodes[6].clone())]),
    ];
    for c in &mut cases {
        c.3.push(Some(nodes[7].clone()));
        c.3.push(Some(nodes[8].clone()));
    }
    cases.push(("r8", "two identical methods, both renamed in one commit", Box::new(|d| {
        let t = std::fs::read_to_string(d.join("shop/pairs.py")).unwrap().replace("def gross(", "def gross_total(").replace("def net(", "def net_total(");
        write(d, "shop/pairs.py", &t);
    }), vec![Some(nodes[0].clone()), Some(nodes[1].clone()), Some(nodes[2].clone()), Some(nodes[3].clone()), Some(nodes[4].clone()), Some(nodes[5].clone()), Some(nodes[6].clone()), Some(func("shop/pairs.py", "Totals.gross_total")), Some(func("shop/pairs.py", "Totals.net_total"))]));
    for (b, _, e, _) in &cases {
        branch(&tmp, b, e);
    }

    let driver_bin = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../s2-python-driver/spike-driver-python/target/release/spike-driver-python");
    let mut driver = Driver::start(&driver_bin)?;
    let g = CliGit::new(&tmp);
    let before_c = g.rev_parse("base")?;
    let before = snapshot(&g, &mut driver, &before_c)?;

    for main in [Main::Edit, Main::Jaccard] {
        let cfg = Config { main, ask_floor: std::env::var("S4_ASK_FLOOR").ok().and_then(|v| v.parse().ok()).unwrap_or(0.5), ..Config::default() };
        println!("\n## ladder, step-5 score = {main:?}, threshold {}, margin {}, min_tokens {}\n", cfg.threshold, cfg.margin, cfg.min_tokens);
        println!("| fixture | node | result | verdict |\n|---|---|---|---|");
        let (mut wrong, mut missed, mut asked, mut ok) = (0, 0, 0, 0);
        for (b, desc, _, truth) in &cases {
            let after_c = g.rev_parse(b)?;
            let after = snapshot(&g, &mut driver, &after_c)?;
            let renames = g.renames(&before_c, &after_c)?;
            let m = matching(&before, &after, &renames);
            for (node, want) in nodes.iter().zip(truth) {
                let out = locate(node, &before, &after, &renames, &m, &cfg);
                let verdict = match (&out, want) {
                    (Outcome::Found { node: got, .. }, Some(w)) if got == w => { ok += 1; "ok" }
                    (Outcome::Found { .. }, _) => { wrong += 1; "**WRONG SILENT MATCH**" }
                    (Outcome::Ask(c), Some(w)) => { asked += 1; if format!("{}::{}", c[0].file, c[0].qualname) == w.to_string() { "asked, top candidate is right" } else { "asked, top candidate is WRONG" } }
                    (Outcome::Ask(_), None) => { asked += 1; "asked (it was deleted)" }
                    (Outcome::Missing, None) => { ok += 1; "ok (Missing)" }
                    (Outcome::Missing, Some(_)) => { missed += 1; "MISSED" }
                };
                // only print the nodes that moved or had trouble; unchanged ones are step 1 and uninteresting
                if !matches!(&out, Outcome::Found { step: 1, .. }) {
                    println!("| {b} {desc} | {node} | {} | {verdict} |", out.short());
                }
            }
        }
        println!("\nok {ok} · asked {asked} · missed {missed} · **wrong silent {wrong}**");
    }
    std::fs::remove_dir_all(&tmp).ok();
    Ok(())
}
