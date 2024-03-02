use anyhow::{bail, Result};
use basti_common::task::{Task, TaskKey, TaskState};
use chrono::Utc;
use etcd_client::{Client, Compare, CompareOp, GetOptions, Txn, TxnOp};
use std::{str::FromStr, time::Duration};

pub async fn create_task(client: &mut Client, duration: Duration, priority: u32) -> Result<Task> {
    let task = Task::generate(priority, duration);
    client
        .put(task.key.to_string(), serde_json::to_vec(&task)?, None)
        .await?;

    Ok(task)
}

pub async fn list_tasks(
    client: &mut Client,
    state: Option<TaskState>,
    options: Option<GetOptions>,
) -> Result<Vec<Task>> {
    let key = match state {
        None => "task_".into(),
        Some(state) => format!("task_{state}_"),
    };

    let response = client
        .get(
            key,
            Some(options.unwrap_or(GetOptions::default()).with_prefix()),
        )
        .await?;

    let mut tasks = Vec::new();
    for kv in response.kvs() {
        kv.mod_revision();
        tasks.push(Task {
            key: TaskKey::from_str(kv.key_str()?)?,
            details: serde_json::from_str(kv.value_str()?)?,
        });
    }

    Ok(tasks)
}

pub async fn acquire_task(client: &mut Client, key: &TaskKey, node_name: String) -> Result<Task> {
    match key.state {
        TaskState::Queued => {}
        _ => bail!("Cannot acquire task that is not queued."),
    }

    let (mut task, revision) = {
        let response = client.get(key.to_string(), None).await?;

        let kv = match response.kvs() {
            [] => bail!("No queued task found."),
            [kv] => kv,
            _ => bail!("Multiple tasks found for key."),
        };

        let task = Task {
            key: TaskKey::from_str(kv.key_str()?)?,
            details: serde_json::from_str(kv.value_str()?)?,
        };

        (task, kv.mod_revision())
    };

    task.key.state = TaskState::Running;
    task.details.assignee = Some(node_name);
    task.details.last_update = Utc::now();

    let txn = Txn::new()
        .when([Compare::mod_revision(
            key.to_string(),
            CompareOp::Equal,
            revision,
        )])
        .and_then([
            TxnOp::delete(key.to_string(), None),
            TxnOp::put(task.key.to_string(), serde_json::to_vec(&task)?, None),
        ]);

    if !client.txn(txn).await?.succeeded() {
        bail!("Transaction failed.")
    }

    Ok(task)
}
