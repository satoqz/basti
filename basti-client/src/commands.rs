use std::{cmp::Ordering, time::Duration};

use clap::Args;
use colored::Colorize;
use uuid::Uuid;

use basti_types::{TaskPriority, TaskState};

use crate::{
    client::Client,
    table,
    util::{self, Compact},
};

#[derive(Debug, Args)]
pub struct SubmitArgs {
    #[clap(long, default_value_t = 10, help = "Task duration in seconds")]
    seconds: u64,
    #[clap(
        long,
        default_value_t = 0,
        help = "Additional task duration in milliseconds"
    )]
    millis: u64,
    #[clap(
        long,
        default_value_t = TaskPriority::default(),
        help = "Task priority, 0 = highest priority"
    )]
    priority: TaskPriority,
}

pub async fn submit_command(args: SubmitArgs, client: Client) -> anyhow::Result<()> {
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
pub struct WatchArgs {
    #[clap(
        long,
        required = false,
        default_value_t = false,
        help = "Keep watching for changes using `watch` tool"
    )]
    watch: bool,
    #[clap(
        long,
        required = false,
        default_value_t = 1.0,
        help = "Refresh interval when watching"
    )]
    watch_interval: f32,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    #[clap(flatten)]
    watch_args: WatchArgs,
    #[clap(long, required = false, help = "Task state to filter by")]
    state: Option<TaskState>,
    #[clap(
        long,
        required = false,
        default_value_t = 50,
        help = "Maximum number tasks to list"
    )]
    limit: u32,
}

pub async fn list_command(args: ListArgs, client: Client) -> anyhow::Result<()> {
    if args.watch_args.watch {
        util::reexec_with_watch(args.watch_args.watch_interval)?;
    }

    let mut tasks = client.list(args.state, Some(args.limit)).await?;

    if tasks.len() == args.limit as usize {
        println!(
            " {} Number of tasks is truncated to limit of {}",
            "⚠".yellow().bold(),
            args.limit
        );
    }

    tasks.sort_by(|a, b| match (a.key.state, b.key.state) {
        (TaskState::Queued, TaskState::Running) => Ordering::Greater,
        (TaskState::Running, TaskState::Queued) => Ordering::Less,
        _ => a.value.cmp(&b.value),
    });

    table::print_tasks(tasks);
    Ok(())
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    #[clap(flatten)]
    watch_args: WatchArgs,
    #[clap(required = true, help = "Tasks to show")]
    ids: Vec<Uuid>,
}

pub async fn show_command(args: ShowArgs, client: Client) -> anyhow::Result<()> {
    if args.watch_args.watch {
        util::reexec_with_watch(args.watch_args.watch_interval)?;
    }

    let tasks = futures::future::join_all(args.ids.compact().into_iter().map(|id| client.find(id)))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    table::print_tasks(tasks);
    Ok(())
}

#[derive(Debug, Args)]
pub struct CancelArgs {
    #[clap(required = true, help = "Tasks to cancel")]
    ids: Vec<Uuid>,
}

pub async fn cancel_command(args: CancelArgs, client: Client) -> anyhow::Result<()> {
    let task_results =
        futures::future::join_all(args.ids.compact().into_iter().map(|id| client.cancel(id))).await;

    for result in task_results {
        match result {
            Ok(task) => println!(
                "{} Canceled task {}",
                "✓".green().bold(),
                task.key.id.to_string().bright_black().italic()
            ),
            Err(err) => println!("{} {}", "✖".red().bold(), err),
        }
    }

    Ok(())
}
