#!/usr/bin/env bash
set -euo pipefail

export DATABASE_URL="postgres:///shop?host=/run/postgresql&user=postgres"
export RUST_LOG="info"

echo "Building server..."
cargo run --manifest-path packages/web/Cargo.toml --release --no-default-features --features server -- --package web
