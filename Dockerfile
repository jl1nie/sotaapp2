FROM rust:slim-bookworm AS builder
WORKDIR /app

ENV SQLX_OFFLINE=true

RUN apt update && apt install -y libssl-dev &&  apt install -y pkg-config
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt update && apt install -y libssl-dev pkg-config ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder ./app/target/release/app ./target/release/app
COPY static/ static/
COPY adapter/migrations/sqlite/ migrations/
ENTRYPOINT ["./target/release/app"]
