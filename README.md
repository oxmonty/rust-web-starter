<h1>
  <img alt="rust logo" src="./assets/rust.svg" width="70" valign="middle">
  &nbsp;rust-web-starter
</h1>

A minimal Rust web service starter.

**Stack:** Axum, Diesel + diesel-async (SQLite or Postgres), utoipa (OpenAPI + Swagger), tower-http (tracing + CORS).

## Quick start

```sh
make run     # http://localhost:4563
make test
```

Rust 1.88 or newer is the only requirement. SQLite is compiled in and migrations run at boot, so there is no setup step and no database to install. Tests run against a temporary SQLite file and need nothing external either.

Swagger UI is at [/docs](http://localhost:4563/docs).

![Swagger UI showing the todos and health endpoint groups](docs/swagger.png)

Run `make` to see every target.

## Configuration

Copy `.env.example` to `.env`. It documents all five variables: `DATABASE_URL`, `BIND_ADDR` (default `0.0.0.0:4563`), `ALLOWED_ORIGINS`, `RUST_LOG` and `LOG_FORMAT`.

`ALLOWED_ORIGINS` is empty by default, so no cross-origin requests are permitted. Set it explicitly in production. CORS is never permissive.

## Logging

Every request logs one line at `info`, carrying method, path, status, latency and a request id:

```
INFO request{method=GET path=/todos request_id="0e40..."}: finished processing request latency=1 ms status=200
```

The id comes from the caller's `x-request-id` header when it sends one, otherwise the service generates a UUID. Either way it goes back on the response, so a client can quote an id and you can grep for it.

Set `LOG_FORMAT=json` in production for one JSON object per line, with the span fields nested under `spans` for your aggregator. `RUST_LOG` takes full [env filter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) syntax, so `RUST_LOG=info,tower_http=debug` adds a "started processing request" line per request, useful when you need to see requests that never finish.

A panicking handler is caught, logged at `error` and returned as a normal 500 error body, rather than dropping the connection and printing to stderr where nothing collects it.

Handlers log only what the access line cannot show, which is why `create_todo` logs the new id and the other routes log nothing. Internal failures and panics are `error`, a failing `/readyz` or a conflict is `warn`, and rejected input is `debug`: the client already gets the reason in the response body, so logging it by default would let any caller flood the log.

## Optional tools

```sh
cargo install bacon                            # make dev, rebuilds and restarts on save
cargo install diesel_cli --no-default-features \
  --features sqlite-bundled,postgres-bundled   # make migrate, make schema
```

Docker is needed for `make up`, `make down` and `make test-pg`.

## Postgres

```sh
make up                                                 # Postgres and the app in containers
cargo build --no-default-features --features postgres   # local build
```

The backends are mutually exclusive and chosen at compile time. `--no-default-features` is required because the default feature is `sqlite`, and enabling both trips a `compile_error!`.

## Adding an entity

1. Copy `src/todos/` to `src/<name>/` and rename through it.
2. Add a migration under **both** `migrations/sqlite/` and `migrations/postgres/`.
3. `make migrate` to apply it. This step is not optional: `make schema` introspects the live database, not the migration files.
4. `make schema` to regenerate `src/db/schema.rs`.
5. In `src/lib.rs`, add `pub mod <name>;` and merge `<name>::router()` into the router.

Use column types both backends share. One `schema.rs` covers both, which is what keeps every `#[cfg]` in the codebase confined to `src/db/mod.rs`.

There is deliberately no generic repository or CRUD macro. Diesel's query builder resists that abstraction, so each entity gets five plain functions instead.

## Notes

- SQLite is not natively async. It runs through diesel-async's `SyncConnectionWrapper`, which is `spawn_blocking` underneath. Postgres is natively async.
- `/healthz` is liveness and never touches the database, so a database blip cannot get the process killed. `/readyz` is readiness and does.
- The `NOT NULL` on SQLite's `todos.id` is load-bearing, not redundant. SQLite's `PRIMARY KEY` does not imply `NOT NULL`, so without it Diesel generates `Nullable<Integer>` and the schema stops matching Postgres.

## License

[MIT](LICENSE)
