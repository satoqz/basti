use etcd_client::{Compare, CompareOp, KvClient, Txn, TxnOp};
use uuid::Uuid;

use basti_types::{Task, TaskKey, TaskState};

use super::{find_task, MaybeRevisionError, Revision};

pub async fn cancel_task(client: &mut KvClient, id: Uuid) -> anyhow::Result<Option<Task>> {
    let Some((task, _)) = find_task(client, id, TaskState::VARIANTS).await? else {
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
) -> Result<(), MaybeRevisionError> {
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
        return Err(MaybeRevisionError::BadRevision);
    }

    Ok(())
}
