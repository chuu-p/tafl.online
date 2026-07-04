# Security Audit — tafl.online

Generated: 2026-07-04

## Critical

- [x] **PostgreSQL trust authentication** — Fixed: scram-sha-256 + password via initialScript
- [x] **pgAdmin exposed to internet with hardcoded password** — Fixed: bind 127.0.0.1, openFirewall=false
- [ ] **Google OAuth secret in plaintext .env** — `.env` has `GOOGLE_CLIENT_SECRET` in cleartext.
  - Effort: S | Priority: P0 | Fix: inject via systemd env from sops

## High

- [x] **Firewall globally disabled** — Fixed: enabled, allows only 22/80/443
- [x] **Grafana on 0.0.0.0 with hardcoded SMTP password** — Fixed: bind 127.0.0.1, removed disable_sanitize_html
- [x] **User password = username** — Fixed: removed password, SSH keys only
- [x] **NOPASSWD sudo for ALL commands** — Fixed: removed NOPASSWD
- [x] **Tempo binds all interfaces** — Fixed: bind 127.0.0.1
- [ ] **Dioxus server on 0.0.0.0** — server listens on all interfaces.
  - Effort: S | Priority: P1 | Fix: bind 127.0.0.1 (not urgent: firewall blocks external access)

## Medium

- [ ] **XSS via `dangerous_inner_html` with QR SVG** — `board_view.rs:285`
  - Effort: S | Priority: P2 | Fix: sanitize SVG or add trust comment

- [ ] **JS eval with user-influenced data** — `board_view.rs:248`
  - Effort: S | Priority: P2 | Fix: use web-sys Clipboard API

- [ ] **No authentication on game API** — `get_game`/`create_game` have no auth
  - Effort: M | Priority: P2 | Fix: implement auth, check is_private

- [ ] **No rate limiting on any endpoint** — unlimited game creation, WebSocket connections
  - Effort: M | Priority: P2 | Fix: nginx limit_req

- [ ] **No WebSocket origin validation** — any webpage can connect
  - Effort: S | Priority: P2 | Fix: validate Origin header

- [x] **Loki auth disabled** — Mitigated: firewall blocks external access
- [x] **Maddy email password is "test"** — Mitigated: firewall + localhost, placeholder password
- [x] **Grafana SMTP password is "test"** — Mitigated: firewall + localhost, placeholder password

## Low

- [ ] **No CSRF on WebSocket endpoints** — origin validation covers this
  - Effort: M | Priority: P3

- [x] **DATABASE_URL uses trust auth** — Fixed: scram-sha-256 with password in connection string
