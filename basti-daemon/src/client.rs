use anyhow::{bail, Result};
use basti_common::task::{Task, TaskKey, TaskState};
use chrono::Utc;
use etcd_client::{
    Client as EtcdClient, Compare, CompareOp, ConnectOptions, GetOptions, Txn, TxnOp,
};
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
    name: String,
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
    #[tracing::instrument(err(Debug))]
    pub async fn connect(name: String, endpoints: Vec<Url>) -> Result<Self> {
        let etcd = EtcdClient::connect(
            &endpoints,
            Some(
                ConnectOptions::default()
                    .with_connect_timeout(Duration::from_secs(2))
                    .with_timeout(Duration::from_secs(2)),
            ),
        )
        .await?;

        Ok(Self {
            etcd,
            name,
            endpoints,
        })
    }

    #[tracing::instrument(skip(self), err(Debug))]
    pub async fn new_connection(&self) -> Result<Self> {
        Self::connect(self.name.clone(), self.endpoints.clone()).await
    }

    #[tracing::instrument(skip(self), err(Debug))]
    pub async fn create_task(&mut self, duration: Duration, priority: u32) -> Result<Task> {
        let task = Task::generate(priority, duration);
        self.put(task.key.to_string(), serde_json::to_vec(&task)?, None)
            .await?;

        Ok(task)
    }

    #[tracing::instrument(skip(self), err(Debug))]
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

    #[tracing::instrument(skip(self), err(Debug))]
    pub async fn acquire_task(&mut self, key: &TaskKey) -> Result<Task> {
        match key.state {
            TaskState::Queued => {}
            _ => bail!("Cannot acquire task that is not queued."),
        }

        let (mut task, revision) = {
            let response = self.get(key.to_string(), None).await?;

            let kv = match response.kvs() {
                [] => bail!("No queued task found."),
                [kv] => kv,
                _ => bail!("Multiple tasks found for key."),
            };

            let task = Task {
                key: TaskKey::from_str(kv.key_str()?)?,
                details: serde_json::from_str(kv.value_str()?)?,
            };

            (task, kv.mod_revision())
        };

        task.key.state = TaskState::Running;
        task.details.assignee = Some(self.name.clone());
        task.details.last_update = Utc::now();

        let txn = Txn::new()
            .when([Compare::mod_revision(
                key.to_string(),
                CompareOp::Equal,
                revision,
            )])
            .and_then([
                TxnOp::delete(key.to_string(), None),
                TxnOp::put(task.key.to_string(), serde_json::to_vec(&task)?, None),
            ]);

        if !self.txn(txn).await?.succeeded() {
            bail!("Transaction failed.")
        }

        Ok(task)
    }
}
