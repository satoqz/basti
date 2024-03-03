use super::revision_based::update_task_with_transaction;
use anyhow::Result;
use basti_common::task::{Task, TaskKey, TaskState};
use etcd_client::{Client, GetOptions, TxnOp};
use std::{str::FromStr, time::Duration};

pub async fn list_tasks(
    client: &mut Client,
    state: Option<TaskState>,
    options: Option<GetOptions>,
) -> Result<Vec<(Task, i64)>> {
    let key = match state {
        None => TaskKey::prefix().to_string(),
        Some(state) => format!("{}_{}", TaskKey::prefix(), state),
    };

    let response = client
        .get(
            key,
            Some(options.unwrap_or(GetOptions::default()).with_prefix()),
        )
        .await?;

    let mut tasks = Vec::new();
    for kv in response.kvs() {
        tasks.push((
            Task {
                key: TaskKey::from_str(kv.key_str()?)?,
                value: serde_json::from_str(kv.value_str()?)?,
            },
            kv.mod_revision(),
        ));
    }

    Ok(tasks)
}

pub async fn create_task(
    client: &mut Client,
    duration: Duration,
    priority: u8,
) -> Result<(Task, i64)> {
    let task = Task::new(priority, duration);

    let revision = update_task_with_transaction(
        client,
        &task,
        &task.key,
        None,
        [TxnOp::put(
            task.key.to_string(),
            serde_json::to_vec(&task)?,
            None,
        )],
    )
    .await?;

    Ok((task, revision))
}
