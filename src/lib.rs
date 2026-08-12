pub mod config;
pub mod db;
pub mod error;
pub mod health;
pub mod telemetry;
pub mod todos;

use std::sync::Arc;

use axum::Json;
use axum::Router;
use serde::Serialize;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_swagger_ui::SwaggerUi;

use crate::config::Config;
use crate::db::DbPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Arc<Config>,
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "rust-web-starter",
        description = "A Rust + Axum + Diesel starter template"
    ),
    tags(
        (name = "meta", description = "Service metadata"),
        (name = "todos", description = "Todo CRUD"),
        (name = "health", description = "Liveness and readiness probes")
    )
)]
struct ApiDoc;

#[derive(Debug, Serialize, ToSchema)]
pub struct IndexBody {
    message: &'static str,
    version: &'static str,
    docs: &'static str,
}

#[utoipa::path(
    tag = "meta",
    get,
    path = "/",
    responses((status = 200, description = "Service metadata", body = IndexBody)),
)]
async fn index() -> Json<IndexBody> {
    Json(IndexBody {
        message: "hello world",
        version: env!("CARGO_PKG_VERSION"),
        docs: "/docs",
    })
}

pub fn app(state: AppState) -> Router {
    let allowed_origins = state.config.allowed_origins.clone();

    let (router, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(index))
        .merge(health::router())
        .merge(todos::router())
        .with_state(state)
        .split_for_parts();

    // listed innermost first: each .layer() wraps what came before it, so on the way in a
    // request meets SetRequestId, then the trace layer (whose span reads the id it just set),
    // then CORS, then the panic guard. A panic therefore becomes a 500 the trace layer sees.
    router
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi))
        .layer(telemetry::catch_panic_layer())
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(telemetry::cors_layer(&allowed_origins))
        .layer(telemetry::trace_layer())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
}
