# stream-rokuo

Rust workspace with:
- `backend/` (`backend` bin): Axum HTTP API (deploy to Fly.io)
- `worker/` (`worker` bin): webhooks + background loops (run on a VPS via Docker Compose)
- `crates/`: shared library crate(s)

## Local development

1. Create a local env file (never commit real secrets):
   - `cp .env-example .env`
2. Run services:
   - Backend: `cargo run -p backend`
   - Worker: `cargo run -p worker`

Notes:
- Both binaries read configuration from environment variables; `dotenvy` is used locally to load `.env` if present.
- Migrations live in `crates/infra/src/postgres/migrations/` (see Diesel docs/`diesel.toml` for usage).

## Docker builds (cargo-chef)

This repo builds two small runtime images from the same `Dockerfile` using multi-stage `cargo-chef` caching.

- Build backend image:
  - `docker build -t stream-rokuo-backend --target runtime-backend .`
- Build worker image:
  - `docker build -t stream-rokuo-worker --target runtime-worker .`

The build context intentionally excludes `.env` via `.dockerignore` to avoid baking secrets into image layers.

## Deployment

### Deploy backend to Fly.io

Prereqs:
- `flyctl` installed and authenticated
- Supabase Postgres connection string for `DATABASE_URL` (no separate DB hosting needed)

Initial setup (new app):
- `fly launch --dockerfile Dockerfile --no-deploy`
- Set the backend listen port to Fly’s internal port (commonly `8080`):
  - `fly secrets set SERVER_PORT_BACKEND=8080`

Set secrets (examples):
- `fly secrets set DATABASE_URL='postgresql://postgres:<password>@db.<project-ref>.supabase.co:5432/postgres?sslmode=require'`
- `fly secrets set SUPABASE_PROJECT_URL='https://...' SUPABASE_JWT_SECRET='...'`
- `fly secrets set STRIPE_SECRET_KEY='...' STRIPE_WEBHOOK_SECRET='...'`
- `fly secrets set WATCH_URL_JWT_SECRET='...'`

Deploy:
- `fly deploy --build-target runtime-backend`

Operate:
- Logs: `fly logs`
- Health endpoint: `GET /api/v1/health-check`

Tip: See `deploy/fly/fly.toml.example` if you want a starting `fly.toml`.

### Deploy worker to a VPS (Docker Compose)

The VPS compose file lives at `deploy/vps/docker-compose.yml` and runs only the worker by default.

1. Install Docker + the Compose plugin on the VPS.
2. Create a directory on the VPS (example): `mkdir -p /opt/stream-rokuo`
3. Copy files to the VPS:
   - `deploy/vps/docker-compose.yml` → `/opt/stream-rokuo/docker-compose.yml`
   - `.env-example` → `/opt/stream-rokuo/.env` (fill required values)
4. Edit the image reference in `/opt/stream-rokuo/docker-compose.yml`:
   - `ghcr.io/<org>/stream-rokuo-worker:latest`
5. Create the shared docker network (one-time):
   - `docker network create orec-net`
6. Start/update:
   - `docker compose pull && docker compose up -d`

Operate:
- Logs: `docker compose logs -f worker`
- Restart: `docker compose restart worker` (also uses `restart: unless-stopped`)
- Health endpoint: `GET /health-check` (on `SERVER_PORT_WORKER`)

## Env management

- Local dev: use `.env` (from `.env-example`).
- Fly.io: use `fly secrets set ...` (preferred over committing env files).
- VPS: keep secrets in `/opt/stream-rokuo/.env` with appropriate file permissions; do not bake secrets into images.

## Logging and healthchecks

- Both services log to stdout/stderr using `tracing`; use `fly logs` / `docker compose logs`.
- Health endpoints:
  - Backend: `GET /api/v1/health-check`
  - Worker: `GET /health-check`
- `deploy/vps/docker-compose.yml` includes a commented-out container healthcheck; enabling it requires an HTTP client (e.g. `curl`) inside the image, or an external healthcheck mechanism.

## Best practices checklist

- Run as non-root inside containers (Dockerfile creates a dedicated user).
- Keep runtime images small (`debian:bookworm-slim` + minimal packages).
- Keep secrets out of images (`.dockerignore` excludes `.env`; use Fly secrets / VPS env file).
- Preserve `cargo-chef` caching: avoid changing `Cargo.toml`/`Cargo.lock` unless dependencies actually change.
- Prefer pinned toolchains/dependencies for reproducible builds (and avoid `latest` tags in production images).
