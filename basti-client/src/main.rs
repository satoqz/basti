mod client;
mod commands;
mod table;

use crate::{
    client::BastiClient,
    commands::{list_command, show_command, submit_command, ListArgs, ShowArgs, SubmitArgs},
};
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
    /// Show a specific task
    Show(ShowArgs),
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let basti = BastiClient::new(cli.cluster)?;

    let result = match cli.command {
        Command::Submit(args) => submit_command(args, basti).await,
        Command::List(args) => list_command(args, basti).await,
        Command::Show(args) => show_command(args, basti).await,
    };

    if let Err(error) = result {
        eprintln!("{} {}", "✖".red().bold(), error);
        process::exit(1);
    }

    Ok(())
}
