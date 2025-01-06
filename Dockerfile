FROM rust:1.83-slim-bookworm AS builder
WORKDIR /app

ARG DATABASE_URL
ENV DATABASE_URL=${DATABASE_URL}

RUN apt update && apt install -y libssl-dev &&  apt install -y pkg-config && apt install -y wget
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt update && apt install -y libssl-dev &&  apt install -y pkg-config && apt install -y wget
WORKDIR /app
RUN adduser admin && chown -R admin /app
USER admin
COPY --from=builder ./app/target/release/app ./target/release/app
COPY static/ static/
#ENV PORT 8080
#EXPOSE $PORT
ENTRYPOINT ["./target/release/app"]
