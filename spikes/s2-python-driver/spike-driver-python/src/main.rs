//! S2 spike: `vnoc-driver-python` as a process. A thin adapter between the (spike) driver
//! protocol and pylspt.
//!
//! WIRE (protocol 1, JSON-RPC with `Content-Length` framing on stdin/stdout):
//!   initialize  { protocol: 1 }                 -> { protocol: 1, name: "python", version }
//!   vnoc/outline   { root, file }               -> pylspt outline nodes
//!   vnoc/callTree  { root, file, entry, depth? }-> pylspt call wire (depth = render_depth)
//!   vnoc/expand    { root, file, entry, depth? }-> same as callTree, rooted at the callee
//!   $/cancelRequest { id }                      -> sets that request's cancel flag
//!
//! stdout IS the protocol. Nothing else may write there, not even a panic message.
#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, anyhow};
use lsp_server::{Connection, ErrorCode, Message, Request, RequestId, Response};
use pylspt::db::{open, open_project};
use pylspt::interp::{Mode, Options, call_wire};
use pylspt::scan::{Ctx, scan_module};
use ruff_db::parsed::parsed_module;
use ruff_db::source::source_text;
use ruff_db::system::SystemPath;
use ruff_source_file::LineIndex;
use serde_json::{Value, json};
use ty_project::ProjectDatabase;

const PROTOCOL: u64 = 1;

/// One salsa database per project root, opened on first use. Workers get a clone:
/// salsa clones share the cache, so a second request on the same root is warm.
type Dbs = Arc<Mutex<HashMap<String, ProjectDatabase>>>;

fn main() -> anyhow::Result<()> {
    // FINDING: without a filter, ty/salsa log every query at trace level (43 MB for one bench run).
    // Default: our own info lines + everyone else's warnings. `VNOC_LOG=debug` overrides.
    let filter = tracing_subscriber::EnvFilter::try_from_env("VNOC_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,spike_driver_python=info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_writer(std::io::stderr).with_ansi(false).init();

    // Dev subcommand for the C1 timings. Not part of the protocol.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("bench") {
        return bench(&args[2..]);
    }
    if args.get(1).map(String::as_str) == Some("isolated") {
        return isolated(&args[2..]);
    }
    if args.get(1).map(String::as_str) == Some("noisy") {
        // C2 self-test: deliberately breaks the rules the driver must obey.
        // Used only to prove the harness catches stdout pollution.
        return noisy();
    }

    let (conn, io) = Connection::stdio();
    handshake(&conn)?;

    let dbs: Dbs = Arc::default();
    // request id -> its cancel flag. The dispatcher sets flags; workers read them.
    let in_flight: Arc<Mutex<HashMap<RequestId, Arc<AtomicBool>>>> = Arc::default();

    // The main loop only dispatches. It never analyses, so cancel / shutdown / EOF stay responsive.
    for msg in &conn.receiver {
        match msg {
            Message::Request(req) => {
                if conn.handle_shutdown(&req)? {
                    break;
                }
                let cancel = Arc::new(AtomicBool::new(false));
                in_flight.lock().unwrap().insert(req.id.clone(), cancel.clone());
                let (sender, dbs, in_flight) = (conn.sender.clone(), dbs.clone(), in_flight.clone());
                std::thread::spawn(move || {
                    let id = req.id.clone();
                    let started = Instant::now();
                    let method = req.method.clone();
                    let resp = run(req, &dbs, &cancel);
                    tracing::info!("{method} took {:?}", started.elapsed());
                    in_flight.lock().unwrap().remove(&id);
                    let _ = sender.send(Message::Response(resp));
                });
            }
            Message::Notification(n) if n.method == "$/cancelRequest" => {
                if let Some(id) = n.params.get("id").and_then(|v| serde_json::from_value::<RequestId>(v.clone()).ok())
                    && let Some(flag) = in_flight.lock().unwrap().get(&id)
                {
                    flag.store(true, Ordering::Relaxed);
                }
            }
            // Unknown notifications are ignored (VS-D68: a driver never dies on them).
            Message::Notification(n) => tracing::debug!("ignored notification {}", n.method),
            Message::Response(_) => {}
        }
    }
    // shutdown, `exit`, or stdin EOF: tell running work to stop and leave.
    for flag in in_flight.lock().unwrap().values() {
        flag.store(true, Ordering::Relaxed);
    }
    drop(conn);
    io.join()?;
    Ok(())
}

