use anyhow::bail;
use basti_task::{CreateTask, Task, TaskPriority, TaskState};
use reqwest::{Method, RequestBuilder};
use serde::de::DeserializeOwned;
use std::time::Duration;
use url::Url;
use uuid::Uuid;

#[derive(Debug)]
pub struct BastiClient {
    endpoints: Vec<Url>,
    http_client: reqwest::Client,
}

impl BastiClient {
    pub fn new(endpoints: Vec<Url>) -> anyhow::Result<Self> {
        Ok(Self {
            endpoints,
            http_client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(3))
                .timeout(Duration::from_secs(3))
                .build()?,
        })
    }

    async fn execute<T, F>(&self, make_request: F) -> anyhow::Result<T>
    where
        T: DeserializeOwned,
        F: Fn(Url) -> RequestBuilder,
    {
        for url in &self.endpoints {
            let request = make_request(url.clone()).build()?;

            let Ok(response) = self.http_client.execute(request).await else {
                continue;
            };

            if !response.status().is_success() {
                bail!("{}", response.text().await?)
            }

            return Ok(response.json().await?);
        }

        bail!("All API endpoints are dead");
    }

    pub async fn submit(&self, duration: Duration, priority: TaskPriority) -> anyhow::Result<Task> {
        let payload = CreateTask { duration, priority };
        self.execute(|mut url| {
            url.set_path("/api/tasks");
            self.http_client.request(Method::POST, url).json(&payload)
        })
        .await
    }

    pub async fn list(
        &self,
        state: Option<TaskState>,
        limit: Option<u32>,
    ) -> anyhow::Result<Vec<Task>> {
        self.execute(|mut url| {
            url.set_path("/api/tasks");

            if let Some(state) = &state {
                url.query_pairs_mut()
                    .append_pair("state", &state.to_string());
            }

            if let Some(limit) = &limit {
                url.query_pairs_mut()
                    .append_pair("limit", &limit.to_string());
            }

            self.http_client.request(Method::GET, url)
        })
        .await
    }

    pub async fn find(&self, id: Uuid) -> anyhow::Result<Task> {
        let path = format!("/api/tasks/{id}");
        self.execute::<Task, _>(|mut url| {
            url.set_path(&path);
            self.http_client.request(Method::GET, url)
        })
        .await
    }

    pub async fn cancel(&self, id: Uuid) -> anyhow::Result<Task> {
        let path = format!("/api/tasks/{id}");
        self.execute::<Task, _>(|mut url| {
            url.set_path(&path);
            self.http_client.request(Method::DELETE, url)
        })
        .await
    }
}
