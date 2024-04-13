all:
  just --justfile {{justfile()}} fmt
  just --justfile {{justfile()}} check
  just --justfile {{justfile()}} clippy
  just --justfile {{justfile()}} test

lint:
  just --justfile {{justfile()}} fmt
  just --justfile {{justfile()}} check
  just --justfile {{justfile()}} clippy

clippy:
  cargo clippy --all-targets

check:
  cargo check --all-targets

fmt:
  cargo fmt --all

migrate:
  sqlx migrate run

test:
  cargo test

dev:
  cargo watch -x "run -- start"

db:
  docker compose up db

db-detached:
  docker compose up db -d

stop-db:
  docker compose down db
