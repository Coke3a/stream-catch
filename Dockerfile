# syntax=docker/dockerfile:1

FROM docker.io/lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

# Diesel (Postgres) needs libpq at build time.
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq-dev \
    pkg-config \
  && rm -rf /var/lib/apt/lists/*

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS deps
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM chef AS build-backend
COPY . .
COPY --from=deps /app/target /app/target
COPY --from=deps /usr/local/cargo /usr/local/cargo
RUN cargo build --release --package backend --bin backend

FROM chef AS build-worker
COPY . .
COPY --from=deps /app/target /app/target
COPY --from=deps /usr/local/cargo /usr/local/cargo
RUN cargo build --release --package worker --bin worker

FROM docker.io/library/debian:trixie-slim AS runtime-backend
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libpq5 \
  && rm -rf /var/lib/apt/lists/*

RUN useradd --system --uid 10001 --create-home app
COPY --from=build-backend /app/target/release/backend /usr/local/bin/backend
USER app
ENTRYPOINT ["/usr/local/bin/backend"]

FROM docker.io/library/debian:trixie-slim AS runtime-worker
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libpq5 \
  && rm -rf /var/lib/apt/lists/*

RUN useradd --system --uid 10001 --create-home app
COPY --from=build-worker /app/target/release/worker /usr/local/bin/worker
USER app
ENTRYPOINT ["/usr/local/bin/worker"]

