use basti_common::task::Task;
use tabled::{
    builder::Builder,
    settings::{
        object::{Columns, Rows},
        Color, Style,
    },
};

pub fn print_task_table(mut tasks: Vec<Task>) {
    tasks.sort_by(|a, b| a.value.cmp(&b.value));

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

    for task in tasks {
        let progress = if task.value.duration.as_secs() == 0 {
            0
        } else {
            (((task.value.duration - task.value.remaining).as_secs_f32()
                / task.value.duration.as_secs_f32())
                * 8_f32) as usize
        };

        builder.push_record([
            task.key.id.to_string(),
            task.key.state.to_string(),
            task.key.priority.to_string(),
            task.value.assignee.unwrap_or("none".into()),
            format!(
                "{}.{:03}s",
                task.value.duration.as_secs(),
                task.value.duration.subsec_millis()
            ),
            format!(
                "{}.{:03}s",
                task.value.remaining.as_secs(),
                task.value.duration.subsec_millis()
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
