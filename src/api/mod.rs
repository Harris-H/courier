pub mod routes;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::state::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/status", get(routes::get_status))
        .route("/api/tasks", get(routes::list_tasks))
        .route("/api/tasks/{name}/run", post(routes::run_task))
        .route("/api/history", get(routes::get_history))
        .route("/api/config", get(routes::get_config))
        .route("/api/sources", get(routes::list_sources))
        .layer(cors)
        .with_state(state)
}

pub async fn start_server(state: Arc<AppState>, port: u16) -> anyhow::Result<()> {
    let app = create_router(state);
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("🌐 Dashboard API listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
