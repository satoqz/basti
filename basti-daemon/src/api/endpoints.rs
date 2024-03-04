use super::errors::{ApiError, ApiErrorKind, ApiResult};
use crate::ops::{cancel_task, create_task, find_task, list_tasks};

use anyhow::{anyhow, Context};
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use basti_task::{CreateTask, Task, TaskState};
use etcd_client::KvClient;
use serde::Deserialize;
use std::fmt::Debug;
use strum::VariantArray;
use uuid::Uuid;

#[tracing::instrument(skip(client), err(Debug))]
pub async fn create_task_endpoint(
    State(mut client): State<KvClient>,
    Json(payload): Json<CreateTask>,
) -> ApiResult<Task> {
    let task = create_task(&mut client, payload.duration, payload.priority)
        .await
        .context("Failed to create task")?;

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
        .context("Failed to list tasks")?;

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
pub async fn cancel_task_endpoint(
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
