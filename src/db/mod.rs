#[cfg(all(feature = "sqlite", feature = "postgres"))]
compile_error!(
    "features `sqlite` and `postgres` are mutually exclusive; use --no-default-features --features postgres"
);
#[cfg(not(any(feature = "sqlite", feature = "postgres")))]
compile_error!("enable exactly one backend feature: `sqlite` or `postgres`");

pub mod schema;

use diesel::IntoSql;
use diesel_async::RunQueryDsl;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use crate::error::AppError;

#[cfg(feature = "sqlite")]
pub type DbConn =
    diesel_async::sync_connection_wrapper::SyncConnectionWrapper<diesel::SqliteConnection>;
#[cfg(feature = "postgres")]
pub type DbConn = diesel_async::AsyncPgConnection;

pub type DbPool = Pool<DbConn>;

#[cfg(feature = "sqlite")]
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/sqlite");
#[cfg(feature = "postgres")]
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/postgres");

// sqlite has no URL scheme of its own; the `sqlite://` prefix only exists so DATABASE_URL looks
// uniform next to postgres's in .env.example, and diesel's SqliteConnection wants a bare path.
// diesel won't create missing intermediate directories itself, so a fresh clone with no `data/`
// dir yet would fail on the very first `cargo run` without this.
#[cfg(feature = "sqlite")]
fn resolve_database_url(database_url: &str) -> anyhow::Result<String> {
    let path = database_url
        .strip_prefix("sqlite://")
        .unwrap_or(database_url);
    if let Some(parent) = std::path::Path::new(path).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(path.to_string())
}

#[cfg(feature = "postgres")]
fn resolve_database_url(database_url: &str) -> anyhow::Result<String> {
    Ok(database_url.to_string())
}

pub async fn build_pool(database_url: &str) -> anyhow::Result<DbPool> {
    let database_url = resolve_database_url(database_url)?;
    let manager = AsyncDieselConnectionManager::<DbConn>::new(database_url);
    let pool = Pool::builder(manager).build()?;
    Ok(pool)
}

// diesel_migrations::MigrationHarness is sync-only; AsyncConnectionWrapper bridges it onto our
// async DbConn, and spawn_blocking keeps its internal block_on off the async runtime threads.
pub async fn run_migrations(database_url: &str) -> anyhow::Result<()> {
    let database_url = resolve_database_url(database_url)?;
    tokio::task::spawn_blocking(move || {
        use diesel::Connection;
        use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
        let mut conn = AsyncConnectionWrapper::<DbConn>::establish(&database_url)?;
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|err: Box<dyn std::error::Error + Send + Sync>| anyhow::anyhow!(err))?;
        Ok::<(), anyhow::Error>(())
    })
    .await??;
    Ok(())
}

pub async fn ping(pool: &DbPool) -> Result<(), AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| AppError::Pool(err.to_string()))?;
    diesel::select(1_i32.into_sql::<diesel::sql_types::Integer>())
        .get_result::<i32>(&mut conn)
        .await?;
    Ok(())
}
