use super::revision_based::try_update_task;
use anyhow::Result;
use basti_task::{Task, TaskKey, TaskPriority, TaskState};
use etcd_client::{GetOptions, KvClient, TxnOp};
use std::{str::FromStr, time::Duration};

pub async fn list_tasks(
    client: &mut KvClient,
    state: Option<TaskState>,
    options: Option<GetOptions>,
) -> Result<Vec<(Task, i64)>> {
    let key = match state {
        None => TaskKey::prefix().to_string(),
        Some(state) => format!("{}_{}", TaskKey::prefix(), state),
    };

    let response = client
        .get(key, Some(options.unwrap_or_default().with_prefix()))
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
    client: &mut KvClient,
    duration: Duration,
    priority: TaskPriority,
) -> Result<(Task, i64)> {
    let task = Task::generate(priority, duration);

    let revision = try_update_task(
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
