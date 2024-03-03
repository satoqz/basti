use basti_common::task::Task;
use tabled::{
    builder::Builder,
    settings::{
        object::{Columns, Rows},
        Color, Style,
    },
};

pub fn print_task_table(mut tasks: Vec<Task>) {
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
}
