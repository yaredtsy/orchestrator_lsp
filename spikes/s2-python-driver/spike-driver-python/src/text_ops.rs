//! S3: requests that work on TEXT (the unsaved editor buffer), not on a path on disk.
//!
//!   vnoc/outline { text }                              -> { nodes, syntax_errors }
//!   vnoc/planIds { text, version, ids: [{line, column, id}] }
//!       -> { version, edits: [TextEdit], written: [...], skipped: [...], text_hash, took_us }
//!
//! The driver decides WHERE an ID goes; it never picks the ID. The caller (the core, in the product)
//! passes `(def location, id)` pairs, where the location is the def's `position` from `vnoc/outline`.
//! Positions on the wire are VS Code's: 0-based line, UTF-16 code-unit column.

use std::time::Instant;

use anyhow::{Context, anyhow};
use pylspt::defs::{DefNode, scan_defs};
use pylspt::edit::{Edit, apply, plan_ids};
use pylspt::positions::to_position;
use pylspt::scan::{Ctx, scan_module};
use ruff_python_ast::{AnyNodeRef, PythonVersion};
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};
use ruff_source_file::{LineIndex, find_newline};
use ruff_text_size::Ranged;
use serde_json::{Value, json};

/// Byte offset -> (line, UTF-16 column), both 0-based, with a precomputed line table.
struct Lines<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> Lines<'a> {
    fn new(text: &'a str) -> Self {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        Lines { text, starts }
    }
    fn at(&self, off: usize) -> Value {
        let line = self.starts.partition_point(|&s| s <= off) - 1;
        let character: usize = self.text[self.starts[line]..off].encode_utf16().count();
        json!({ "line": line, "character": character })
    }
}

fn parse(text: &str) -> ruff_python_parser::Parsed<ruff_python_ast::ModModule> {
    let options = ParseOptions::from(Mode::Module).with_target_version(PythonVersion::PY313);
    parse_unchecked(text, options).try_into_module().expect("Mode::Module always produces a module")
}

pub fn handle(method: &str, text: &str, p: &Value) -> anyhow::Result<Value> {
    match method {
        "vnoc/outline" => outline(text),
        "vnoc/planIds" => plan(text, p),
        "vnoc/planFileId" => plan_file_id(text, p),
        "vnoc/planRemoveIds" => plan_remove(text, p),
        other => Err(anyhow!("`{other}` does not take `text`")),
    }
}

fn outline(text: &str) -> anyhow::Result<Value> {
    let parsed = parse(text);
    let index = LineIndex::from_source_text(text);
    let ctx = Ctx::new(text, &index);
    let nodes = scan_module(&ctx, &parsed.syntax().body);
    let errors: Vec<(usize, String)> = parsed
        .errors()
        .iter()
        .map(|e| (to_position(&index, text, e.location).line, e.error.to_string()))
        .collect();
    let mut nodes = serde_json::to_value(&nodes)?;
    crate::mark_errors(&mut nodes, &errors);
    let errs: Vec<Value> = errors.iter().map(|(l, m)| json!({ "line": l, "message": m })).collect();
    Ok(json!({ "nodes": nodes, "syntax_errors": errs }))
}

struct Planned {
    line: usize,
    column: usize,
    id: String,
    edit: Edit,
}

