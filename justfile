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

@test:
  poetry run pytest -x
