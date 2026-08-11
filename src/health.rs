use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::db;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthBody {
    status: &'static str,
}

#[utoipa::path(
    get,
    path = "/healthz",
    responses((status = 200, description = "Service is up", body = HealthBody)),
)]
async fn healthz() -> Json<HealthBody> {
    tracing::debug!("healthz");
    Json(HealthBody { status: "ok" })
}

#[utoipa::path(
    get,
    path = "/readyz",
    responses(
        (status = 200, description = "Service is ready", body = HealthBody),
        (status = 503, description = "Service is degraded", body = HealthBody),
    ),
)]
async fn readyz(State(state): State<AppState>) -> (StatusCode, Json<HealthBody>) {
    tracing::debug!("readyz");
    match db::ping(&state.pool).await {
        Ok(()) => (StatusCode::OK, Json(HealthBody { status: "ready" })),
        Err(err) => {
            tracing::debug!(error = %err, "readyz check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthBody { status: "degraded" }),
            )
        }
    }
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(healthz))
        .routes(routes!(readyz))
}
