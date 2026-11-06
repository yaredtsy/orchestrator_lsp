//! The small Python repo the fixtures run on, and the git plumbing to build branches of it.

use std::path::Path;
use std::process::Command;

pub const PRICING: &str = r#""""Pricing rules."""

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

pub const ORDERS: &str = r#"class OrderService:
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

pub const TEXT: &str = r#"import re


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

/// Two methods with identical bodies: the twin fixture.
pub const PAIRS: &str = r#"class Totals:
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

pub const MATH: &str = r#"def clamp(value, low, high):
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

pub fn git(dir: &Path, args: &[&str]) {
    let o = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .unwrap();
    assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
}

pub fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

pub fn read(dir: &Path, rel: &str) -> String {
    std::fs::read_to_string(dir.join(rel)).unwrap()
}

/// The `base` branch with every file above committed.
pub fn base(dir: &Path) {
    git(dir, &["init", "-q", "-b", "base"]);
    for (p, t) in [
        ("shop/__init__.py", ""),
        ("shop/pricing.py", PRICING),
        ("shop/orders.py", ORDERS),
        ("shop/util/__init__.py", ""),
        ("shop/util/text.py", TEXT),
        ("shop/util/math_ops.py", MATH),
        ("shop/pairs.py", PAIRS),
    ] {
        write(dir, p, t);
    }
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "base"]);
}

/// One branch off `base`: apply `edit`, commit.
pub fn branch(dir: &Path, name: &str, edit: &dyn Fn(&Path)) {
    git(dir, &["checkout", "-q", "-b", name, "base"]);
    edit(dir);
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", name]);
}
