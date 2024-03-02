use anyhow::{bail, Result};
use basti_common::task::{CreateTaskPayload, Task, TaskState};
use reqwest::Method;
use std::time::Duration;
use url::Url;

const TASKS_ENDPOINT: &str = "/api/tasks";

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

    pub async fn list(&self, state: Option<TaskState>) -> Result<Vec<Task>> {
        for endpoint in &self.endpoints {
            let mut endpoint = endpoint.clone();
            endpoint.set_path(TASKS_ENDPOINT);

            if let Some(ref state) = state {
                endpoint
                    .query_pairs_mut()
                    .append_pair("state", &state.to_string());
            }

            let request = self.http_client.request(Method::GET, endpoint).build()?;
            let Ok(response) = self.http_client.execute(request).await else {
                continue;
            };

            if !response.status().is_success() {
                bail!("{}", response.text().await?)
            }

            return Ok(response.json().await?);
        }

        bail!("All API endpoints are dead.")
    }

    pub async fn submit(&self, duration: Duration, priority: u32) -> Result<Task> {
        let payload = CreateTaskPayload { duration, priority };

        for endpoint in &self.endpoints {
            let mut endpoint = endpoint.clone();
            endpoint.set_path(TASKS_ENDPOINT);

            let request = self
                .http_client
                .request(Method::POST, endpoint)
                .json(&payload)
                .build()?;

            let Ok(response) = self.http_client.execute(request).await else {
                continue;
            };

            if !response.status().is_success() {
                bail!("{}", response.text().await?)
            }

            return Ok(response.json().await?);
        }

        bail!("All API endpoints are dead.")
    }
}
