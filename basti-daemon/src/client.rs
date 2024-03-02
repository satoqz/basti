use anyhow::Result;
use basti_common::task::{Task, TaskKey, TaskState};
use etcd_client::{Client as EtcdClient, ConnectOptions, GetOptions};
use std::{
    ops::{Deref, DerefMut},
    str::FromStr,
    time::Duration,
};
use url::Url;

#[derive(Clone)]
pub struct Client {
    etcd: EtcdClient,
    endpoints: Vec<Url>,
}

impl Deref for Client {
    type Target = EtcdClient;
    fn deref(&self) -> &Self::Target {
        &self.etcd
    }
}

impl DerefMut for Client {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.etcd
    }
}

impl Client {
    pub async fn connect(endpoints: Vec<Url>) -> Result<Self> {
        let etcd = EtcdClient::connect(
            &endpoints,
            Some(
                ConnectOptions::default()
                    .with_connect_timeout(Duration::from_secs(2))
                    .with_timeout(Duration::from_secs(2)),
            ),
        )
        .await?;
        Ok(Self { etcd, endpoints })
    }

    pub async fn new_connection(&self) -> Result<Self> {
        Self::connect(self.endpoints.clone()).await
    }

    pub async fn create_task(&mut self, duration: Duration, priority: u32) -> Result<Task> {
        let task = Task::generate(priority, duration);
        self.put(task.key.to_string(), serde_json::to_string(&task)?, None)
            .await?;

        Ok(task)
    }

    pub async fn list_tasks(
        &mut self,
        state: Option<TaskState>,
        options: GetOptions,
    ) -> Result<Vec<(Task, i64)>> {
        let key = match state {
            None => "task_".into(),
            Some(state) => format!("task_{state}_"),
        };

        let response = self.get(key, Some(options.with_prefix())).await?;

        let mut tasks = Vec::new();
        for kv in response.kvs() {
            kv.mod_revision();
            tasks.push((
                Task {
                    key: TaskKey::from_str(kv.key_str()?)?,
                    details: serde_json::from_str(kv.value_str()?)?,
                },
                kv.mod_revision(),
            ));
        }

        Ok(tasks)
    }
}
