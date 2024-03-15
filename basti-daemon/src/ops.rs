use std::time::Duration;

use anyhow::{anyhow, bail};
use chrono::Utc;
use etcd_client::{
    Compare, CompareOp, GetOptions, KvClient, SortOrder, SortTarget, Txn, TxnOp, TxnOpResponse,
};
use uuid::Uuid;

use basti_types::{PriorityKey, Task, TaskKey, TaskPriority, TaskState, WorkerName};

#[derive(Debug)]
pub struct Revision(i64);

pub async fn create_task(
    client: &mut KvClient,
    duration: Duration,
    priority: TaskPriority,
) -> anyhow::Result<Task> {
    let task = Task::generate(priority, duration);

    let txn = Txn::new().and_then([
        TxnOp::put(&task.key, bson::to_vec(&task)?, None),
        TxnOp::put(&PriorityKey::from(&task), [], None),
    ]);

    client.txn(txn).await?;

    Ok(task)
}

pub async fn list_priorities(
    client: &mut KvClient,
    limit: i64,
) -> anyhow::Result<Vec<PriorityKey>> {
    let response = client
        .get(
            [PriorityKey::PREFIX],
            Some(
                GetOptions::default()
                    .with_limit(limit)
                    .with_sort(SortTarget::Key, SortOrder::Ascend)
                    .with_prefix(),
            ),
        )
        .await?;

    let mut priorities = Vec::new();
    for kv in response.kvs() {
        priorities.push(PriorityKey::try_from(kv.key())?);
    }

    Ok(priorities)
}

pub async fn list_tasks(
    client: &mut KvClient,
    state: Option<TaskState>,
    limit: i64,
) -> anyhow::Result<Vec<(Task, Revision)>> {
    let key = match state {
        None => vec![TaskKey::PREFIX],
        Some(state) => vec![TaskKey::PREFIX, state.into()],
    };

    let response = client
        .get(
            key,
            Some(GetOptions::default().with_limit(limit).with_prefix()),
        )
        .await?;

    let mut tasks = Vec::new();
    for kv in response.kvs() {
        tasks.push((
            Task {
                key: TaskKey::try_from(kv.key())?,
                value: bson::from_slice(kv.value())?,
            },
            Revision(kv.mod_revision()),
        ));
    }

    Ok(tasks)
}

pub async fn find_task(
    client: &mut KvClient,
    id: Uuid,
    try_states: &[TaskState],
) -> anyhow::Result<Option<(Task, Revision)>> {
    let txn = Txn::new().and_then(
        try_states
            .iter()
            .map(|state| TxnOp::get(&TaskKey::new(*state, id), None))
            .collect::<Vec<_>>(),
    );

    let response = client.txn(txn).await?;

    let kvs = response
        .op_responses()
        .into_iter()
        .flat_map(|op_response| match op_response {
            TxnOpResponse::Get(mut get_response) => get_response.take_kvs(),
            _ => vec![],
        })
        .collect::<Vec<_>>();

    let kv = match kvs.as_slice() {
        [] => return Ok(None),
        [kv] => kv,
        _ => bail!("inconsistent database, task exists with several states"),
    };

    let task = Task {
        key: TaskKey::try_from(kv.key())?,
        value: bson::from_slice(kv.value())?,
    };

    Ok(Some((task, Revision(kv.mod_revision()))))
}

async fn update_task_with_revision(
    client: &mut KvClient,
    revision: Revision,
    old_key: &TaskKey,
    new_key: &TaskKey,
    mut operations: Vec<TxnOp>,
) -> anyhow::Result<Option<Revision>> {
    operations.push(TxnOp::get(new_key, None));

    let txn = Txn::new()
        .when([Compare::mod_revision(old_key, CompareOp::Equal, revision.0)])
        .and_then(operations);

    let response = client.txn(txn).await.map_err(anyhow::Error::from)?;

    if !response.succeeded() {
        return Ok(None);
    }

    let op_responses = response.op_responses();
    let Some(TxnOpResponse::Get(get_response)) = op_responses.last() else {
        return Err(anyhow!(
            "last op-response in transaction was not the expected get response"
        ));
    };

    let Some(kv) = get_response.kvs().first() else {
        return Err(anyhow!("get response has no kv pair"));
    };

    Ok(Some(Revision(kv.mod_revision())))
}

pub async fn requeue_task(
    client: &mut KvClient,
    mut task: Task,
    revision: Revision,
) -> anyhow::Result<Option<(Task, Revision)>> {
    let initial_key = task.key;

    task.key.state = TaskState::Queued;
    task.value.updated_at = Utc::now();
    task.value.assignee = None;

    Ok(update_task_with_revision(
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
    .await?
    .map(|revision| (task, revision)))
}

pub async fn acquire_task(
    client: &mut KvClient,
    mut task: Task,
    revision: Revision,
    name: WorkerName,
) -> anyhow::Result<Option<(Task, Revision)>> {
    let initial_key = task.key;

    task.key.state = TaskState::Running;
    task.value.assignee = Some(name);
    task.value.updated_at = Utc::now();

    Ok(update_task_with_revision(
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
    .await?
    .map(|revision| (task, revision)))
}

pub async fn progress_task(
    client: &mut KvClient,
    mut task: Task,
    revision: Revision,
    progress: Duration,
) -> anyhow::Result<Option<(Task, Revision)>> {
    task.value.remaining -= progress;
    task.value.updated_at = Utc::now();

    Ok(update_task_with_revision(
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
    .await?
    .map(|revision| (task, revision)))
}

pub async fn cancel_task(client: &mut KvClient, id: Uuid) -> anyhow::Result<Option<Task>> {
    let Some((task, _)) = find_task(client, id, &TaskState::VARIANTS).await? else {
        return Ok(None);
    };

    let operations = TaskState::VARIANTS
        .iter()
        .map(|state| TxnOp::delete(&TaskKey::new(*state, id), None))
        .collect::<Vec<_>>();

    let txn = Txn::new().and_then(operations);
    client.txn(txn).await?;

    Ok(Some(task))
}

pub async fn finish_task(
    client: &mut KvClient,
    key: &TaskKey,
    revision: Revision,
) -> anyhow::Result<Option<()>> {
    let txn = Txn::new()
        .when([Compare::mod_revision(
            &key.clone(),
            CompareOp::Equal,
            revision.0,
        )])
        .and_then([TxnOp::delete(key, None)]);

    if !client
        .txn(txn)
        .await
        .map_err(anyhow::Error::from)?
        .succeeded()
    {
        return Ok(None);
    }

    Ok(Some(()))
}
