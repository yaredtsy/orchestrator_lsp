//! S1 / C3: the same `vnoc/ping` server as `spike-core`, written with `async-lsp`.
//! Compare with `../spike-core/src/main.rs`.
#![deny(clippy::print_stdout)]

use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::server::LifecycleLayer;
use async_lsp::tracing::TracingLayer;
use async_lsp::lsp_types::{InitializeResult, ServerCapabilities, notification, request};
use serde::{Deserialize, Serialize};
use std::ops::ControlFlow;
use tower::ServiceBuilder;

// With `lsp-server` a method was just a string in a `match`. Here every method is a *type*
// that says what its params and result are. The router then parses and serializes for us.
enum Ping {}

#[derive(Serialize, Deserialize)]
struct Pong {
    pong: bool,
    pid: u32,
}

impl request::Request for Ping {
    type Params = ();
    type Result = Pong;
    const METHOD: &'static str = "vnoc/ping";
}

struct State;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (server, _client) = async_lsp::MainLoop::new_server(|_client| {
        let mut router = Router::new(State);
        router
            .request::<request::Initialize, _>(|_, _| async move {
                Ok(InitializeResult {
                    capabilities: ServerCapabilities::default(),
                    server_info: None,
                })
            })
            .request::<Ping, _>(|_, ()| async move {
                Ok(Pong { pong: true, pid: std::process::id() })
            })
            .request::<request::Shutdown, _>(|_, _| async move { Ok(()) })
            // An unhandled notification is a *fatal* error in async-lsp (lsp-server ignored it).
            .notification::<notification::Initialized>(|_, _| ControlFlow::Continue(()));

        // Layers wrap the router like onion skins: lifecycle (initialize/shutdown/exit rules),
        // panic -> error response instead of dying, concurrency (run handlers in parallel), tracing.
        ServiceBuilder::new()
            .layer(TracingLayer::default())
            .layer(LifecycleLayer::default())
            .layer(CatchUnwindLayer::default())
            .layer(ConcurrencyLayer::default())
            .service(router)
    });

    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .init();

    let (stdin, stdout) = (
        async_lsp::stdio::PipeStdin::lock_tokio().unwrap(),
        async_lsp::stdio::PipeStdout::lock_tokio().unwrap(),
    );
    server.run_buffered(stdin, stdout).await.unwrap();
}
