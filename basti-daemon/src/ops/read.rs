use anyhow::bail;
use basti_types::{PriorityKey, Task, TaskKey, TaskState};
use etcd_client::{GetOptions, KvClient, SortOrder, SortTarget, Txn, TxnOp, TxnOpResponse};
use uuid::Uuid;

use super::Revision;

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
        priorities.push(PriorityKey::try_from(kv.key())?)
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

pub async fn find_task<S>(
    client: &mut KvClient,
    id: Uuid,
    try_states: S,
) -> anyhow::Result<Option<(Task, Revision)>>
where
    S: IntoIterator<Item = TaskState>,
{
    let txn = Txn::new().and_then(
        try_states
            .into_iter()
            .map(|state| TxnOp::get(&TaskKey::new(state, id), None))
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
        _ => bail!("Inconsistent database, task exists with several states"),
    };

    let task = Task {
        key: TaskKey::try_from(kv.key())?,
        value: bson::from_slice(kv.value())?,
    };

    Ok(Some((task, Revision(kv.mod_revision()))))
}
