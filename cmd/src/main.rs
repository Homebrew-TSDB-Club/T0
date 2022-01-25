#![warn(
    missing_debug_implementations,
    clippy::explicit_iter_loop,
    clippy::use_self,
    clippy::clone_on_ref_ptr,
    clippy::future_not_send,
    clippy::use_debug
)]

use clap::Parser;
use context::Context;
use mimalloc::MiMalloc;
use proto::prometheus::remote_server::RemoteServer;
use proto::query::query_server::QueryServer;
use query::Query;
use std::sync::Arc;
use storage::Storage;
use tokio::runtime;
use tonic::transport::Server;
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
    let addr = args.addr.parse()?;

    tracing_subscriber::fmt::init();
    info!("hello, world");
    info!("HTTP server hosts on {}", addr);
    info!("HTTP server uses {} cores", args.server_cores);
    info!("Storage component uses {} cores", args.storage_cores);

    let cores = (0..args.storage_cores).map(|id| id * 2).collect::<Vec<_>>();
    let storage = Arc::new(Storage::new(&cores, Arc::new(Context::new())));
    let query = Query::new(Arc::clone(&storage));

    let runtime = runtime::Builder::new_multi_thread()
        .worker_threads(args.server_cores)
        .enable_all()
        .build()?;

    debug!("start tokio runtime");
    runtime.block_on(async move {
        Server::builder()
            .accept_http1(false)
            .add_service(RemoteServer::new(storage.to_grpc_server()))
            .add_service(QueryServer::new(query))
            .serve(addr)
            .await
    })?;

    Ok(())
}
