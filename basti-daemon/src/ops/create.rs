use anyhow::Result;
use basti_task::{Task, TaskPriority};
use etcd_client::KvClient;
use std::time::Duration;

pub async fn create_task(
    client: &mut KvClient,
    duration: Duration,
    priority: TaskPriority,
) -> Result<Task> {
    let task = Task::generate(priority, duration);

    client
        .put(task.key.to_string(), serde_json::to_vec(&task)?, None)
        .await?;

    Ok(task)
}
