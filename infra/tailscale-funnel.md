# Tailscale Funnel for tafl.online

## Prerequisites

Tailscale is already enabled on toph (`services.tailscale.enable = true`).

Authorize the machine on your tailnet:
```bash
tailscale up
```

## Expose the taste backend (gRPC-Web)

```bash
# Enable Funnel (runs in background)
sudo tailscale funnel --bg 50051

# This makes the service available at:
# https://toph.<your-tailnet>.ts.net:443
# (Funnel handles HTTPS termination, proxies to localhost:50051)
```

The taste server speaks gRPC-Web on port 50051. Browsers connect via HTTPS (Tailscale Funnel terminates TLS automatically).

## Verify

```bash
tailscale status
tailscale funnel status
```

## Frontend

For the frontend SPA, you need a web server (nginx/caddy) that:
1. Serves the built Dioxus WASM app from `crates/tafl.online/target/dx/tafl-online/debug/web/public/`
2. Proxies `/tafl.v1.*` → `http://127.0.0.1:50051`

Example nginx config:

```
server {
    listen 8080;
    root /var/lib/tafl-web/public;
    location /tafl.v1/ {
        proxy_pass http://127.0.0.1:50051;
        proxy_http_version 1.1;
    }
}
```

Then expose nginx instead:
```bash
tailscale funnel --bg 8080
```
