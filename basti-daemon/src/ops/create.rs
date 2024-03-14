use std::time::Duration;

use etcd_client::{KvClient, Txn, TxnOp};

use basti_types::{PriorityKey, Task, TaskPriority};

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