/// `initialize { protocol: 1 }` -> `{ protocol, name, version }`. A wrong protocol is refused with a reason.
fn handshake(conn: &Connection) -> anyhow::Result<()> {
    let (id, params) = conn.initialize_start()?;
    let asked = params.get("protocol").and_then(Value::as_u64);
    if asked != Some(PROTOCOL) {
        let msg = format!("driver speaks protocol {PROTOCOL}, core asked for {asked:?}");
        conn.sender.send(Message::Response(Response::new_err(id, ErrorCode::InvalidRequest as i32, msg.clone())))?;
        return Err(anyhow!(msg));
    }
    conn.initialize_finish(id, json!({ "protocol": PROTOCOL, "name": "python", "version": env!("CARGO_PKG_VERSION") }))?;
    Ok(())
}

fn run(req: Request, dbs: &Dbs, cancel: &AtomicBool) -> Response {
    // A request cancelled before it starts costs nothing.
    if cancel.load(Ordering::Relaxed) {
        return Response::new_err(req.id, ErrorCode::RequestCanceled as i32, "cancelled".into());
    }
    let result = handle(&req, dbs);
    // FINDING: pylspt has `Limits::deadline` but no external cancel flag, so a solve that is already
    // running cannot be interrupted. We can only drop its answer. A hook in solver.rs would fix that.
    if cancel.load(Ordering::Relaxed) {
        return Response::new_err(req.id, ErrorCode::RequestCanceled as i32, "cancelled".into());
    }
    match result {
        Ok(v) => Response::new_ok(req.id, v),
        Err(e) => Response::new_err(req.id, ErrorCode::InternalError as i32, format!("{e:#}")),
    }
}

fn str_param<'a>(p: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    p.get(key).and_then(Value::as_str).with_context(|| format!("missing string param `{key}`"))
}

fn utf8_path(p: &str) -> anyhow::Result<std::path::PathBuf> {
    std::fs::canonicalize(p).with_context(|| format!("cannot open {p}"))
}

fn db_for(dbs: &Dbs, root: &str) -> anyhow::Result<ProjectDatabase> {
    let mut map = dbs.lock().unwrap();
    if let Some(db) = map.get(root) {
        return Ok(db.clone());
    }
    let root_path = utf8_path(root)?;
    let sys = SystemPath::from_std_path(&root_path).ok_or_else(|| anyhow!("root is not UTF-8"))?;
    let db = open_project(sys)?;
    map.insert(root.to_string(), db.clone());
    Ok(db)
}

fn handle(req: &Request, dbs: &Dbs) -> anyhow::Result<Value> {
    let p = &req.params;
    let db = db_for(dbs, str_param(p, "root")?)?;
    let file_path = utf8_path(str_param(p, "file")?)?;
    let file_path = SystemPath::from_std_path(&file_path).ok_or_else(|| anyhow!("file is not UTF-8"))?;
    let (file, program_file) = open(&db, file_path)?;

    match req.method.as_str() {
        "vnoc/outline" => {
            let parsed = parsed_module(&db, program_file.python_file(&db)).load(&db);
            let source = source_text(&db, file);
            let index = LineIndex::from_source_text(&source);
            let ctx = Ctx::new(&source, &index);
            let nodes = scan_module(&ctx, &parsed.syntax().body);
            // C17: the parser is error tolerant and lists every error with a range. The nodes carry no
            // error flag, so the adapter marks each def whose line range contains an error line.
            let errors: Vec<(usize, String)> = parsed
                .errors()
                .iter()
                .map(|e| (pylspt::positions::to_position(&index, &source, e.location).line, e.error.to_string()))
                .collect();
            let mut nodes = serde_json::to_value(&nodes)?;
            mark_errors(&mut nodes, &errors);
            let errs: Vec<Value> = errors.iter().map(|(l, m)| json!({ "line": l, "message": m })).collect();
            Ok(json!({ "nodes": nodes, "syntax_errors": errs }))
        }
        "vnoc/callTree" | "vnoc/expand" => {
            let entry = str_param(p, "entry")?;
            let mut opts = Options { mode: Mode::Summary, require_ids: false, ..Options::default() };
            opts.limits.render_depth = p.get("depth").and_then(Value::as_u64).map(|d| d as u32);
            let (wire, _) = call_wire(&db, file, entry, &opts).ok_or_else(|| anyhow!("no function `{entry}`"))?;
            Ok(serde_json::to_value(&wire)?)
        }
        other => Err(anyhow!("unknown method {other}")),
    }
}

