use super::find_task;
use anyhow::Result;
use basti_task::{Task, TaskKey, TaskState};
use etcd_client::{KvClient, Txn, TxnOp};
use strum::IntoEnumIterator;
use uuid::Uuid;

pub async fn cancel_task(client: &mut KvClient, id: Uuid) -> Result<Option<Task>> {
    let Some(task) = find_task(client, id).await? else {
        return Ok(None);
    };

    let operations = TaskState::iter()
        .map(|state| TxnOp::delete(TaskKey { state, id }.to_string(), None))
        .collect::<Vec<_>>();

    let txn = Txn::new().and_then(operations);
    client.txn(txn).await?;

    Ok(Some(task))
}

pub async fn finish_task(client: &mut KvClient, id: Uuid) -> Result<()> {
    unimplemented!()
}
