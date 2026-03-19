pub mod routes;

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

use crate::state::AppState;

/// Authentication middleware: validates Bearer token against configured api_key
async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if let Some(api_key) = &state.config.general.api_key {
        if !api_key.is_empty() {
            let auth_header = req
                .headers()
                .get("Authorization")
                .and_then(|v| v.to_str().ok());

            match auth_header {
                Some(header) if header.starts_with("Bearer ") => {
                    let token = &header[7..];
                    if token != api_key {
                        return Err(StatusCode::UNAUTHORIZED);
                    }
                }
                _ => return Err(StatusCode::UNAUTHORIZED),
            }
        }
    }
    Ok(next.run(req).await)
}

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
        .route("/api/tasks/{name}/toggle", put(routes::toggle_task))
        .route("/api/history", get(routes::get_history))
        .route("/api/history/clear", delete(routes::clear_history))
        .route("/api/history/batch", post(routes::delete_history))
        .route("/api/history/{index}/content", get(routes::get_history_content))
        .route("/api/config", get(routes::get_config))
        .route("/api/config/feishu", put(routes::update_feishu_config))
        .route("/api/config/email", put(routes::update_email_config))
        .route("/api/config/llm", put(routes::update_llm_config))
        .route("/api/sources", get(routes::list_sources))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
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
