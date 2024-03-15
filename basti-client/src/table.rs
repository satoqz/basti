use chrono::Utc;
use tabled::{
    builder::Builder,
    settings::{
        object::{Columns, Rows},
        Color, Style,
    },
};

use basti_types::{Task, TaskState, WorkerName};

const PROGRESS_BAR_LENGTH: usize = 16;

pub fn print_tasks(tasks: Vec<Task>) {
    let mut builder = Builder::new();
    builder.push_record([
        "ID",
        "State",
        "Priority",
        "Assignee",
        "Duration",
        "Remaining",
        "Progress",
    ]);

    let now = Utc::now();

    for task in tasks {
        let optimistic_remaining = if task.key.state == TaskState::Running {
            task.value
                .remaining
                .saturating_sub((now - task.value.updated_at).to_std().unwrap_or_default())
        } else {
            task.value.remaining
        };

        let progress = if task.value.duration.as_secs() == 0 {
            0
        } else {
            (((task.value.duration - optimistic_remaining).as_secs_f32()
                / task.value.duration.as_secs_f32())
                * (PROGRESS_BAR_LENGTH) as f32) as usize
        };

        builder.push_record([
            task.key.id.to_string(),
            task.key.state.to_string(),
            task.value.priority.to_string(),
            task.value
                .assignee
                .map_or_else(|| "none".into(), WorkerName::into_inner),
            format!(
                "{}.{:03}s",
                task.value.duration.as_secs(),
                task.value.duration.subsec_millis()
            ),
            format!(
                "{}.{:03}s",
                optimistic_remaining.as_secs(),
                optimistic_remaining.subsec_millis()
            ),
            format!(
                "{}{}",
                "█".repeat(progress),
                " ".repeat(PROGRESS_BAR_LENGTH - progress)
            ),
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::modern_rounded())
        .modify(Columns::last(), Color::FG_GREEN)
        .modify(Rows::first(), Color::FG_WHITE | Color::BOLD);
    println!("{table}");
}
