use basti_common::task::{CreateTaskPayload, Task, TaskState};
use reqwest::Method;
use std::{fmt::Display, time::Duration};
use url::Url;

const TASKS_ENDPOINT: &str = "/api/tasks";

#[derive(Debug)]
pub struct BastiClient {
    endpoints: Vec<Url>,
    http_client: reqwest::Client,
}

#[derive(Debug)]
pub enum Error {
    Death,
    Api(String),
    Reqwest(reqwest::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Death => write!(f, "All API endpoints are dead."),
            Self::Api(message) => write!(f, "API Error: {message}"),
            Self::Reqwest(error) => write!(f, "Request Error: {error}"),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::Reqwest(value)
    }
}

impl BastiClient {
    pub fn new(endpoints: Vec<Url>) -> Self {
        Self {
            endpoints,
            http_client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(2))
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap(),
        }
    }

    pub async fn list(&self, filter: Option<TaskState>) -> Result<Vec<Task>, Error> {
        for endpoint in &self.endpoints {
            let mut endpoint = endpoint.clone();
            endpoint.set_path(TASKS_ENDPOINT);

            if let Some(ref filter) = filter {
                endpoint
                    .query_pairs_mut()
                    .append_pair("type", &filter.to_string());
            }

            let request = self.http_client.request(Method::GET, endpoint).build()?;
            let Ok(response) = self.http_client.execute(request).await else {
                continue;
            };

            if !response.status().is_success() {
                let error = response.text().await?;
                return Err(Error::Api(error));
            }

            return Ok(response.json().await?);
        }

        Err(Error::Death)
    }

    pub async fn submit(&self, duration: Duration, priority: u32) -> Result<Task, Error> {
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
                let error = response.text().await?;
                return Err(Error::Api(error));
            }

            return Ok(response.json().await?);
        }

        Err(Error::Death)
    }
}
