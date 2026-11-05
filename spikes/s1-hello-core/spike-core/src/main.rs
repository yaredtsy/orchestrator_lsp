//! S1 spike-core: the smallest possible "core" process.
//!
//! The VS Code extension starts this binary and talks to it over stdin/stdout
//! using JSON-RPC with `Content-Length:` headers (the LSP wire format).
//!
//! IMPORTANT: stdout is the *wire*. If anything else prints there, the framing
//! breaks. That is why the lint below forbids `println!`, and logs go to stderr.
#![deny(clippy::print_stdout)]

use lsp_server::{Connection, Message, Response};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    // Logs -> stderr. VS Code shows this in the Output panel.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    // `Connection::stdio()` spawns two helper threads that read/write stdin and
    // stdout and turn them into channels. `io_threads` must be joined at the end.
    let (connection, io_threads) = Connection::stdio();

    // The LSP handshake: wait for `initialize`, reply with our capabilities.
    // We have none (we are not a real language server), so an empty object.
    connection.initialize(json!({}))?;
    tracing::info!("initialized, pid={}", std::process::id());

    // Main loop: `receiver` is a channel of incoming messages.
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                // `shutdown` is the client saying "we are done". lsp-server has a helper.
                if connection.handle_shutdown(&req)? {
                    break;
                }
                let result = match req.method.as_str() {
                    "vnoc/ping" => json!({ "pong": true, "pid": std::process::id() }),
                    other => {
                        // Unknown method: reply with an error, never stay silent,
                        // or the client waits forever.
                        let err = Response::new_err(
                            req.id,
                            lsp_server::ErrorCode::MethodNotFound as i32,
                            format!("unknown method {other}"),
                        );
                        connection.sender.send(Message::Response(err))?;
                        continue;
                    }
                };
                connection
                    .sender
                    .send(Message::Response(Response::new_ok(req.id, result)))?;
            }
            // Notifications (e.g. `initialized`, `exit`) need no reply.
            Message::Notification(n) => tracing::debug!("notification {}", n.method),
            Message::Response(_) => {}
        }
    }

    io_threads.join()?;
    Ok(())
}
