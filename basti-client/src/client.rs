use anyhow::{bail, Result};
use basti_common::task::{CreateTaskPayload, Task, TaskState};
use reqwest::{Method, RequestBuilder};
use serde::de::DeserializeOwned;
use std::time::Duration;
use url::Url;

#[derive(Debug)]
pub struct BastiClient {
    endpoints: Vec<Url>,
    http_client: reqwest::Client,
}

impl BastiClient {
    pub fn new(endpoints: Vec<Url>) -> Result<Self> {
        Ok(Self {
            endpoints,
            http_client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(2))
                .timeout(Duration::from_secs(2))
                .build()?,
        })
    }

    async fn execute<T, F>(&self, make_request: F) -> Result<T>
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

    pub async fn list(&self, state: Option<TaskState>) -> Result<Vec<Task>> {
        self.execute(|mut url| {
            url.set_path("/api/tasks");

            if let Some(ref state) = state {
                url.query_pairs_mut()
                    .append_pair("state", &state.to_string());
            }

            self.http_client.request(Method::GET, url)
        })
        .await
    }

    pub async fn submit(&self, duration: Duration, priority: u32) -> Result<Task> {
        let payload = CreateTaskPayload { duration, priority };
        self.execute(|mut url| {
            url.set_path("/api/tasks");
            self.http_client.request(Method::POST, url).json(&payload)
        })
        .await
    }
}
