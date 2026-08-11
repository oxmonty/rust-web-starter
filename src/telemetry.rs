use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::LogFormat;

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

pub fn trace_layer()
-> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>>
{
    TraceLayer::new_for_http()
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
