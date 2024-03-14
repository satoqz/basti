mod client;
mod commands;
mod table;
mod util;

use std::process;

use clap::{Parser, Subcommand};
use colored::Colorize;
use url::Url;

use crate::{client::BastiClient, commands::*};

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    #[clap(
        long,
        env = "BASTICTL_CLUSTER",
        default_value = "http://127.0.0.1:1337",
        use_value_delimiter = true,
        help = "Comma-separated list of cluster endpoints"
    )]
    cluster: Vec<Url>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Submit a new task
    Submit(SubmitArgs),
    /// List tasks
    List(ListArgs),
    /// Show specific tasks
    Show(ShowArgs),
    /// Cancel tasks
    Cancel(CancelArgs),
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let basti = BastiClient::new(cli.cluster)?;

    let result = match cli.command {
        Command::Submit(args) => submit_command(args, basti).await,
        Command::List(args) => list_command(args, basti).await,
        Command::Show(args) => show_command(args, basti).await,
        Command::Cancel(args) => cancel_command(args, basti).await,
    };

    if let Err(err) = result {
        println!("{} {}", "✖".red().bold(), err);
        process::exit(1);
    }

    Ok(())
}
