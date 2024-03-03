use crate::{client::BastiClient, table::print_task_table};
use anyhow::Result;
use basti_common::task::TaskState;
use clap::Args;
use colored::Colorize;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Args)]
pub struct SubmitArgs {
    #[clap(
        long,
        default_value = "5",
        help = "Task priority, 0 = highest priority"
    )]
    priority: u32,

    #[clap(long, default_value = "10", help = "Task duration in seconds")]
    seconds: u64,

    #[clap(
        long,
        default_value = "0",
        help = "Additional task duration in milliseconds"
    )]
    millis: u64,
}

pub async fn submit_command(args: SubmitArgs, client: BastiClient) -> Result<()> {
    let task = client
        .submit(
            Duration::from_secs(args.seconds) + Duration::from_millis(args.millis),
            args.priority,
        )
        .await?;

    println!(
        "{} Created task {}",
        "✓".green().bold(),
        task.key.id.to_string().bright_black().italic()
    );

    Ok(())
}

#[derive(Debug, Args)]
pub struct ListArgs {
    #[clap(long, required = false, help = "Task state to filter by")]
    state: Option<TaskState>,
}

pub async fn list_command(args: ListArgs, client: BastiClient) -> Result<()> {
    let tasks = client.list(args.state).await?;
    print_task_table(tasks);
    Ok(())
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    #[clap(required = true, help = "Task id to find")]
    id: Uuid,
}

pub async fn show_command(args: ShowArgs, client: BastiClient) -> Result<()> {
    let task = client.find(args.id).await?;
    print_task_table(vec![task]);
    Ok(())
}
