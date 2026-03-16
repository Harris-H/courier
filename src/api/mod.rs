pub mod routes;

use std::sync::Arc;

use axum::{
    routing::{get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

use crate::state::AppState;

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        .route("/api/status", get(routes::get_status))
        .route("/api/tasks", get(routes::list_tasks))
        .route("/api/tasks/{name}/run", post(routes::run_task))
        .route("/api/tasks/{name}/schedule", put(routes::update_task_schedule))
        .route("/api/history", get(routes::get_history))
        .route("/api/history/{index}/content", get(routes::get_history_content))
        .route("/api/config", get(routes::get_config))
        .route("/api/config/feishu", put(routes::update_feishu_config))
        .route("/api/config/llm", put(routes::update_llm_config))
        .route("/api/sources", get(routes::list_sources))
        .layer(cors)
        .with_state(state);

    // Serve static frontend files (Vue.js build output)
    let static_dir = std::path::PathBuf::from("web/dist");
    if static_dir.exists() {
        let serve_dir = ServeDir::new(&static_dir)
            .not_found_service(ServeFile::new(static_dir.join("index.html")));
        api.fallback_service(serve_dir)
    } else {
        api
    }
}

pub async fn start_server(state: Arc<AppState>, port: u16) -> anyhow::Result<()> {
    let app = create_router(state);
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("🌐 Dashboard API listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
