mod client;
mod commands;
mod table;
mod util;

use crate::{client::BastiClient, commands::*};
use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::process;
use url::Url;

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    #[clap(
        long,
        default_value = "http://127.0.0.1:1337",
        use_value_delimiter = true,
        env = "BASTI_CLUSTER",
        help = "Comma-separeted list of cluster endpoints"
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
    /// Wait for tasks to complete
    Wait(WaitArgs),
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let basti = BastiClient::new(cli.cluster)?;

    let result = match cli.command {
        Command::Submit(args) => submit_command(args, basti).await,
        Command::List(args) => list_command(args, basti).await,
        Command::Show(args) => show_command(args, basti).await,
        Command::Cancel(args) => cancel_command(args, basti).await,
        Command::Wait(args) => wait_command(args, basti).await,
    };

    if let Err(error) = result {
        eprintln!("{} {}", "✖".red().bold(), error);
        process::exit(1);
    }

    Ok(())
}
