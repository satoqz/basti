mod api;
mod ops;
mod worker;

use anyhow::Result;
use clap::Parser;
use etcd_client::{Client, ConnectOptions};
use std::{net::SocketAddr, num::NonZeroUsize, time::Duration};
use tracing::Level;
use url::Url;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(
        long,
        default_value = "http://127.0.0.1:2379",
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

    #[clap(long, required = true, help = "Name of the node")]
    name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .pretty()
        .init();

    let client = Client::connect(
        args.etcd,
        Some(
            ConnectOptions::default()
                .with_connect_timeout(Duration::from_secs(2))
                .with_timeout(Duration::from_secs(2)),
        ),
    )
    .await?;

    if let Some(workers) = NonZeroUsize::new(args.workers) {
        worker::run_detached(workers, client.clone(), args.name).await;
    }

    api::run(args.listen, client).await
}
