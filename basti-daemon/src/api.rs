use crate::{api_error::*, ops::*};
use anyhow::{anyhow, Context};
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use basti_common::task::{CreateTaskPayload, Task, TaskState};
use etcd_client::Client;
use serde::Deserialize;
use std::{fmt::Debug, net::SocketAddr};
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use uuid::Uuid;

#[tracing::instrument(skip_all)]
pub async fn run(addr: SocketAddr, client: Client) -> anyhow::Result<()> {
    let trace_layer = TraceLayer::new_for_http()
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO))
        .on_failure(DefaultOnFailure::new().level(Level::WARN));

    let app = Router::new()
        .route("/api/tasks", post(create_task_endpoint))
        .route("/api/tasks", get(list_tasks_endpoint))
        .route("/api/tasks/:id", get(find_task_endpoint))
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
    State(mut client): State<Client>,
    Json(payload): Json<CreateTaskPayload>,
) -> ApiResult<Task> {
    let task = create_task(&mut client, payload.duration, payload.priority)
        .await
        .context("Failed to create task")?;

    Ok((StatusCode::CREATED, Json(task)))
}

#[derive(Debug, Deserialize)]
struct ListTasksParams {
    state: Option<TaskState>,
}

#[tracing::instrument(skip(client), err(Debug))]
async fn list_tasks_endpoint(
    State(mut client): State<Client>,
    Query(params): Query<ListTasksParams>,
) -> ApiResult<Vec<Task>> {
    let tasks = list_tasks(&mut client, params.state, None)
        .await
        .context("Failed to list tasks")?;

    Ok((
        StatusCode::OK,
        Json(tasks.into_iter().map(|(task, _)| task).collect()),
    ))
}

#[tracing::instrument(skip(client), err(Debug))]
async fn find_task_endpoint(
    State(mut client): State<Client>,
    Path(id): Path<Uuid>,
) -> ApiResult<Task> {
    match find_task_by_id(&mut client, id).await? {
        Some(task) => Ok((StatusCode::OK, Json(task))),
        None => Err(ApiError {
            kind: ApiErrorKind::NotFound,
            inner: anyhow!("Task does not exist"),
        }),
    }
}
