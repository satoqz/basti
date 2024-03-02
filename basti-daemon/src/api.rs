use axum::{
    extract::{Json, Query, Request, State},
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use basti_common::task::{CreateTaskPayload, Task, TaskKey, TaskState};
use etcd_client::{Client, GetOptions, SortTarget};
use serde::Deserialize;
use std::{net::SocketAddr, str::FromStr};

enum Error {
    SerdeJson(serde_json::Error),
    Etcd(etcd_client::Error),
    TaskData(basti_common::task::Error),
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value)
    }
}

impl From<etcd_client::Error> for Error {
    fn from(value: etcd_client::Error) -> Self {
        Self::Etcd(value)
    }
}

impl From<basti_common::task::Error> for Error {
    fn from(value: basti_common::task::Error) -> Self {
        Self::TaskData(value)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::SerdeJson(err) => tracing::warn!("serde json error: {err}"),
            Self::Etcd(err) => tracing::error!("etcd error: {err}"),
            Self::TaskData(err) => tracing::warn!("task data error: {err}"),
        }

        (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    }
}

type HandlerResult<T> = Result<T, Error>;

pub async fn run(addr: SocketAddr, name: &str, etcd: Client) {
    let name_header = HeaderValue::from_str(&name.to_ascii_lowercase()).unwrap();
    let app = Router::new()
        .route("/api/tasks", get(list_tasks))
        .route("/api/tasks", post(create_task))
        .with_state(etcd)
        .layer(middleware::from_fn(move |req: Request, next: Next| {
            let name_header = name_header.clone();
            async move {
                let mut res = next.run(req).await;
                res.headers_mut().insert("x-served-by", name_header);
                res
            }
        }));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("listening at {addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn create_task(
    State(mut etcd): State<Client>,
    Json(payload): Json<CreateTaskPayload>,
) -> HandlerResult<(StatusCode, Json<Task>)> {
    let task = Task::generate(payload.priority, payload.duration);

    etcd.put(task.key.to_string(), serde_json::to_string(&task)?, None)
        .await?;
    tracing::info!("created task {}", task.key.id);

    Ok((StatusCode::CREATED, Json(task)))
}

#[derive(Debug, Deserialize)]
struct ListTasksParams {
    #[serde(rename = "type")]
    task_type: Option<TaskState>,
}

async fn list_tasks(
    State(mut etcd): State<Client>,
    Query(params): Query<ListTasksParams>,
) -> HandlerResult<(StatusCode, Json<Vec<Task>>)> {
    let prefix = match params.task_type {
        None => "task_".into(),
        Some(task_type) => format!("task_{task_type}_"),
    };

    let response = etcd
        .get(
            prefix,
            Some(
                GetOptions::new()
                    .with_prefix()
                    .with_sort(SortTarget::Mod, etcd_client::SortOrder::Ascend)
                    .with_limit(100),
            ),
        )
        .await?;

    let mut tasks = Vec::new();
    for kv in response.kvs() {
        tasks.push(Task {
            key: TaskKey::from_str(kv.key_str()?)?,
            details: serde_json::from_str(kv.value_str()?)?,
        });
    }

    Ok((StatusCode::OK, Json(tasks)))
}
