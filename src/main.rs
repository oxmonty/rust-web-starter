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

    tracing::info!(bind_addr, "listening");

    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
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
