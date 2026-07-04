# Security Audit — tafl.online

Generated: 2026-07-04

## Critical

- [ ] **Google OAuth secret in plaintext .env** — `.env` has `GOOGLE_CLIENT_SECRET` in cleartext. Move to sops-nix.
  - Effort: S | Priority: P0 | Fix: inject via systemd env from sops

- [ ] **PostgreSQL trust authentication** — `postgresql.nix:20-23` allows any local process to connect as any user with no password.
  - Effort: M | Priority: P0 | Fix: scram-sha-256 + sops-nix for passwords

- [ ] **pgAdmin exposed to internet with hardcoded password** — `pgadmin.nix` binds `0.0.0.0`, `openFirewall = true`, password is `YourSecurePassword123`.
  - Effort: S | Priority: P0 | Fix: bind 127.0.0.1, remove openFirewall, sops for password

## High

- [ ] **Firewall globally disabled** — `base.nix:16` `networking.firewall.enable = false`. Every service is exposed.
  - Effort: S | Priority: P1 | Fix: enable firewall, allow only 80/443

- [ ] **Grafana on 0.0.0.0 with hardcoded SMTP password** — `grafana.nix` exposes admin panel, `password = "test"`, `disable_sanitize_html = true`.
  - Effort: S | Priority: P1 | Fix: bind 127.0.0.1, sops for password, remove sanitize disable

- [ ] **User password = username** — `base.nix:67` `password = "chuu"`.
  - Effort: S | Priority: P1 | Fix: remove password, rely on SSH keys

- [ ] **NOPASSWD sudo for ALL commands** — `base.nix:79-89`.
  - Effort: S | Priority: P1 | Fix: restrict command list or require password

- [ ] **Tempo binds all interfaces** — `tempo.nix` exposes OTLP/gRPC/Zipkin/Jaeger on 0.0.0.0.
  - Effort: S | Priority: P1 | Fix: bind 127.0.0.1

- [ ] **Dioxus server on 0.0.0.0** — server listens on all interfaces, should be 127.0.0.1 behind nginx.
  - Effort: S | Priority: P1 | Fix: bind 127.0.0.1

## Medium

- [ ] **XSS via `dangerous_inner_html` with QR SVG** — `board_view.rs:285`. SVG from `fast_qr` rendered as raw HTML.
  - Effort: S | Priority: P2 | Fix: sanitize SVG output or add trust assumption comment

- [ ] **JS eval with user-influenced data** — `board_view.rs:248` clipboard copy uses string interpolation in `eval()`.
  - Effort: S | Priority: P2 | Fix: use `js_sys::Array::of1` or web-sys Clipboard API

- [ ] **No authentication on game API** — `get_game`/`create_game` have no auth. `is_private` field exists but is never checked.
  - Effort: M | Priority: P2 | Fix: implement auth, check is_private

- [ ] **No rate limiting on any endpoint** — unlimited game creation, WebSocket connections.
  - Effort: M | Priority: P2 | Fix: nginx `limit_req` or tower rate limit

- [ ] **No WebSocket origin validation** — any webpage can connect and make moves.
  - Effort: S | Priority: P2 | Fix: validate Origin header

- [ ] **Loki auth disabled** — `loki-local-config.yaml:1` `auth_enabled: false`.
  - Effort: S | Priority: P2 | Fix: bind 127.0.0.1 only

- [ ] **Maddy email password is "test"** — `maddy.nix:8`, world-readable in nix store.
  - Effort: S | Priority: P2 | Fix: sops-nix

- [ ] **Grafana SMTP password is "test"** — `grafana.nix:18`.
  - Effort: S | Priority: P2 | Fix: sops-nix

## Low

- [ ] **No CSRF on WebSocket endpoints** — Dioxus server functions may have built-in CSRF, but WebSocket endpoints don't.
  - Effort: M | Priority: P3 | Fix: origin validation covers this

- [ ] **DATABASE_URL uses trust auth** — `tafl-online.nix:79`, breaks when postgres auth is fixed.
  - Effort: S | Priority: P3 | Fix: sops-nix for password when fixing postgres auth
