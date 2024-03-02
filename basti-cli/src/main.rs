mod client;
mod list;
mod submit;

use crate::{
    client::BastiClient,
    list::{list_command, ListArgs},
    submit::{submit_command, SubmitArgs},
};
use clap::{Parser, Subcommand};
use url::Url;

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    #[clap(
        long,
        short,
        required = true,
        use_value_delimiter = true,
        env = "BASTI_CLUSTER",
        help = "Comma-separeted list of cluster endpoints"
    )]
    cluster: Vec<Url>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Submit tasks
    Submit(SubmitArgs),
    /// List tasks
    List(ListArgs),
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    let basti = BastiClient::new(cli.cluster);
    match cli.command {
        Command::Submit(args) => submit_command(args, basti).await,
        Command::List(args) => list_command(args, basti).await,
    };
}