fn plan(text: &str, p: &Value) -> anyhow::Result<Value> {
    let started = Instant::now();
    let given: Vec<(usize, usize, String)> = p
        .get("ids")
        .and_then(Value::as_array)
        .context("missing `ids`")?
        .iter()
        .map(|v| {
            Ok((
                v["line"].as_u64().context("ids[].line")? as usize,
                v["column"].as_u64().context("ids[].column")? as usize,
                v["id"].as_str().context("ids[].id")?.to_string(),
            ))
        })
        .collect::<anyhow::Result<_>>()?;

    let parsed = parse(text);
    let index = LineIndex::from_source_text(text);
    let root = AnyNodeRef::from(parsed.syntax());
    let nl = find_newline(text).map_or("\n", |(_, ending)| ending.as_str());
    let defs = scan_defs(&parsed.syntax().body);

    // A def that contains a syntax error is skipped on its own (VS-D31); the others still get their ID.
    let error_ranges: Vec<_> = parsed.errors().iter().map(|e| e.location).collect();

    let mut planned: Vec<Planned> = Vec::new();
    let mut skipped: Vec<Value> = Vec::new();
    let mut used = vec![false; given.len()];

    fn walk(
        defs: &[DefNode],
        ctx: &mut Walk<'_>,
    ) {
        for def in defs {
            if def.id.is_none() {
                let pos = to_position(ctx.index, ctx.text, def.range);
                let hit = ctx.given.iter().position(|(l, c, _)| *l == pos.line && *c == pos.column);
                if let Some(i) = hit {
                    ctx.used[i] = true;
                    if ctx.errors.iter().any(|r| def.range.intersect(*r).is_some()) {
                        ctx.skipped.push(json!({ "line": pos.line, "column": pos.column, "reason": "syntax error inside this def" }));
                    } else {
                        // plan_ids takes its ID from a closure with no idea which def asks, so we call it
                        // for ONE def at a time (children stripped) and hand back that def's ID.
                        let id = ctx.given[i].2.clone();
                        let single = DefNode { children: vec![], ..def.clone() };
                        let edits = plan_ids(std::slice::from_ref(&single), ctx.root, ctx.text, ctx.nl, &mut || id.clone());
                        match edits.into_iter().next() {
                            Some(edit) => ctx.planned.push(Planned { line: pos.line, column: pos.column, id, edit }),
                            None => ctx.skipped.push(json!({ "line": pos.line, "column": pos.column, "reason": "no slot for an ID" })),
                        }
                    }
                }
            }
            walk(&def.children, ctx);
        }
    }
    struct Walk<'a> {
        index: &'a LineIndex,
        text: &'a str,
        root: AnyNodeRef<'a>,
        nl: &'a str,
        given: &'a [(usize, usize, String)],
        used: &'a mut [bool],
        errors: &'a [ruff_text_size::TextRange],
        planned: &'a mut Vec<Planned>,
        skipped: &'a mut Vec<Value>,
    }
    walk(&defs, &mut Walk {
        index: &index, text, root, nl, given: &given, used: &mut used,
        errors: &error_ranges, planned: &mut planned, skipped: &mut skipped,
    });

    // Edits against the ORIGINAL text, ascending. VS Code applies a WorkspaceEdit's edits all against the original.
    planned.sort_by_key(|pl| pl.edit.offset);
    let old = Lines::new(text);
    let edits: Vec<Value> = planned
        .iter()
        .map(|pl| {
            let at = old.at(pl.edit.offset.to_usize());
            json!({ "range": { "start": at, "end": at }, "newText": pl.edit.text })
        })
        .collect();

    // Where each ID text ends up in the NEW text: every earlier edit shifts it right.
    let new_text = apply(text, planned.iter().map(|pl| pl.edit.clone()).collect());
    let new = Lines::new(&new_text);
    let mut shift = 0usize;
    let written: Vec<Value> = planned
        .iter()
        .map(|pl| {
            let start = pl.edit.offset.to_usize() + shift;
            let end = start + pl.edit.text.len();
            shift += pl.edit.text.len();
            json!({
                "id": pl.id, "line": pl.line, "column": pl.column,
                "span": { "start": new.at(start), "end": new.at(end) },
                "text": pl.edit.text,
            })
        })
        .collect();

    let unmatched: Vec<Value> = given.iter().zip(&used).filter(|(_, u)| !**u)
        .map(|((l, c, id), _)| json!({ "line": l, "column": c, "id": id })).collect();

    Ok(json!({
        "version": p.get("version").cloned().unwrap_or(Value::Null),
        "edits": edits, "written": written, "skipped": skipped, "unmatched": unmatched,
        "text_hash": blake3::hash(new_text.as_bytes()).to_hex().to_string(),
        "took_us": started.elapsed().as_micros() as u64,
    }))
}

// ── C20: the file-level `# FileID:` line, and exact-text remove ────────────────────────────────

fn is_encoding_line(l: &str) -> bool {
    let t = l.trim_start();
    t.starts_with('#') && (t.contains("coding:") || t.contains("coding="))
}

