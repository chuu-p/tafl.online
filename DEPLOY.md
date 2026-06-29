# Deploy

## Prerequisites

```sh
rustup target add wasm32-unknown-unknown
```

## Build

```sh
dx bundle --package web --release
```

Output goes to `target/dx/web/release/web/`:

```
target/dx/web/release/web/
├── public/          # static frontend (WASM, HTML, CSS, JS, assets)
├── server           # backend binary (serves API + frontend)
└── server.d/        # sidecar files
```

## Option A — Integrated (single host)

The `server` binary serves both the API and the static frontend. Deploy as a single unit:

```sh
./target/dx/web/release/web/server
```

Set `IP` and `PORT` env vars to bind to a specific address (defaults: `0.0.0.0:8080`).

## Option B — Separate (frontend + backend)

Serve the frontend from a CDN / static host (nginx, S3, Cloudflare Pages, etc.) and run the backend separately with `DIOXUS_SERVER_URL` pointing at it.

| Part | What to deploy | Example |
|------|---------------|---------|
| Frontend | `public/` directory | nginx, Cloudflare Pages, S3 + CloudFront |
| Backend  | `server` binary + sidecar files | Fly.io, Railway, a VPS with systemd |

The backend listens on `$IP:$PORT` (default `0.0.0.0:8080`). The frontend must be configured to call the backend URL — set the server URL in client code:

```rust
// in main.rs, before dioxus::launch(App)
#[cfg(not(feature = "server"))]
server_fn::client::set_server_url("https://api.yourdomain.com");
```

### Docker (multi-stage)

```dockerfile
FROM rust:1 AS builder
RUN cargo install cargo-chef dioxus-cli
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN dx bundle --package web --release

FROM debian:bookworm-slim AS runtime
RUN apt update && apt install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/dx/web/release/web/ /app
ENV PORT=8080 IP=0.0.0.0
EXPOSE 8080
WORKDIR /app
ENTRYPOINT ["/app/server"]
```

### Fly.io

```sh
fly launch
fly deploy
```

Add the `Dockerfile` above to your project root, then `fly launch` auto-detects it.

### GitHub Actions (CI/CD)

```yaml
name: Deploy
on:
  push:
    branches: [main]
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: superfly/flyctl-actions/setup-flyctl@master
      - run: flyctl deploy --remote-only
        env:
          FLY_API_TOKEN: ${{ secrets.FLY_API_TOKEN }}
```
