FROM rust:1.76-slim-bookworm as builder

WORKDIR /app

ENV \
  SQLX_OFFLINE=true

COPY . /app

RUN cargo build --release

FROM debian:bookworm-slim

ENV \
  CARGO_HOME=/usr/local/cargo \
  RUSTUP_HOME=/usr/local/rustup

WORKDIR /app

COPY --from=builder /app/target/release/open-edc /bin

# RUN : \
#   && apt-get update \
#   && apt-get install -y --no-install-recommends \
#   build-essential \
#   ca-certificates \
#   libssl-dev \
#   pkg-config \
#   && apt-get clean \
#   && rm -rf /var/lib/apt/lists/* \
#   && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

ENTRYPOINT ["/bin/open-edc", "start"]
