//! The documented nodes and the refactors. A case lists only what MOVED; every other node is expected where it was.
//! To add a fixture: add a branch edit in `cases()` and list the nodes whose truth changed.

use std::path::Path;

use spike_match::model::Node;

use super::repo::{git, read, write};

pub fn func(path: &str, q: &str) -> Node {
    Node::Function { path: path.into(), qualname: q.into() }
}
fn file(path: &str) -> Node {
    Node::File { path: path.into() }
}
fn folder(path: &str) -> Node {
    Node::Folder { path: path.into() }
}

/// Indexes into `nodes()`.
pub mod n {
    pub const APPLY_DISCOUNT: usize = 0;
    pub const TAX_EU: usize = 1;
    pub const PARSE_ORDER: usize = 2;
    pub const SLUGIFY: usize = 3;
    pub const CLAMP: usize = 4;
    pub const MATH_FILE: usize = 5;
    pub const UTIL_FOLDER: usize = 6;
    pub const GROSS: usize = 7;
    pub const NET: usize = 8;
}

/// The documented nodes, as they are at the `base` commit.
pub fn nodes() -> Vec<Node> {
    vec![
        func("shop/pricing.py", "apply_discount"),
        func("shop/pricing.py", "tax_eu"),
        func("shop/orders.py", "OrderService.parse_order"),
        func("shop/util/text.py", "slugify"),
        func("shop/util/math_ops.py", "clamp"),
        file("shop/util/math_ops.py"),
        folder("shop/util"),
        func("shop/pairs.py", "Totals.gross"),
        func("shop/pairs.py", "Totals.net"),
    ]
}

pub struct Case {
    pub branch: &'static str,
    pub desc: &'static str,
    pub edit: Box<dyn Fn(&Path)>,
    /// (node index, where it should be now; None = it is gone and must stay Missing)
    pub moved: Vec<(usize, Option<Node>)>,
}

impl Case {
    /// The expected position of every node: unchanged unless `moved` says otherwise.
    pub fn truth(&self, nodes: &[Node]) -> Vec<Option<Node>> {
        let mut t: Vec<Option<Node>> = nodes.iter().cloned().map(Some).collect();
        for (i, to) in &self.moved {
            t[*i] = to.clone();
        }
        t
    }
}

pub fn cases() -> Vec<Case> {
    use n::*;
    vec![
        Case {
            branch: "r1",
            desc: "rename a function",
            edit: Box::new(|d| write(d, "shop/pricing.py", &read(d, "shop/pricing.py").replace("def apply_discount", "def apply_rate_discount"))),
            moved: vec![(APPLY_DISCOUNT, Some(func("shop/pricing.py", "apply_rate_discount")))],
        },
        Case {
            branch: "r2",
            desc: "move a file",
            edit: Box::new(|d| git(d, &["mv", "shop/pricing.py", "shop/billing.py"])),
            moved: vec![(APPLY_DISCOUNT, Some(func("shop/billing.py", "apply_discount"))), (TAX_EU, Some(func("shop/billing.py", "tax_eu")))],
        },
        Case {
            branch: "r3",
            desc: "rename a folder",
            edit: Box::new(|d| git(d, &["mv", "shop/util", "shop/helpers"])),
            moved: vec![
                (SLUGIFY, Some(func("shop/helpers/text.py", "slugify"))),
                (CLAMP, Some(func("shop/helpers/math_ops.py", "clamp"))),
                (MATH_FILE, Some(file("shop/helpers/math_ops.py"))),
                (UTIL_FOLDER, Some(folder("shop/helpers"))),
            ],
        },
        Case {
            branch: "r4",
            desc: "move AND edit a file",
            edit: Box::new(|d| {
                git(d, &["mv", "shop/orders.py", "shop/sales.py"]);
                let t = read(d, "shop/sales.py")
                    .replace("line.get(\"qty\", 1)", "line.get(\"quantity\", 1)")
                    .replace("round(total, 2)", "round(total, 4)")
                    .replace("raw.get(\"note\", \"\")", "raw.get(\"note\", \"\").strip()");
                write(d, "shop/sales.py", &t);
            }),
            moved: vec![(PARSE_ORDER, Some(func("shop/sales.py", "OrderService.parse_order")))],
        },
        Case {
            branch: "r5",
            desc: "move a function to another file",
            edit: Box::new(|d| {
                let t = read(d, "shop/util/text.py");
                let (slug, rest) = t.split_once("def truncate").unwrap();
                let slug = slug.trim_start_matches("import re\n\n\n").trim_end().to_string() + "\n";
                write(d, "shop/util/text.py", &format!("def truncate{rest}"));
                write(d, "shop/util/strings.py", &format!("import re\n\n\n{slug}"));
            }),
            moved: vec![(SLUGIFY, Some(func("shop/util/strings.py", "slugify")))],
        },
        Case {
            branch: "r6",
            desc: "rename a function AND change its body",
            edit: Box::new(|d| {
                let t = read(d, "shop/util/math_ops.py");
                let (_, rest) = t.split_once("def average").unwrap();
                let limit = "def limit(value, lower, upper, strict=False):\n    \"\"\"Restrict value to lower..upper.\"\"\"\n    if lower > upper:\n        raise ValueError(\"lower must not exceed upper\")\n    if strict and (value < lower or value > upper):\n        raise ValueError(\"value out of range\")\n    return max(lower, min(value, upper))\n\n\n";
                write(d, "shop/util/math_ops.py", &format!("{limit}def average{rest}"));
            }),
            moved: vec![(CLAMP, Some(func("shop/util/math_ops.py", "limit")))],
        },
        Case {
            branch: "r7",
            desc: "delete a function whose sibling looks the same (extra)",
            edit: Box::new(|d| {
                let t = read(d, "shop/pricing.py");
                let (head, rest) = t.split_once("def tax_eu").unwrap();
                let (_, us) = rest.split_once("def tax_us").unwrap();
                write(d, "shop/pricing.py", &format!("{head}def tax_us{us}"));
            }),
            moved: vec![(TAX_EU, None)],
        },
        Case {
            branch: "r8",
            desc: "two identical methods, both renamed in one commit (extra)",
            edit: Box::new(|d| write(d, "shop/pairs.py", &read(d, "shop/pairs.py").replace("def gross(", "def gross_total(").replace("def net(", "def net_total("))),
            moved: vec![(GROSS, Some(func("shop/pairs.py", "Totals.gross_total"))), (NET, Some(func("shop/pairs.py", "Totals.net_total")))],
        },
    ]
}
