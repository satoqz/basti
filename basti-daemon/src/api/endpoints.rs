use std::fmt::Debug;

use anyhow::{anyhow, Context};
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use etcd_client::KvClient;
use serde::Deserialize;
use uuid::Uuid;

use basti_types::{CreateTask, Task, TaskState};

use super::errors::{ApiError, ApiErrorKind, ApiResult};
use crate::ops::{cancel_task, create_task, find_task, list_tasks};

#[tracing::instrument(skip(client), err(Debug))]
pub async fn create_task_endpoint(
    State(mut client): State<KvClient>,
    Json(payload): Json<CreateTask>,
) -> ApiResult<Task> {
    let task = create_task(&mut client, payload.duration, payload.priority)
        .await
        .context("failed to create task")?;

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
) -> ApiResult<Vec<Task>> {
    let tasks = list_tasks(&mut client, params.state, params.limit.unwrap_or(50))
        .await
        .context("failed to list tasks")?;

    Ok((
        StatusCode::OK,
        Json(tasks.into_iter().map(|(task, _)| task).collect()),
    ))
}

#[tracing::instrument(skip(client), err(Debug))]
pub async fn find_task_endpoint(
    State(mut client): State<KvClient>,
    Path(id): Path<Uuid>,
) -> ApiResult<Task> {
    match find_task(&mut client, id, TaskState::VARIANTS)
        .await
        .context(format!("failed to find task {id}"))?
    {
        Some((task, _)) => Ok((StatusCode::OK, Json(task))),
        None => Err(ApiError {
            kind: ApiErrorKind::NotFound,
            inner: anyhow!("task {id} does not exist"),
        }),
    }
}

#[tracing::instrument(skip(client), err(Debug))]
pub async fn cancel_task_endpoint(
    State(mut client): State<KvClient>,
    Path(id): Path<Uuid>,
) -> ApiResult<Task> {
    match cancel_task(&mut client, id)
        .await
        .context(format!("failed to cancel task {id}"))?
    {
        Some(task) => Ok((StatusCode::OK, Json(task))),
        None => Err(ApiError {
            kind: ApiErrorKind::NotFound,
            inner: anyhow!("task {id} does not exist"),
        }),
    }
}
