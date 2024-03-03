use anyhow::{bail, Result};
use basti_task::{Task, TaskKey, TaskState};
use etcd_client::{KvClient, Txn, TxnOp, TxnOpResponse};
use std::str::FromStr;
use uuid::Uuid;

async fn find_pointer(client: &mut KvClient, id: Uuid) -> Result<Option<(TaskKey, i64)>> {
    let response = client.get(PointerKey(id).to_string(), None).await?;

    let Some(kv) = response.kvs().first() else {
        return Ok(None);
    };

    Ok(Some((
        TaskKey::from_str(kv.value_str()?)?,
        kv.mod_revision(),
    )))
}

pub async fn find_task(client: &mut KvClient, id: Uuid) -> Result<Option<Task>> {
    let txn = Txn::new().and_then(
        TaskState::value_variants()
            .iter()
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
        value: serde_json::from_str(kv.value_str()?)?,
    };

    Ok(Some(task))
}

pub async fn cancel_task(client: &mut KvClient, id: Uuid) -> Result<Option<Task>> {
    let Some((_, revision)) = find_pointer(client, id).await? else {
        return Ok(None);
    };

    unimplemented!()
    // let Some(task) = find_task(client, id).await? else {
    //     return Ok(None);
    // };

    // let txn = Txn::new().and_then(
    //     TaskState::value_variants()
    //         .into_iter()
    //         .map(|state| TxnOp::delete(format!("task_{state}_{id}"), None))
    //         .collect::<Vec<_>>(),
    // );

    // let response = client.txn(txn).await?;
    // if !response.succeeded() {
    //     bail!("Transaction failed")
    // }

    // Ok(Some(task))
}
