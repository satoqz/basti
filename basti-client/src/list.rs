use crate::client::BastiClient;
use basti_common::task::TaskState;
use clap::Args;
use colored::Colorize;
use std::process;
use tabled::{
    builder::Builder,
    settings::{object::Rows, Color, Style},
};

#[derive(Debug, Args)]
pub struct ListArgs {
    #[clap(long, required = false, help = "Task state to filter by")]
    state: Option<TaskState>,
}

pub async fn list_command(args: ListArgs, client: BastiClient) {
    let tasks = match client.list(args.state).await {
        Ok(tasks) => tasks,
        Err(error) => {
            eprintln!("{} {}", "✖".red().bold(), error);
            process::exit(1)
        }
    };

    let mut builder = Builder::new();
    builder.push_record([
        "ID",
        "State",
        "Assignee",
        "Duration",
        "Remaining",
        "Priority",
    ]);

    for task in tasks {
        builder.push_record([
            task.key.id.to_string(),
            task.key.state.to_string(),
            task.details.assignee.unwrap_or("none".into()),
            format!("{}s", task.details.duration.as_secs()),
            format!("{}s", task.details.remaining.as_secs()),
            task.details.priority.to_string(),
        ])
    }

    let mut table = builder.build();
    table
        .with(Style::modern_rounded())
        .modify(Rows::first(), Color::BOLD);

    eprintln!("{}", table)
}
