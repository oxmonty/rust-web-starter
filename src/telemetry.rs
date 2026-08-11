use axum::Json;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::{
    DefaultOnBodyChunk, DefaultOnEos, DefaultOnRequest, DefaultOnResponse, MakeSpan, TraceLayer,
};
use tracing::{Level, Span};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::LogFormat;
use crate::error::ErrorBody;

// the header SetRequestIdLayer::x_request_id writes, re-declared because tower-http
// does not export the constant
const REQUEST_ID_HEADER: &str = "x-request-id";

pub fn init(log_format: LogFormat) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let registry = tracing_subscriber::registry().with(env_filter);
    match log_format {
        LogFormat::Pretty => registry.with(tracing_subscriber::fmt::layer()).init(),
        LogFormat::Json => registry
            .with(tracing_subscriber::fmt::layer().json())
            .init(),
    }
}

/// Span every request event is recorded under, carrying the fields you need to group by
/// when reading logs: method, path, and the id that ties a client's report to its lines.
#[derive(Clone, Copy)]
pub struct RequestSpan;

impl<B> MakeSpan<B> for RequestSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        tracing::info_span!(
            "request",
            method = %request.method(),
            path = %request.uri().path(),
            request_id = request
                .headers()
                .get(REQUEST_ID_HEADER)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default(),
        )
    }
}

// tower-http defaults every trace level to DEBUG, so a service running at the default
// RUST_LOG=info logs no requests at all. Raise the response event, which carries status and
// latency, and leave on_request at DEBUG: one access log line per request, not two.
// on_failure is disabled rather than left at its default: it would log a second ERROR per
// failed request saying only "response failed, status 500", which AppError has already logged
// with the actual cause. It also classifies every 5xx as an error, so a database restart would
// report ERROR on each /readyz probe when the service is deliberately answering "not ready".
// The rule this relies on: routes report failure by returning AppError, which does the logging.
pub fn trace_layer() -> TraceLayer<
    SharedClassifier<ServerErrorsAsFailures>,
    RequestSpan,
    DefaultOnRequest,
    DefaultOnResponse,
    DefaultOnBodyChunk,
    DefaultOnEos,
    (),
> {
    TraceLayer::new_for_http()
        .make_span_with(RequestSpan)
        .on_response(DefaultOnResponse::new().level(Level::INFO))
        .on_failure(())
}

// a panicking handler otherwise kills the connection and reports through the default panic
// hook, which writes to stderr and never reaches structured log output
pub fn catch_panic_layer()
-> CatchPanicLayer<fn(Box<dyn std::any::Any + Send + 'static>) -> Response> {
    CatchPanicLayer::custom(log_panic)
}

fn log_panic(err: Box<dyn std::any::Any + Send + 'static>) -> Response {
    let details = err
        .downcast_ref::<&str>()
        .map(|panic| panic.to_string())
        .or_else(|| err.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_string());

    tracing::error!(panic = %details, "handler panicked");

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorBody {
            error: "internal_error".to_string(),
            message: "An internal error occurred.".to_string(),
        }),
    )
        .into_response()
}

// default is no allowed origins; templates get copy-pasted straight to prod, so
// CorsLayer::permissive() is never an option here
pub fn cors_layer(allowed_origins: &[String]) -> CorsLayer {
    if allowed_origins.is_empty() {
        return CorsLayer::new();
    }

    // a typo'd origin would otherwise be dropped silently and surface only as a browser CORS
    // failure with nothing in the logs
    let origins = allowed_origins
        .iter()
        .filter_map(|origin| match origin.parse() {
            Ok(value) => Some(value),
            Err(_) => {
                tracing::warn!(origin = %origin, "ignoring unparseable CORS origin");
                None
            }
        });

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(Any)
        .allow_headers(Any)
}
