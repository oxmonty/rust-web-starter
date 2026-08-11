use std::sync::Arc;

use diesel_async::RunQueryDsl;
use rust_web_starter::config::{Config, LogFormat};
use rust_web_starter::db;
use rust_web_starter::{AppState, app};

#[derive(Debug, serde::Deserialize)]
pub struct Todo {
    pub id: i32,
    pub title: String,
    pub description: Option<String>,
    pub done: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, serde::Deserialize)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
}

pub struct TestApp {
    pub base_url: String,
    pub client: reqwest::Client,
    // held for the test's duration; dropping deletes the sqlite temp file mid-test
    _temp_dir: Option<tempfile::TempDir>,
}

impl TestApp {
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}

// DATABASE_URL set -> use it as-is. Unset or empty -> fresh temp-file sqlite db.
// An empty value has to count as unset: env::var returns Ok("") for a variable set to the
// empty string, and a CI matrix that defines DATABASE_URL for only one leg supplies exactly that.
// Never `:memory:`: each pooled connection gets its own separate empty database.
pub async fn spawn_app() -> TestApp {
    let configured_url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|url| !url.trim().is_empty());

    let (database_url, temp_dir) = match configured_url {
        Some(url) => (url, None),
        None => {
            let temp_dir = tempfile::tempdir().expect("create temp dir");
            let db_path = temp_dir.path().join("test.db");
            (format!("sqlite://{}", db_path.display()), Some(temp_dir))
        }
    };

    let pool = db::build_pool(&database_url).await.expect("build pool");

    // keyed on the URL scheme, not on whether a temp dir was made: the advisory lock only
    // exists on postgres, so conflating the two turns a config slip into a confusing failure
    if !database_url.starts_with("postgres") {
        db::run_migrations(&database_url)
            .await
            .expect("run migrations");
    } else {
        // shared postgres db: parallel tests would otherwise race concurrent CREATE TABLEs
        // against the same catalog. Serialize migrations with a session advisory lock.
        let mut conn = pool.get().await.expect("get conn for migration lock");
        diesel::sql_query("SELECT pg_advisory_lock(727369)")
            .execute(&mut conn)
            .await
            .expect("acquire migration lock");
        drop(conn);
        db::run_migrations(&database_url)
            .await
            .expect("run migrations");
        let mut conn = pool.get().await.expect("get conn for migration unlock");
        diesel::sql_query("SELECT pg_advisory_unlock(727369)")
            .execute(&mut conn)
            .await
            .ok();
    }

    let state = AppState {
        pool,
        config: Arc::new(Config {
            database_url: database_url.clone(),
            bind_addr: "127.0.0.1:0".to_string(),
            allowed_origins: Vec::new(),
            log_format: LogFormat::Pretty,
        }),
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        axum::serve(listener, app(state))
            .await
            .expect("server error");
    });

    TestApp {
        base_url: format!("http://{addr}"),
        client: reqwest::Client::new(),
        _temp_dir: temp_dir,
    }
}
