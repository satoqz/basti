mod api;
mod ops;
mod worker;

use std::{net::SocketAddr, num::NonZeroUsize, process::exit, str::FromStr, time::Duration};

use clap::Parser;
use etcd_client::{Client, ConnectOptions};
use tokio::signal;
use url::Url;

use basti_types::WorkerName;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(long, env = "BASTID_NAME", help = "Name of the node")]
    name: Option<WorkerName>,

    #[clap(
        long,
        env = "BASTID_WORKERS",
        default_value_t = 1,
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

fn default_worker_name() -> WorkerName {
    hostname::get()
        .ok()
        .and_then(|name| name.into_string().ok())
        .and_then(|name| WorkerName::from_str(&name).ok())
        .unwrap_or_default()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    tracing_subscriber::fmt().with_target(false).init();

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

    let run_worker = {
        let client = client.clone();
        |amount| async move {
            worker::run(
                amount,
                client,
                args.name.unwrap_or_else(default_worker_name),
            )
            .await;
        }
    };

    let run_api = || async move {
        if let Err(err) = api::run(args.listen, client).await {
            tracing::error!("api exited with error: {err}");
            exit(1);
        }
    };

    match (NonZeroUsize::new(args.workers), args.no_api) {
        (Some(amount), false) => {
            tokio::join!(run_api(), run_worker(amount));
        }
        (Some(amount), true) => run_worker(amount).await,
        (None, false) => run_api().await,
        (None, true) => tracing::warn!("nothing to do"),
    };

    Ok(())
}

pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to listen for event");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
