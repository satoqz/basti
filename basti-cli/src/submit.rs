use crate::client::BastiClient;
use clap::Args;
use colored::Colorize;
use std::{process, time::Duration};

#[derive(Debug, Args)]
pub struct SubmitArgs {
    /// Task priority, higher = higher priority, 0 = lowest priority
    #[clap(long, default_value = "0")]
    priority: u32,
    /// Task duration in seconds
    #[clap(long, default_value = "0")]
    seconds: u64,
    /// Additional task duration in milliseconds
    #[clap(long, default_value = "0")]
    millis: u64,
}

pub async fn submit_command(args: SubmitArgs, client: BastiClient) {
    let task = match client
        .submit(
            Duration::from_secs(args.seconds) + Duration::from_millis(args.millis),
            args.priority,
        )
        .await
    {
        Ok(task) => task,
        Err(error) => {
            eprintln!("{} {}", "✖".red().bold(), error);
            process::exit(1)
        }
    };

    eprintln!(
        "{} Created task {}",
        "✓".green().bold(),
        task.key.id.to_string().bright_black().italic()
    );
}