use crate::client::BastiClient;
use anyhow::Result;
use basti_common::task::TaskState;
use clap::Args;
use tabled::{
    builder::Builder,
    settings::{object::Rows, Color, Style},
};

#[derive(Debug, Args)]
pub struct ListArgs {
    #[clap(long, required = false, help = "Task state to filter by")]
    state: Option<TaskState>,
}

pub async fn list_command(args: ListArgs, client: BastiClient) -> Result<()> {
    let tasks = client.list(args.state).await?;

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
            task.details.priority.to_string(),
        ])
    }

    let mut table = builder.build();
    table
        .with(Style::modern_rounded())
        .modify(Rows::first(), Color::BOLD);
    eprintln!("{}", table);

    Ok(())
}
