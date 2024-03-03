use crate::client::BastiClient;
use anyhow::Result;
use basti_common::task::TaskState;
use clap::Args;
use colored::Colorize;
use std::time::Duration;
use tabled::{
    builder::Builder,
    settings::{
        object::{Columns, Rows},
        Color, Style,
    },
};

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
    let mut tasks = client.list(args.state).await?;
    tasks.sort_by(|a, b| a.details.cmp(&b.details));

    let mut builder = Builder::new();
    builder.push_record([
        "ID",
        "State",
        "Assignee",
        "Priority",
        "Duration",
        "Remaining",
        "Progress",
    ]);

    for task in tasks {
        let progress = if task.details.duration.as_secs() == 0 {
            0
        } else {
            (((task.details.duration - task.details.remaining).as_secs_f32()
                / task.details.duration.as_secs_f32())
                * 8 as f32) as usize
        };

        builder.push_record([
            task.key.id.to_string(),
            task.key.state.to_string(),
            task.details.assignee.unwrap_or("none".into()),
            task.details.priority.to_string(),
            format!(
                "{}.{:03}s",
                task.details.duration.as_secs(),
                task.details.duration.subsec_millis()
            ),
            format!(
                "{}.{:03}s",
                task.details.remaining.as_secs(),
                task.details.duration.subsec_millis()
            ),
            "█".repeat(progress),
        ])
    }

    let mut table = builder.build();
    table
        .with(Style::modern_rounded())
        .modify(Columns::last(), Color::FG_GREEN)
        .modify(Rows::first(), Color::FG_WHITE | Color::BOLD);
    println!("{}", table);

    Ok(())
}
