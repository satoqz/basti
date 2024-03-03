use anyhow::{bail, Result};
use basti_common::task::{Task, TaskKey, TaskState};
use chrono::Utc;
use clap::ValueEnum;
use etcd_client::{Client, Compare, CompareOp, GetOptions, Txn, TxnOp, TxnOpResponse};
use std::{str::FromStr, time::Duration};
use uuid::Uuid;

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
) -> Result<Vec<(Task, i64)>> {
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
        tasks.push((
            Task {
                key: TaskKey::from_str(kv.key_str()?)?,
                details: serde_json::from_str(kv.value_str()?)?,
            },
            kv.mod_revision(),
        ));
    }

    Ok(tasks)
}

pub async fn find_task_by_id(client: &mut Client, id: Uuid) -> Result<Option<Task>> {
    let txn = Txn::new().and_then(
        TaskState::value_variants()
            .into_iter()
            .map(|state| TxnOp::get(format!("task_{state}_{id}"), None))
            .collect::<Vec<_>>(),
    );

    let response = client.txn(txn).await?;
    if !response.succeeded() {
        bail!("Transaction failed")
    }

    let maybe_kv = response
        .op_responses()
        .into_iter()
        .flat_map(|op_response| match op_response {
            TxnOpResponse::Get(mut get_response) => get_response.take_kvs(),
            _ => vec![],
        })
        .next();

    let Some(kv) = maybe_kv else { return Ok(None) };
    let task = Task {
        key: TaskKey::from_str(kv.key_str()?)?,
        details: serde_json::from_str(kv.value_str()?)?,
    };

    Ok(Some(task))
}

async fn update_task_with_transaction<V>(
    client: &mut Client,
    task: &Task,
    revision: i64,
    initial_key: &TaskKey,
    operations: V,
) -> Result<i64>
where
    V: Into<Vec<TxnOp>>,
{
    let mut operations: Vec<TxnOp> = operations.into();
    operations.push(TxnOp::get(task.key.to_string(), None));

    let txn = Txn::new()
        .when([Compare::mod_revision(
            initial_key.to_string(),
            CompareOp::Equal,
            revision,
        )])
        .and_then(operations);

    let response = client.txn(txn).await?;
    if !response.succeeded() {
        bail!("Transaction failed")
    }

    let op_responses = response.op_responses();
    let Some(TxnOpResponse::Get(get_response)) = op_responses.last() else {
        bail!("Last op-response in transaction was not the expected get response")
    };

    let Some(kv) = get_response.kvs().first() else {
        bail!("Get response has no kv pair")
    };

    Ok(kv.mod_revision())
}

pub async fn acquire_task(
    client: &mut Client,
    mut task: Task,
    revision: i64,
    node_name: String,
) -> Result<(Task, i64)> {
    let initial_key = task.key.clone();

    task.key.state = TaskState::Running;
    task.details.assignee = Some(node_name);
    task.details.last_update = Utc::now();

    let revision = update_task_with_transaction(
        client,
        &task,
        revision,
        &initial_key,
        [
            TxnOp::delete(initial_key.to_string(), None),
            TxnOp::put(
                task.key.to_string(),
                serde_json::to_vec(&task.details)?,
                None,
            ),
        ],
    )
    .await?;

    Ok((task, revision))
}

pub async fn progress_task(
    client: &mut Client,
    mut task: Task,
    revision: i64,
    progress: Duration,
) -> Result<(Task, i64)> {
    task.details.remaining -= progress;
    task.details.last_update = Utc::now();

    let revision = update_task_with_transaction(
        client,
        &task,
        revision,
        &task.key,
        [TxnOp::put(
            task.key.to_string(),
            serde_json::to_vec(&task.details)?,
            None,
        )],
    )
    .await?;

    Ok((task, revision))
}

pub async fn requeue_task(
    client: &mut Client,
    mut task: Task,
    revision: i64,
) -> Result<(Task, i64)> {
    let initial_key = task.key.clone();

    task.key.state = TaskState::Queued;
    task.details.assignee = None;
    task.details.last_update = Utc::now();

    let revision = update_task_with_transaction(
        client,
        &task,
        revision,
        &initial_key,
        [
            TxnOp::delete(initial_key.to_string(), None),
            TxnOp::put(
                task.key.to_string(),
                serde_json::to_vec(&task.details)?,
                None,
            ),
        ],
    )
    .await?;

    Ok((task, revision))
}

pub async fn finish_task(client: &mut Client, task: &Task, revision: i64) -> Result<()> {
    let txn = Txn::new()
        .when([Compare::mod_revision(
            task.key.to_string(),
            CompareOp::Equal,
            revision,
        )])
        .and_then([TxnOp::delete(task.key.to_string(), None)]);

    if !client.txn(txn).await?.succeeded() {
        bail!("Transaction failed")
    }

    Ok(())
}
