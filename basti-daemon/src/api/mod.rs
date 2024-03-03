mod endpoints;
mod errors;

use crate::api::endpoints::*;
use anyhow::Context;
use axum::{
    routing::{delete, get, post},
    Router,
};
use etcd_client::KvClient;
use std::net::SocketAddr;
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

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
