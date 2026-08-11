use std::sync::Arc;

use rust_web_starter::config::Config;
use rust_web_starter::{AppState, app, db};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("invalid configuration: {err}");
            std::process::exit(1);
        }
    };

    rust_web_starter::telemetry::init(config.log_format);

    if let Err(err) = db::run_migrations(&config.database_url).await {
        tracing::error!(error = %err, "failed to run migrations");
        std::process::exit(1);
    }

    let pool = match db::build_pool(&config.database_url).await {
        Ok(pool) => pool,
        Err(err) => {
            tracing::error!(error = %err, "failed to build database pool");
            std::process::exit(1);
        }
    };

    let bind_addr = config.bind_addr.clone();
    let state = AppState {
        pool,
        config: Arc::new(config),
    };

    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!(error = %err, bind_addr, "failed to bind");
            std::process::exit(1);
        }
    };

    // bind_addr is usually 0.0.0.0, which isn't a URL you can open; report loopback instead
    let url = match listener.local_addr() {
        Ok(addr) if addr.ip().is_unspecified() => format!("http://localhost:{}", addr.port()),
        Ok(addr) => format!("http://{addr}"),
        Err(_) => format!("http://{bind_addr}"),
    };

    tracing::info!(bind_addr, "listening on {url} (docs at {url}/docs)");

    // every other startup failure logs and exits; panicking here instead would report through
    // the default panic hook, which writes to stderr and bypasses the log format entirely
    if let Err(err) = axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        tracing::error!(error = %err, "server error");
        std::process::exit(1);
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
