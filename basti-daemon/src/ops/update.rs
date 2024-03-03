use anyhow::Result;
use etcd_client::KvClient;

pub async fn acquire_task(client: &mut KvClient) -> Result<()> {
    unimplemented!()
}

pub async fn progress_task(client: &mut KvClient) -> Result<()> {
    unimplemented!()
}

pub async fn requeue_task(client: &mut KvClient) -> Result<()> {
    unimplemented!()
}
