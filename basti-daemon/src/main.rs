mod api;
mod ops;
mod worker;

use anyhow::bail;
use basti_types::Name;
use clap::Parser;
use etcd_client::{Client, ConnectOptions};
use std::{net::SocketAddr, num::NonZeroUsize, time::Duration};
use tracing::Level;
use url::Url;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(long, env = "BASTID_NAME", required = true, help = "Name of the node")]
    name: Name,

    #[clap(
        long,
        env = "BASTID_WORKERS",
        default_value_t = 3,
        help = "Number of workers to run"
    )]
    workers: usize,

    #[clap(
        long,
        env = "BASTID_NO_API",
        default_value_t = false,
        help = "Don't expose an API service"
    )]
    no_api: bool,

    #[clap(
        long,
        env = "BASTID_LISTEN",
        default_value = "127.0.0.1:1337",
        help = "API endpoint to listen on"
    )]
    listen: SocketAddr,

    #[clap(
        long,
        env = "BASTID_ETCD",
        default_value = "http://127.0.0.1:2379",
        use_value_delimiter = true,
        help = "Comma-separated list of etcd endpoints"
    )]
    etcd: Vec<Url>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
    .await?
    .kv_client();

    match (NonZeroUsize::new(args.workers), args.no_api) {
        (Some(amount), false) => {
            tokio::select! {
                _ = worker::run(amount, client.clone(), args.name) => bail!("worker exited unexpectedly"),
                result = api::run(args.listen, client) => result?,
            }
        }
        (Some(amount), true) => worker::run(amount, client, args.name).await,
        (None, false) => api::run(args.listen, client.clone()).await?,
        (None, true) => bail!("nothing to do: running 0 workers and no api service"),
    };

    Ok(())
}
