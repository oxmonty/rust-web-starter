pub mod config;
pub mod db;
pub mod error;
pub mod health;
pub mod telemetry;
pub mod todos;

use std::sync::Arc;

use axum::Router;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::config::Config;
use crate::db::DbPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Arc<Config>,
}

#[derive(OpenApi)]
#[openapi(info(
    title = "rust-web-starter",
    description = "A Rust + Axum + Diesel starter template"
))]
struct ApiDoc;

pub fn app(state: AppState) -> Router {
    let allowed_origins = state.config.allowed_origins.clone();

    let (router, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(health::router())
        .merge(todos::router())
        .with_state(state)
        .split_for_parts();

    router
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi))
        .layer(telemetry::trace_layer())
        .layer(telemetry::cors_layer(&allowed_origins))
}