/// Sets `has_syntax_error` on every def whose line range contains an error line.
/// Returns whether this subtree contains one, so parents are marked too.
fn mark_errors(nodes: &mut Value, errors: &[(usize, String)]) -> bool {
    let mut any = false;
    for n in nodes.as_array_mut().into_iter().flatten() {
        let pos = &n["position"];
        let (a, b) = (pos["line"].as_u64().unwrap_or(0) as usize, pos["end_line"].as_u64().unwrap_or(0) as usize);
        let mut hit = errors.iter().any(|(l, _)| (a..=b).contains(l));
        mark_errors(&mut n["children"], errors);
        // a def is "bad" only if an error is inside it, so say which children are bad too
        hit |= n["children"].as_array().is_some_and(|c| c.iter().any(|x| x["has_syntax_error"] == true));
        n["has_syntax_error"] = Value::Bool(hit);
        any |= hit;
    }
    any
}

/// `bench <root> <file> <entry>`: C1. Cold = fresh database, warm = same database again.
fn bench(args: &[String]) -> anyhow::Result<()> {
    let [root, file, entry] = args else { return Err(anyhow!("usage: bench <root> <file> <entry>")) };
    for depth in [Some(1u32), Some(3), Some(5), None] {
        for pass in ["cold", "warm"] {
            let dbs: Dbs = Arc::default();
            let t = Instant::now();
            let db = db_for(&dbs, root)?;
            let open_t = t.elapsed();
            let run_once = |db: &ProjectDatabase| -> anyhow::Result<(usize, std::time::Duration)> {
                let fp = utf8_path(file)?;
                let fp = SystemPath::from_std_path(&fp).unwrap().to_owned();
                let (f, _) = open(db, &fp)?;
                let mut opts = Options { mode: Mode::Summary, ..Options::default() };
                opts.limits.render_depth = depth;
                let t = Instant::now();
                let (wire, _) = call_wire(db, f, entry, &opts).ok_or_else(|| anyhow!("no function"))?;
                Ok((serde_json::to_string(&wire)?.len(), t.elapsed()))
            };
            if pass == "warm" {
                run_once(&db)?; // prime
            }
            let (bytes, took) = run_once(&db)?;
            tracing::info!("depth={depth:?} {pass}: open_project {open_t:?}, call_wire {took:?}, json {bytes} bytes");
        }
    }
    Ok(())
}

/// `isolated <root> <file> <entry> <prime-entry>`: C1 without the fixed ty warm-up.
/// Per depth: fresh database, solve `prime-entry` (a tiny function) to load ty/typeshed, then time `entry`.
fn isolated(args: &[String]) -> anyhow::Result<()> {
    let [root, file, entry, prime] = args else { return Err(anyhow!("usage: isolated <root> <file> <entry> <prime-entry>")) };
    for depth in [Some(1u32), Some(2), Some(3), Some(5), None] {
        let dbs: Dbs = Arc::default();
        let db = db_for(&dbs, root)?;
        let fp = utf8_path(file)?;
        let fp = SystemPath::from_std_path(&fp).unwrap().to_owned();
        let (f, _) = open(&db, &fp)?;
        let mut opts = Options { mode: Mode::Summary, ..Options::default() };
        opts.limits.render_depth = Some(1);
        let t = Instant::now();
        call_wire(&db, f, prime, &opts).ok_or_else(|| anyhow!("no prime function"))?;
        let prime_t = t.elapsed();
        opts.limits.render_depth = depth;
        let t = Instant::now();
        let (wire, _) = call_wire(&db, f, entry, &opts).ok_or_else(|| anyhow!("no function"))?;
        let took = t.elapsed();
        tracing::info!("depth={depth:?}: prime {prime_t:?}, target {took:?}, json {} bytes", serde_json::to_string(&wire)?.len());
    }
    Ok(())
}

fn noisy() -> anyhow::Result<()> {
    #[allow(clippy::print_stdout)]
    {
        println!("this line must be caught by the guard");
    }
    Ok(())
}
