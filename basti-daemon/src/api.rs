use std::net::SocketAddr;

use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Router,
};
use etcd_client::KvClient;
use serde::Deserialize;
use tokio::signal;
use uuid::Uuid;

use basti_types::{CreateTask, Task, TaskState};

use crate::ops::{cancel_task, create_task, find_task, list_tasks};

pub async fn run(addr: SocketAddr, client: KvClient) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/api/tasks", post(create_task_endpoint))
        .route("/api/tasks", get(list_tasks_endpoint))
        .route("/api/tasks/:id", get(find_task_endpoint))
        .route("/api/tasks/:id", delete(cancel_task_endpoint))
        .with_state(client);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Listening at http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(anyhow::Error::from)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to listen for event");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[derive(Debug)]
pub struct Error(anyhow::Error);
pub type Result<T> = std::result::Result<T, Error>;

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal API error").into_response()
    }
}

#[tracing::instrument(skip(client), err(Debug))]
pub async fn create_task_endpoint(
    State(mut client): State<KvClient>,
    Json(payload): Json<CreateTask>,
) -> Result<(StatusCode, Json<Task>)> {
    let task = create_task(&mut client, payload.duration, payload.priority).await?;
    Ok((StatusCode::CREATED, Json(task)))
}

#[derive(Debug, Deserialize)]
pub struct ListTasksParams {
    state: Option<TaskState>,
    limit: Option<i64>,
}

#[tracing::instrument(skip(client), err(Debug))]
pub async fn list_tasks_endpoint(
    State(mut client): State<KvClient>,
    Query(params): Query<ListTasksParams>,
) -> Result<(StatusCode, Json<Vec<Task>>)> {
    let tasks = list_tasks(&mut client, params.state, params.limit.unwrap_or(50)).await?;
    Ok((
        StatusCode::OK,
        Json(tasks.into_iter().map(|(task, _)| task).collect()),
    ))
}

#[tracing::instrument(skip(client), err(Debug))]
pub async fn find_task_endpoint(
    State(mut client): State<KvClient>,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    Ok(
        match find_task(&mut client, id, &TaskState::VARIANTS).await? {
            Some((task, _)) => (StatusCode::OK, Json(task)).into_response(),
            None => (StatusCode::NOT_FOUND, "Task not found").into_response(),
        },
    )
}

#[tracing::instrument(skip(client), err(Debug))]
pub async fn cancel_task_endpoint(
    State(mut client): State<KvClient>,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    Ok(match cancel_task(&mut client, id).await? {
        Some(task) => (StatusCode::OK, Json(task)).into_response(),
        None => (StatusCode::NOT_FOUND, "Task not found").into_response(),
    })
}
