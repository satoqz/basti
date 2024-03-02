mod api;
mod client;
mod worker;

use crate::client::Client;
use anyhow::Result;
use clap::Parser;
use std::{net::SocketAddr, num::NonZeroUsize};
use tracing::Level;
use url::Url;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(
        long,
        required = true,
        use_value_delimiter = true,
        help = "Comma-separated list of etcd endpoints"
    )]
    etcd: Vec<Url>,

    #[clap(
        long,
        default_value = "127.0.0.1:1337",
        help = "API endpoint to listen on"
    )]
    listen: SocketAddr,

    #[clap(long, default_value = "3", help = "Number of workers to run")]
    workers: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .pretty()
        .init();

    let client = Client::connect(args.etcd).await.unwrap();
    let api_handle = api::run(args.listen, client.clone());

    if let Some(workers) = NonZeroUsize::new(args.workers) {
        tokio::select! {
            result = api_handle => {
                tracing::warn!("API exited unexpectedly!");
                result
            },
            result = worker::run(workers, client.clone()) =>  {
                tracing::warn!("Worker exited unexpectedly!");
                result
            }
        }
    } else {
        let result = api_handle.await;
        tracing::warn!("API exited unexpectedly!");
        return result;
    }
}
