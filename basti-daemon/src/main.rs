mod api;
mod worker;

use clap::Parser;
use etcd_client::{Client, ConnectOptions};
use std::{net::SocketAddr, time::Duration};
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

    #[clap(long, required = true, help = "Name of the node")]
    name: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    tracing_subscriber::fmt().without_time().init();

    let etcd = Client::connect(
        cli.etcd,
        Some(
            ConnectOptions::new()
                .with_connect_timeout(Duration::from_secs(3))
                .with_timeout(Duration::from_secs(3)),
        ),
    )
    .await
    .unwrap();

    tokio::join!(
        api::run(cli.listen, cli.name, etcd.clone()),
        worker::run(cli.workers, etcd.clone())
    );
}
