use crate::{
    api_error::{ApiError, ApiErrorKind, ApiResult},
    ops::{cancel_task, create_task, find_task, list_tasks},
};
use anyhow::{anyhow, Context};
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Router,
};
use basti_task::{CreateTask, Task, TaskState};
use etcd_client::KvClient;
use serde::Deserialize;
use std::{fmt::Debug, net::SocketAddr};
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use uuid::Uuid;

#[tracing::instrument(skip_all)]
pub async fn run(addr: SocketAddr, client: KvClient) -> anyhow::Result<()> {
    let trace_layer = TraceLayer::new_for_http()
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO))
        .on_failure(DefaultOnFailure::new().level(Level::WARN));

    let app = Router::new()
        .route("/api/tasks", post(create_task_endpoint))
        .route("/api/tasks", get(list_tasks_endpoint))
        .route("/api/tasks/:id", get(find_task_endpoint))
        .route("/api/tasks/:id", delete(cancel_task_endpoint))
        .layer(trace_layer)
        .with_state(client);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind address")?;
    tracing::info!("Listening at http://{addr}");

    axum::serve(listener, app)
        .await
        .context("Failed to serve HTTP")
}

#[tracing::instrument(skip(client), err(Debug))]
async fn create_task_endpoint(
    State(mut client): State<KvClient>,
    Json(payload): Json<CreateTask>,
) -> ApiResult<Task> {
    let task = create_task(&mut client, payload.duration, payload.priority)
        .await
        .context("Failed to create task")?;

    Ok((StatusCode::CREATED, Json(task)))
}

#[derive(Debug, Deserialize)]
struct ListTasksParams {
    state: Option<TaskState>,
    limit: Option<i64>,
}

#[tracing::instrument(skip(client), err(Debug))]
async fn list_tasks_endpoint(
    State(mut client): State<KvClient>,
    Query(params): Query<ListTasksParams>,
) -> ApiResult<Vec<Task>> {
    let tasks = list_tasks(&mut client, params.state, params.limit.unwrap_or(50))
        .await
        .context("Failed to list tasks")?;

    Ok((
        StatusCode::OK,
        Json(tasks.into_iter().map(|(task, _)| task).collect()),
    ))
}

#[tracing::instrument(skip(client), err(Debug))]
async fn find_task_endpoint(
    State(mut client): State<KvClient>,
    Path(id): Path<Uuid>,
) -> ApiResult<Task> {
    match find_task(&mut client, id)
        .await
        .context(format!("Failed to find task {id}"))?
    {
        Some((task, _)) => Ok((StatusCode::OK, Json(task))),
        None => Err(ApiError {
            kind: ApiErrorKind::NotFound,
            inner: anyhow!("Task {id} does not exist"),
        }),
    }
}

#[tracing::instrument(skip(client), err(Debug))]
async fn cancel_task_endpoint(
    State(mut client): State<KvClient>,
    Path(id): Path<Uuid>,
) -> ApiResult<Task> {
    match cancel_task(&mut client, id)
        .await
        .context(format!("Failed to cancel task {id}"))?
    {
        Some(task) => Ok((StatusCode::OK, Json(task))),
        None => Err(ApiError {
            kind: ApiErrorKind::NotFound,
            inner: anyhow!("Task {id} does not exist"),
        }),
    }
}
