ARG open-edc_ENVIRONMENT=dev

FROM python:3.12-slim-bullseye as builder

ENV \
  PYTHONUNBUFFERED=true \
  POETRY_HOME="/opt/poetry" \
  POETRY_NO_INTERACTION=1 \
  POETRY_VIRTUALENVS_CREATE=false \
  VIRTUAL_ENV="/opt/venv"

WORKDIR /open-edc

RUN : \
  && apt-get update \
  && apt-get install -y --no-install-recommends \
  curl \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

RUN python -m venv $VIRTUAL_ENV
ENV PATH="$VIRTUAL_ENV/bin:$PATH"

RUN curl -sSL https://install.python-poetry.org/install-poetry.py | python -
ENV PATH="$POETRY_HOME/bin:$PATH"

COPY pyproject.toml poetry.lock /open-edc/

RUN poetry install --without dev --no-root --no-cache

copy . /open-edc

RUN poetry install --only-root --no-cache


FROM python:3.12-slim-bullseye as prod

WORKDIR /open-edc

ENV PYTHONBUFFERED=true

COPY --from=builder /opt/venv /opt/venv
COPY --from=builder /open-edc/open_edc /open-edc/open_edc

ENV PATH="/opt/venv/bin:$PATH"

EXPOSE 80

ENTRYPOINT ["gunicorn", "open_edc.main:app", "--forwarded-allow-ips='*'", "--access-logfile", "-", "-k", "uvicorn.workers.UvicornWorker", "-b", ":8000"]


FROM builder as dev

ENV PYTHONBUFFERED=true

EXPOSE 8000

ENTRYPOINT ["uvicorn", "open_edc.main:app", "--host", "0.0.0.0", "--port", "8000", "--reload"]
