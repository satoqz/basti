use std::time::Duration;

use anyhow::{bail, Result};
use basti_task::{Task, TaskKey, TaskState};
use chrono::Utc;
use etcd_client::{Compare, CompareOp, KvClient, Txn, TxnOp, TxnOpResponse};

pub async fn try_update_task<V>(
    client: &mut KvClient,
    task: &Task,
    initial_key: &TaskKey,
    revision: Option<i64>,
    operations: V,
) -> Result<i64>
where
    V: Into<Vec<TxnOp>>,
{
    let mut txn = Txn::new();

    if let Some(revision) = revision {
        txn = txn.when([Compare::mod_revision(
            initial_key.to_string(),
            CompareOp::Equal,
            revision,
        )]);
    }

    let mut operations = operations.into();
    operations.push(TxnOp::put(
        PointerKey(task.key.id).to_string(),
        task.key.to_string(),
        None,
    ));
    operations.push(TxnOp::get(task.key.to_string(), None));
    txn = txn.and_then(operations);

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

pub async fn try_acquire_task(
    client: &mut KvClient,
    mut task: Task,
    revision: i64,
    node_name: String,
) -> Result<(Task, i64)> {
    let initial_key = task.key.clone();

    task.key.state = TaskState::Running;
    task.value.assignee = Some(node_name);
    task.value.last_update = Utc::now();

    let revision = try_update_task(
        client,
        &task,
        &initial_key,
        Some(revision),
        [
            TxnOp::delete(initial_key.to_string(), None),
            TxnOp::put(task.key.to_string(), serde_json::to_vec(&task.value)?, None),
        ],
    )
    .await?;

    Ok((task, revision))
}

pub async fn try_progress_task(
    client: &mut KvClient,
    mut task: Task,
    revision: i64,
    progress: Duration,
) -> Result<(Task, i64)> {
    task.value.remaining -= progress;
    task.value.last_update = Utc::now();

    let revision = try_update_task(
        client,
        &task,
        &task.key,
        Some(revision),
        [TxnOp::put(
            task.key.to_string(),
            serde_json::to_vec(&task.value)?,
            None,
        )],
    )
    .await?;

    Ok((task, revision))
}

pub async fn try_requeue_task(
    client: &mut KvClient,
    mut task: Task,
    revision: i64,
) -> Result<(Task, i64)> {
    let initial_key = task.key.clone();

    task.key.state = TaskState::Queued;
    task.value.assignee = None;
    task.value.last_update = Utc::now();

    let revision = try_update_task(
        client,
        &task,
        &initial_key,
        Some(revision),
        [
            TxnOp::delete(initial_key.to_string(), None),
            TxnOp::put(task.key.to_string(), serde_json::to_vec(&task.value)?, None),
        ],
    )
    .await?;

    Ok((task, revision))
}

pub async fn try_finish_task(client: &mut KvClient, task: &Task, revision: i64) -> Result<()> {
    let txn = Txn::new()
        .when([Compare::mod_revision(
            task.key.to_string(),
            CompareOp::Equal,
            revision,
        )])
        .and_then([
            TxnOp::delete(PointerKey(task.key.id).to_string(), None),
            TxnOp::delete(task.key.to_string(), None),
        ]);

    if !client.txn(txn).await?.succeeded() {
        bail!("Transaction failed")
    }

    Ok(())
}
