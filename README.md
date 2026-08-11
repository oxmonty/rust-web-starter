# rust-web-starter

A minimal Rust web service starter.

**Stack:** Axum 0.8, Diesel 2.3 + diesel-async (SQLite or Postgres), utoipa (OpenAPI + Swagger), tower-http (tracing + CORS), bacon.

## Prerequisites

Rust 1.93 or newer (the crate uses edition 2024). That alone is enough for `make run` and `make test`, since SQLite is bundled and compiled from source.

The rest are only needed for specific targets:

```sh
cargo install bacon                     # make dev
cargo install diesel_cli --no-default-features \
  --features sqlite-bundled,postgres-bundled   # make migrate, make schema
```

Docker is needed for `make up` and `make test-pg`.

## Quick start

```sh
make dev
```

Defaults to SQLite. No database to install, no Docker, no setup step. Migrations run automatically at boot.

- API: http://localhost:4563
- Swagger UI: http://localhost:4563/docs

![Swagger UI showing the todos and health endpoint groups](docs/swagger.png)

Copy `.env.example` to `.env` to change any setting.

## Postgres

```sh
make up                                                 # Postgres + app in containers
cargo build --no-default-features --features postgres   # local build
```

The two backends are mutually exclusive and chosen at compile time. Plain `--features postgres` enables both and deliberately fails with a `compile_error!`, so always pass `--no-default-features` with it.

## Make targets

Run `make` (or `make help`) to see every target with a description.

## Adding an entity

Copy `src/todos/` to `src/<name>/` and rename through it, add a migration under **both** `migrations/sqlite/` and `migrations/postgres/`, run `make schema`, then register the router in `src/lib.rs`.

There is deliberately no generic repository or CRUD macro. Diesel's query builder resists that kind of abstraction (its own maintainers advise against it), and the five query functions per entity are short enough that copying a directory is cheaper than parameterizing one. Diesel's derives are the base model layer.

Keep new columns to types both backends share. There is a single `schema.rs` covering both, which is what keeps every `#[cfg]` in the codebase confined to `src/db/mod.rs`.

## Notes

- `ALLOWED_ORIGINS` is empty by default, so no cross-origin requests are permitted. Set it explicitly in production. CORS is never permissive.
- SQLite is not natively async. It runs through diesel-async's `SyncConnectionWrapper`, which is `spawn_blocking` underneath. Postgres is natively async.
- `/healthz` is liveness and never touches the database, so a database blip cannot get the process killed. `/readyz` is readiness and does.
- The `NOT NULL` on the SQLite `todos.id` column is load-bearing, not redundant. SQLite reports `notnull=0` for any `INTEGER PRIMARY KEY` because it is a rowid alias, so without it Diesel generates `Nullable<Integer>` and no longer matches Postgres.

## License

[MIT](LICENSE)
