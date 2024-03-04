use basti_task::{PriorityKey, Task, TaskPriority};
use etcd_client::{KvClient, Txn, TxnOp};
use std::time::Duration;

pub async fn create_task(
    client: &mut KvClient,
    duration: Duration,
    priority: TaskPriority,
) -> anyhow::Result<Task> {
    let task = Task::generate(priority, duration);

    let txn = Txn::new().and_then([
        TxnOp::put(task.key.to_string(), bson::to_vec(&task)?, None),
        TxnOp::put(PriorityKey::from(&task).to_string(), "", None),
    ]);

    client.txn(txn).await?;

    Ok(task)
}
