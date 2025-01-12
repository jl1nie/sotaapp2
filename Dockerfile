FROM rust:1.83-slim-bookworm AS builder
WORKDIR /app

ARG DATABASE_URL
ENV DATABASE_URL=${DATABASE_URL}

RUN apt update && apt install -y libssl-dev &&  apt install -y pkg-config 
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt update && apt install -y libssl-dev &&  apt install -y pkg-config 
WORKDIR /app
RUN adduser admin && chown -R admin /app
USER admin
COPY --from=builder ./app/target/release/app ./target/release/app
COPY static/ static/
ENTRYPOINT ["./target/release/app"]
