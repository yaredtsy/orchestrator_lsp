//! A tiny client for the S2 driver (spawned as a process; framing is Content-Length JSON-RPC).

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use anyhow::{Context, anyhow};
use serde_json::{Value, json};

use crate::model::Def;

pub struct Driver {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    next: u64,
}

impl Driver {
    pub fn start(bin: &Path) -> anyhow::Result<Self> {
        let mut child = Command::new(bin).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().context("spawn driver")?;
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut d = Driver { child, stdin, out, next: 0 };
        d.call("initialize", json!({ "protocol": 1 }))?;
        d.send(&json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }))?;
        Ok(d)
    }

    fn send(&mut self, v: &Value) -> anyhow::Result<()> {
        let body = serde_json::to_vec(v)?;
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len())?;
        self.stdin.write_all(&body)?;
        self.stdin.flush()?;
        Ok(())
    }

    fn call(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        self.next += 1;
        let id = self.next;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        loop {
            let mut len = 0usize;
            loop {
                let mut line = String::new();
                if self.out.read_line(&mut line)? == 0 {
                    return Err(anyhow!("driver closed stdout"));
                }
                let line = line.trim_end();
                if line.is_empty() {
                    break;
                }
                if let Some(v) = line.strip_prefix("Content-Length:") {
                    len = v.trim().parse()?;
                }
            }
            let mut buf = vec![0u8; len];
            self.out.read_exact(&mut buf)?;
            let msg: Value = serde_json::from_slice(&buf)?;
            if msg["id"] == json!(id) {
                if let Some(e) = msg.get("error") {
                    return Err(anyhow!("driver error: {e}"));
                }
                return Ok(msg["result"].clone());
            }
        }
    }

    /// Every def in `text`, with fingerprints. `file` is stamped on each def.
    pub fn defs(&mut self, file: &str, text: &str) -> anyhow::Result<Vec<Def>> {
        let r = self.call("vnoc/defs", json!({ "text": text }))?;
        let mut defs: Vec<Def> = serde_json::from_value(r["defs"].clone())?;
        for d in &mut defs {
            d.file = file.to_string();
        }
        Ok(defs)
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        let _ = self.call("shutdown", Value::Null);
        let _ = self.send(&json!({ "jsonrpc": "2.0", "method": "exit" }));
        let _ = self.child.wait();
    }
}
