FROM node:22-slim AS admin-builder
WORKDIR /admin
COPY admin/package*.json ./
RUN npm ci
COPY admin/ ./
RUN npm run build

FROM rust:slim-bookworm AS builder
WORKDIR /app

ENV SQLX_OFFLINE=true

RUN apt update && apt install -y libssl-dev pkg-config curl
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
# sqlite3: デプロイの前のバックアップで、動いている DB の写しを作る(VACUUM INTO。.github/workflows/deploy.yml)
RUN apt update && apt install -y libssl-dev pkg-config ca-certificates sqlite3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder ./app/target/release/app ./target/release/app
COPY --from=admin-builder /static/ static/
COPY adapter/migrations/sqlite/ migrations/
ENTRYPOINT ["./target/release/app"]
