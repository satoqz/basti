use super::{find_task, MaybeRevisionError, Revision};
use basti_task::{Task, TaskKey, TaskState};
use etcd_client::{Compare, CompareOp, KvClient, Txn, TxnOp};
use strum::VariantArray;
use uuid::Uuid;

pub async fn cancel_task(client: &mut KvClient, id: Uuid) -> anyhow::Result<Option<Task>> {
    let Some((task, _)) = find_task(client, id, TaskState::VARIANTS).await? else {
        return Ok(None);
    };

    let operations = TaskState::VARIANTS
        .into_iter()
        .map(|state| TxnOp::delete(TaskKey { state: *state, id }.to_string(), None))
        .collect::<Vec<_>>();

    let txn = Txn::new().and_then(operations);
    client.txn(txn).await?;

    Ok(Some(task))
}

pub async fn finish_task(
    client: &mut KvClient,
    key: &TaskKey,
    revision: Revision,
) -> Result<(), MaybeRevisionError> {
    let txn = Txn::new()
        .when([Compare::mod_revision(
            key.to_string(),
            CompareOp::Equal,
            revision.0,
        )])
        .and_then([TxnOp::delete(key.to_string(), None)]);

    if !client
        .txn(txn)
        .await
        .map_err(anyhow::Error::from)?
        .succeeded()
    {
        return Err(MaybeRevisionError::BadRevision);
    }

    Ok(())
}
