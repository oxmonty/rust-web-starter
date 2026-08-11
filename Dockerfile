# syntax=docker/dockerfile:1
ARG RUST_VERSION=1.93.1

FROM rust:${RUST_VERSION}-slim-bookworm AS chef
RUN cargo install cargo-chef --version 0.1.77 --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# pq-sys "bundled" compiles libpq from source but still links against the
# system's OpenSSL (dynamically) unless openssl-sys/vendored is added — it is
# not, per Cargo.toml — so the builder needs headers+lib, not just a compiler.
# curl is not optional: utoipa-swagger-ui's build script downloads the Swagger UI
# bundle at compile time and shells out to curl when its `reqwest` feature is off.
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --no-default-features --features postgres --recipe-path recipe.json
COPY . .
RUN cargo build --release --no-default-features --features postgres

FROM debian:bookworm-slim AS runtime
# libssl3 matches the OpenSSL 3.0.x the builder's libssl-dev linked against on
# the same Debian release; curl is only here for the compose healthcheck.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r appuser && useradd -r -g appuser -u 10001 appuser

# migrations are embedded into the binary at compile time (embed_migrations!),
# so nothing beyond the binary itself needs to ship here.
COPY --from=builder /app/target/release/rust-web-starter /usr/local/bin/rust-web-starter

USER appuser
EXPOSE 4563
ENTRYPOINT ["/usr/local/bin/rust-web-starter"]
