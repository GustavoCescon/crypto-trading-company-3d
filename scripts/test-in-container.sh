#!/bin/sh
set -eu
if [ -f .env ]; then . ./.env; fi
export DATABASE_URL="${DATABASE_URL:-postgres://ctc:${POSTGRES_PASSWORD}@postgres:5432/ctc}"
export REDIS_URL="${REDIS_URL:-redis://redis:6379}"
cargo test --locked --workspace -- --include-ignored
