lint:
  just --justfile {{justfile()}} fmt
  just --justfile {{justfile()}} clippy
  just --justfile {{justfile()}} check

clippy:
  cargo clippy --all-targets

check:
  cargo check --all-targets

fmt:
  cargo fmt --all

test:
  cargo test

dev: db
  cargo watch -x "run -- start"

db:
  docker compose up db -d

stop-db:
  docker compose down db