/// `# FileID: <id>` goes AFTER the shebang, the PEP 263 encoding line and the leading comment block (a license
/// header), as its own line. A file that already has one gets no edit.
fn plan_file_id(text: &str, p: &Value) -> anyhow::Result<Value> {
    let id = p.get("id").and_then(Value::as_str).context("missing `id`")?;
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut i = 0;
    if lines.first().is_some_and(|l| l.starts_with("#!")) {
        i = 1;
    }
    if i < 2 && lines.get(i).is_some_and(|l| is_encoding_line(l)) {
        i += 1;
    }
    while let Some(l) = lines.get(i).filter(|l| l.trim_start().starts_with('#')) {
        if let Some(existing) = l.trim_start().strip_prefix("# FileID:") {
            return Ok(json!({ "version": p.get("version").cloned().unwrap_or(Value::Null), "edits": [], "written": [], "existing": existing.trim() }));
        }
        i += 1;
    }
    let offset: usize = lines[..i].iter().map(|l| l.len()).sum();
    let nl = find_newline(text).map_or("\n", |(_, e)| e.as_str());
    let mut ins = format!("# FileID: {id}{nl}");
    if offset == text.len() && !text.is_empty() && !text.ends_with('\n') {
        ins = format!("{nl}{ins}"); // the file ends without a newline
    }
    let old = Lines::new(text);
    let at = old.at(offset);
    let new_text = format!("{}{}{}", &text[..offset], ins, &text[offset..]);
    let new = Lines::new(&new_text);
    Ok(json!({
        "version": p.get("version").cloned().unwrap_or(Value::Null),
        "edits": [{ "range": { "start": at, "end": at }, "newText": ins }],
        "written": [{ "id": id, "span": { "start": new.at(offset), "end": new.at(offset + ins.len()) }, "text": ins }],
        "text_hash": blake3::hash(new_text.as_bytes()).to_hex().to_string(),
    }))
}

/// Removes IDs by the EXACT text the writer recorded. The text is searched in the current buffer, so edits
/// elsewhere in the file do not matter. Not found exactly once (e.g. the user edited the `ID:` line by hand)
/// -> nothing changes for that item and the line holding the ID is shown. A removal that would add syntax errors is refused.
fn plan_remove(text: &str, p: &Value) -> anyhow::Result<Value> {
    let items = p.get("items").and_then(Value::as_array).context("missing `items`")?;
    let before_errors = parse(text).errors().len();
    let mut ranges: Vec<(usize, usize, String)> = Vec::new();
    let mut mismatch: Vec<Value> = Vec::new();
    for it in items {
        let id = it["id"].as_str().context("items[].id")?;
        let want = it["text"].as_str().context("items[].text")?;
        let hits: Vec<usize> = text.match_indices(want).map(|(i, _)| i).collect();
        if let [at] = hits[..] {
            ranges.push((at, at + want.len(), id.to_string()));
        } else {
            let line = text.lines().enumerate().find(|(_, l)| l.contains(id)).map(|(n, l)| json!({ "line": n, "text": l }));
            let reason = if hits.is_empty() { "the recorded text is not in the file any more" } else { "the recorded text appears more than once" };
            mismatch.push(json!({ "id": id, "reason": reason, "shown": line }));
        }
    }
    ranges.sort();
    let mut new_text = String::with_capacity(text.len());
    let mut last = 0;
    for (a, b, _) in &ranges {
        new_text.push_str(&text[last..*a]);
        last = *b;
    }
    new_text.push_str(&text[last..]);
    if !ranges.is_empty() && parse(&new_text).errors().len() > before_errors {
        return Ok(json!({ "version": p.get("version").cloned().unwrap_or(Value::Null), "edits": [], "removed": [], "mismatch": mismatch, "refused": "removing would leave the file with more syntax errors" }));
    }
    let old = Lines::new(text);
    let edits: Vec<Value> = ranges.iter().map(|(a, b, _)| json!({ "range": { "start": old.at(*a), "end": old.at(*b) }, "newText": "" })).collect();
    Ok(json!({
        "version": p.get("version").cloned().unwrap_or(Value::Null),
        "edits": edits, "removed": ranges.iter().map(|r| r.2.clone()).collect::<Vec<_>>(), "mismatch": mismatch,
        "text_hash": blake3::hash(new_text.as_bytes()).to_hex().to_string(),
    }))
}
