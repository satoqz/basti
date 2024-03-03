use basti_task::Task;
use etcd_client::KvClient;

pub async fn acquire_task(
    client: &mut KvClient,
    mut task: Task,
    node_name: String,
) -> anyhow::Result<(Task, i64)> {
    unimplemented!()
}

pub async fn progress_task(client: &mut KvClient) -> anyhow::Result<()> {
    unimplemented!()
}

pub async fn requeue_task(client: &mut KvClient, mut task: Task) -> anyhow::Result<()> {
    unimplemented!()
}
