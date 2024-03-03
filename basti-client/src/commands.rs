use crate::{client::BastiClient, table::print_task_table, util::Compact};
use anyhow::Result;
use basti_common::task::{TaskPriority, TaskState};
use clap::Args;
use colored::Colorize;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Args)]
pub struct SubmitArgs {
    #[clap(
        long,
        default_value = "TaskPriority::default",
        help = "Task priority, 0 = highest priority"
    )]
    priority: TaskPriority,

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

    #[clap(
        long,
        required = false,
        default_value = "50",
        help = "Maximum number tasks to list"
    )]
    limit: u32,
}

pub async fn list_command(args: ListArgs, client: BastiClient) -> Result<()> {
    let tasks = client.list(args.state, Some(args.limit)).await?;
    if tasks.len() == args.limit as usize {
        println!(
            " {} Number of tasks is truncated to limit of {}",
            "⚠".yellow().bold(),
            args.limit
        )
    }
    print_task_table(tasks);
    Ok(())
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    #[clap(required = true, help = "Tasks to show")]
    ids: Vec<Uuid>,
}

pub async fn show_command(args: ShowArgs, client: BastiClient) -> Result<()> {
    let tasks = futures::future::join_all(args.ids.compact().into_iter().map(|id| client.find(id)))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    print_task_table(tasks);
    Ok(())
}

#[derive(Debug, Args)]
pub struct CancelArgs {
    #[clap(required = true, help = "Tasks to cancel")]
    ids: Vec<Uuid>,
}

pub async fn cancel_command(args: CancelArgs, client: BastiClient) -> Result<()> {
    let task_results =
        futures::future::join_all(args.ids.compact().into_iter().map(|id| client.cancel(id))).await;

    for result in task_results {
        match result {
            Ok(task) => println!(
                "{} Canceled task {}",
                "✓".green().bold(),
                task.key.id.to_string().bright_black().italic()
            ),
            Err(error) => println!("{} {}", "✖".red().bold(), error),
        }
    }

    Ok(())
}
