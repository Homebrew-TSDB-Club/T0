#![warn(
    missing_debug_implementations,
    clippy::explicit_iter_loop,
    clippy::use_self,
    clippy::clone_on_ref_ptr,
    clippy::future_not_send,
    clippy::use_debug
)]

mod tcp;

use clap::Parser;
use context::Context;
use mimalloc::MiMalloc;
use query::QueryServer;
use std::sync::Arc;
use storage::StorageServer;
use tokio::runtime;
use tracing::{debug, info};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Parser, Debug)]
#[clap(
    name = "t0",
    about = "A real-time in-memory distributed timeseries database."
)]
struct Args {
    /// HTTP server address.
    #[clap(short, long, default_value = "[::1]:1107")]
    addr: String,
    // Storage cores.
    #[clap(long, default_value_t = default_cores())]
    storage_cores: usize,
    // HTTP Server cores.
    #[clap(long, default_value_t = default_cores())]
    server_cores: usize,
}

fn default_cores() -> usize {
    num_cpus::get() / 2
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = clap::Parser::parse();
    let addr = args.addr;

    tracing_subscriber::fmt::init();
    info!("hello, world");
    info!("HTTP server hosts on {}", addr);
    info!("HTTP server uses {} cores", args.server_cores);
    info!("Storage component uses {} cores", args.storage_cores);

    let cores = (0..args.storage_cores).map(|id| id * 2).collect::<Vec<_>>();
    let storage = Arc::new(StorageServer::new(&cores, Arc::new(Context::new())));

    let runtime = runtime::Builder::new_multi_thread()
        .worker_threads(args.server_cores)
        .enable_all()
        .build()?;

    debug!("start tokio runtime");
    runtime.block_on(async move {
        let server = Arc::new(
            tcp::Server::bind(
                addr,
                Arc::clone(&storage),
                Arc::new(QueryServer::new(Arc::clone(&storage))),
            )
            .await?,
        );
        server.serve().await
    })?;

    Ok(())
}
