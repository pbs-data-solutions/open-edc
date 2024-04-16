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

prepare:
  # Prepare sqlx for offline build
  cargo sqlx prepare

test:
  cargo test

dev: migrate
  cargo watch -x "run -- start"

db:
  docker compose up db

db-detached:
  docker compose up db -d

stop-db:
  docker compose down db

docker:
  docker compose up db valkey

docker-full:
  docker compose up

docker-down:
  docker compose down

docker-build-dev: prepare
  docker build -t open-edc:dev .
