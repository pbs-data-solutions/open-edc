@build-dev:
  docker build -t open-edc:dev .

@build-prod:
  docker build -t open-edc:latest --target prod .

@dev: && docker-stop
  -docker compose up open-edc db --build

@docker-stop:
  docker compose down

@lint:
  echo mypy
  just --justfile {{justfile()}} mypy
  echo ruff
  just --justfile {{justfile()}} ruff
  echo ruff-format
  just --justfile {{justfile()}} ruff-format

@mypy:
  poetry run mypy .

@ruff:
  poetry run ruff check .

@ruff-format:
  poetry run ruff format open_edc tests

@start-db:
  docker compose up -d db

@test: start-db && docker-stop
  poetry run pytest -x

@test-lf: start-db && docker-stop
  poetry run pytest -x --lf

@test-ci: start-db && docker-stop
  poetry run pytest
