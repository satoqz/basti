use super::{errors::MaybeRevisionError, Revision};
use anyhow::anyhow;
use basti_task::{PriorityKey, Task, TaskKey, TaskState};
use chrono::Utc;
use etcd_client::{Compare, CompareOp, KvClient, Txn, TxnOp, TxnOpResponse};
use std::time::Duration;

async fn update_task_with_revision(
    client: &mut KvClient,
    revision: Revision,
    old_key: &TaskKey,
    new_key: &TaskKey,
    mut operations: Vec<TxnOp>,
) -> Result<Revision, MaybeRevisionError> {
    operations.push(TxnOp::get(new_key, None));

    let txn = Txn::new()
        .when([Compare::mod_revision(old_key, CompareOp::Equal, revision.0)])
        .and_then(operations);

    let response = client.txn(txn).await.map_err(anyhow::Error::from)?;

    if !response.succeeded() {
        return Err(MaybeRevisionError::BadRevision);
    }

    let op_responses = response.op_responses();
    let Some(TxnOpResponse::Get(get_response)) = op_responses.last() else {
        return Err(
            anyhow!("Last op-response in transaction was not the expected get response").into(),
        );
    };

    let Some(kv) = get_response.kvs().first() else {
        return Err(anyhow!("Get response has no kv pair").into());
    };

    Ok(Revision(kv.mod_revision()))
}

pub async fn requeue_task(
    client: &mut KvClient,
    mut task: Task,
    revision: Revision,
) -> Result<(Task, Revision), MaybeRevisionError> {
    let initial_key = task.key;

    task.key.state = TaskState::Queued;
    task.value.updated_at = Utc::now();
    task.value.assignee = None;

    let revision = update_task_with_revision(
        client,
        revision,
        &initial_key,
        &task.key,
        vec![
            TxnOp::delete(&initial_key, None),
            TxnOp::put(
                &task.key,
                bson::to_vec(&task).map_err(anyhow::Error::from)?,
                None,
            ),
            TxnOp::put(&PriorityKey::from(&task), "", None),
        ],
    )
    .await?;

    Ok((task, revision))
}

pub async fn acquire_task(
    client: &mut KvClient,
    mut task: Task,
    revision: Revision,
    node_name: String,
) -> Result<(Task, Revision), MaybeRevisionError> {
    let initial_key = task.key;

    task.key.state = TaskState::Running;
    task.value.assignee = Some(node_name);
    task.value.updated_at = Utc::now();

    let revision = update_task_with_revision(
        client,
        revision,
        &initial_key,
        &task.key,
        vec![
            TxnOp::delete(&initial_key, None),
            TxnOp::delete(&PriorityKey::from(&task), None),
            TxnOp::put(
                &task.key,
                bson::to_vec(&task).map_err(anyhow::Error::from)?,
                None,
            ),
        ],
    )
    .await?;

    Ok((task, revision))
}

pub async fn progress_task(
    client: &mut KvClient,
    mut task: Task,
    revision: Revision,
    progress: Duration,
) -> Result<(Task, Revision), MaybeRevisionError> {
    task.value.remaining -= progress;
    task.value.updated_at = Utc::now();

    let revision = update_task_with_revision(
        client,
        revision,
        &task.key,
        &task.key,
        vec![TxnOp::put(
            &task.key,
            bson::to_vec(&task).map_err(anyhow::Error::from)?,
            None,
        )],
    )
    .await?;

    Ok((task, revision))
}
